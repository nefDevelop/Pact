use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Account {
    pub id: i32,
    pub uid: String,
    pub user_id: i32,
    pub folder_id: i32,
    pub is_favorite: bool,
    pub title: String,
    pub url: Option<String>,
    pub username_site: Option<String>,
    pub encrypted_pass: String,
    pub icon_data: Option<String>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct TOTPSetting {
    pub id: i32,
    pub account_id: i32,
    pub encrypted_seed: String,
    pub issuer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CreateAccount {
    pub user_id: i32,
    pub folder_id: i32,
    pub title: String,
    pub url: Option<String>,
    pub username_site: Option<String>,
    pub encrypted_pass: String,
    pub is_favorite: bool,
    pub icon_data: Option<String>,
    pub uid: Option<String>,
}

impl From<Account> for CreateAccount {
    fn from(a: Account) -> Self {
        Self {
            user_id: a.user_id,
            folder_id: a.folder_id,
            title: a.title,
            url: a.url,
            username_site: a.username_site,
            encrypted_pass: a.encrypted_pass,
            is_favorite: a.is_favorite,
            icon_data: a.icon_data,
            uid: Some(a.uid),
        }
    }
}
