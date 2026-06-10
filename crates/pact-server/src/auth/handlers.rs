use axum::{
    extract::{State, Json, Path},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
};
use std::sync::Arc;
use rand::{RngCore, rngs::OsRng};

use crate::db::Database;
use crate::auth::session::SharedSessionManager;
use crate::storage::StorageManager;
use crate::schemas::{UserRegister, UserLogin, AuthResponse, UsersResponse, UserListItem};
use crate::models::CreateUser;
use crate::error::{AppError, Result};

pub async fn get_users(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
) -> Result<impl IntoResponse> {
    let path = storage.get_path()
        .map(|p| p.to_string_lossy().to_string());

    if path.is_none() {
        tracing::debug!("[AUTH] get_users: No hay bóveda vinculada.");
        return Ok(Json(UsersResponse {
            users: vec![],
            source: "local".to_string(),
            vault_path: "".to_string(),
            vault_status: "empty".to_string(),
        }));
    }

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
        vault_path: path.unwrap_or_default(),
        vault_status: "ready".to_string(),
    }))
}

pub async fn get_salt(
    State(db): State<Arc<dyn Database>>,
    Path(username): Path<String>,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&username).await?
        .ok_or(AppError::NotFound)?;
    
    Ok(Json(serde_json::json!({ "salt": user.salt.clone() })))
}

pub async fn register(
    State(db): State<Arc<dyn Database>>,
    Json(payload): Json<UserRegister>,
) -> Result<impl IntoResponse> {
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
    println!("[🏎️ PERFORMANCE] Argon2id servidor completado en {:?}", t_kdf.elapsed());

    // 4. Guardar en DB
    let t_db = std::time::Instant::now();
    let new_user = CreateUser {
        username: payload.username.clone(),
        master_hash,
        salt: salt_hex,
        color: payload.color.clone(),
    };

    db.create_user(&new_user).await?;
    println!("[🏎️ PERFORMANCE] Escritura en DB completada en {:?}", t_db.elapsed());
    println!("[🏎️ PERFORMANCE] Registro total completado en {:?}", t_total.elapsed());

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
    Json(payload): Json<UserLogin>,
) -> Result<impl IntoResponse> {
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
            println!("[🏎️ PERFORMANCE] Argon2id login en servidor: {:?}", t_kdf.elapsed());
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
    let token = session_mgr.create_session(user.username.clone());
    println!("[🏎️ PERFORMANCE] Login total completado en {:?}", t_total.elapsed());

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
    json_payload: Json<UserLogin>,
) -> Result<impl IntoResponse> {
    login(state_db, state_session_mgr, json_payload).await
}

pub async fn lock(
    headers: HeaderMap,
    State(session_mgr): State<SharedSessionManager>,
) -> impl IntoResponse {
    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            let token = auth_str.trim_start_matches("Bearer ").trim();
            session_mgr.remove_session(token);
        }
    }
    StatusCode::OK
}

pub async fn update_user(
    State(db): State<Arc<dyn Database>>,
    Json(payload): Json<crate::schemas::UserUpdate>,
) -> Result<impl IntoResponse> {
    db.update_user(&payload.old_name, &payload.new_name, &payload.new_color).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_user(
    State(db): State<Arc<dyn Database>>,
    State(_session_mgr): State<SharedSessionManager>,
    Path(username): Path<String>,
    Json(payload): Json<crate::schemas::UserDelete>,
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

    // 2. Borrar de la base de datos
    db.delete_user(&username).await?;
    
    // 3. (Opcional) Invalidar todas las sesiones de este usuario
    // session_mgr no soporta invalidación por usuario actualmente, 
    // pero al menos el usuario ya no podrá loguearse.

    Ok(StatusCode::OK)
}
