use std::sync::Arc;
use pact_server::db::{Database, SqliteDatabase};
use pact_server::sync::engine::SyncEngine;
use pact_server::models::{CreateUser, CreateFolder, CreateAccount};
use uuid::Uuid;

async fn setup_test_db() -> Arc<dyn Database> {
    let db_id = Uuid::new_v4().to_string();
    let path = format!("/tmp/pact_test_{}.db", db_id);
    let url = format!("sqlite:{}?mode=rwc", path);
    
    let db = SqliteDatabase::new(&url).await.unwrap();
    db.migrate().await.expect("Migration failed");
    
    Arc::new(db)
}

#[tokio::test]
async fn test_sync_folder_creation_local_to_remote() {
    let local_db = setup_test_db().await;
    let remote_db = setup_test_db().await;
    
    // 1. Create user in both
    let user_id = local_db.create_user(&CreateUser {
        username: "testuser".to_string(),
        master_hash: "hash".to_string(),
        salt: "salt".to_string(),
        color: Some("#ffffff".to_string()),
    }).await.unwrap();
    
    // Same user in remote (simulate shared account)
    remote_db.create_user(&CreateUser {
        username: "testuser".to_string(),
        master_hash: "hash".to_string(),
        salt: "salt".to_string(),
        color: Some("#ffffff".to_string()),
    }).await.unwrap();

    // 2. Create folder locally
    let folder_name = "Work".to_string();
    local_db.create_folder(&CreateFolder {
        user_id,
        name: folder_name.clone(),
        color: Some("#000000".to_string()),
        icon: Some("folder".to_string()),
        uid: None,
    }).await.unwrap();

    // 3. Run Sync
    let engine = SyncEngine::new(local_db.as_ref(), remote_db.as_ref(), user_id);
    let plan = engine.generate_plan().await.unwrap();
    
    assert_eq!(plan.to_push_folders.len(), 1);
    assert_eq!(plan.to_pull_folders.len(), 0);
    
    engine.apply_plan(&plan).await.unwrap();

    // 4. Verify remote has the folder
    let remote_folders = remote_db.get_folders_by_user(user_id).await.unwrap();
    assert_eq!(remote_folders.len(), 1);
    assert_eq!(remote_folders[0].name, folder_name);
    assert!(remote_folders[0].uid.len() > 0);
}

#[tokio::test]
async fn test_sync_account_mapping() {
    let local_db = setup_test_db().await;
    let remote_db = setup_test_db().await;
    
    let user_id = local_db.create_user(&CreateUser {
        username: "testuser".to_string(),
        master_hash: "hash".to_string(),
        salt: "salt".to_string(),
        color: Some("#ffffff".to_string()),
    }).await.unwrap();
    
    remote_db.create_user(&CreateUser {
        username: "testuser".to_string(),
        master_hash: "hash".to_string(),
        salt: "salt".to_string(),
        color: Some("#ffffff".to_string()),
    }).await.unwrap();

    // 1. Local: Folder A + Account B
    let folder_id = local_db.create_folder(&CreateFolder {
        user_id,
        name: "Financial".to_string(),
        color: Some("blue".to_string()),
        icon: Some("bank".to_string()),
        uid: Some("folder-uid-123".to_string()),
    }).await.unwrap();

    local_db.create_account(&CreateAccount {
        user_id,
        folder_id: Some(folder_id),
        title: "Bank Admin".to_string(),
        url: Some("bank.com".to_string()),
        username_site: Some("user1".to_string()),
        encrypted_pass: "secret".to_string(),
        is_favorite: false,
        icon_data: None,
        uid: Some("account-uid-456".to_string()),
    }).await.unwrap();

    // 2. Sync
    let engine = SyncEngine::new(local_db.as_ref(), remote_db.as_ref(), user_id);
    let plan = engine.generate_plan().await.unwrap();
    engine.apply_plan(&plan).await.unwrap();

    // 3. Verify Remote
    let remote_folders = remote_db.get_folders_by_user(user_id).await.unwrap();
    let remote_accounts = remote_db.get_accounts_by_user(user_id).await.unwrap();
    
    assert_eq!(remote_folders.len(), 1);
    assert_eq!(remote_accounts.len(), 1);
    
    // Check that account in remote points to folder in remote
    assert_eq!(remote_accounts[0].folder_id, Some(remote_folders[0].id));
    assert_eq!(remote_folders[0].uid, "folder-uid-123");
    assert_eq!(remote_accounts[0].uid, "account-uid-456");
}

#[tokio::test]
async fn test_sync_deletion_propagation() {
    let local_db = setup_test_db().await;
    let remote_db = setup_test_db().await;
    
    let user_id = local_db.create_user(&CreateUser {
        username: "deluser".to_string(),
        master_hash: "h".to_string(),
        salt: "s".to_string(),
        color: None,
    }).await.unwrap();

    remote_db.create_user(&CreateUser {
        username: "deluser".to_string(),
        master_hash: "h".to_string(),
        salt: "s".to_string(),
        color: None,
    }).await.unwrap();

    // 1. Setup Synced Account
    let acc_uid = "to-be-deleted".to_string();
    let create_a = CreateAccount {
        user_id,
        folder_id: None,
        title: "Bye Bye".to_string(),
        url: None,
        username_site: None,
        encrypted_pass: "p".to_string(),
        is_favorite: false,
        icon_data: None,
        uid: Some(acc_uid.clone()),
    };
    
    local_db.create_account(&create_a).await.unwrap();
    remote_db.create_account(&create_a).await.unwrap();
}
