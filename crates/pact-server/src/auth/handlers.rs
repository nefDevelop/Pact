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
        return Ok(Json(UsersResponse {
            users: vec![],
            source: "local".to_string(),
            vault_path: "".to_string(),
            vault_status: "empty".to_string(),
        }));
    }

    let users_list = db.get_all_users().await?.into_iter().map(|u| UserListItem {
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
    let master_hash_bytes = pact_core::crypto::derivar_masterkey(&payload.master_password, salt_hex.as_bytes())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("KDF error: {:?}", e)))?;
    let master_hash = hex::encode(master_hash_bytes);

    // 4. Guardar en DB
    let new_user = CreateUser {
        username: payload.username.clone(),
        master_hash,
        salt: salt_hex,
        color: payload.color.clone(),
    };

    db.create_user(&new_user).await?;

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
    // 1. Buscar usuario
    let user_opt = db.get_user_by_username(&payload.username).await?;
    
    // Usamos un flag para indicar si el usuario existe, pero seguimos procesando para evitar timing attacks básicos
    // Aunque Argon2 ya toma su tiempo, es buena práctica.
    let (authorized, user) = match user_opt {
        Some(u) => {
            let login_hash_bytes = pact_core::crypto::derivar_masterkey(&payload.master_password, u.salt.as_bytes())
                .map_err(|e| AppError::Internal(anyhow::anyhow!("KDF error: {:?}", e)))?;
            let login_hash = hex::encode(login_hash_bytes);
            (login_hash == u.master_hash, Some(u))
        },
        None => {
            // Ejecutamos una derivación ficticia para que el tiempo de respuesta sea similar
            let _ = pact_core::crypto::derivar_masterkey(&payload.master_password, b"dummy-salt-16bytes")
                .map_err(|e| AppError::Internal(anyhow::anyhow!("KDF error: {:?}", e)))?;
            (false, None)
        }
    };

    if !authorized {
        // Retraso artificial para mitigar ataques de fuerza bruta
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        return Err(AppError::Unauthorized);
    }

    let user = user.unwrap();

    // 2. Crear sesión
    let token = session_mgr.create_session(user.username.clone());

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
