use axum::{
    routing::{get, post}, 
    Router, Json, extract::State
};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
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

#[tokio::main]
async fn main() {
    // Inicializar tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "pact_server=debug,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Iniciando Pact Server...");

    // Inicialización de la base de datos
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:pact.db".to_string());
    
    if db_url.starts_with("sqlite:") {
        let path = db_url.trim_start_matches("sqlite:");
        if !std::path::Path::new(path).exists() {
            tracing::info!("Base de datos no encontrada, creando: {}", path);
            std::fs::File::create(path).unwrap();
        }
    }

    let sqlite_db = SqliteDatabase::new(&db_url).await.expect("No se pudo conectar a la DB");
    sqlite_db.migrate().await.expect("Error al ejecutar migraciones");
    
    let db: Arc<dyn Database> = Arc::new(sqlite_db);
    let session_mgr = Arc::new(SessionManager::new());
    let storage = Arc::new(StorageManager::new(Some(std::path::PathBuf::from("pact.pcty"))));

    let state = AppState {
        db: db.clone(),
        session_mgr,
        storage,
    };

    // Definimos las rutas
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/status", get(status_handler))
        // Rutas de Autenticación
        .route("/auth/register", post(auth::handlers::register))
        .route("/auth/login", post(auth::handlers::login))
        .route("/auth/salt/:username", get(auth::handlers::get_salt))
        .route("/auth/unlock", post(auth::handlers::unlock))
        .route("/auth/lock", post(auth::handlers::lock))
        .route("/auth/users", get(auth::handlers::get_users))
        // Rutas de Bóveda
        .route("/vault/folders", get(vault::handlers::get_folders).post(vault::handlers::create_folder))
        .route("/vault/accounts", get(vault::handlers::get_accounts).post(vault::handlers::create_account))
        .route("/vault/config/path", get(vault::handlers::get_vault_path).post(vault::handlers::set_vault_path))
        .route("/vault/pending-entries", get(vault::handlers::get_pending_entries))
        .route("/vault/pending-entries/:id", axum::routing::delete(vault::handlers::delete_pending_entry))
        .route("/vault/pending-count/:username", get(vault::handlers::get_pending_count))
        // Rutas de Sincronización
        .route("/sync/run", post(sync::handlers::run_sync))
        .route("/sync/apply", post(sync::handlers::apply_sync))
        .route("/sync/check", get(sync::handlers::check_sync))
        .route("/sync/config", get(sync::handlers::get_sync_config).post(sync::handlers::set_sync_config))
        // Rutas de Sistema
        .route("/system/machine-id", get(machine_id_handler))
        // Servir archivos estáticos
        .fallback_service(tower_http::services::ServeDir::new("gui/dist"))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8002));
    tracing::info!("Pact Server escuchando en http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_handler() -> &'static str {
    "OK"
}

async fn status_handler(State(state): State<AppState>) -> Json<StatusResponse> {
    Json(StatusResponse {
        status: "Running".to_string(),
        version: "0.1.0-alpha".to_string(),
        engine: "Rust/Axum".to_string(),
        database: "SQLite (Connected)".to_string(),
        active_sessions: state.session_mgr.active_sessions(), 
    })
}

async fn machine_id_handler() -> Json<MachineIdResponse> {
    Json(MachineIdResponse {
        machine_id: "pact-rust-dev-device".to_string(),
    })
}
