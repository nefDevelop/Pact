use async_trait::async_trait;
use sqlx::{sqlite::SqlitePool, mysql::MySqlPool, Error};
use crate::models::{User, CreateUser, Folder, CreateFolder, Account, CreateAccount, SyncConfigModel, PendingAccountModel, TOTPSetting};
use uuid::Uuid;

#[async_trait]
pub trait Database: Send + Sync {
    // Usuarios
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Error>;
    async fn get_all_users(&self) -> Result<Vec<User>, Error>;
    async fn create_user(&self, user: &CreateUser) -> Result<i32, Error>;
    async fn update_user(&self, username: &str, new_name: &str, color: &str) -> Result<(), Error>;
    async fn delete_user(&self, username: &str) -> Result<(), Error>;
    
    // Carpetas
    async fn get_folders_by_user(&self, user_id: i32) -> Result<Vec<Folder>, Error>;
    async fn get_folder_by_uid(&self, user_id: i32, uid: &str) -> Result<Option<Folder>, Error>;
    async fn create_folder(&self, folder: &CreateFolder) -> Result<i32, Error>;
    async fn update_folder(&self, id: i32, folder: &CreateFolder, version: i32) -> Result<(), Error>;
    async fn delete_folder(&self, id: i32) -> Result<(), Error>;
    
    // Cuentas
    async fn get_accounts_by_user(&self, user_id: i32) -> Result<Vec<Account>, Error>;
    async fn get_account_by_uid(&self, user_id: i32, uid: &str) -> Result<Option<Account>, Error>;
    async fn create_account(&self, account: &CreateAccount) -> Result<i32, Error>;
    async fn update_account(&self, id: i32, account: &CreateAccount, version: i32) -> Result<(), Error>;
    async fn delete_account(&self, id: i32) -> Result<(), Error>;
    async fn restore_account(&self, id: i32) -> Result<(), Error>;
    async fn empty_trash(&self, user_id: i32) -> Result<i32, Error>;

    // Configuración
    async fn get_sync_config(&self, user_id: i32) -> Result<Option<SyncConfigModel>, Error>;
    async fn set_sync_config(&self, user_id: i32, config: &SyncConfigModel) -> Result<(), Error>;

    // Cuentas Pendientes (Secure Inbox)
    async fn get_pending_accounts(&self, username: &str) -> Result<Vec<PendingAccountModel>, Error>;
    async fn delete_pending_account(&self, id: i32) -> Result<(), Error>;
    async fn get_pending_count(&self, username: &str) -> Result<i32, Error>;

    // TOTP
    async fn create_totp_setting(&self, account_id: i32, encrypted_seed: &str, issuer: Option<&str>) -> Result<(), Error>;
    async fn get_totp_setting_by_account(&self, account_id: i32) -> Result<Option<TOTPSetting>, Error>;

    // Sistema
    async fn reset(&self) -> Result<(), Error>;
}

macro_rules! impl_database {
    (
        $ty:ty,
        $pool:ty,
        last_insert_id_method: $last_insert_id_method:ident,
        $result_ty:ty,
        timestamp: $timestamp:expr,
        upsert_clause: $upsert:expr,
        count_ty: $count_ty:ty,
    ) => {
        #[async_trait]
        impl Database for $ty {
            async fn reset(&self) -> Result<(), Error> {
                sqlx::query("DELETE FROM accounts").execute(&self.pool).await?;
                sqlx::query("DELETE FROM folders").execute(&self.pool).await?;
                sqlx::query("DELETE FROM sync_configs").execute(&self.pool).await?;
                sqlx::query("DELETE FROM pending_accounts").execute(&self.pool).await?;
                sqlx::query("DELETE FROM users").execute(&self.pool).await?;
                Ok(())
            }

            async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Error> {
                let t0 = std::time::Instant::now();
                let res = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
                    .bind(username)
                    .fetch_optional(&self.pool)
                    .await;
                tracing::trace!("SQL get_user_by_username ({}) completado en {:?}", username, t0.elapsed());
                res
            }

            async fn get_all_users(&self) -> Result<Vec<User>, Error> {
                let t0 = std::time::Instant::now();
                let res = sqlx::query_as::<_, User>("SELECT * FROM users")
                    .fetch_all(&self.pool)
                    .await;
                tracing::trace!("SQL get_all_users completado en {:?}", t0.elapsed());
                res
            }

            async fn create_user(&self, user: &CreateUser) -> Result<i32, Error> {
                let res = sqlx::query("INSERT INTO users (username, master_hash, salt, color) VALUES (?, ?, ?, ?)")
                    .bind(&user.username)
                    .bind(&user.master_hash)
                    .bind(&user.salt)
                    .bind(&user.color)
                    .execute(&self.pool)
                    .await?;
                Ok(res.$last_insert_id_method() as i32)
            }

            async fn update_user(&self, username: &str, new_name: &str, color: &str) -> Result<(), Error> {
                sqlx::query("UPDATE users SET username = ?, color = ? WHERE username = ?")
                    .bind(new_name)
                    .bind(color)
                    .bind(username)
                    .execute(&self.pool)
                    .await?;
                Ok(())
            }

            async fn delete_user(&self, username: &str) -> Result<(), Error> {
                sqlx::query("DELETE FROM users WHERE username = ?")
                    .bind(username)
                    .execute(&self.pool)
                    .await?;
                Ok(())
            }

            async fn get_folders_by_user(&self, user_id: i32) -> Result<Vec<Folder>, Error> {
                sqlx::query_as::<_, Folder>("SELECT * FROM folders WHERE user_id = ?")
                    .bind(user_id)
                    .fetch_all(&self.pool)
                    .await
            }

            async fn get_folder_by_uid(&self, _user_id: i32, uid: &str) -> Result<Option<Folder>, Error> {
                sqlx::query_as::<_, Folder>("SELECT * FROM folders WHERE uid = ?")
                    .bind(uid)
                    .fetch_optional(&self.pool)
                    .await
            }

            async fn create_folder(&self, folder: &CreateFolder) -> Result<i32, Error> {
                let uid = folder.uid.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
                let res = sqlx::query("INSERT INTO folders (uid, user_id, name, color, icon) VALUES (?, ?, ?, ?, ?)")
                    .bind(uid)
                    .bind(folder.user_id)
                    .bind(&folder.name)
                    .bind(&folder.color)
                    .bind(&folder.icon)
                    .execute(&self.pool)
                    .await?;
                Ok(res.$last_insert_id_method() as i32)
            }

            async fn update_folder(&self, id: i32, folder: &CreateFolder, version: i32) -> Result<(), Error> {
                sqlx::query("UPDATE folders SET name = ?, color = ?, icon = ?, version = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(&folder.name)
                    .bind(&folder.color)
                    .bind(&folder.icon)
                    .bind(version)
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
                Ok(())
            }

            async fn delete_folder(&self, id: i32) -> Result<(), Error> {
                sqlx::query("DELETE FROM folders WHERE id = ?")
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
                Ok(())
            }

            async fn get_accounts_by_user(&self, user_id: i32) -> Result<Vec<Account>, Error> {
                sqlx::query_as::<_, Account>(
                    "SELECT a.*, CASE WHEN t.id IS NOT NULL THEN 1 ELSE 0 END as has_totp \
                     FROM accounts a \
                     LEFT JOIN totp_settings t ON a.id = t.account_id \
                     WHERE a.user_id = ?"
                )
                    .bind(user_id)
                    .fetch_all(&self.pool)
                    .await
            }

            async fn get_account_by_uid(&self, _user_id: i32, uid: &str) -> Result<Option<Account>, Error> {
                sqlx::query_as::<_, Account>(
                    "SELECT a.*, CASE WHEN t.id IS NOT NULL THEN 1 ELSE 0 END as has_totp \
                     FROM accounts a \
                     LEFT JOIN totp_settings t ON a.id = t.account_id \
                     WHERE a.uid = ?"
                )
                    .bind(uid)
                    .fetch_optional(&self.pool)
                    .await
            }

            async fn create_account(&self, account: &CreateAccount) -> Result<i32, Error> {
                let uid = account.uid.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
                let res = sqlx::query("INSERT INTO accounts (uid, user_id, folder_id, title, url, username_site, encrypted_pass, is_favorite, icon_data) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
                    .bind(uid)
                    .bind(account.user_id)
                    .bind(account.folder_id)
                    .bind(&account.title)
                    .bind(&account.url)
                    .bind(&account.username_site)
                    .bind(&account.encrypted_pass)
                    .bind(account.is_favorite)
                    .bind(&account.icon_data)
                    .execute(&self.pool)
                    .await?;
                Ok(res.$last_insert_id_method() as i32)
            }

            async fn update_account(&self, id: i32, account: &CreateAccount, version: i32) -> Result<(), Error> {
                sqlx::query("UPDATE accounts SET folder_id = ?, title = ?, url = ?, username_site = ?, encrypted_pass = ?, is_favorite = ?, icon_data = ?, version = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(account.folder_id)
                    .bind(&account.title)
                    .bind(&account.url)
                    .bind(&account.username_site)
                    .bind(&account.encrypted_pass)
                    .bind(account.is_favorite)
                    .bind(&account.icon_data)
                    .bind(version)
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
                Ok(())
            }

            async fn delete_account(&self, id: i32) -> Result<(), Error> {
                let sql = format!("UPDATE accounts SET deleted_at = {} WHERE id = ?", $timestamp);
                sqlx::query(&sql)
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
                Ok(())
            }

            async fn restore_account(&self, id: i32) -> Result<(), Error> {
                sqlx::query("UPDATE accounts SET deleted_at = NULL WHERE id = ?")
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
                Ok(())
            }

            async fn empty_trash(&self, user_id: i32) -> Result<i32, Error> {
                let res = sqlx::query("DELETE FROM accounts WHERE user_id = ? AND deleted_at IS NOT NULL")
                    .bind(user_id)
                    .execute(&self.pool)
                    .await?;
                Ok(res.rows_affected() as i32)
            }

            async fn get_sync_config(&self, user_id: i32) -> Result<Option<SyncConfigModel>, Error> {
                sqlx::query_as::<_, SyncConfigModel>("SELECT * FROM sync_configs WHERE user_id = ?")
                    .bind(user_id)
                    .fetch_optional(&self.pool)
                    .await
            }

            async fn set_sync_config(&self, user_id: i32, config: &SyncConfigModel) -> Result<(), Error> {
                let sql = format!(
                    "INSERT INTO sync_configs (user_id, remote_host, remote_port, remote_user, remote_pass, remote_name, preferred_mode) 
                     VALUES (?, ?, ?, ?, ?, ?, ?) {}",
                    $upsert
                );
                sqlx::query(&sql)
                    .bind(user_id)
                    .bind(&config.remote_host)
                    .bind(config.remote_port)
                    .bind(&config.remote_user)
                    .bind(&config.remote_pass)
                    .bind(&config.remote_name)
                    .bind(&config.preferred_mode)
                    .execute(&self.pool)
                    .await?;
                Ok(())
            }

            async fn get_pending_accounts(&self, username: &str) -> Result<Vec<PendingAccountModel>, Error> {
                sqlx::query_as::<_, PendingAccountModel>("SELECT * FROM pending_accounts WHERE username = ?")
                    .bind(username)
                    .fetch_all(&self.pool)
                    .await
            }

            async fn delete_pending_account(&self, id: i32) -> Result<(), Error> {
                sqlx::query("DELETE FROM pending_accounts WHERE id = ?")
                    .bind(id)
                    .execute(&self.pool)
                    .await?;
                Ok(())
            }

            async fn get_pending_count(&self, username: &str) -> Result<i32, Error> {
                let row: ($count_ty,) = sqlx::query_as("SELECT COUNT(*) FROM pending_accounts WHERE username = ?")
                    .bind(username)
                    .fetch_one(&self.pool)
                    .await?;
                Ok(row.0 as i32)
            }

            async fn create_totp_setting(&self, account_id: i32, encrypted_seed: &str, issuer: Option<&str>) -> Result<(), Error> {
                sqlx::query("INSERT INTO totp_settings (account_id, encrypted_seed, issuer) VALUES (?, ?, ?)")
                    .bind(account_id)
                    .bind(encrypted_seed)
                    .bind(issuer)
                    .execute(&self.pool)
                    .await?;
                Ok(())
            }

            async fn get_totp_setting_by_account(&self, account_id: i32) -> Result<Option<TOTPSetting>, Error> {
                sqlx::query_as::<_, TOTPSetting>("SELECT * FROM totp_settings WHERE account_id = ?")
                    .bind(account_id)
                    .fetch_optional(&self.pool)
                    .await
            }
        }
    };
}

pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub async fn new(url: &str) -> Result<Self, Error> {
        let pool = SqlitePool::connect(url).await?;
        Ok(Self { pool })
    }

    pub async fn migrate(&self) -> Result<(), Error> {
        sqlx::migrate!("../../migrations").run(&self.pool).await?;
        Ok(())
    }
}

impl_database!(
    SqliteDatabase,
    SqlitePool,
    last_insert_id_method: last_insert_rowid,
    sqlx::sqlite::SqliteQueryResult,
    timestamp: "CURRENT_TIMESTAMP",
    upsert_clause: "ON CONFLICT(user_id) DO UPDATE SET remote_host = excluded.remote_host, remote_port = excluded.remote_port, remote_user = excluded.remote_user, remote_pass = excluded.remote_pass, remote_name = excluded.remote_name, preferred_mode = excluded.preferred_mode",
    count_ty: i32,
);

pub struct MysqlDatabase {
    pool: MySqlPool,
}

impl MysqlDatabase {
    pub async fn new(url: &str) -> Result<Self, Error> {
        let pool = MySqlPool::connect(url).await?;
        Ok(Self { pool })
    }
}

impl_database!(
    MysqlDatabase,
    MySqlPool,
    last_insert_id_method: last_insert_id,
    sqlx::mysql::MySqlQueryResult,
    timestamp: "NOW()",
    upsert_clause: "ON DUPLICATE KEY UPDATE remote_host = VALUES(remote_host), remote_port = VALUES(remote_port), remote_user = VALUES(remote_user), remote_pass = VALUES(remote_pass), remote_name = VALUES(remote_name), preferred_mode = VALUES(preferred_mode)",
    count_ty: i64,
);
