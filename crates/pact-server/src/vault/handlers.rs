use axum::{
    extract::{State, Json, Path},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use crate::db::Database;
use crate::storage::StorageManager;
use crate::schemas::{FolderCreate, AccountCreate, PendingAccount, VaultPath};
use crate::models::{CreateFolder, CreateAccount};
use crate::auth::middleware::AuthenticatedUser;
use crate::error::{AppError, Result};

pub async fn get_vault_path(
    State(storage): State<Arc<StorageManager>>,
) -> impl IntoResponse {
    let path = storage.get_path();
    Json(serde_json::json!({ 
        "path": path.map(|p| p.to_string_lossy().to_string()) 
    }))
}

pub async fn set_vault_path(
    State(storage): State<Arc<StorageManager>>,
    Json(payload): Json<VaultPath>,
) -> Result<StatusCode> {
    let path = payload.path.map(std::path::PathBuf::from);
    tracing::info!("Recibida petición para cambiar boveda a: {:?}", path);
    
    if let Some(p) = &path {
        if !p.exists() {
            tracing::info!("El archivo no existe, inicializando boveda en {:?}", p);
            storage.init_vault(p).map_err(|e| AppError::BadRequest(format!("No se pudo inicializar la bóveda: {}", e)))?;
        } else {
            tracing::info!("El archivo ya existe, validando integridad...");
            if !storage.validate_vault(p).map_err(|e| AppError::Internal(anyhow::anyhow!("Error al validar: {}", e)))? {
                return Err(AppError::BadRequest("El archivo seleccionado no es una bóveda de Pact válida".to_string()));
            }
            tracing::info!("Bóveda validada correctamente.");
        }
    }
    
    storage.set_path(path);
    Ok(StatusCode::OK)
}

// --- FOLDERS ---

pub async fn get_folders(
    State(db): State<Arc<dyn Database>>,
    AuthenticatedUser(session): AuthenticatedUser,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let folders = db.get_folders_by_user(user.id).await?;
    Ok(Json(folders))
}

pub async fn create_folder(
    State(db): State<Arc<dyn Database>>,
    AuthenticatedUser(session): AuthenticatedUser,
    Json(payload): Json<FolderCreate>,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    tracing::debug!("Creando carpeta '{}' para usuario {}", payload.name, session.username);

    let new_folder = CreateFolder {
        user_id: user.id,
        name: payload.name.clone(),
        color: payload.color.clone(),
        icon: payload.icon.clone(),
        uid: None,
    };

    let id = db.create_folder(&new_folder).await?;
    Ok((StatusCode::CREATED, Json(id)))
}

// --- ACCOUNTS ---

pub async fn get_accounts(
    State(db): State<Arc<dyn Database>>,
    AuthenticatedUser(session): AuthenticatedUser,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let accounts = db.get_accounts_by_user(user.id).await?;
    Ok(Json(accounts))
}

pub async fn create_account(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    AuthenticatedUser(session): AuthenticatedUser,
    Json(payload): Json<AccountCreate>,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    tracing::debug!("Creando cuenta '{}' en carpeta ID {:?}", payload.title, payload.folder_id);

    let new_acc = CreateAccount {
        user_id: user.id,
        folder_id: payload.folder_id,
        title: payload.title.clone(),
        url: payload.url.clone(),
        username_site: payload.username_site.clone(),
        encrypted_pass: payload.encrypted_pass.clone(),
        is_favorite: payload.is_favorite,
        icon_data: payload.icon_data.clone(),
        uid: None,
    };

    let id = db.create_account(&new_acc).await?;
    let _ = storage.save_db(&session.username, std::path::Path::new("pact.db"));
    
    Ok((StatusCode::CREATED, Json(id)))
}

// --- PENDING ACCOUNTS (SECURE INBOX) ---

pub async fn get_pending_entries(
    State(db): State<Arc<dyn Database>>,
    AuthenticatedUser(session): AuthenticatedUser,
) -> Result<impl IntoResponse> {
    let accounts = db.get_pending_accounts(&session.username).await?;
    let res: Vec<PendingAccount> = accounts.into_iter().map(|a| PendingAccount {
        id: a.id,
        username: a.username.clone(),
        data_blob: a.data_blob.clone(),
    }).collect();
    
    Ok(Json(res))
}

pub async fn delete_pending_entry(
    State(db): State<Arc<dyn Database>>,
    Path(id): Path<i32>,
    _user: AuthenticatedUser, 
) -> Result<impl IntoResponse> {
    db.delete_pending_account(id).await?;
    Ok(StatusCode::OK)
}

pub async fn get_pending_count(
    State(db): State<Arc<dyn Database>>,
    Path(username): Path<String>,
) -> Result<impl IntoResponse> {
    let count = db.get_pending_count(&username).await?;
    Ok(Json(serde_json::json!({ "count": count })))
}
