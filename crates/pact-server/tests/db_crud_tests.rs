use std::sync::Arc;
use pact_server::db::{Database, SqliteDatabase};
use pact_server::models::{CreateUser, CreateFolder, CreateAccount, SyncConfigModel};
use uuid::Uuid;

async fn setup_test_db() -> Arc<dyn Database> {
    let db_id = Uuid::new_v4().to_string();
    let url = format!("sqlite:file:{}?mode=memory&cache=shared", db_id);
    let db = SqliteDatabase::new(&url).await.unwrap();
    db.migrate().await.expect("Migration failed");
    Arc::new(db)
}

async fn create_test_user(db: &dyn Database, username: &str) -> i32 {
    db.create_user(&CreateUser {
        username: username.to_string(),
        master_hash: "hash123".to_string(),
        salt: "salt123".to_string(),
        color: Some("#ff0000".to_string()),
    }).await.unwrap()
}

#[tokio::test]
async fn test_user_create_read_update_delete() {
    let db = setup_test_db().await;

    let user_id = create_test_user(db.as_ref(), "alice").await;
    assert!(user_id > 0);

    let user = db.get_user_by_username("alice").await.unwrap().unwrap();
    assert_eq!(user.username, "alice");
    assert_eq!(user.master_hash, "hash123");
    assert_eq!(user.color, Some("#ff0000".to_string()));

    db.update_user("alice", "alice_updated", "#00ff00").await.unwrap();
    let updated = db.get_user_by_username("alice_updated").await.unwrap().unwrap();
    assert_eq!(updated.username, "alice_updated");
    assert_eq!(updated.color, Some("#00ff00".to_string()));
    assert!(db.get_user_by_username("alice").await.unwrap().is_none());

    db.delete_user("alice_updated").await.unwrap();
    assert!(db.get_user_by_username("alice_updated").await.unwrap().is_none());
}

#[tokio::test]
async fn test_folder_crud_with_version_tracking() {
    let db = setup_test_db().await;
    let user_id = create_test_user(db.as_ref(), "bob").await;

    let folder_id = db.create_folder(&CreateFolder {
        user_id,
        name: "Work".to_string(),
        color: Some("blue".to_string()),
        icon: Some("briefcase".to_string()),
        uid: Some("folder-uid-001".to_string()),
    }).await.unwrap();
    assert!(folder_id > 0);

    let folders = db.get_folders_by_user(user_id).await.unwrap();
    assert_eq!(folders.len(), 1);
    assert_eq!(folders[0].name, "Work");
    assert_eq!(folders[0].uid, "folder-uid-001");
    assert_eq!(folders[0].version, 1);

    let updated_folder = CreateFolder {
        user_id,
        name: "Work Updated".to_string(),
        color: Some("red".to_string()),
        icon: Some("star".to_string()),
        uid: Some("folder-uid-001".to_string()),
    };
    db.update_folder(folder_id, &updated_folder, 5).await.unwrap();

    let folders = db.get_folders_by_user(user_id).await.unwrap();
    assert_eq!(folders[0].name, "Work Updated");
    assert_eq!(folders[0].version, 5);

    db.delete_folder(folder_id).await.unwrap();
    let folders = db.get_folders_by_user(user_id).await.unwrap();
    assert_eq!(folders.len(), 0);
}

#[tokio::test]
async fn test_delete_user_removes_user_record() {
    let db = setup_test_db().await;
    let user_id = create_test_user(db.as_ref(), "cascade_user").await;

    db.create_folder(&CreateFolder {
        user_id,
        name: "Folder1".to_string(),
        color: None,
        icon: None,
        uid: None,
    }).await.unwrap();

    assert!(db.get_user_by_username("cascade_user").await.unwrap().is_some());

    db.delete_user("cascade_user").await.unwrap();

    assert!(db.get_user_by_username("cascade_user").await.unwrap().is_none());
}

#[tokio::test]
async fn test_account_create_read_update_delete_restore() {
    let db = setup_test_db().await;
    let user_id = create_test_user(db.as_ref(), "charlie").await;

    let acc_id = db.create_account(&CreateAccount {
        user_id,
        folder_id: None,
        title: "GitHub".to_string(),
        url: Some("github.com".to_string()),
        username_site: Some("charlie".to_string()),
        encrypted_pass: "encrypted_secret".to_string(),
        is_favorite: false,
        icon_data: None,
        uid: Some("acc-uid-001".to_string()),
    }).await.unwrap();
    assert!(acc_id > 0);

    let accounts = db.get_accounts_by_user(user_id).await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].title, "GitHub");
    assert!(accounts[0].deleted_at.is_none());

    let updated = CreateAccount {
        user_id,
        folder_id: None,
        title: "GitHub Updated".to_string(),
        url: Some("github.com".to_string()),
        username_site: Some("charlie2".to_string()),
        encrypted_pass: "new_encrypted".to_string(),
        is_favorite: true,
        icon_data: None,
        uid: Some("acc-uid-001".to_string()),
    };
    db.update_account(acc_id, &updated, 2).await.unwrap();

    let accounts = db.get_accounts_by_user(user_id).await.unwrap();
    assert_eq!(accounts[0].title, "GitHub Updated");
    assert_eq!(accounts[0].version, 2);
    assert!(accounts[0].is_favorite);

    db.delete_account(acc_id).await.unwrap();
    let accounts = db.get_accounts_by_user(user_id).await.unwrap();
    assert!(accounts[0].deleted_at.is_some());

    db.restore_account(acc_id).await.unwrap();
    let accounts = db.get_accounts_by_user(user_id).await.unwrap();
    assert!(accounts[0].deleted_at.is_none());
}

#[tokio::test]
async fn test_account_soft_delete_and_empty_trash() {
    let db = setup_test_db().await;
    let user_id = create_test_user(db.as_ref(), "dave").await;

    let acc1 = db.create_account(&CreateAccount {
        user_id,
        folder_id: None,
        title: "Acc1".to_string(),
        url: None,
        username_site: None,
        encrypted_pass: "p1".to_string(),
        is_favorite: false,
        icon_data: None,
        uid: None,
    }).await.unwrap();

    let acc2 = db.create_account(&CreateAccount {
        user_id,
        folder_id: None,
        title: "Acc2".to_string(),
        url: None,
        username_site: None,
        encrypted_pass: "p2".to_string(),
        is_favorite: false,
        icon_data: None,
        uid: None,
    }).await.unwrap();

    db.delete_account(acc1).await.unwrap();

    let all_accounts = db.get_accounts_by_user(user_id).await.unwrap();
    assert_eq!(all_accounts.len(), 2);
    let deleted = all_accounts.iter().find(|a| a.id == acc1).unwrap();
    assert!(deleted.deleted_at.is_some());
    let alive = all_accounts.iter().find(|a| a.id == acc2).unwrap();
    assert!(alive.deleted_at.is_none());

    let deleted_count = db.empty_trash(user_id).await.unwrap();
    assert_eq!(deleted_count, 1);

    let remaining = db.get_accounts_by_user(user_id).await.unwrap();
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0].id, acc2);
}

#[tokio::test]
async fn test_account_favorites() {
    let db = setup_test_db().await;
    let user_id = create_test_user(db.as_ref(), "eve").await;

    let acc_id = db.create_account(&CreateAccount {
        user_id,
        folder_id: None,
        title: "FavAccount".to_string(),
        url: None,
        username_site: None,
        encrypted_pass: "p".to_string(),
        is_favorite: false,
        icon_data: None,
        uid: None,
    }).await.unwrap();

    let mut acc: CreateAccount = db.get_accounts_by_user(user_id).await.unwrap()[0].clone().into();
    acc.is_favorite = true;
    db.update_account(acc_id, &acc, 1).await.unwrap();

    let accounts = db.get_accounts_by_user(user_id).await.unwrap();
    assert!(accounts[0].is_favorite);
}

#[tokio::test]
async fn test_sync_config_upsert() {
    let db = setup_test_db().await;
    let user_id = create_test_user(db.as_ref(), "frank").await;

    assert!(db.get_sync_config(user_id).await.unwrap().is_none());

    let config = SyncConfigModel {
        user_id,
        remote_host: "db.example.com".to_string(),
        remote_port: 3306,
        remote_user: "sync_user".to_string(),
        remote_pass: "encrypted_pass".to_string(),
        remote_name: "pact_remote".to_string(),
        preferred_mode: Some("push".to_string()),
    };
    db.set_sync_config(user_id, &config).await.unwrap();

    let stored = db.get_sync_config(user_id).await.unwrap().unwrap();
    assert_eq!(stored.remote_host, "db.example.com");
    assert_eq!(stored.remote_port, 3306);
    assert_eq!(stored.remote_user, "sync_user");
    assert_eq!(stored.preferred_mode, Some("push".to_string()));

    let updated_config = SyncConfigModel {
        user_id,
        remote_host: "db2.example.com".to_string(),
        remote_port: 3307,
        remote_user: "sync_user2".to_string(),
        remote_pass: "encrypted_pass2".to_string(),
        remote_name: "pact_remote2".to_string(),
        preferred_mode: Some("pull".to_string()),
    };
    db.set_sync_config(user_id, &updated_config).await.unwrap();

    let stored = db.get_sync_config(user_id).await.unwrap().unwrap();
    assert_eq!(stored.remote_host, "db2.example.com");
    assert_eq!(stored.remote_port, 3307);
}

#[tokio::test]
async fn test_pending_accounts_empty_by_default() {
    let db = setup_test_db().await;

    assert_eq!(db.get_pending_count("alice").await.unwrap(), 0);
    assert!(db.get_pending_accounts("alice").await.unwrap().is_empty());

    assert_eq!(db.get_pending_count("nonexistent").await.unwrap(), 0);
    assert!(db.get_pending_accounts("nonexistent").await.unwrap().is_empty());
}

#[tokio::test]
async fn test_totp_settings_create_and_get() {
    let db = setup_test_db().await;
    let user_id = create_test_user(db.as_ref(), "grace").await;

    let acc_id = db.create_account(&CreateAccount {
        user_id,
        folder_id: None,
        title: "TOTP Account".to_string(),
        url: None,
        username_site: None,
        encrypted_pass: "p".to_string(),
        is_favorite: false,
        icon_data: None,
        uid: None,
    }).await.unwrap();

    assert!(db.get_totp_setting_by_account(acc_id).await.unwrap().is_none());

    db.create_totp_setting(acc_id, "encrypted_seed_abc", Some("GitHub")).await.unwrap();

    let setting = db.get_totp_setting_by_account(acc_id).await.unwrap().unwrap();
    assert_eq!(setting.account_id, acc_id);
    assert_eq!(setting.encrypted_seed, "encrypted_seed_abc");
    assert_eq!(setting.issuer, Some("GitHub".to_string()));
}

#[tokio::test]
async fn test_get_folders_by_user_empty() {
    let db = setup_test_db().await;
    let user_id = create_test_user(db.as_ref(), "empty_user").await;

    let folders = db.get_folders_by_user(user_id).await.unwrap();
    assert!(folders.is_empty());
}

#[tokio::test]
async fn test_get_accounts_by_user_returns_all_including_deleted() {
    let db = setup_test_db().await;
    let user_id = create_test_user(db.as_ref(), "filter_user").await;

    let acc1 = db.create_account(&CreateAccount {
        user_id, folder_id: None, title: "Active".to_string(),
        url: None, username_site: None, encrypted_pass: "p1".to_string(),
        is_favorite: false, icon_data: None, uid: None,
    }).await.unwrap();

    db.create_account(&CreateAccount {
        user_id, folder_id: None, title: "Also Active".to_string(),
        url: None, username_site: None, encrypted_pass: "p2".to_string(),
        is_favorite: false, icon_data: None, uid: None,
    }).await.unwrap();

    db.delete_account(acc1).await.unwrap();

    let accounts = db.get_accounts_by_user(user_id).await.unwrap();
    assert_eq!(accounts.len(), 2);
    let deleted = accounts.iter().find(|a| a.id == acc1).unwrap();
    assert!(deleted.deleted_at.is_some());
    let alive = accounts.iter().find(|a| a.title == "Also Active").unwrap();
    assert!(alive.deleted_at.is_none());
}

#[tokio::test]
async fn test_reset_clears_all_tables() {
    let db = setup_test_db().await;
    let user_id = create_test_user(db.as_ref(), "reset_user").await;

    db.create_folder(&CreateFolder {
        user_id, name: "F".to_string(), color: None, icon: None, uid: None,
    }).await.unwrap();

    db.create_account(&CreateAccount {
        user_id, folder_id: None, title: "A".to_string(),
        url: None, username_site: None, encrypted_pass: "p".to_string(),
        is_favorite: false, icon_data: None, uid: None,
    }).await.unwrap();

    assert!(!db.get_all_users().await.unwrap().is_empty());
    assert!(!db.get_folders_by_user(user_id).await.unwrap().is_empty());
    assert!(!db.get_accounts_by_user(user_id).await.unwrap().is_empty());

    db.reset().await.unwrap();

    assert!(db.get_all_users().await.unwrap().is_empty());
    assert!(db.get_folders_by_user(user_id).await.unwrap().is_empty());
    assert!(db.get_accounts_by_user(user_id).await.unwrap().is_empty());
}
