//! Tests de la implementación MySQL/MariaDB.
//!
//! Se ejecutan solo si `PACT_TEST_MYSQL_URL` está definida (p. ej. en CI con un
//! servicio MariaDB). En local, sin esa variable, se omiten sin fallar.

use std::sync::Arc;
use pact_server::db::{Database, MysqlDatabase};
use pact_server::models::{CreateUser, CreateFolder, CreateAccount};

async fn setup() -> Option<Arc<MysqlDatabase>> {
    let url = std::env::var("PACT_TEST_MYSQL_URL").ok()?;
    let db = MysqlDatabase::new(&url).await.expect("No se pudo conectar a MySQL");
    db.reset().await.expect("No se pudo limpiar la base de datos");
    Some(Arc::new(db))
}

#[tokio::test]
async fn test_mysql_crud_and_trash() {
    let Some(db) = setup().await else {
        eprintln!("skip: PACT_TEST_MYSQL_URL no definida");
        return;
    };

    let user_id = db.create_user(&CreateUser {
        username: "mysql_user".to_string(),
        master_hash: "hash".to_string(),
        salt: "salt".to_string(),
        color: Some("#123456".to_string()),
    }).await.unwrap();

    let folder_id = db.create_folder(&CreateFolder {
        user_id,
        name: "Cifrado".to_string(),
        color: Some("#000000".to_string()),
        icon: Some("folder".to_string()),
        uid: Some("mysql-folder-uid".to_string()),
    }).await.unwrap();

    let acc_id = db.create_account(&CreateAccount {
        user_id,
        folder_id: Some(folder_id),
        title: "Cuenta".to_string(),
        url: Some("example.com".to_string()),
        username_site: Some("user".to_string()),
        encrypted_pass: "blob".to_string(),
        is_favorite: false,
        icon_data: None,
        uid: Some("mysql-account-uid".to_string()),
    }).await.unwrap();

    let accounts = db.get_accounts_by_user(user_id).await.unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].version, 1);

    db.delete_account(acc_id).await.unwrap();
    let accounts = db.get_accounts_by_user(user_id).await.unwrap();
    assert!(accounts[0].deleted_at.is_some());
    assert_eq!(accounts[0].version, 2); // el borrado incrementa la versión

    db.restore_account(acc_id).await.unwrap();
    let accounts = db.get_accounts_by_user(user_id).await.unwrap();
    assert!(accounts[0].deleted_at.is_none());

    db.delete_folder(folder_id).await.unwrap();
    assert!(db.get_folders_by_user(user_id).await.unwrap().is_empty());
}

#[tokio::test]
async fn test_mysql_totp_upsert_and_delete() {
    let Some(db) = setup().await else {
        eprintln!("skip: PACT_TEST_MYSQL_URL no definida");
        return;
    };

    let user_id = db.create_user(&CreateUser {
        username: "mysql_totp".to_string(),
        master_hash: "h".to_string(),
        salt: "s".to_string(),
        color: None,
    }).await.unwrap();

    let acc_id = db.create_account(&CreateAccount {
        user_id,
        folder_id: None,
        title: "2FA".to_string(),
        url: None,
        username_site: None,
        encrypted_pass: "p".to_string(),
        is_favorite: false,
        icon_data: None,
        uid: Some("mysql-totp-uid".to_string()),
    }).await.unwrap();

    db.create_totp_setting(acc_id, "seed-1", None).await.unwrap();
    db.set_totp_setting(acc_id, "seed-2", None).await.unwrap();
    let setting = db.get_totp_setting_by_account(acc_id).await.unwrap().unwrap();
    assert_eq!(setting.encrypted_seed, "seed-2");

    db.delete_totp_setting_by_account(acc_id).await.unwrap();
    assert!(db.get_totp_setting_by_account(acc_id).await.unwrap().is_none());
}

#[tokio::test]
async fn test_mysql_sync_config_upsert() {
    let Some(db) = setup().await else {
        eprintln!("skip: PACT_TEST_MYSQL_URL no definida");
        return;
    };
    use pact_server::models::SyncConfigModel;

    let user_id = db.create_user(&CreateUser {
        username: "mysql_sync".to_string(),
        master_hash: "h".to_string(),
        salt: "s".to_string(),
        color: None,
    }).await.unwrap();

    let make = |host: &str| SyncConfigModel {
        user_id,
        remote_host: host.to_string(),
        remote_port: 3306,
        remote_user: "u".to_string(),
        remote_pass: "blob".to_string(),
        remote_name: "db".to_string(),
        preferred_mode: Some("remote".to_string()),
    };

    db.set_sync_config(user_id, &make("host-a")).await.unwrap();
    db.set_sync_config(user_id, &make("host-b")).await.unwrap();
    let cfg = db.get_sync_config(user_id).await.unwrap().unwrap();
    assert_eq!(cfg.remote_host, "host-b");
}
