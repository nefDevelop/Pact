use axum::{
    extract::{State, Json, Path, ConnectInfo},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
};
use std::sync::Arc;
use std::net::SocketAddr;
use rand::{RngCore, rngs::OsRng};
use blake2::Blake2bMac512;
use blake2::digest::{KeyInit, Mac};

use crate::AppState;
use crate::db::Database;
use crate::auth::ratelimit::SharedRateLimiter;
use crate::auth::session::SharedSessionManager;
use crate::auth::middleware::AuthenticatedUser;
use crate::storage::StorageManager;
use crate::schemas::{UserRegister, UserLogin, AuthResponse, UsersResponse, UserListItem};
use crate::models::CreateUser;
use crate::error::{AppError, Result};

/// Valida el nombre de usuario. Se usa como clave en la DB y como nombre de
/// archivo dentro de la bóveda, por lo que no puede contener separadores de
/// ruta ni caracteres de control.
fn validate_username(username: &str) -> Result<()> {
    let len = username.chars().count();
    if len == 0 || len > 64 {
        return Err(AppError::BadRequest("El usuario debe tener entre 1 y 64 caracteres".to_string()));
    }
    if username.contains('/') || username.contains('\\') || username.contains("..") {
        return Err(AppError::BadRequest("El usuario contiene caracteres no permitidos".to_string()));
    }
    if username.chars().any(|c| c.is_control()) {
        return Err(AppError::BadRequest("El usuario contiene caracteres no permitidos".to_string()));
    }
    Ok(())
}

/// Construye la clave del rate limiter combinando IP y usuario cuando la IP
/// está disponible. Esto evita que un atacante bloquee a una víctima agotando
/// sus intentos desde otra máquina.
fn rate_key(prefix: &str, username: &str, connect_info: &Option<ConnectInfo<SocketAddr>>) -> String {
    match connect_info {
        Some(ConnectInfo(addr)) => format!("{}:{}:{}", prefix, addr.ip(), username),
        None => format!("{}:{}", prefix, username),
    }
}

/// Clave de rate limit por IP para endpoints pre-login.
fn ip_rate_key(prefix: &str, connect_info: &Option<ConnectInfo<SocketAddr>>) -> String {
    match connect_info {
        Some(ConnectInfo(addr)) => format!("{}:{}", prefix, addr.ip()),
        None => format!("{}:unknown", prefix),
    }
}

/// Salt determinista para usuarios inexistentes. Evita que `get_salt` revele la
/// existencia de una cuenta (anti-enumeración): siempre responde 200 con un
/// salt con el mismo formato que uno real.
fn fake_salt(secret: &str, username: &str) -> String {
    let mut mac = <Blake2bMac512 as KeyInit>::new_from_slice(secret.as_bytes())
        .expect("Blake2b acepta claves de 1 a 64 bytes");
    mac.update(username.as_bytes());
    let digest = mac.finalize().into_bytes();
    hex::encode(&digest[..16])
}

pub async fn get_users(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    State(app): State<AppState>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
) -> Result<impl IntoResponse> {
    if !app.public_limiter.check(&ip_rate_key("users", &connect_info)) {
        return Err(AppError::BadRequest("Demasiadas peticiones. Espera un momento.".to_string()));
    }

    let path = storage.get_path();

    if path.is_none() {
        tracing::debug!("[AUTH] get_users: No hay bóveda vinculada.");
        return Ok(Json(UsersResponse {
            users: vec![],
            source: "local".to_string(),
            vault_path: "".to_string(),
            vault_status: "empty".to_string(),
        }));
    }

    // Solo exponemos el nombre del archivo, no la ruta completa del sistema.
    let vault_name = path.as_ref()
        .and_then(|p| p.file_name())
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let users = db.get_all_users().await?;
    tracing::debug!("[AUTH] get_users: {} usuarios encontrados en DB local.", users.len());

    let users_list = users.into_iter().map(|u| UserListItem {
        id: u.id,
        username: u.username.clone(),
        status: "active".to_string(),
        color: u.color.clone(),
    }).collect();

    Ok(Json(UsersResponse {
        users: users_list,
        source: "local".to_string(),
        vault_path: vault_name,
        vault_status: "ready".to_string(),
    }))
}

pub async fn get_salt(
    State(db): State<Arc<dyn Database>>,
    State(app): State<AppState>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Path(username): Path<String>,
) -> Result<impl IntoResponse> {
    if !app.public_limiter.check(&ip_rate_key("salt", &connect_info)) {
        return Err(AppError::BadRequest("Demasiadas peticiones. Espera un momento.".to_string()));
    }

    // Anti-enumeración: un usuario inexistente devuelve un salt determinista
    // falso (200) indistinguible de uno real, en lugar de 404.
    let salt = match db.get_user_by_username(&username).await? {
        Some(user) => user.salt.clone(),
        None => fake_salt(&app.machine_id, &username),
    };

    Ok(Json(serde_json::json!({ "salt": salt })))
}

pub async fn register(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    State(limiter): State<SharedRateLimiter>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(payload): Json<UserRegister>,
) -> Result<impl IntoResponse> {
    validate_username(&payload.username)?;

    let key = rate_key("register", &payload.username, &connect_info);
    if !limiter.check(&key) {
        return Err(AppError::BadRequest("Demasiados intentos. Espera un minuto.".to_string()));
    }
    let t_total = std::time::Instant::now();
    
    // 1. Verificar si el usuario ya existe
    if db.get_user_by_username(&payload.username).await?.is_some() {
        return Err(AppError::BadRequest("El usuario ya existe".to_string()));
    }

    // 2. Usar salt enviado por el cliente o generar uno nuevo
    let salt_hex = match payload.salt {
        Some(s) => s,
        None => {
            let mut salt_bytes = [0u8; 16];
            OsRng.fill_bytes(&mut salt_bytes);
            hex::encode(salt_bytes)
        }
    };

    // 3. El servidor recibe la 'authKey' del cliente en el campo 'master_password'
    // Aplicamos Argon2id de nuevo para proteger la base de datos
    let t_kdf = std::time::Instant::now();
    let master_hash_bytes = pact_core::crypto::derivar_masterkey(&payload.master_password, salt_hex.as_bytes())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("KDF error: {:?}", e)))?;
    let master_hash = hex::encode(master_hash_bytes);
    tracing::debug!("Argon2id servidor completado en {:?}", t_kdf.elapsed());

    // 4. Guardar en DB
    let t_db = std::time::Instant::now();
    let new_user = CreateUser {
        username: payload.username.clone(),
        master_hash,
        salt: salt_hex,
        color: payload.color.clone(),
    };

    db.create_user(&new_user).await?;
    storage.mark_dirty(&payload.username);
    tracing::debug!("Escritura en DB completada en {:?}", t_db.elapsed());
    tracing::debug!("Registro total completado en {:?}", t_total.elapsed());

    Ok((
        StatusCode::CREATED, 
        Json(AuthResponse { 
            status: "Usuario creado".to_string(),
            username: new_user.username.clone(),
            token: "".to_string(), 
            color: new_user.color.clone(),
            source: "local".to_string(),
        })
    ))
}

pub async fn login(
    State(db): State<Arc<dyn Database>>,
    State(session_mgr): State<SharedSessionManager>,
    State(limiter): State<SharedRateLimiter>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    Json(payload): Json<UserLogin>,
) -> Result<impl IntoResponse> {
    let key = rate_key("login", &payload.username, &connect_info);
    if !limiter.check(&key) {
        return Err(AppError::BadRequest("Demasiados intentos. Espera un minuto.".to_string()));
    }
    let t_total = std::time::Instant::now();
    
    // 1. Buscar usuario
    let t_fetch = std::time::Instant::now();
    let user_opt = db.get_user_by_username(&payload.username).await?;
    tracing::debug!("[🏎️ PERFORMANCE] Búsqueda de usuario en DB: {:?}", t_fetch.elapsed());
    
    // Usamos un flag para indicar si el usuario existe, pero seguimos procesando para evitar timing attacks básicos
    let (authorized, user) = match user_opt {
        Some(u) => {
            let t_kdf = std::time::Instant::now();
            let login_hash_bytes = pact_core::crypto::derivar_masterkey(&payload.master_password, u.salt.as_bytes())
                .map_err(|e| AppError::Internal(anyhow::anyhow!("KDF error: {:?}", e)))?;
            let login_hash = hex::encode(login_hash_bytes);
            tracing::debug!("Argon2id login en servidor: {:?}", t_kdf.elapsed());
            (login_hash == u.master_hash, Some(u))
        },
        None => {
            let _ = pact_core::crypto::derivar_masterkey(&payload.master_password, b"dummy-salt-16bytes")
                .map_err(|e| AppError::Internal(anyhow::anyhow!("KDF error: {:?}", e)))?;
            (false, None)
        }
    };

    if !authorized {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        return Err(AppError::Unauthorized);
    }

    let user = user.unwrap();

    // 2. Crear sesión
    let token = session_mgr.create_session(user.username.clone())
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("No se pudo crear sesión (límite alcanzado)")))?;
    tracing::debug!("Login total completado en {:?}", t_total.elapsed());

    Ok(Json(AuthResponse {
        status: "Login exitoso".to_string(),
        username: user.username.clone(),
        token,
        color: user.color.clone(),
        source: "local".to_string(),
    }))
}

pub async fn unlock(
    state_db: State<Arc<dyn Database>>,
    state_session_mgr: State<SharedSessionManager>,
    state_limiter: State<SharedRateLimiter>,
    connect_info: Option<ConnectInfo<SocketAddr>>,
    json_payload: Json<UserLogin>,
) -> Result<impl IntoResponse> {
    login(state_db, state_session_mgr, state_limiter, connect_info, json_payload).await
}

pub async fn lock(
    headers: HeaderMap,
    State(session_mgr): State<SharedSessionManager>,
) -> impl IntoResponse {
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            let token = auth_str.trim_start_matches("Bearer ").trim();
            session_mgr.remove_session(token);
            return StatusCode::OK;
        }
    }
    StatusCode::BAD_REQUEST
}

pub async fn update_user(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    AuthenticatedUser(_session): AuthenticatedUser,
    Json(payload): Json<crate::schemas::UserUpdate>,
) -> Result<impl IntoResponse> {
    validate_username(&payload.new_name)?;
    db.update_user(&payload.old_name, &payload.new_name, &payload.new_color).await?;

    // La bóveda guarda un `<usuario>.db` por cuenta: hay que renombrarlo para
    // que el usuario siga siendo localizable tras el cambio de nombre.
    if payload.old_name != payload.new_name {
        if let Err(e) = storage.rename_user_db(&payload.old_name, &payload.new_name) {
            tracing::warn!("[AUTH] No se pudo renombrar la DB en la bóveda: {}", e);
        }
    }
    storage.mark_dirty(&payload.new_name);
    Ok(StatusCode::OK)
}

pub async fn delete_user(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    State(session_mgr): State<SharedSessionManager>,
    AuthenticatedUser(_session): AuthenticatedUser,
    Path(username): Path<String>,
    Json(payload): Json<crate::schemas::MasterPasswordOnly>,
) -> Result<impl IntoResponse> {
    // 1. Validar password antes de borrar
    let user = db.get_user_by_username(&username).await?
        .ok_or(AppError::NotFound)?;

    let login_hash_bytes = pact_core::crypto::derivar_masterkey(&payload.master_password, user.salt.as_bytes())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("KDF error: {:?}", e)))?;
    let login_hash = hex::encode(login_hash_bytes);

    if login_hash != user.master_hash {
        return Err(AppError::Unauthorized);
    }

    // 2. Invalidar todas las sesiones de este usuario
    session_mgr.invalidate_user_sessions(&username);

    // 3. Limpiar pending_accounts de este usuario
    let pending = db.get_pending_accounts(&username).await?;
    for p in &pending {
        let _ = db.delete_pending_account(p.id).await;
    }

    // 4. Borrar de la base de datos
    db.delete_user(&username).await?;

    // 5. Eliminar también su DB del contenedor .pcty para no dejarla huérfana.
    if let Err(e) = storage.delete_user_db(&username) {
        tracing::warn!("[AUTH] No se pudo eliminar la DB de '{}' de la bóveda: {}", username, e);
    }

    Ok(StatusCode::OK)
}
