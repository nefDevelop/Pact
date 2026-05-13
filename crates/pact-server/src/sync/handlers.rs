use axum::{
    extract::{State, Json},
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
};
use std::sync::Arc;
use crate::db::{Database, MysqlDatabase};
use crate::auth::session::SharedSessionManager;
use crate::schemas::{SyncConfig, ErrorResponse};
use crate::sync::engine::SyncEngine;
use crate::models::SyncConfigModel;

// Helper para obtener el username de la sesión
fn get_username(headers: &HeaderMap, session_mgr: &SharedSessionManager) -> Option<String> {
    let auth_header = headers.get("Authorization")?.to_str().ok()?;
    let token = auth_header.trim_start_matches("Bearer ").trim();
    session_mgr.get_session(token).map(|s| s.username)
}

pub async fn run_sync(
    State(local_db): State<Arc<dyn Database>>,
    State(session_mgr): State<SharedSessionManager>,
    headers: HeaderMap,
    payload: Option<Json<SyncConfig>>,
) -> impl IntoResponse {
    let username = match get_username(&headers, &session_mgr) {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "No autorizado".to_string() })).into_response(),
    };

    let user = match local_db.get_user_by_username(&username).await {
        Ok(Some(u)) => u,
        _ => return (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "Usuario no encontrado".to_string() })).into_response(),
    };

    // Obtener config de la DB si no se proporcionó en el payload
    let config = if let Some(Json(c)) = payload {
        c
    } else {
        match local_db.get_sync_config(user.id).await {
            Ok(Some(cfg)) => SyncConfig {
                host: cfg.remote_host,
                user: cfg.remote_user,
                password: cfg.remote_pass,
                db_name: cfg.remote_name,
                port: Some(cfg.remote_port as u16),
                preferred_mode: cfg.preferred_mode,
            },
            _ => return (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: "Configuración de sincronización faltante".to_string() })).into_response(),
        }
    };

    // Conectar a la DB remota
    let remote_url = format!("mysql://{}:{}@{}:{}/{}", config.user, config.password, config.host, config.port.unwrap_or(3306), config.db_name);
    let remote_db = match MysqlDatabase::new(&remote_url).await {
        Ok(db) => db,
        Err(e) => return (StatusCode::BAD_REQUEST, Json(ErrorResponse { error: format!("Error de conexión remota: {}", e) })).into_response(),
    };

    let engine = SyncEngine::new(local_db.as_ref(), &remote_db, user.id);

    // 1. Generar plan
    let plan = match engine.generate_plan().await {
        Ok(p) => p,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("Error generando plan: {}", e) })).into_response(),
    };

    // 2. Aplicar plan
    match engine.apply_plan(&plan).await {
        Ok(_) => (StatusCode::OK, Json(plan)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: format!("Error aplicando plan: {}", e) })).into_response(),
    }
}

pub async fn get_sync_config(
    State(local_db): State<Arc<dyn Database>>,
    State(session_mgr): State<SharedSessionManager>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let username = match get_username(&headers, &session_mgr) {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "No autorizado".to_string() })).into_response(),
    };

    let user = match local_db.get_user_by_username(&username).await {
        Ok(Some(u)) => u,
        _ => return (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "Usuario no encontrado".to_string() })).into_response(),
    };

    match local_db.get_sync_config(user.id).await {
        Ok(Some(cfg)) => Json(SyncConfig {
            host: cfg.remote_host,
            user: cfg.remote_user,
            password: "".to_string(), // No devolver password por seguridad
            db_name: cfg.remote_name,
            port: Some(cfg.remote_port as u16),
            preferred_mode: cfg.preferred_mode,
        }).into_response(),
        Ok(None) => StatusCode::NOT_FOUND.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn set_sync_config(
    State(local_db): State<Arc<dyn Database>>,
    State(session_mgr): State<SharedSessionManager>,
    headers: HeaderMap,
    Json(payload): Json<SyncConfig>,
) -> impl IntoResponse {
    let username = match get_username(&headers, &session_mgr) {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "No autorizado".to_string() })).into_response(),
    };

    let user = match local_db.get_user_by_username(&username).await {
        Ok(Some(u)) => u,
        _ => return (StatusCode::NOT_FOUND, Json(ErrorResponse { error: "Usuario no encontrado".to_string() })).into_response(),
    };

    let model = SyncConfigModel {
        user_id: user.id,
        remote_host: payload.host,
        remote_port: payload.port.unwrap_or(3306) as i32,
        remote_user: payload.user,
        remote_pass: payload.password,
        remote_name: payload.db_name,
        preferred_mode: payload.preferred_mode,
    };

    match local_db.set_sync_config(user.id, &model).await {
        Ok(_) => StatusCode::OK.into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

pub async fn apply_sync() -> impl IntoResponse {
    // Por ahora run_sync hace ambas cosas (plan + apply)
    StatusCode::OK
}

pub async fn check_sync(
    State(local_db): State<Arc<dyn Database>>,
    State(session_mgr): State<SharedSessionManager>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let username = match get_username(&headers, &session_mgr) {
        Some(u) => u,
        None => return (StatusCode::UNAUTHORIZED, "No autorizado".to_string()).into_response(),
    };

    let user = match local_db.get_user_by_username(&username).await {
        Ok(Some(u)) => u,
        _ => return (StatusCode::NOT_FOUND, "Usuario no encontrado".to_string()).into_response(),
    };

    // Obtener config
    let cfg = match local_db.get_sync_config(user.id).await {
        Ok(Some(c)) => c,
        _ => return Json(serde_json::json!({ "has_changes": false })).into_response(),
    };

    // Conectar a remota
    let remote_url = format!("mysql://{}:{}@{}:{}/{}", cfg.remote_user, cfg.remote_pass, cfg.remote_host, cfg.remote_port, cfg.remote_name);
    let remote_db = match MysqlDatabase::new(&remote_url).await {
        Ok(db) => db,
        Err(_) => return Json(serde_json::json!({ "has_changes": false })).into_response(),
    };

    let engine = SyncEngine::new(local_db.as_ref(), &remote_db, user.id);
    let plan = match engine.generate_plan().await {
        Ok(p) => p,
        Err(_) => return Json(serde_json::json!({ "has_changes": false })).into_response(),
    };

    let has_changes = !plan.to_pull_folders.is_empty() || !plan.to_pull_accounts.is_empty();
    
    Json(serde_json::json!({ "has_changes": has_changes })).into_response()
}
