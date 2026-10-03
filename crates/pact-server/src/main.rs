use axum::{
    routing::{get, post}, 
    Router, Json, extract::State
};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer, limit::RequestBodyLimitLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use pact_server::{
    db::{Database, SqliteDatabase},
    auth::session::SessionManager,
    auth::middleware::AuthenticatedUser,
    storage::StorageManager,
    AppState, auth, vault, sync
};

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    version: String,
    engine: String,
    database: String,
}

#[derive(Serialize)]
struct MachineIdResponse {
    machine_id: String,
}

/// Orígenes que el servidor acepta por defecto.
///
/// - `tauri://localhost` y `http://tauri.localhost`: webview de Tauri en
///   Linux/macOS y Windows respectivamente. El frontend Tauri siempre habla con
///   el sidecar desde un origen distinto al de la API, así que necesita CORS.
/// - `http://localhost:8088` / `127.0.0.1:8088`: `make dev` (Vite).
/// - `http://127.0.0.1:8002`: `make run-server` servido por el propio servidor
///   (mismo origen, se incluye por robustez).
///
/// El modo web servido por este mismo backend es *same-origin* y por tanto no
/// necesita CORS en absoluto.
const DEFAULT_ALLOWED_ORIGINS: &[&str] = &[
    "tauri://localhost",
    "http://tauri.localhost",
    "http://localhost:8088",
    "http://127.0.0.1:8088",
    "http://localhost:8002",
    "http://127.0.0.1:8002",
];

/// Construye la capa de CORS a partir de una allowlist.
///
/// `PACT_CORS_ORIGIN` añade orígenes separados por comas (útil cuando el
/// frontend web se sirve desde otro dominio). Nunca degradamos a modo
/// permisivo: un `CorsLayer::permissive()` en un servidor que guarda una
/// bóveda de contraseñas permitiría que cualquier página local leyera las
/// respuestas de los endpoints públicos (`/auth/users`, `/auth/salt/:user`,
/// `/status`).
fn cors_layer() -> CorsLayer {
    let mut origins: Vec<String> = DEFAULT_ALLOWED_ORIGINS.iter().map(|s| s.to_string()).collect();

    if let Ok(extra) = std::env::var("PACT_CORS_ORIGIN") {
        for origin in extra.split(',') {
            let origin = origin.trim();
            if origin.is_empty() {
                continue;
            }
            if origin == "*" {
                tracing::error!(
                    "PACT_CORS_ORIGIN=* ya no se admite (sería CORS permisivo). \
                     Ignorando; usa una lista de orígenes separada por comas."
                );
                continue;
            }
            if !origin.starts_with("http://") && !origin.starts_with("https://")
                && !origin.starts_with("tauri://")
            {
                tracing::error!("Origen de CORS inválido ({}), se ignora", origin);
                continue;
            }
            origins.push(origin.to_string());
        }
    }

    let mut headers: Vec<axum::http::HeaderValue> = Vec::with_capacity(origins.len());
    for origin in &origins {
        match origin.parse::<axum::http::HeaderValue>() {
            Ok(v) => headers.push(v),
            Err(_) => tracing::error!("No se pudo parsear el origen '{}', se ignora", origin),
        }
    }
    tracing::info!("CORS: {} orígenes permitidos", headers.len());

    CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::list(headers))
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::PATCH, axum::http::Method::DELETE])
        .allow_headers([axum::http::header::AUTHORIZATION, axum::http::header::CONTENT_TYPE])
}

/// Directorio de datos persistente de Pact.
///
/// Antes el `machine-id` de reserva se escribía como `pact.machine` en el
/// directorio de trabajo, que cambia según desde dónde se lance el servidor y
/// no es escribible en un sistema de ficheros de solo lectura. Ahora se resuelve
/// por `PACT_DATA_DIR` y, si no, por el directorio de datos del usuario (XDG en
/// Linux, `%APPDATA%` en Windows, `~/Library/Application Support` en macOS).
fn data_dir() -> Option<std::path::PathBuf> {
    if let Ok(dir) = std::env::var("PACT_DATA_DIR") {
        if !dir.trim().is_empty() {
            return Some(std::path::PathBuf::from(dir));
        }
    }

    let base: std::path::PathBuf = if cfg!(target_os = "windows") {
        match std::env::var("APPDATA") {
            Ok(a) => std::path::PathBuf::from(a),
            Err(_) => return None,
        }
    } else if cfg!(target_os = "macos") {
        match std::env::var("HOME") {
            Ok(h) => std::path::PathBuf::from(h).join("Library/Application Support"),
            Err(_) => return None,
        }
    } else {
        match std::env::var("XDG_DATA_HOME") {
            Ok(x) => std::path::PathBuf::from(x),
            Err(_) => match std::env::var("HOME") {
                Ok(h) => std::path::PathBuf::from(h).join(".local/share"),
                Err(_) => return None,
            },
        }
    };

    let dir = base.join("pact");
    match std::fs::create_dir_all(&dir) {
        Ok(()) => Some(dir),
        Err(e) => {
            tracing::warn!("No se pudo crear el directorio de datos ({}): {}", dir.display(), e);
            None
        }
    }
}

/// Directorio de datos con reserva: si no se puede determinar (ni por variable
/// ni por el directorio del usuario), cae al actual en vez de fallar.
///
/// El directorio de trabajo solo sirve si es escribible: en el AppImage es el
/// montaje squashfs de solo lectura.
fn data_dir_or_cwd() -> std::path::PathBuf {
    if let Some(dir) = data_dir() {
        return dir;
    }
    tracing::warn!(
        "No se pudo determinar el directorio de datos; se usa el actual. \
         Define PACT_DATA_DIR si el proceso no puede escribir en él."
    );
    std::path::PathBuf::from(".")
}

fn read_machine_id() -> String {
    let paths = ["/etc/machine-id", "/var/lib/dbus/machine-id"];
    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            let id = content.trim().to_string();
            if !id.is_empty() && id.len() >= 32 {
                return id;
            }
        }
    }

    // Sin machine-id del sistema: generamos uno persistente para que el
    // fingerprint por hardware siga siendo estable entre reinicios.
    tracing::warn!("No se pudo leer machine-id del sistema, generando uno persistente");

    let Some(dir) = data_dir() else {
        tracing::error!(
            "Sin directorio de datos disponible: se generará un machine-id efímero. \
             Define PACT_DATA_DIR para que sea estable entre reinicios."
        );
        return uuid::Uuid::new_v4().to_string();
    };

    let fallback_path = dir.join("pact.machine");
    if let Ok(content) = std::fs::read_to_string(&fallback_path) {
        let id = content.trim().to_string();
        if !id.is_empty() {
            return id;
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    if let Err(e) = std::fs::write(&fallback_path, &id) {
        tracing::error!("No se pudo guardar el machine-id en {}: {}", fallback_path.display(), e);
    } else {
        // Contiene un identificador de máquina: solo el usuario debe leerlo.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&fallback_path, std::fs::Permissions::from_mode(0o600));
        }
        tracing::info!("machine-id persistente guardado en {}", fallback_path.display());
    }
    id
}

#[tokio::main]
async fn main() {
    // Por defecto `info`: en producción no queremos volcar cada request/response.
    // Súbelo a `debug` solo para diagnóstico local (`RUST_LOG=debug`).
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "pact_server=info,tower_http=warn".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Iniciando Pact Server...");

    let machine_id = read_machine_id();
    tracing::info!("Machine ID: {}", machine_id);

    // Configuración desde entorno.
    //
    // Las rutas por defecto van al directorio de datos del usuario, NO al
    // directorio de trabajo. En el AppImage el CWD es el punto de montaje
    // squashfs de solo lectura, así que `sqlite:pact.db` relativo fallaba con
    // "Read-only file system" y el servidor salía con código 1.
    let default_dir = data_dir_or_cwd();
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        format!("sqlite:{}", default_dir.join("pact.db").to_string_lossy())
    });
    // Una variable presente pero vacía equivale a no definirla: si no, una ruta
    // vacía se colaba como bóveda "configurada" y el frontend no podía ofrecer
    // la pantalla de abrir/crear.
    let vault_path = std::env::var("PACT_VAULT_PATH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default_dir.join("pact.pcty").to_string_lossy().to_string());
    let listen_addr = std::env::var("PACT_LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8002".to_string());
    let limiter_cleanup_secs: u64 = std::env::var("PACT_LIMITER_CLEANUP_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    let flush_interval_secs: u64 = std::env::var("PACT_FLUSH_INTERVAL_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(2);
    
    // Esta versión solo soporta SQLite como base local. MySQL/MariaDB se usa
    // únicamente como destino de la sincronización remota.
    if !db_url.starts_with("sqlite:") {
        tracing::error!(
            "DATABASE_URL solo admite SQLite (p. ej. 'sqlite:pact.db'); recibido '{}'. \
             MySQL/MariaDB se configura como destino de sincronización, no como DB local.",
            db_url
        );
        std::process::exit(1);
    }

    let path = db_url.trim_start_matches("sqlite:");
    let db_path = std::path::Path::new(path);
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::error!("No se pudo crear el directorio de la DB ({}): {}", parent.display(), e);
                std::process::exit(1);
            }
        }
    }
    if !db_path.exists() {
        tracing::info!("Base de datos no encontrada, creando: {}", db_path.display());
        if let Err(e) = std::fs::File::create(db_path) {
            tracing::error!("No se pudo crear el archivo de DB: {}", e);
            std::process::exit(1);
        }
        // Contiene metadatos de la bóveda: solo el usuario debe leerlo.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = std::fs::set_permissions(db_path, std::fs::Permissions::from_mode(0o600)) {
                tracing::warn!("No se pudieron restringir los permisos de {}: {}", db_path.display(), e);
            }
        }
    }

    let sqlite_db = match SqliteDatabase::new(&db_url).await {
        Ok(db) => db,
        Err(e) => {
            tracing::error!("No se pudo conectar a la DB: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = sqlite_db.migrate().await {
        tracing::error!("Error al ejecutar migraciones: {}", e);
        std::process::exit(1);
    }
    
    let db: Arc<dyn Database> = Arc::new(sqlite_db);
    let session_mgr = Arc::new(SessionManager::new());
    // Solo vinculamos la bóveda si el archivo existe: una ruta por defecto que
    // aún no se ha creado no es una bóveda abierta. Así el primer arranque deja
    // `get_path() == None`, el frontend recibe `vault_status: "empty"` y muestra
    // la pantalla de crear/abrir en lugar de pedir un usuario de la nada. La
    // ruta elegida por el usuario sí se pasa siempre (exista o haya que crearla).
    let vault_explicit = std::env::var("PACT_VAULT_PATH")
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);
    let vault_exists = std::path::Path::new(&vault_path).exists();
    let initial_vault = if vault_explicit || vault_exists {
        Some(std::path::PathBuf::from(&vault_path))
    } else {
        tracing::info!(
            "Sin bóveda vinculada en {}; se mostrará la pantalla de crear/abrir.",
            vault_path
        );
        None
    };
    let storage = Arc::new(StorageManager::new(initial_vault));

    let auth_limiter = pact_server::auth::ratelimit::auth_rate_limiter();
    let public_limiter = pact_server::auth::ratelimit::public_rate_limiter();

    // Limpieza periódica de los rate limiters
    let limiter_cleanup = auth_limiter.clone();
    let public_limiter_cleanup = public_limiter.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(limiter_cleanup_secs)).await;
            limiter_cleanup.cleanup_expired();
            public_limiter_cleanup.cleanup_expired();
        }
    });

    // Guardado diferido de la bóveda
    let storage_flusher = storage.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(flush_interval_secs)).await;
            let saved = storage_flusher.flush_dirty();
            if saved > 0 {
                tracing::debug!("Guardado diferido: {} usuarios persistidos", saved);
            }
        }
    });

    let state = AppState {
        db: db.clone(),
        session_mgr,
        storage,
        machine_id: machine_id.clone(),
        auth_limiter,
        public_limiter,
        db_type: "SQLite".to_string(),
    };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/status", get(status_handler))
        .route("/auth/register", post(auth::handlers::register))
        .route("/auth/login", post(auth::handlers::login))
        .route("/auth/salt/:username", get(auth::handlers::get_salt))
        .route("/auth/update", post(auth::handlers::update_user))
        .route("/auth/:username", axum::routing::delete(auth::handlers::delete_user))
        .route("/auth/unlock", post(auth::handlers::unlock))
        .route("/auth/lock", post(auth::handlers::lock))
        .route("/auth/users", get(auth::handlers::get_users))
        .route("/vault/folders", get(vault::handlers::get_folders).post(vault::handlers::create_folder))
        .route("/vault/folders/:id", axum::routing::put(vault::handlers::update_folder).delete(vault::handlers::delete_folder))
        .route("/vault/accounts", get(vault::handlers::get_accounts).post(vault::handlers::create_account))
        .route("/vault/accounts/:id", axum::routing::put(vault::handlers::update_account).delete(vault::handlers::delete_account))
        .route("/vault/accounts/:id/favorite", axum::routing::patch(vault::handlers::toggle_favorite))
        .route("/vault/accounts/:id/restore", post(vault::handlers::restore_account))
        .route("/vault/accounts/trash/empty", axum::routing::delete(vault::handlers::empty_trash))
        .route("/vault/export", post(vault::handlers::export_vault))
        .route("/vault/config/path", get(vault::handlers::get_vault_path).post(vault::handlers::set_vault_path))
        .route("/vault/pending-entries", get(vault::handlers::get_pending_entries))
        .route("/vault/pending-entries/:id", axum::routing::delete(vault::handlers::delete_pending_entry))
        .route("/vault/pending-count/:username", get(vault::handlers::get_pending_count))
        .route("/vault/upload", post(vault::handlers::upload_vault))
        .route("/vault/initialize", post(vault::handlers::initialize_vault))
        .route("/vault/totp/:id", get(vault::handlers::get_totp_seed))
        .route("/vault/accounts/:id/icon", axum::routing::patch(vault::handlers::update_account_icon))
        .route("/vault/icon", get(vault::handlers::proxy_icon_image))
        .route("/sync/run", post(sync::handlers::run_sync))
        .route("/sync/apply", post(sync::handlers::apply_sync))
        .route("/sync/check", get(sync::handlers::check_sync))
        .route("/sync/test", post(sync::handlers::test_sync_connection))
        .route("/sync/config", get(sync::handlers::get_sync_config).post(sync::handlers::set_sync_config))
        .route("/system/machine-id", get(machine_id_handler))
        .fallback_service(tower_http::services::ServeDir::new("gui/dist"))
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .layer(cors_layer())
        .with_state(state);

    let addr: SocketAddr = match listen_addr.parse() {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("PACT_LISTEN_ADDR inválido ({}), usando 127.0.0.1:8002", e);
            SocketAddr::from(([127, 0, 0, 1], 8002))
        }
    };
    tracing::info!("Pact Server escuchando en http://{}", addr);
    
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("No se pudo iniciar el servidor en {}: {}", addr, e);
            std::process::exit(1);
        }
    };
    // `into_make_service_with_connect_info` habilita ConnectInfo<SocketAddr>
    // para el rate limiting por IP en los endpoints de autenticación.
    if let Err(e) = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await {
        tracing::error!("Error del servidor: {}", e);
        std::process::exit(1);
    }
}

async fn health_handler() -> &'static str {
    "OK"
}

// Endpoint público: no expone el número de sesiones activas (permitiría a
    // cualquier página local que lo consultara inferir si el usuario está
    // usando la app). La versión y el motor son inocuos y útiles en soporte.
async fn status_handler(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "Running".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        engine: "Rust/Axum".to_string(),
        database: format!("{} (Connected)", state.db_type),
    })
}

async fn machine_id_handler(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> Json<MachineIdResponse> {
    Json(MachineIdResponse {
        machine_id: state.machine_id.clone(),
    })
}
