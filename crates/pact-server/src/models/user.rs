use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(Debug, Serialize, Deserialize, FromRow, Clone, Zeroize, ZeroizeOnDrop)]
pub struct User {
    pub id: i32,
    #[zeroize(skip)] // Username is not as sensitive
    pub username: String,
    pub master_hash: String,
    pub salt: String,
    #[zeroize(skip)]
    pub color: Option<String>,
    #[zeroize(skip)]
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Clone, Zeroize, ZeroizeOnDrop)]
pub struct CreateUser {
    pub username: String,
    pub master_hash: String,
    pub salt: String,
    pub color: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct PendingAccountModel {
    pub id: i32,
    pub username: String,
    pub data_blob: String,
}
