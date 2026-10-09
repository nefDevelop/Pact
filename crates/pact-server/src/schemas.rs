use serde::{Serialize, Deserialize};

#[derive(Debug, Deserialize)]
pub struct UserRegister {
    pub username: String,
    pub master_password: String,
    pub salt: Option<String>,
    pub color: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserLogin {
    pub username: String,
    pub master_password: String,
}

#[derive(Debug, Deserialize)]
pub struct UserUpdate {
    pub old_name: String,
    pub new_name: String,
    pub new_color: String,
}

#[derive(Debug, Deserialize)]
pub struct UserDelete {
    pub username: String,
    pub master_password: String,
}

#[derive(Debug, Deserialize)]
pub struct MasterPasswordOnly {
    pub master_password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub status: String,
    pub username: String,
    pub token: String,
    pub color: Option<String>,
    pub source: String, // "local", "remote", "hybrid"
}

#[derive(Debug, Serialize)]
pub struct UsersResponse {
    pub users: Vec<UserListItem>,
    pub source: String,
    pub vault_path: String,
    pub vault_status: String,
}

#[derive(Debug, Serialize)]
pub struct UserListItem {
    pub id: i32,
    pub username: String,
    pub status: String,
    pub color: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PendingAccount {
    pub id: i32,
    pub username: String,
    pub data_blob: String,
}

#[derive(Debug, Deserialize)]
pub struct VaultPath {
    pub path: Option<String>,
    pub force_init: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Serialize)]
pub struct VaultExport {
    pub username: String,
    pub folders: Vec<crate::models::Folder>,
    pub accounts: Vec<crate::models::Account>,
}

// --- FOLDERS ---

#[derive(Debug, Deserialize)]
pub struct FolderCreate {
    pub name: String,
    pub color: Option<String>,
    pub icon: Option<String>,
}

// --- ACCOUNTS ---

#[derive(Debug, Deserialize)]
pub struct AccountCreate {
    pub folder_id: Option<i32>,
    pub title: String,
    pub url: Option<String>,
    pub username_site: Option<String>,
    pub encrypted_pass: String,
    pub totp_seed: Option<String>,
    #[serde(default)]
    pub is_favorite: bool,
    pub icon_data: Option<String>,
}

// --- SYNC ---

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SyncConfig {
    pub host: String,
    pub user: String,
    pub password: String,
    pub db_name: String,
    pub port: Option<u16>,
    pub preferred_mode: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncPlan {
    pub to_push_folders: Vec<String>, // UIDs
    pub to_pull_folders: Vec<String>,
    pub conflict_folders: Vec<String>,
    pub to_push_accounts: Vec<String>,
    pub to_pull_accounts: Vec<String>,
    pub conflict_accounts: Vec<String>,
}
