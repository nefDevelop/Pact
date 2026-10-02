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
    storage::StorageManager,
    AppState, auth, vault, sync
};

#[derive(Serialize)]
struct StatusResponse {
    status: String,
    version: String,
    engine: String,
    database: String,
    active_sessions: usize,
}

#[derive(Serialize)]
struct MachineIdResponse {
    machine_id: String,
}

fn cors_layer() -> CorsLayer {
    let origin = std::env::var("PACT_CORS_ORIGIN").unwrap_or_else(|_| "http://localhost:8088".to_string());
    if origin == "*" || origin.is_empty() {
        tracing::warn!("CORS en modo permisivo (PACT_CORS_ORIGIN=*) - solo para desarrollo");
        return CorsLayer::permissive();
    }
    tracing::info!("CORS permitiendo origen: {}", origin);
    let header_value = match origin.parse::<axum::http::HeaderValue>() {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("PACT_CORS_ORIGIN inválido ({}), usando permisivo", e);
            return CorsLayer::permissive();
        }
    };
    CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::exact(header_value))
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::PATCH, axum::http::Method::DELETE])
        .allow_headers([axum::http::header::AUTHORIZATION, axum::http::header::CONTENT_TYPE])
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
    tracing::warn!("No se pudo leer machine-id del sistema, generando uno persistente");
    let fallback_path = std::path::Path::new("pact.machine");
    if let Ok(content) = std::fs::read_to_string(fallback_path) {
        let id = content.trim().to_string();
        if !id.is_empty() {
            return id;
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    let _ = std::fs::write(fallback_path, &id);
    id
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "pact_server=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Iniciando Pact Server...");

    let machine_id = read_machine_id();
    tracing::info!("Machine ID: {}", machine_id);

    // Configuración desde entorno
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:pact.db".to_string());
    let vault_path = std::env::var("PACT_VAULT_PATH").unwrap_or_else(|_| "pact.pcty".to_string());
    let listen_addr = std::env::var("PACT_LISTEN_ADDR").unwrap_or_else(|_| "127.0.0.1:8002".to_string());
    let limiter_cleanup_secs: u64 = std::env::var("PACT_LIMITER_CLEANUP_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(300);
    let flush_interval_secs: u64 = std::env::var("PACT_FLUSH_INTERVAL_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(2);
    
    if db_url.starts_with("sqlite:") {
        let path = db_url.trim_start_matches("sqlite:");
        if !std::path::Path::new(path).exists() {
            tracing::info!("Base de datos no encontrada, creando: {}", path);
            if let Err(e) = std::fs::File::create(path) {
                tracing::error!("No se pudo crear el archivo de DB: {}", e);
                std::process::exit(1);
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
    let storage = Arc::new(StorageManager::new(Some(std::path::PathBuf::from(&vault_path))));

    let sync_crypto = Arc::new(pact_server::crypto::SyncCrypto::new(&machine_id));
    let auth_limiter = pact_server::auth::ratelimit::auth_rate_limiter();

    // Limpieza periódica del rate limiter
    let limiter_cleanup = auth_limiter.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(limiter_cleanup_secs)).await;
            limiter_cleanup.cleanup_expired();
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
        sync_crypto,
        auth_limiter,
        db_type: if db_url.starts_with("sqlite:") { "SQLite".to_string() } else { "MySQL".to_string() },
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
        .route("/vault/totp/:id", get(vault::handlers::get_totp_code))
        .route("/vault/accounts/:id/icon", axum::routing::patch(vault::handlers::update_account_icon))
        .route("/vault/proxy-icon", get(vault::handlers::proxy_icon))
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

async fn status_handler(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "Running".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        engine: "Rust/Axum".to_string(),
        database: format!("{} (Connected)", state.db_type),
        active_sessions: state.session_mgr.active_sessions(), 
    })
}

async fn machine_id_handler(
    State(state): State<AppState>,
) -> Json<MachineIdResponse> {
    Json(MachineIdResponse {
        machine_id: state.machine_id.clone(),
    })
}
