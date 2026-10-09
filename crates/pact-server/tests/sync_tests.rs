use std::sync::Arc;
use pact_server::db::{Database, SqliteDatabase};
use pact_server::sync::engine::SyncEngine;
use pact_server::models::{CreateUser, CreateFolder, CreateAccount};
use uuid::Uuid;

async fn setup_test_db() -> Arc<dyn Database> {
    let db_id = Uuid::new_v4().to_string();
    let url = format!("sqlite:file:{}?mode=memory&cache=shared", db_id);
    let db = SqliteDatabase::new(&url).await.unwrap();
    db.migrate().await.expect("Migration failed");
    Arc::new(db)
}

#[tokio::test]
async fn test_sync_folder_creation_local_to_remote() {
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

    let folder_name = "Work".to_string();
    local_db.create_folder(&CreateFolder {
        user_id,
        name: folder_name.clone(),
        color: Some("#000000".to_string()),
        icon: Some("folder".to_string()),
        uid: None,
    }).await.unwrap();

    let engine = SyncEngine::new(local_db.as_ref(), remote_db.as_ref(), user_id);
    let plan = engine.generate_plan().await.unwrap();

    assert_eq!(plan.to_push_folders.len(), 1);
    assert_eq!(plan.to_pull_folders.len(), 0);

    engine.apply_plan(&plan).await.unwrap();

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

    let engine = SyncEngine::new(local_db.as_ref(), remote_db.as_ref(), user_id);
    let plan = engine.generate_plan().await.unwrap();
    engine.apply_plan(&plan).await.unwrap();

    let remote_folders = remote_db.get_folders_by_user(user_id).await.unwrap();
    let remote_accounts = remote_db.get_accounts_by_user(user_id).await.unwrap();

    assert_eq!(remote_folders.len(), 1);
    assert_eq!(remote_accounts.len(), 1);
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

    let engine = SyncEngine::new(local_db.as_ref(), remote_db.as_ref(), user_id);
    let plan = engine.generate_plan().await.unwrap();

    assert!(plan.to_push_folders.is_empty());
    assert!(plan.to_pull_folders.is_empty());
    assert!(plan.to_push_accounts.is_empty());
    assert!(plan.to_pull_accounts.is_empty());

    let local_accounts = local_db.get_accounts_by_user(user_id).await.unwrap();
    let remote_accounts = remote_db.get_accounts_by_user(user_id).await.unwrap();
    assert_eq!(local_accounts.len(), 1);
    assert_eq!(remote_accounts.len(), 1);
    assert_eq!(local_accounts[0].uid, acc_uid);
    assert_eq!(remote_accounts[0].uid, acc_uid);
}

#[tokio::test]
async fn test_sync_pull_only() {
    let local_db = setup_test_db().await;
    let remote_db = setup_test_db().await;

    let user_id = local_db.create_user(&CreateUser {
        username: "pulluser".to_string(),
        master_hash: "h".to_string(),
        salt: "s".to_string(),
        color: None,
    }).await.unwrap();

    remote_db.create_user(&CreateUser {
        username: "pulluser".to_string(),
        master_hash: "h".to_string(),
        salt: "s".to_string(),
        color: None,
    }).await.unwrap();

    remote_db.create_folder(&CreateFolder {
        user_id,
        name: "Remote Only".to_string(),
        color: None,
        icon: None,
        uid: Some("remote-folder-uid".to_string()),
    }).await.unwrap();

    let engine = SyncEngine::new(local_db.as_ref(), remote_db.as_ref(), user_id);
    let plan = engine.generate_plan().await.unwrap();

    assert!(plan.to_push_folders.is_empty());
    assert_eq!(plan.to_pull_folders.len(), 1);
    assert_eq!(plan.to_pull_folders[0], "remote-folder-uid");

    engine.apply_plan(&plan).await.unwrap();

    let local_folders = local_db.get_folders_by_user(user_id).await.unwrap();
    assert_eq!(local_folders.len(), 1);
    assert_eq!(local_folders[0].name, "Remote Only");
    assert_eq!(local_folders[0].uid, "remote-folder-uid");
}

#[tokio::test]
async fn test_sync_bidirectional() {
    let local_db = setup_test_db().await;
    let remote_db = setup_test_db().await;

    let user_id = local_db.create_user(&CreateUser {
        username: "bidir".to_string(),
        master_hash: "h".to_string(),
        salt: "s".to_string(),
        color: None,
    }).await.unwrap();

    remote_db.create_user(&CreateUser {
        username: "bidir".to_string(),
        master_hash: "h".to_string(),
        salt: "s".to_string(),
        color: None,
    }).await.unwrap();

    local_db.create_folder(&CreateFolder {
        user_id,
        name: "Local Folder".to_string(),
        color: None,
        icon: None,
        uid: Some("local-uid".to_string()),
    }).await.unwrap();

    remote_db.create_folder(&CreateFolder {
        user_id,
        name: "Remote Folder".to_string(),
        color: None,
        icon: None,
        uid: Some("remote-uid".to_string()),
    }).await.unwrap();

    let engine = SyncEngine::new(local_db.as_ref(), remote_db.as_ref(), user_id);
    let plan = engine.generate_plan().await.unwrap();

    assert_eq!(plan.to_push_folders.len(), 1);
    assert!(plan.to_push_folders.contains(&"local-uid".to_string()));
    assert_eq!(plan.to_pull_folders.len(), 1);
    assert!(plan.to_pull_folders.contains(&"remote-uid".to_string()));

    engine.apply_plan(&plan).await.unwrap();

    let local_folders = local_db.get_folders_by_user(user_id).await.unwrap();
    let remote_folders = remote_db.get_folders_by_user(user_id).await.unwrap();
    assert_eq!(local_folders.len(), 2);
    assert_eq!(remote_folders.len(), 2);

    let local_names: Vec<_> = local_folders.iter().map(|f| f.name.as_str()).collect();
    assert!(local_names.contains(&"Local Folder"));
    assert!(local_names.contains(&"Remote Folder"));

    let remote_names: Vec<_> = remote_folders.iter().map(|f| f.name.as_str()).collect();
    assert!(remote_names.contains(&"Local Folder"));
    assert!(remote_names.contains(&"Remote Folder"));
}

#[tokio::test]
async fn test_sync_version_conflict_last_write_wins() {
    let local_db = setup_test_db().await;
    let remote_db = setup_test_db().await;

    let user_id = local_db.create_user(&CreateUser {
        username: "conflict".to_string(),
        master_hash: "h".to_string(),
        salt: "s".to_string(),
        color: None,
    }).await.unwrap();

    remote_db.create_user(&CreateUser {
        username: "conflict".to_string(),
        master_hash: "h".to_string(),
        salt: "s".to_string(),
        color: None,
    }).await.unwrap();

    let folder_uid = "conflict-folder".to_string();

    local_db.create_folder(&CreateFolder {
        user_id,
        name: "Local Version".to_string(),
        color: None,
        icon: None,
        uid: Some(folder_uid.clone()),
    }).await.unwrap();

    remote_db.create_folder(&CreateFolder {
        user_id,
        name: "Remote Version".to_string(),
        color: None,
        icon: None,
        uid: Some(folder_uid.clone()),
    }).await.unwrap();

    let local_folders = local_db.get_folders_by_user(user_id).await.unwrap();
    let remote_folders = remote_db.get_folders_by_user(user_id).await.unwrap();
    assert_eq!(local_folders[0].version, 1);
    assert_eq!(remote_folders[0].version, 1);

    local_db.update_folder(
        local_folders[0].id,
        &local_folders[0].clone().into(),
        3,
    ).await.unwrap();

    let engine = SyncEngine::new(local_db.as_ref(), remote_db.as_ref(), user_id);
    let plan = engine.generate_plan().await.unwrap();

    assert_eq!(plan.to_push_folders.len(), 1);
    assert!(plan.to_push_folders.contains(&folder_uid));
    assert!(plan.to_pull_folders.is_empty());

    engine.apply_plan(&plan).await.unwrap();

    let remote_folders = remote_db.get_folders_by_user(user_id).await.unwrap();
    assert_eq!(remote_folders[0].name, "Local Version");
    assert_eq!(remote_folders[0].version, 3);
}
