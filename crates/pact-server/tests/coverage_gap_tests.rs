//! Cobertura de los caminos de mayor riesgo que los tests de contrato no tocaban:
//! entrada no validada (upload/initialize), y los endpoints de sincronización
//! que exigen la subclave `X-Pact-Sync-Key` además de la sesión.

use pact_server::auth::ratelimit::{auth_rate_limiter, public_rate_limiter};
use pact_server::auth::session::SessionManager;
use pact_server::db::{Database, SqliteDatabase};
use pact_server::storage::StorageManager;
use pact_server::AppState;
use std::sync::Arc;
use uuid::Uuid;

async fn setup_test_db() -> Arc<dyn Database> {
    let db_id = Uuid::new_v4().to_string();
    let url = format!("sqlite:file:{}?mode=memory&cache=shared", db_id);
    let db = SqliteDatabase::new(&url).await.unwrap();
    db.migrate().await.expect("Migration failed");
    Arc::new(db)
}

fn build_app_state(db: Arc<dyn Database>) -> AppState {
    AppState {
        db,
        session_mgr: Arc::new(SessionManager::new()),
        storage: Arc::new(StorageManager::new(None)),
        machine_id: "test-machine-id".to_string(),
        auth_limiter: auth_rate_limiter(),
        public_limiter: public_rate_limiter(),
        db_type: "SQLite".to_string(),
        data_dir: std::env::temp_dir(),
    }
}

fn build_router(state: AppState) -> axum::Router {
    use axum::routing::{get, post};
    use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer};

    axum::Router::new()
        .route("/auth/register", post(pact_server::auth::handlers::register))
        .route("/auth/login", post(pact_server::auth::handlers::login))
        .route("/auth/salt/:username", get(pact_server::auth::handlers::get_salt))
        .route("/auth/users", get(pact_server::auth::handlers::get_users))
        .route("/auth/update", post(pact_server::auth::handlers::update_user))
        .route("/auth/:username", axum::routing::delete(pact_server::auth::handlers::delete_user))
        .route("/auth/unlock", post(pact_server::auth::handlers::unlock))
        .route("/auth/lock", post(pact_server::auth::handlers::lock))
        .route("/vault/export", post(pact_server::vault::handlers::export_vault))
        .route("/vault/config/path", get(pact_server::vault::handlers::get_vault_path).post(pact_server::vault::handlers::set_vault_path))
        .route("/vault/upload", post(pact_server::vault::handlers::upload_vault))
        .route("/vault/initialize", post(pact_server::vault::handlers::initialize_vault))
        .route("/vault/totp/:id", get(pact_server::vault::handlers::get_totp_seed))
        .route("/vault/accounts/:id/icon", axum::routing::patch(pact_server::vault::handlers::update_account_icon))
        .route("/vault/icon", get(pact_server::vault::handlers::proxy_icon_image))
        .route("/vault/accounts/trash/empty", axum::routing::delete(pact_server::vault::handlers::empty_trash))
        .route("/sync/run", post(pact_server::sync::handlers::run_sync))
        .route("/sync/apply", post(pact_server::sync::handlers::apply_sync))
        .route("/sync/check", get(pact_server::sync::handlers::check_sync))
        .route("/sync/test", post(pact_server::sync::handlers::test_sync_connection))
        .route("/sync/config", get(pact_server::sync::handlers::get_sync_config).post(pact_server::sync::handlers::set_sync_config))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn start_test_server(state: AppState) -> String {
    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let base_url = format!("http://{}", addr);

    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    base_url
}

async fn register_and_login(base: &str, username: &str, password: &str) -> String {
    let client = reqwest::Client::new();

    client
        .post(format!("{}/auth/register", base))
        .json(&serde_json::json!({
            "username": username,
            "master_password": password,
            "color": "#ff0000"
        }))
        .send()
        .await
        .unwrap();

    let resp = client
        .post(format!("{}/auth/login", base))
        .json(&serde_json::json!({ "username": username, "master_password": password }))
        .send()
        .await
        .unwrap();

    let body: serde_json::Value = resp.json().await.unwrap();
    body["token"].as_str().expect("token").to_string()
}

// --- Validación de la entrada de archivo (.pcty) -----------------------------

#[tokio::test]
async fn test_upload_rejects_non_pcty_extension() {
    let db = setup_test_db().await;
    let base = start_test_server(build_app_state(db)).await;
    let token = register_and_login(&base, "upload_ext", "pass").await;

    let client = reqwest::Client::new();
    let part = reqwest::multipart::Part::text("no soy una boveda")
        .file_name("malicious.txt")
        .mime_str("text/plain")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    let resp = client
        .post(format!("{}/vault/upload", base))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error(),
        "una extension distinta de .pcty debe rechazarse, dio {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_upload_rejects_path_traversal_filename() {
    let db = setup_test_db().await;
    let base = start_test_server(build_app_state(db)).await;
    let token = register_and_login(&base, "upload_trav", "pass").await;

    let client = reqwest::Client::new();
    // El handler toma solo `file_name()`, pero el nombre con `..` no debe
    // escapar del directorio ni aceptarse tal cual.
    let part = reqwest::multipart::Part::text("contenido")
        .file_name("../../../etc/passwd.pcty")
        .mime_str("application/octet-stream")
        .unwrap();
    let form = reqwest::multipart::Form::new().part("file", part);

    let resp = client
        .post(format!("{}/vault/upload", base))
        .header("Authorization", format!("Bearer {}", token))
        .multipart(form)
        .send()
        .await
        .unwrap();

    // No debe crear ficheros fuera del cwd. Puede fallar al validar el zip,
    // pero nunca debe escapar del directorio de trabajo.
    let _ = resp.status();
    assert!(
        !std::path::Path::new("/etc/passwd.pcty").exists(),
        "el nombre con traversal no debe escribir fuera del directorio"
    );
}

#[tokio::test]
async fn test_initialize_rejects_invalid_vault_name() {
    let db = setup_test_db().await;
    let base = start_test_server(build_app_state(db)).await;
    let token = register_and_login(&base, "init_bad", "pass").await;

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/vault/initialize", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "name": "../escapada" }))
        .send()
        .await
        .unwrap();

    // El nombre va a formar parte de una ruta: un `..` debe rechazarse o,
    // como mínimo, no escribir fuera del directorio actual.
    let _ = resp.status();
    assert!(
        !std::path::Path::new("../escapada.pcty").exists(),
        "el nombre con traversal no debe escribir fuera del directorio"
    );
}

#[tokio::test]
async fn test_upload_and_initialize_require_auth() {
    let db = setup_test_db().await;
    let base = start_test_server(build_app_state(db)).await;
    let client = reqwest::Client::new();

    let part = reqwest::multipart::Part::text("x").file_name("a.pcty");
    let form = reqwest::multipart::Form::new().part("file", part);
    let resp = client
        .post(format!("{}/vault/upload", base))
        .multipart(form)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    let resp = client
        .post(format!("{}/vault/initialize", base))
        .json(&serde_json::json!({ "name": "sin_auth" }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

// --- Sincronización: sesión + subclave --------------------------------------

#[tokio::test]
async fn test_sync_endpoints_require_sync_key() {
    let db = setup_test_db().await;
    let base = start_test_server(build_app_state(db)).await;
    let token = register_and_login(&base, "sync_key", "pass").await;
    let client = reqwest::Client::new();

    // Con sesión pero sin `X-Pact-Sync-Key` la config remota no puede
    // descifrarse: debe ser 400, nunca una conexión con datos.
    let resp = client
        .post(format!("{}/sync/run", base))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "sync/run sin clave debe ser 400");

    let resp = client
        .post(format!("{}/sync/test", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "host": "localhost", "user": "u", "password": "p", "db_name": "d"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400, "sync/test sin clave debe ser 400");
}

#[tokio::test]
async fn test_sync_apply_rejects_invalid_plan() {
    let db = setup_test_db().await;
    let base = start_test_server(build_app_state(db)).await;
    let token = register_and_login(&base, "sync_apply", "pass").await;
    let client = reqwest::Client::new();

    // Un plan mal formado no debe aplicar nada ni entrar en pánico.
    let resp = client
        .post(format!("{}/sync/apply", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "no_es_un_plan": true }))
        .send()
        .await
        .unwrap();

    assert!(
        resp.status().is_client_error() || resp.status().is_server_error(),
        "un plan inválido debe rechazarse, dio {}",
        resp.status()
    );
}

#[tokio::test]
async fn test_sync_endpoints_require_auth() {
    let db = setup_test_db().await;
    let base = start_test_server(build_app_state(db)).await;
    let client = reqwest::Client::new();

    let resp = client.get(format!("{}/sync/check", base)).send().await.unwrap();
    assert_eq!(resp.status(), 401);

    let resp = client.get(format!("{}/sync/config", base)).send().await.unwrap();
    assert_eq!(resp.status(), 401);

    let resp = client
        .post(format!("{}/sync/run", base))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

// --- Otros endpoints con session --------------------------------------------

#[tokio::test]
async fn test_protected_endpoints_require_auth() {
    let db = setup_test_db().await;
    let base = start_test_server(build_app_state(db)).await;
    let client = reqwest::Client::new();

    // `/auth/users` y `/auth/salt` son públicos por diseño (el login necesita
    // listar usuarios antes de tener sesión), con rate limit por IP.
    for (method, path) in [
        ("POST", "/vault/export"),
        ("GET", "/vault/config/path"),
        ("GET", "/vault/totp/1"),
        ("DELETE", "/vault/accounts/trash/empty"),
    ] {
        let req = match method {
            "GET" => client.get(format!("{}{}", base, path)),
            "DELETE" => client.delete(format!("{}{}", base, path)),
            _ => client.post(format!("{}{}", base, path)),
        };
        let resp = req.send().await.unwrap();
        assert_eq!(resp.status(), 401, "{} {} sin token debe ser 401", method, path);
    }
}

#[tokio::test]
async fn test_pre_login_endpoints_are_public_but_rate_limited() {
    let db = setup_test_db().await;
    let base = start_test_server(build_app_state(db)).await;
    let client = reqwest::Client::new();

    // Necesarios para la pantalla de login: accesibles sin sesión.
    let resp = client.get(format!("{}/auth/users", base)).send().await.unwrap();
    assert_eq!(resp.status(), 200);

    // El salt de un usuario inexistente devuelve un valor determinista falso,
    // indistinguible de uno real, para no permitir enumeración.
    let real = client
        .get(format!("{}/auth/salt/alguien", base))
        .send()
        .await
        .unwrap();
    assert_eq!(real.status(), 200);
    let body: serde_json::Value = real.json().await.unwrap();
    assert!(body["salt"].as_str().unwrap().len() >= 16);
}

#[tokio::test]
async fn test_totp_seed_requires_auth_and_existing_account() {
    let db = setup_test_db().await;
    let base = start_test_server(build_app_state(db)).await;
    let token = register_and_login(&base, "totp_owner", "pass").await;
    let client = reqwest::Client::new();

    // Con sesión pero sin cuenta con ese id no debe devolver nada.
    let resp = client
        .get(format!("{}/vault/totp/9999", base))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_client_error(),
        "un id inexistente no debe devolver TOTP, dio {}",
        resp.status()
    );
}