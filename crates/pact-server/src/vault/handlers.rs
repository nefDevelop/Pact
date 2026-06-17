use axum::{
    extract::{State, Json, Path, Query, Multipart},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use std::collections::HashMap;
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
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    user_opt: Option<AuthenticatedUser>,
    Json(payload): Json<VaultPath>,
) -> Result<StatusCode> {
    let path = payload.path.map(std::path::PathBuf::from);
    let force_init = payload.force_init.unwrap_or(false);
    tracing::info!("[VAULT] Petición de cambio de bóveda: {:?} (force_init: {})", path, force_init);
    
    let mut is_new = false;

    if let Some(p) = &path {
        if !p.exists() || force_init {
            tracing::info!("[VAULT] Creando nueva bóveda en: {:?}", p);
            storage.init_vault(p).map_err(|e| AppError::BadRequest(format!("No se pudo inicializar la bóveda: {}", e)))?;
            is_new = true;
        } else {
            tracing::info!("[VAULT] Validando bóveda existente en: {:?}", p);
            if !storage.validate_vault(p).map_err(|e| AppError::Internal(anyhow::anyhow!("Error al validar: {}", e)))? {
                tracing::error!("[VAULT] Firma de bóveda no válida en: {:?}", p);
                return Err(AppError::BadRequest("El archivo seleccionado no es una bóveda de Pact válida".to_string()));
            }
            tracing::info!("[VAULT] Bóveda validada correctamente.");
        }
    }
    
    storage.set_path(path.clone());

    if let Some(p) = path {
        tracing::info!("[VAULT] Aplicando cambios en DB local para la ruta: {:?}", p);
        db.reset().await.map_err(|e| AppError::Internal(e.into()))?;

        if !is_new {
            if let Ok(users) = storage.list_users() {
                tracing::info!("[VAULT] Usuarios detectados en bóveda: {:?}", users);
                if let Some(first_user) = users.first() {
                    tracing::info!("[VAULT] Extrayendo base de datos inicial del usuario: {}", first_user);
                    let _ = storage.extract_db(first_user, std::path::Path::new("pact.db"));
                } else {
                    tracing::warn!("[VAULT] Bóveda vacía, no hay usuarios que cargar.");
                }
            }
        }
    } else {
        tracing::warn!("[VAULT] Se ha desvinculado la bóveda actual.");
    }

    if let Some(AuthenticatedUser(session)) = user_opt {
        tracing::info!("[VAULT] Usuario autenticado '{}' detectado durante el cambio.", session.username);
        if is_new {
            tracing::info!("[VAULT] Persistiendo DB actual de '{}' en la nueva bóveda.", session.username);
            let _ = storage.save_db(&session.username, std::path::Path::new("pact.db"));
        } else {
            tracing::info!("[VAULT] Intentando recuperar DB de '{}' de la bóveda cargada.", session.username);
            let _ = storage.extract_db(&session.username, std::path::Path::new("pact.db"));
        }
    }

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

pub async fn update_folder(
    State(db): State<Arc<dyn Database>>,
    Path(id): Path<i32>,
    AuthenticatedUser(session): AuthenticatedUser,
    Json(payload): Json<FolderCreate>,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    // Verificar propiedad (opcional pero recomendado)
    let folders = db.get_folders_by_user(user.id).await?;
    let folder = folders.into_iter().find(|f| f.id == id).ok_or(AppError::NotFound)?;

    let updated = CreateFolder {
        user_id: user.id,
        name: payload.name,
        color: payload.color,
        icon: payload.icon,
        uid: Some(folder.uid),
    };

    db.update_folder(id, &updated, folder.version + 1).await?;
    Ok(StatusCode::OK)
}

pub async fn delete_folder(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    Path(id): Path<i32>,
    AuthenticatedUser(session): AuthenticatedUser,
) -> Result<impl IntoResponse> {
    db.delete_folder(id).await?;
    storage.mark_dirty(&session.username);
    Ok(StatusCode::OK)
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
    storage.mark_dirty(&session.username);

    if let Some(ref seed) = payload.totp_seed {
        let _ = db.create_totp_setting(id, seed, None).await;
    }
    
    Ok((StatusCode::CREATED, Json(id)))
}

pub async fn update_account(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    Path(id): Path<i32>,
    AuthenticatedUser(session): AuthenticatedUser,
    Json(payload): Json<AccountCreate>,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let accounts = db.get_accounts_by_user(user.id).await?;
    let account = accounts.into_iter().find(|a| a.id == id).ok_or(AppError::NotFound)?;

    let updated = CreateAccount {
        user_id: user.id,
        folder_id: payload.folder_id,
        title: payload.title,
        url: payload.url,
        username_site: payload.username_site,
        encrypted_pass: payload.encrypted_pass,
        is_favorite: payload.is_favorite,
        icon_data: payload.icon_data,
        uid: Some(account.uid),
    };

    db.update_account(id, &updated, account.version + 1).await?;
    storage.mark_dirty(&session.username);
    Ok(StatusCode::OK)
}

pub async fn toggle_favorite(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    Path(id): Path<i32>,
    AuthenticatedUser(session): AuthenticatedUser,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let accounts = db.get_accounts_by_user(user.id).await?;
    let account = accounts.into_iter().find(|a| a.id == id).ok_or(AppError::NotFound)?;

    let mut updated: CreateAccount = account.clone().into();
    updated.is_favorite = !account.is_favorite;

    db.update_account(id, &updated, account.version + 1).await?;
    storage.mark_dirty(&session.username);
    
    Ok(Json(serde_json::json!({ "is_favorite": updated.is_favorite })))
}

pub async fn delete_account(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    Path(id): Path<i32>,
    AuthenticatedUser(session): AuthenticatedUser,
) -> Result<impl IntoResponse> {
    db.delete_account(id).await?;
    storage.mark_dirty(&session.username);
    Ok(StatusCode::OK)
}

pub async fn restore_account(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    Path(id): Path<i32>,
    AuthenticatedUser(session): AuthenticatedUser,
) -> Result<impl IntoResponse> {
    db.restore_account(id).await?;
    storage.mark_dirty(&session.username);
    Ok(StatusCode::OK)
}

pub async fn empty_trash(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    AuthenticatedUser(session): AuthenticatedUser,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let count = db.empty_trash(user.id).await?;
    storage.mark_dirty(&session.username);
    Ok(Json(serde_json::json!({ "deleted_count": count })))
}

pub async fn export_vault(
    State(db): State<Arc<dyn Database>>,
    _user: AuthenticatedUser,
    Json(payload): Json<crate::schemas::UserDelete>,
) -> Result<impl IntoResponse> {
    // 1. Validar password
    let user = db.get_user_by_username(&payload.username).await?
        .ok_or(AppError::NotFound)?;

    let login_hash_bytes = pact_core::crypto::derivar_masterkey(&payload.master_password, user.salt.as_bytes())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("KDF error: {:?}", e)))?;
    let login_hash = hex::encode(login_hash_bytes);

    if login_hash != user.master_hash {
        return Err(AppError::Unauthorized);
    }

    // 2. Obtener datos
    let folders = db.get_folders_by_user(user.id).await?;
    let accounts = db.get_accounts_by_user(user.id).await?;

    let export = crate::schemas::VaultExport {
        username: user.username.clone(),
        folders,
        accounts,
    };

    Ok(Json(export))
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

pub async fn upload_vault(
    State(storage): State<Arc<StorageManager>>,
    AuthenticatedUser(_session): AuthenticatedUser,
    mut multipart: Multipart,
) -> Result<impl IntoResponse> {
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        if field.name() == Some("file") {
            let file_name = field.file_name().unwrap_or("uploaded.pcty").to_string();
            let data = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
            let path = std::path::PathBuf::from(&file_name);
            std::fs::write(&path, &data).map_err(|e| AppError::Internal(e.into()))?;
            storage.set_path(Some(path));
            return Ok(StatusCode::OK);
        }
    }
    Err(AppError::BadRequest("No se encontró el archivo en la solicitud".to_string()))
}

pub async fn initialize_vault(
    State(storage): State<Arc<StorageManager>>,
    AuthenticatedUser(_session): AuthenticatedUser,
    Json(payload): Json<HashMap<String, String>>,
) -> Result<impl IntoResponse> {
    let name = payload.get("name").map(|s| s.as_str()).unwrap_or("vault");
    let file_name = format!("{}.pcty", name);
    let path = std::path::PathBuf::from(&file_name);
    storage.init_vault(&path).map_err(|e| AppError::BadRequest(e.to_string()))?;
    storage.set_path(Some(path.clone()));
    Ok(Json(serde_json::json!({ "path": path.to_string_lossy().to_string() })))
}

pub async fn get_totp_code(
    State(db): State<Arc<dyn Database>>,
    AuthenticatedUser(session): AuthenticatedUser,
    Path(id): Path<i32>,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let accounts = db.get_accounts_by_user(user.id).await?;
    if !accounts.iter().any(|a| a.id == id) {
        return Err(AppError::NotFound);
    }

    let setting = db.get_totp_setting_by_account(id).await?
        .ok_or(AppError::NotFound)?;

    let code = crate::vault::totp::get_totp_code(&setting.encrypted_seed);
    Ok(Json(serde_json::json!({ "code": code })))
}

pub async fn update_account_icon(
    State(db): State<Arc<dyn Database>>,
    State(storage): State<Arc<StorageManager>>,
    Path(id): Path<i32>,
    AuthenticatedUser(session): AuthenticatedUser,
    Json(payload): Json<HashMap<String, String>>,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let accounts = db.get_accounts_by_user(user.id).await?;
    let account = accounts.into_iter().find(|a| a.id == id).ok_or(AppError::NotFound)?;
    let icon_data = payload.get("icon_data").cloned().unwrap_or_default();

    let mut updated: CreateAccount = account.clone().into();
    updated.icon_data = Some(icon_data);
    db.update_account(id, &updated, account.version + 1).await?;
    storage.mark_dirty(&session.username);
    Ok(StatusCode::OK)
}

pub async fn proxy_icon(
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse> {
    let url = params.get("url").ok_or(AppError::BadRequest("Falta parámetro url".to_string()))?;

    let parsed = url::Url::parse(url).map_err(|_| AppError::BadRequest("URL inválida".to_string()))?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(AppError::BadRequest("Solo se permiten URLs HTTP/HTTPS".to_string()));
    }
    let host = parsed.host_str().unwrap_or("");
    if host == "localhost" || host == "127.0.0.1" || host == "0.0.0.0" || host.starts_with("10.") || host.starts_with("192.168.") || host.starts_with("172.") {
        return Err(AppError::BadRequest("URL no permitida".to_string()));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| AppError::Internal(e.into()))?;

    let response = client.get(url)
        .send()
        .await
        .map_err(|e| AppError::BadRequest(format!("Error al descargar: {}", e)))?;

    let bytes = response.bytes().await.map_err(|e| AppError::BadRequest(format!("Error al leer: {}", e)))?;
    use base64::Engine;
    let base64_str = base64::engine::general_purpose::STANDARD.encode(&bytes);

    Ok(Json(serde_json::json!({ "base64": base64_str })))
}
