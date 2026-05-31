use std::sync::Arc;
use pact_server::db::{Database, SqliteDatabase};
use pact_server::models::{CreateUser, CreateFolder, CreateAccount};
use pact_server::sync::engine::SyncEngine;
use uuid::Uuid;

async fn setup_test_db() -> Arc<dyn Database> {
    let db_id = Uuid::new_v4().to_string();
    let url = format!("sqlite:file:{}?mode=memory&cache=shared", db_id);
    let db = SqliteDatabase::new(&url).await.unwrap();
    db.migrate().await.expect("Migration failed");
    Arc::new(db)
}

#[tokio::test]
async fn test_user_lifecycle_use_case() {
    let db = setup_test_db().await;
    
    // 1. Registro de usuario
    let user_id = db.create_user(&CreateUser {
        username: "alice".to_string(),
        master_hash: "hash".to_string(),
        salt: "salt".to_string(),
        color: Some("#ff0000".to_string()),
    }).await.expect("Failed to create user");

    // 2. Crear una categoría
    let folder_id = db.create_folder(&CreateFolder {
        user_id,
        name: "Social".to_string(),
        color: Some("blue".to_string()),
        icon: Some("users".to_string()),
        uid: None,
    }).await.expect("Failed to create folder");

    // 3. Añadir una cuenta
    let acc_id = db.create_account(&CreateAccount {
        user_id,
        folder_id: Some(folder_id), // folder_id es i32
        title: "Twitter".to_string(),
        url: Some("twitter.com".to_string()),
        username_site: Some("alice_tw".to_string()),
        encrypted_pass: "cifrado".to_string(),
        is_favorite: false,
        icon_data: None,
        uid: None,
    }).await.expect("Failed to create account");

    // 4. Marcar como favorita y actualizar
    let account = db.get_account_by_uid(user_id, &db.get_accounts_by_user(user_id).await.unwrap()[0].uid).await.unwrap().unwrap();
    let mut update_acc: CreateAccount = account.clone().into();
    update_acc.is_favorite = true;
    db.update_account(acc_id, &update_acc, account.version + 1).await.expect("Failed to update account");

    // 5. Verificar estado final
    let accounts = db.get_accounts_by_user(user_id).await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert!(accounts[0].is_favorite);
    assert_eq!(accounts[0].version, 2);
}

#[tokio::test]
async fn test_sync_complex_conflict_use_case() {
    let local = setup_test_db().await;
    let remote = setup_test_db().await;
    
    let user_info = CreateUser {
        username: "bob".to_string(),
        master_hash: "h".to_string(),
        salt: "s".to_string(),
        color: None,
    };
    
    let uid = local.create_user(&user_info).await.unwrap();
    remote.create_user(&user_info).await.unwrap();

    // Bob crea un folder en local
    let f_uid = "shared-folder".to_string();
    local.create_folder(&CreateFolder {
        user_id: uid,
        name: "Local Name".to_string(),
        uid: Some(f_uid.clone()),
        color: None,
        icon: None,
    }).await.unwrap();

    // Mientras tanto, en remoto ya existía con otro nombre pero mayor versión
    remote.create_folder(&CreateFolder {
        user_id: uid,
        name: "Remote Name (Newer)".to_string(),
        uid: Some(f_uid.clone()),
        color: None,
        icon: None,
    }).await.unwrap();
    
    let rf = remote.get_folder_by_uid(uid, &f_uid).await.unwrap().unwrap();
    remote.update_folder(rf.id, &CreateFolder {
        user_id: uid,
        name: "Remote Name (Newer)".to_string(),
        uid: Some(f_uid.clone()),
        color: None,
        icon: None,
    }, 10).await.unwrap();

    // Sincronizar
    let engine = SyncEngine::new(local.as_ref(), remote.as_ref(), uid);
    let plan = engine.generate_plan().await.unwrap();
    assert!(plan.to_pull_folders.contains(&f_uid));
    
    engine.apply_plan(&plan).await.unwrap();

    // Local debe haberse actualizado
    let lf = local.get_folder_by_uid(uid, &f_uid).await.unwrap().unwrap();
    assert_eq!(lf.name, "Remote Name (Newer)");
}
