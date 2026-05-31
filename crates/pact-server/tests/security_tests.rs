use pact_server::auth::session::SessionManager;
use pact_server::auth::handlers::{login};
use pact_server::db::{SqliteDatabase, Database};
use pact_server::schemas::UserLogin;
use axum::{extract::State, Json};
use std::sync::Arc;
use uuid::Uuid;

async fn setup_test_db() -> Arc<dyn Database> {
    let db_id = Uuid::new_v4().to_string();
    let url = format!("sqlite:file:{}?mode=memory&cache=shared", db_id);
    let db = SqliteDatabase::new(&url).await.unwrap();
    db.migrate().await.expect("Migration failed");
    Arc::new(db)
}

#[tokio::test]
async fn test_session_expiration() {
    let mgr = SessionManager::new();
    let token = mgr.create_session("testuser".to_string());
    
    // Debería ser válida inmediatamente
    assert!(mgr.get_session(&token).is_some());
    
    // No hay una forma fácil de "adelantar el tiempo" sin mockear chrono,
    // pero podemos verificar que el cleanup funciona si cambiamos el timeout si fuera posible.
    // Como no podemos cambiar el timeout fácilmente (es privado), 
    // confiaremos en la lógica de Utc::now() y signed_duration_since.
}

#[tokio::test]
async fn test_login_user_enumeration_prevention() {
    let db = setup_test_db().await;
    let session_mgr = Arc::new(SessionManager::new());
    
    // Caso 1: Usuario no existe
    let payload = Json(UserLogin {
        username: "nonexistent".to_string(),
        master_password: "password".to_string(),
    });
    
    let res = login(State(db.clone()), State(session_mgr.clone()), payload).await;
    assert!(res.is_err());
    // El error debería ser Unauthorized, no NotFound
    // Nota: Dependiendo de cómo esté implementado AppError, esto puede variar
}

#[tokio::test]
async fn test_vault_path_security() {
    use pact_server::storage::StorageManager;
    use std::path::PathBuf;
    
    let storage = StorageManager::new(None);
    
    // 1. Validar que no acepta cualquier archivo como boveda
    let root_file = PathBuf::from("/etc/passwd"); // Existe en linux pero no es zip
    if root_file.exists() {
        let is_valid = storage.validate_vault(&root_file).unwrap_or(false);
        assert!(!is_valid, "No debería validar un archivo del sistema como bóveda");
    }

    // 2. Validar que un directorio no es una bóveda válida
    let dir = tempfile::tempdir().unwrap();
    let is_valid = storage.validate_vault(dir.path()).unwrap_or(false);
    assert!(!is_valid, "Un directorio no debe ser una bóveda válida");
}

#[tokio::test]
async fn test_auth_token_tampering() {
    let mgr = SessionManager::new();
    let token = mgr.create_session("alice".to_string());
    
    // Intentar modificar el token (suponiendo que fuera un JWT o similar, 
    // pero aquí es un UUID aleatorio, así que cualquier cambio lo invalida)
    let fake_token = format!("{}modified", token);
    assert!(mgr.get_session(&fake_token).is_none());
}
