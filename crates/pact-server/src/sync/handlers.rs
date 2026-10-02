use axum::{
    extract::{State, Json},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use crate::crypto::SyncCrypto;
use crate::db::{Database, MysqlDatabase};
use crate::auth::middleware::AuthenticatedUser;
use crate::schemas::SyncConfig;
use crate::sync::engine::SyncEngine;
use crate::models::SyncConfigModel;
use crate::error::AppError;

/// Conecta con la base de datos remota sin exponer la URL (que contiene la
/// contraseña) en los mensajes de error hacia el cliente.
async fn connect_remote(config: &SyncConfig) -> Result<MysqlDatabase, AppError> {
    let remote_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        config.user,
        config.password,
        config.host,
        config.port.unwrap_or(3306),
        config.db_name
    );
    MysqlDatabase::new(&remote_url).await.map_err(|e| {
        tracing::warn!("[SYNC] Error de conexión remota: {}", e);
        AppError::BadRequest("No se pudo conectar con la base de datos remota".to_string())
    })
}

pub async fn run_sync(
    State(local_db): State<Arc<dyn Database>>,
    State(sync_crypto): State<Arc<SyncCrypto>>,
    AuthenticatedUser(session): AuthenticatedUser,
    payload: Option<Json<SyncConfig>>,
) -> Result<impl IntoResponse, AppError> {
    let user = local_db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    // Obtener config de la DB si no se proporcionó en el payload
    let config = if let Some(Json(c)) = payload {
        c
    } else {
        let cfg = local_db.get_sync_config(user.id).await?
            .ok_or_else(|| AppError::BadRequest("Configuración de sincronización faltante".to_string()))?;
        SyncConfig {
            host: cfg.remote_host,
            user: cfg.remote_user,
            password: sync_crypto.decrypt(&cfg.remote_pass),
            db_name: cfg.remote_name,
            port: Some(cfg.remote_port as u16),
            preferred_mode: cfg.preferred_mode,
        }
    };

    let remote_db = connect_remote(&config).await?;
    let engine = SyncEngine::new(local_db.as_ref(), &remote_db, user.id);

    // Calculamos el plan de reconciliación sin aplicarlo. La aplicación se
    // solicita explícitamente con POST /sync/apply para que el cliente pueda
    // revisar los cambios (y eventuales conflictos) antes de confirmarlos.
    let plan = engine.generate_plan().await?;
    Ok((StatusCode::OK, Json(plan)))
}

pub async fn get_sync_config(
    State(local_db): State<Arc<dyn Database>>,
    AuthenticatedUser(session): AuthenticatedUser,
) -> Result<Response, AppError> {
    let user = local_db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    match local_db.get_sync_config(user.id).await? {
        Some(cfg) => Ok(Json(SyncConfig {
            host: cfg.remote_host,
            user: cfg.remote_user,
            password: String::new(),
            db_name: cfg.remote_name,
            port: Some(cfg.remote_port as u16),
            preferred_mode: cfg.preferred_mode,
        }).into_response()),
        None => Err(AppError::NotFound),
    }
}

pub async fn set_sync_config(
    State(local_db): State<Arc<dyn Database>>,
    State(sync_crypto): State<Arc<SyncCrypto>>,
    AuthenticatedUser(session): AuthenticatedUser,
    Json(payload): Json<SyncConfig>,
) -> Result<impl IntoResponse, AppError> {
    let user = local_db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let model = SyncConfigModel {
        user_id: user.id,
        remote_host: payload.host,
        remote_port: payload.port.unwrap_or(3306) as i32,
        remote_user: payload.user,
        remote_pass: sync_crypto.encrypt(&payload.password),
        remote_name: payload.db_name,
        preferred_mode: payload.preferred_mode,
    };

    local_db.set_sync_config(user.id, &model).await?;
    Ok(StatusCode::OK)
}

pub async fn apply_sync(
    State(local_db): State<Arc<dyn Database>>,
    State(sync_crypto): State<Arc<SyncCrypto>>,
    AuthenticatedUser(session): AuthenticatedUser,
    Json(plan): Json<crate::schemas::SyncPlan>,
) -> Result<impl IntoResponse, AppError> {
    let user = local_db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let cfg = local_db.get_sync_config(user.id).await?
        .ok_or_else(|| AppError::BadRequest("Configuración de sincronización faltante".to_string()))?;

    let config = SyncConfig {
        host: cfg.remote_host,
        user: cfg.remote_user,
        password: sync_crypto.decrypt(&cfg.remote_pass),
        db_name: cfg.remote_name,
        port: Some(cfg.remote_port as u16),
        preferred_mode: cfg.preferred_mode,
    };
    let remote_db = connect_remote(&config).await?;

    let engine = SyncEngine::new(local_db.as_ref(), &remote_db, user.id);
    engine.apply_plan(&plan).await?;
    Ok(StatusCode::OK)
}

pub async fn test_sync_connection(
    AuthenticatedUser(_session): AuthenticatedUser,
    Json(payload): Json<SyncConfig>,
) -> Result<StatusCode, AppError> {
    connect_remote(&payload).await?;
    Ok(StatusCode::OK)
}

pub async fn check_sync(
    State(local_db): State<Arc<dyn Database>>,
    State(sync_crypto): State<Arc<SyncCrypto>>,
    AuthenticatedUser(session): AuthenticatedUser,
) -> impl IntoResponse {
    let user = match local_db.get_user_by_username(&session.username).await {
        Ok(Some(u)) => u,
        _ => return Json(serde_json::json!({ "has_changes": false })),
    };

    // Obtener config
    let cfg = match local_db.get_sync_config(user.id).await {
        Ok(Some(c)) => c,
        _ => return Json(serde_json::json!({ "has_changes": false })),
    };

    let config = SyncConfig {
        host: cfg.remote_host,
        user: cfg.remote_user,
        password: sync_crypto.decrypt(&cfg.remote_pass),
        db_name: cfg.remote_name,
        port: Some(cfg.remote_port as u16),
        preferred_mode: cfg.preferred_mode,
    };
    let remote_db = match connect_remote(&config).await {
        Ok(db) => db,
        Err(_) => return Json(serde_json::json!({ "has_changes": false })),
    };

    let engine = SyncEngine::new(local_db.as_ref(), &remote_db, user.id);
    let plan = match engine.generate_plan().await {
        Ok(p) => p,
        Err(_) => return Json(serde_json::json!({ "has_changes": false })),
    };

    let has_changes = !plan.to_pull_folders.is_empty() || !plan.to_pull_accounts.is_empty();
    Json(serde_json::json!({ "has_changes": has_changes }))
}
