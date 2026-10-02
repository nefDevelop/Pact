use std::sync::Arc;
use pact_server::{
    db::{Database, SqliteDatabase},
    auth::session::SessionManager,
    storage::StorageManager,
    AppState, crypto::SyncCrypto,
    auth::ratelimit::auth_rate_limiter,
};
use pact_server::models::CreateUser;
use uuid::Uuid;
use serde_json::Value;

async fn setup_test_db() -> Arc<dyn Database> {
    let db_id = Uuid::new_v4().to_string();
    let url = format!("sqlite:file:{}?mode=memory&cache=shared", db_id);
    let db = SqliteDatabase::new(&url).await.unwrap();
    db.migrate().await.expect("Migration failed");
    Arc::new(db)
}

fn build_app_state(db: Arc<dyn Database>) -> AppState {
    let session_mgr = Arc::new(SessionManager::new());
    let storage = Arc::new(StorageManager::new(None));
    let sync_crypto = Arc::new(SyncCrypto::new("test-machine-id"));
    let auth_limiter = auth_rate_limiter();

    AppState {
        db,
        session_mgr,
        storage,
        machine_id: "test-machine-id".to_string(),
        sync_crypto,
        auth_limiter,
        db_type: "SQLite".to_string(),
    }
}

fn build_router(state: AppState) -> axum::Router {
    use axum::routing::{get, post};
    use tower_http::{cors::CorsLayer, trace::TraceLayer, limit::RequestBodyLimitLayer};

    axum::Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/system/machine-id", get(system_machine_id_handler))
        .route("/auth/register", post(pact_server::auth::handlers::register))
        .route("/auth/login", post(pact_server::auth::handlers::login))
        .route("/auth/salt/:username", get(pact_server::auth::handlers::get_salt))
        .route("/auth/update", post(pact_server::auth::handlers::update_user))
        .route("/auth/:username", axum::routing::delete(pact_server::auth::handlers::delete_user))
        .route("/auth/unlock", post(pact_server::auth::handlers::unlock))
        .route("/auth/lock", post(pact_server::auth::handlers::lock))
        .route("/auth/users", get(pact_server::auth::handlers::get_users))
        .route("/vault/folders", get(pact_server::vault::handlers::get_folders).post(pact_server::vault::handlers::create_folder))
        .route("/vault/folders/:id", axum::routing::put(pact_server::vault::handlers::update_folder).delete(pact_server::vault::handlers::delete_folder))
        .route("/vault/accounts", get(pact_server::vault::handlers::get_accounts).post(pact_server::vault::handlers::create_account))
        .route("/vault/accounts/:id", axum::routing::put(pact_server::vault::handlers::update_account).delete(pact_server::vault::handlers::delete_account))
        .route("/vault/accounts/:id/favorite", axum::routing::patch(pact_server::vault::handlers::toggle_favorite))
        .route("/vault/accounts/:id/restore", post(pact_server::vault::handlers::restore_account))
        .route("/vault/accounts/trash/empty", axum::routing::delete(pact_server::vault::handlers::empty_trash))
        .route("/vault/export", post(pact_server::vault::handlers::export_vault))
        .route("/vault/config/path", get(pact_server::vault::handlers::get_vault_path).post(pact_server::vault::handlers::set_vault_path))
        .route("/vault/pending-entries", get(pact_server::vault::handlers::get_pending_entries))
        .route("/vault/pending-entries/:id", axum::routing::delete(pact_server::vault::handlers::delete_pending_entry))
        .route("/vault/pending-count/:username", get(pact_server::vault::handlers::get_pending_count))
        .route("/vault/upload", post(pact_server::vault::handlers::upload_vault))
        .route("/vault/initialize", post(pact_server::vault::handlers::initialize_vault))
        .route("/vault/totp/:id", get(pact_server::vault::handlers::get_totp_seed))
        .route("/vault/accounts/:id/icon", axum::routing::patch(pact_server::vault::handlers::update_account_icon))
        .route("/vault/proxy-icon", get(pact_server::vault::handlers::proxy_icon))
        .route("/sync/run", post(pact_server::sync::handlers::run_sync))
        .route("/sync/apply", post(pact_server::sync::handlers::apply_sync))
        .route("/sync/check", get(pact_server::sync::handlers::check_sync))
        .route("/sync/test", post(pact_server::sync::handlers::test_sync_connection))
        .route("/sync/config", get(pact_server::sync::handlers::get_sync_config).post(pact_server::sync::handlers::set_sync_config))
        .layer(TraceLayer::new_for_http())
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

async fn system_machine_id_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({ "machine_id": state.machine_id }))
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

    client.post(format!("{}/auth/register", base))
        .json(&serde_json::json!({
            "username": username,
            "master_password": password,
            "color": "#ff0000"
        }))
        .send()
        .await
        .unwrap();

    let resp = client.post(format!("{}/auth/login", base))
        .json(&serde_json::json!({
            "username": username,
            "master_password": password
        }))
        .send()
        .await
        .unwrap();

    let body: Value = resp.json().await.unwrap();
    body["token"].as_str().unwrap().to_string()
}

async fn create_test_user_direct(db: &dyn Database, username: &str) -> i32 {
    db.create_user(&CreateUser {
        username: username.to_string(),
        master_hash: "hash123".to_string(),
        salt: "salt123".to_string(),
        color: Some("#ff0000".to_string()),
    }).await.unwrap()
}

#[tokio::test]
async fn test_health_endpoint() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/health", base))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body = resp.text().await.unwrap();
    assert_eq!(body, "OK");
}

#[tokio::test]
async fn test_machine_id_endpoint_returns_json() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/system/machine-id", base))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(body.get("machine_id").is_some());
}

#[tokio::test]
async fn test_register_user_success() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/auth/register", base))
        .json(&serde_json::json!({
            "username": "alice",
            "master_password": "pass123",
            "color": "#00ff00"
        }))
        .send().await.unwrap();

    assert_eq!(resp.status(), 201);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["username"], "alice");
}

#[tokio::test]
async fn test_register_user_duplicate_returns_400() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "username": "alice",
        "master_password": "pass123"
    });

    let resp1 = client.post(format!("{}/auth/register", base))
        .json(&payload).send().await.unwrap();
    assert_eq!(resp1.status(), 201);

    let resp2 = client.post(format!("{}/auth/register", base))
        .json(&payload).send().await.unwrap();
    assert_eq!(resp2.status(), 400);
}

#[tokio::test]
async fn test_login_success_returns_token() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    client.post(format!("{}/auth/register", base))
        .json(&serde_json::json!({
            "username": "bob",
            "master_password": "secret"
        }))
        .send().await.unwrap();

    let resp = client.post(format!("{}/auth/login", base))
        .json(&serde_json::json!({
            "username": "bob",
            "master_password": "secret"
        }))
        .send().await.unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(!body["token"].as_str().unwrap().is_empty());
    assert_eq!(body["username"], "bob");
}

#[tokio::test]
async fn test_login_wrong_password_returns_401() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    client.post(format!("{}/auth/register", base))
        .json(&serde_json::json!({
            "username": "charlie",
            "master_password": "correct"
        }))
        .send().await.unwrap();

    let resp = client.post(format!("{}/auth/login", base))
        .json(&serde_json::json!({
            "username": "charlie",
            "master_password": "wrong"
        }))
        .send().await.unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_login_nonexistent_user_returns_401() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/auth/login", base))
        .json(&serde_json::json!({
            "username": "nobody",
            "master_password": "pass"
        }))
        .send().await.unwrap();

    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_get_salt_returns_valid_salt() {
    let db = setup_test_db().await;
    create_test_user_direct(db.as_ref(), "salt_user").await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/auth/salt/salt_user", base))
        .send().await.unwrap();

    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(!body["salt"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_lock_invalidates_session() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "lock_user", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/auth/lock", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client.get(format!("{}/vault/folders", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_delete_user_cascades() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "delete_me", "pass").await;
    let client = reqwest::Client::new();

    // Borrar exige sesión autenticada y la contraseña maestra.
    let resp = client.delete(format!("{}/auth/delete_me", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "master_password": "pass" }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client.post(format!("{}/auth/login", base))
        .json(&serde_json::json!({
            "username": "delete_me",
            "master_password": "pass"
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_get_users_returns_list() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    // `get_users` solo lista usuarios cuando hay una bóveda vinculada.
    state.storage.set_path(Some(std::path::PathBuf::from("/tmp/pact_test.pcty")));
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    client.post(format!("{}/auth/register", base))
        .json(&serde_json::json!({
            "username": "user1",
            "master_password": "pass"
        }))
        .send().await.unwrap();

    let resp = client.get(format!("{}/auth/users", base))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(!body["users"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_vault_endpoints_return_401_without_token() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();

    let resp = client.get(format!("{}/vault/folders", base)).send().await.unwrap();
    assert_eq!(resp.status(), 401);

    let resp = client.post(format!("{}/vault/folders", base))
        .json(&serde_json::json!({ "name": "Test" }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 401);

    let resp = client.get(format!("{}/vault/accounts", base)).send().await.unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_create_folder_and_get_folders() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "folder_user", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/vault/folders", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "name": "Work",
            "color": "#0000ff",
            "icon": "briefcase"
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);
    let folder_id: i32 = resp.json().await.unwrap();
    assert!(folder_id > 0);

    let resp = client.get(format!("{}/vault/folders", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let folders = body.as_array().unwrap();
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0]["name"], "Work");
}

#[tokio::test]
async fn test_update_folder() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "upd_folder", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/vault/folders", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "name": "Old Name" }))
        .send().await.unwrap();
    let folder_id: i32 = resp.json().await.unwrap();

    let resp = client.put(format!("{}/vault/folders/{}", base, folder_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "name": "New Name" }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client.get(format!("{}/vault/folders", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body[0]["name"], "New Name");
}

#[tokio::test]
async fn test_delete_folder() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "del_folder", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/vault/folders", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "name": "To Delete" }))
        .send().await.unwrap();
    let folder_id: i32 = resp.json().await.unwrap();

    let resp = client.delete(format!("{}/vault/folders/{}", base, folder_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client.get(format!("{}/vault/folders", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let body: Value = resp.json().await.unwrap();
    assert!(body.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_create_account_and_get_accounts() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "acc_user", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "title": "GitHub",
            "url": "github.com",
            "username_site": "user",
            "encrypted_pass": "encrypted_secret",
            "is_favorite": false
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 201);
    let acc_id: i32 = resp.json().await.unwrap();
    assert!(acc_id > 0);

    let resp = client.get(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    let accounts = body.as_array().unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0]["title"], "GitHub");
}

#[tokio::test]
async fn test_update_account() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "upd_acc", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "title": "Old Title",
            "encrypted_pass": "old"
        }))
        .send().await.unwrap();
    let acc_id: i32 = resp.json().await.unwrap();

    let resp = client.put(format!("{}/vault/accounts/{}", base, acc_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "title": "New Title",
            "encrypted_pass": "new"
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client.get(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body[0]["title"], "New Title");
}

#[tokio::test]
async fn test_delete_account_soft() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "del_acc", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "title": "To Delete",
            "encrypted_pass": "p"
        }))
        .send().await.unwrap();
    let acc_id: i32 = resp.json().await.unwrap();

    let resp = client.delete(format!("{}/vault/accounts/{}", base, acc_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    // Las cuentas en papelera solo se devuelven con include_deleted=true.
    let resp = client.get(format!("{}/vault/accounts?include_deleted=true", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let body: Value = resp.json().await.unwrap();
    let accounts = body.as_array().unwrap();
    assert_eq!(accounts.len(), 1);
    assert!(accounts[0]["deleted_at"].is_string());

    // Por defecto, la cuenta borrada no aparece.
    let resp = client.get(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn test_toggle_favorite() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "fav_user", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "title": "Fav Account",
            "encrypted_pass": "p",
            "is_favorite": false
        }))
        .send().await.unwrap();
    let acc_id: i32 = resp.json().await.unwrap();

    let resp = client.patch(format!("{}/vault/accounts/{}/favorite", base, acc_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["is_favorite"], true);

    let resp = client.patch(format!("{}/vault/accounts/{}/favorite", base, acc_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["is_favorite"], false);
}

#[tokio::test]
async fn test_restore_account() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "restore_user", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "title": "Restorable",
            "encrypted_pass": "p"
        }))
        .send().await.unwrap();
    let acc_id: i32 = resp.json().await.unwrap();

    client.delete(format!("{}/vault/accounts/{}", base, acc_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();

    let resp = client.post(format!("{}/vault/accounts/{}/restore", base, acc_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client.get(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_empty_trash() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "trash_user", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "title": "Trashed",
            "encrypted_pass": "p"
        }))
        .send().await.unwrap();
    let acc_id: i32 = resp.json().await.unwrap();

    client.delete(format!("{}/vault/accounts/{}", base, acc_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();

    let resp = client.delete(format!("{}/vault/accounts/trash/empty", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["deleted_count"], 1);
}

#[tokio::test]
async fn test_export_vault_wrong_password() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "export_user", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/vault/export", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "username": "export_user",
            "master_password": "wrong_pass"
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn test_vault_path_get() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "path_user", "pass").await;
    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/vault/config/path", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn test_pending_count() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/vault/pending-count/alice", base))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["count"], 0);
}

#[tokio::test]
async fn test_get_sync_config_empty() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "sync_user", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.get(format!("{}/sync/config", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_set_sync_config() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "sync_setter", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/sync/config", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "host": "db.example.com",
            "user": "admin",
            "password": "secret",
            "db_name": "pact_db",
            "port": 3306
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client.get(format!("{}/sync/config", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["host"], "db.example.com");
    assert_eq!(body["user"], "admin");
}

#[tokio::test]
async fn test_vault_endpoints_without_token_return_401() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();

    let endpoints = vec![
        ("GET", format!("{}/vault/folders", base)),
        ("POST", format!("{}/vault/folders", base)),
        ("GET", format!("{}/vault/accounts", base)),
        ("POST", format!("{}/vault/accounts", base)),
        ("DELETE", format!("{}/vault/accounts/1", base)),
        ("PATCH", format!("{}/vault/accounts/1/favorite", base)),
        ("POST", format!("{}/vault/accounts/1/restore", base)),
        ("DELETE", format!("{}/vault/accounts/trash/empty", base)),
        ("GET", format!("{}/vault/pending-entries", base)),
        ("POST", format!("{}/auth/update", base)),
        ("GET", format!("{}/vault/proxy-icon", base)),
        ("POST", format!("{}/vault/initialize", base)),
        ("POST", format!("{}/vault/upload", base)),
        ("GET", format!("{}/vault/config/path", base)),
    ];

    for (method, url) in endpoints {
        let resp = match method {
            "GET" => client.get(&url).send().await.unwrap(),
            "POST" => client.post(&url).json(&serde_json::json!({})).send().await.unwrap(),
            "DELETE" => client.delete(&url).send().await.unwrap(),
            "PATCH" => client.patch(&url).send().await.unwrap(),
            _ => unreachable!(),
        };
        assert_eq!(resp.status(), 401, "Expected 401 for {} {}", method, url);
    }
}

#[tokio::test]
async fn test_export_vault_with_correct_password() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "export_ok", "mypassword").await;
    let client = reqwest::Client::new();

    client.post(format!("{}/vault/folders", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "name": "Secrets" }))
        .send().await.unwrap();

    let resp = client.post(format!("{}/vault/export", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "username": "export_ok",
            "master_password": "mypassword"
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["username"], "export_ok");
    assert!(!body["folders"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_update_user() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "old_name", "pass").await;
    let client = reqwest::Client::new();

    // Renombrar exige sesión autenticada.
    let resp = client.post(format!("{}/auth/update", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "old_name": "old_name",
            "new_name": "new_name",
            "new_color": "#00ff00"
        }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client.get(format!("{}/auth/salt/new_name", base))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client.get(format!("{}/auth/salt/old_name", base))
        .send().await.unwrap();
    assert_eq!(resp.status(), 404);
}

#[tokio::test]
async fn test_set_sync_config_password_cleared_on_get() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "sync_sec", "pass").await;
    let client = reqwest::Client::new();

    client.post(format!("{}/sync/config", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "host": "db.example.com",
            "user": "admin",
            "password": "supersecret",
            "db_name": "pact_db"
        }))
        .send().await.unwrap();

    let resp = client.get(format!("{}/sync/config", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["password"], "");
    assert_eq!(body["host"], "db.example.com");
}

#[tokio::test]
async fn test_unlock_returns_token() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    client.post(format!("{}/auth/register", base))
        .json(&serde_json::json!({ "username": "unlock_user", "master_password": "pass" }))
        .send().await.unwrap();

    let resp = client.post(format!("{}/auth/unlock", base))
        .json(&serde_json::json!({ "username": "unlock_user", "master_password": "pass" }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert!(!body["token"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn test_register_rejects_unsafe_username() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let client = reqwest::Client::new();
    let resp = client.post(format!("{}/auth/register", base))
        .json(&serde_json::json!({ "username": "../evil", "master_password": "pass" }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn test_totp_seed_is_stored_opaque() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "totp_user", "pass").await;
    let client = reqwest::Client::new();

    // El cliente envía el seed ya cifrado; el servidor debe devolverlo tal cual
    // sin descifrarlo ni interpretarlo (Zero-Knowledge).
    let opaque_seed = "{\"nonce\":\"aa\",\"ciphertext\":\"bb\",\"tag\":\"cc\"}";
    let resp = client.post(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "title": "2FA",
            "encrypted_pass": "p",
            "totp_seed": opaque_seed
        }))
        .send().await.unwrap();
    let acc_id: i32 = resp.json().await.unwrap();

    let resp = client.get(format!("{}/vault/totp/{}", base, acc_id))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body["seed"], opaque_seed);
}

#[tokio::test]
async fn test_update_account_icon() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "icon_user", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.post(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "title": "Icon", "encrypted_pass": "p" }))
        .send().await.unwrap();
    let acc_id: i32 = resp.json().await.unwrap();

    let resp = client.patch(format!("{}/vault/accounts/{}/icon", base, acc_id))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "icon_data": "data:image/png;base64,AAAA" }))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200);

    let resp = client.get(format!("{}/vault/accounts", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    let body: Value = resp.json().await.unwrap();
    assert_eq!(body[0]["icon_data"], "data:image/png;base64,AAAA");
}

#[tokio::test]
async fn test_proxy_icon_blocks_private_addresses() {
    let db = setup_test_db().await;
    let state = build_app_state(db);
    let base = start_test_server(state).await;

    let token = register_and_login(&base, "proxy_user", "pass").await;
    let client = reqwest::Client::new();

    let resp = client.get(format!("{}/vault/proxy-icon?url=http://127.0.0.1/favicon.ico", base))
        .header("Authorization", format!("Bearer {}", token))
        .send().await.unwrap();
    assert_eq!(resp.status(), 400);
}
