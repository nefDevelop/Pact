use axum::{
    extract::{State, Json},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use crate::db::{Database, MysqlDatabase};
use crate::auth::middleware::AuthenticatedUser;
use crate::schemas::SyncConfig;
use crate::sync::engine::SyncEngine;
use crate::models::SyncConfigModel;
use crate::error::AppError;

/// Extrae la subclave de sincronización del cliente (hex, 32 bytes).
fn sync_key_from_headers(headers: &HeaderMap) -> Result<Vec<u8>, AppError> {
    let hex_key = headers
        .get("x-pact-sync-key")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| AppError::BadRequest("Falta la clave de sincronización".to_string()))?;

    let key = hex::decode(hex_key)
        .map_err(|_| AppError::BadRequest("Clave de sincronización inválida".to_string()))?;

    if key.len() != 32 {
        return Err(AppError::BadRequest("Clave de sincronización inválida".to_string()));
    }
    Ok(key)
}

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

/// Carga la configuración remota de la DB y descifra la contraseña con la clave
/// aportada por el cliente en la cabecera `X-Pact-Sync-Key`.
async fn load_remote_config(
    local_db: &dyn Database,
    user_id: i32,
    headers: &HeaderMap,
) -> Result<SyncConfig, AppError> {
    let cfg = local_db.get_sync_config(user_id).await?
        .ok_or_else(|| AppError::BadRequest("Configuración de sincronización faltante".to_string()))?;

    let key = sync_key_from_headers(headers)?;
    let password = crate::crypto::decrypt_with_key(&cfg.remote_pass, &key)
        .map_err(|_| AppError::BadRequest("No se pudo descifrar la configuración remota".to_string()))?;

    Ok(SyncConfig {
        host: cfg.remote_host,
        user: cfg.remote_user,
        password,
        db_name: cfg.remote_name,
        port: Some(cfg.remote_port as u16),
        preferred_mode: cfg.preferred_mode,
    })
}

pub async fn run_sync(
    State(local_db): State<Arc<dyn Database>>,
    AuthenticatedUser(session): AuthenticatedUser,
    headers: HeaderMap,
    payload: Option<Json<SyncConfig>>,
) -> Result<impl IntoResponse, AppError> {
    let user = local_db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    // Si el cliente envía la config explícita, se usa tal cual; si no, se lee de
    // la DB y se descifra con la clave en tránsito.
    let config = match payload {
        Some(Json(c)) => c,
        None => load_remote_config(local_db.as_ref(), user.id, &headers).await?,
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

    // No tener configuración remota es el estado inicial normal, no un error.
    // Devolvemos `200` con cuerpo `null` para que el cliente no trate la
    // ausencia de config como un fallo (y no ensucie la consola con 404).
    let config = local_db.get_sync_config(user.id).await?.map(|cfg| SyncConfig {
        host: cfg.remote_host,
        user: cfg.remote_user,
        password: String::new(),
        db_name: cfg.remote_name,
        port: Some(cfg.remote_port as u16),
        preferred_mode: cfg.preferred_mode,
    });

    Ok(Json(config).into_response())
}

pub async fn set_sync_config(
    State(local_db): State<Arc<dyn Database>>,
    AuthenticatedUser(session): AuthenticatedUser,
    Json(payload): Json<SyncConfig>,
) -> Result<impl IntoResponse, AppError> {
    let user = local_db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let existing = local_db.get_sync_config(user.id).await?;

    // El cliente envía la contraseña ya cifrada E2E. Si viene vacía, se conserva
    // la existente (no se borra al guardar sin reescribirla). Si no hay host, se
    // limpia (desvinculación).
    let remote_pass = if payload.host.trim().is_empty() {
        String::new()
    } else if payload.password.trim().is_empty() {
        existing.map(|c| c.remote_pass).unwrap_or_default()
    } else {
        payload.password
    };

    let model = SyncConfigModel {
        user_id: user.id,
        remote_host: payload.host,
        remote_port: payload.port.unwrap_or(3306) as i32,
        remote_user: payload.user,
        remote_pass,
        remote_name: payload.db_name,
        preferred_mode: payload.preferred_mode,
    };

    local_db.set_sync_config(user.id, &model).await?;
    Ok(StatusCode::OK)
}

pub async fn apply_sync(
    State(local_db): State<Arc<dyn Database>>,
    AuthenticatedUser(session): AuthenticatedUser,
    headers: HeaderMap,
    Json(plan): Json<crate::schemas::SyncPlan>,
) -> Result<impl IntoResponse, AppError> {
    let user = local_db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let config = load_remote_config(local_db.as_ref(), user.id, &headers).await?;
    let remote_db = connect_remote(&config).await?;

    let engine = SyncEngine::new(local_db.as_ref(), &remote_db, user.id);
    engine.apply_plan(&plan).await?;
    Ok(StatusCode::OK)
}

pub async fn test_sync_connection(
    AuthenticatedUser(_session): AuthenticatedUser,
    Json(payload): Json<SyncConfig>,
) -> Result<StatusCode, AppError> {
    // Prueba efímera: la contraseña llega en claro y no se persiste.
    connect_remote(&payload).await?;
    Ok(StatusCode::OK)
}

pub async fn check_sync(
    State(local_db): State<Arc<dyn Database>>,
    AuthenticatedUser(session): AuthenticatedUser,
    headers: HeaderMap,
) -> impl IntoResponse {
    let user = match local_db.get_user_by_username(&session.username).await {
        Ok(Some(u)) => u,
        _ => return Json(serde_json::json!({ "has_changes": false })),
    };

    let config = match load_remote_config(local_db.as_ref(), user.id, &headers).await {
        Ok(c) => c,
        Err(_) => return Json(serde_json::json!({ "has_changes": false })),
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
