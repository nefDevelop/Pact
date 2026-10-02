use axum::{
    extract::{State, Json, Path, Query, Multipart},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;
use std::collections::HashMap;
use std::net::IpAddr;
use crate::db::Database;
use crate::storage::StorageManager;
use crate::schemas::{FolderCreate, AccountCreate, AccountQuery, PendingAccount, VaultPath};
use crate::models::{CreateFolder, CreateAccount};
use crate::auth::middleware::AuthenticatedUser;
use crate::error::{AppError, Result};

const MAX_TITLE_LEN: usize = 200;
// Los nombres de carpeta llegan cifrados E2E, por lo que el límite es holgado
// (el servidor no puede medir la longitud del texto plano).
const MAX_NAME_LEN: usize = 2048;
const MAX_ICON_BYTES: usize = 1024 * 1024; // 1 MB

/// Comprueba si una IP resuelta apunta a un rango privado, loopback o de
/// enlace local. Se usa para mitigar SSRF en el proxy de iconos.
fn is_forbidden_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || (octets[0] == 100 && (64..128).contains(&octets[1])) // CGNAT 100.64.0.0/10
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_unique_local()
                || v6.is_unicast_link_local()
        }
    }
}

pub async fn get_vault_path(
    State(storage): State<Arc<StorageManager>>,
    _user: AuthenticatedUser,
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

    // El primer arranque (sin bóveda vinculada) se permite sin sesión para poder
    // crear/abrir la bóveda inicial. Cambiar de bóveda cuando ya hay usuarios
    // (lo que resetea la DB local) exige estar autenticado.
    if storage.get_path().is_some() {
        let has_users = !db.get_all_users().await.map_err(|e| AppError::Internal(e.into()))?.is_empty();
        if has_users && user_opt.is_none() {
            return Err(AppError::Unauthorized);
        }
    }

    let mut is_new = false;

    if let Some(p) = &path {
        // Solo aceptamos archivos .pcty para evitar apuntar a rutas arbitrarias.
        match p.extension().and_then(|e| e.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case("pcty") => {}
            _ => return Err(AppError::BadRequest("La bóveda debe tener extensión .pcty".to_string())),
        }

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

    if path.is_some() {
        tracing::info!("[VAULT] Paso 1: Ruta fijada, reseteando DB local");
        db.reset().await.map_err(|e| AppError::Internal(e.into()))?;

        if !is_new {
            tracing::info!("[VAULT] Paso 2: Listando usuarios en la bóveda...");
            match storage.list_users() {
                Ok(users) => {
                    tracing::info!("[VAULT] Usuarios encontrados en bóveda: {:?} (total: {})", users, users.len());
                    if let Some(first_user) = users.first() {
                        tracing::info!("[VAULT] Paso 3: Extrayendo DB del usuario '{}' a pact.db", first_user);
                        match storage.extract_db(first_user, std::path::Path::new("pact.db")) {
                            Ok(true) => tracing::info!("[VAULT] DB de '{}' extraída correctamente a pact.db", first_user),
                            Ok(false) => tracing::warn!("[VAULT] No se encontró archivo {}.db dentro de la bóveda", first_user),
                            Err(e) => tracing::error!("[VAULT] Error extrayendo DB de '{}': {:?}", first_user, e),
                        }
                    } else {
                        tracing::warn!("[VAULT] Bóveda vacía, no hay usuarios que cargar.");
                    }
                }
                Err(e) => {
                    tracing::error!("[VAULT] Error al listar usuarios: {:?}", e);
                }
            }
        } else {
            tracing::info!("[VAULT] Bóveda nueva, no se listan usuarios.");
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

    let name = payload.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_LEN {
        return Err(AppError::BadRequest(format!("El nombre de la carpeta debe tener entre 1 y {} caracteres", MAX_NAME_LEN)));
    }

    tracing::debug!("Creando carpeta '{}' para usuario {}", name, session.username);

    let new_folder = CreateFolder {
        user_id: user.id,
        name: name.to_string(),
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

    // Verificar propiedad
    let folders = db.get_folders_by_user(user.id).await?;
    let folder = folders.into_iter().find(|f| f.id == id).ok_or(AppError::NotFound)?;

    let name = payload.name.trim();
    if name.is_empty() || name.chars().count() > MAX_NAME_LEN {
        return Err(AppError::BadRequest(format!("El nombre de la carpeta debe tener entre 1 y {} caracteres", MAX_NAME_LEN)));
    }

    let updated = CreateFolder {
        user_id: user.id,
        name: name.to_string(),
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
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    // Verificar propiedad: la carpeta debe pertenecer al usuario autenticado.
    if !db.get_folders_by_user(user.id).await?.iter().any(|f| f.id == id) {
        return Err(AppError::NotFound);
    }

    db.delete_folder(id).await?;
    storage.mark_dirty(&session.username);
    Ok(StatusCode::OK)
}

// --- ACCOUNTS ---

pub async fn get_accounts(
    State(db): State<Arc<dyn Database>>,
    AuthenticatedUser(session): AuthenticatedUser,
    Query(query): Query<AccountQuery>,
) -> Result<impl IntoResponse> {
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    let mut accounts = db.get_accounts_by_user(user.id).await?;

    // Por defecto se ocultan las cuentas en papelera; el cliente pide
    // `include_deleted=true` cuando navega a la vista de papelera.
    if !query.include_deleted {
        accounts.retain(|a| a.deleted_at.is_none());
    }
    if let Some(favorite) = query.is_favorite {
        accounts.retain(|a| a.is_favorite == favorite);
    }
    if let Some(folder_id) = query.folder_id {
        accounts.retain(|a| a.folder_id == Some(folder_id));
    }

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

    if payload.title.trim().is_empty() || payload.title.chars().count() > MAX_TITLE_LEN {
        return Err(AppError::BadRequest(format!("El título debe tener entre 1 y {} caracteres", MAX_TITLE_LEN)));
    }
    if payload.encrypted_pass.is_empty() {
        return Err(AppError::BadRequest("La contraseña cifrada es obligatoria".to_string()));
    }

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

    // El seed TOTP llega ya cifrado E2E con la MasterKey del cliente. El
    // servidor solo lo almacena como blob opaco y nunca puede generar códigos.
    if let Some(seed) = payload.totp_seed.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if let Err(e) = db.create_totp_setting(id, seed, None).await {
            tracing::warn!("[VAULT] No se pudo guardar el seed TOTP de la cuenta {}: {}", id, e);
        }
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

    if payload.title.trim().is_empty() || payload.title.chars().count() > MAX_TITLE_LEN {
        return Err(AppError::BadRequest(format!("El título debe tener entre 1 y {} caracteres", MAX_TITLE_LEN)));
    }
    if payload.encrypted_pass.is_empty() {
        return Err(AppError::BadRequest("La contraseña cifrada es obligatoria".to_string()));
    }

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
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    // Verificar propiedad: la cuenta debe pertenecer al usuario autenticado.
    if !db.get_accounts_by_user(user.id).await?.iter().any(|a| a.id == id) {
        return Err(AppError::NotFound);
    }

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
    let user = db.get_user_by_username(&session.username).await?
        .ok_or(AppError::NotFound)?;

    if !db.get_accounts_by_user(user.id).await?.iter().any(|a| a.id == id) {
        return Err(AppError::NotFound);
    }

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
    AuthenticatedUser(session): AuthenticatedUser,
) -> Result<impl IntoResponse> {
    // Verificar que la entrada pendiente pertenece al usuario autenticado.
    let owns_entry = db.get_pending_accounts(&session.username).await?
        .iter().any(|p| p.id == id);
    if !owns_entry {
        return Err(AppError::NotFound);
    }

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
            let raw_name = field.file_name().unwrap_or("uploaded.pcty");
            // Nos quedamos solo con el nombre base para evitar path traversal.
            let file_name = std::path::Path::new(raw_name)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("uploaded.pcty")
                .to_string();

            if !file_name.to_ascii_lowercase().ends_with(".pcty") {
                return Err(AppError::BadRequest("El archivo debe ser una bóveda .pcty".to_string()));
            }

            let data = field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
            let path = std::path::PathBuf::from(&file_name);
            std::fs::write(&path, &data).map_err(|e| AppError::Internal(e.into()))?;

            // Validar la firma antes de vincular la bóveda.
            if !storage.validate_vault(&path).map_err(|e| AppError::Internal(e.into()))? {
                let _ = std::fs::remove_file(&path);
                return Err(AppError::BadRequest("El archivo no es una bóveda de Pact válida".to_string()));
            }

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
    let raw_name = payload.get("name").map(|s| s.as_str()).unwrap_or("vault");
    // Sanear el nombre: solo alfanuméricos, espacios, guiones y guiones bajos.
    let cleaned: String = raw_name
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .collect();
    let name = cleaned.trim();
    let name = if name.is_empty() { "vault" } else { name };
    let file_name = format!("{}.pcty", name);
    let path = std::path::PathBuf::from(&file_name);
    storage.init_vault(&path).map_err(|e| AppError::BadRequest(e.to_string()))?;
    storage.set_path(Some(path.clone()));
    Ok(Json(serde_json::json!({ "path": path.to_string_lossy().to_string() })))
}

/// Devuelve el seed TOTP **cifrado** de una cuenta. El cliente es el único que
/// puede descifrarlo con su MasterKey y generar el código; el servidor nunca
/// ve el seed en claro (Zero-Knowledge).
pub async fn get_totp_seed(
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

    Ok(Json(serde_json::json!({ "seed": setting.encrypted_seed })))
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
    _user: AuthenticatedUser,
) -> Result<impl IntoResponse> {
    let url = params.get("url").ok_or(AppError::BadRequest("Falta parámetro url".to_string()))?;

    let parsed = url::Url::parse(url).map_err(|_| AppError::BadRequest("URL inválida".to_string()))?;
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(AppError::BadRequest("Solo se permiten URLs HTTP/HTTPS".to_string()));
    }

    let host = parsed.host_str().ok_or(AppError::BadRequest("URL sin host".to_string()))?;
    let port = parsed.port_or_known_default().unwrap_or(443);

    // Resolvemos el DNS y rechazamos cualquier IP en rangos privados/loopback
    // para mitigar SSRF (incluye metadata cloud 169.254.169.254, etc.).
    let addrs = tokio::net::lookup_host((host, port)).await
        .map_err(|_| AppError::BadRequest("No se pudo resolver el host".to_string()))?;
    let mut any = false;
    for addr in addrs {
        any = true;
        if is_forbidden_ip(addr.ip()) {
            return Err(AppError::BadRequest("URL no permitida".to_string()));
        }
    }
    if !any {
        return Err(AppError::BadRequest("No se pudo resolver el host".to_string()));
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AppError::Internal(e.into()))?;

    let response = client.get(url)
        .send()
        .await
        .map_err(|e| AppError::BadRequest(format!("Error al descargar: {}", e)))?;

    if let Some(len) = response.content_length() {
        if len as usize > MAX_ICON_BYTES {
            return Err(AppError::BadRequest("Imagen demasiado grande".to_string()));
        }
    }

    let bytes = response.bytes().await.map_err(|e| AppError::BadRequest(format!("Error al leer: {}", e)))?;
    if bytes.len() > MAX_ICON_BYTES {
        return Err(AppError::BadRequest("Imagen demasiado grande".to_string()));
    }

    use base64::Engine;
    let base64_str = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(Json(serde_json::json!({ "base64": base64_str })))
}
