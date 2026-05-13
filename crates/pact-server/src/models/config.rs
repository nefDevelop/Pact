use serde::{Serialize, Deserialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct SyncConfigModel {
    pub user_id: i32,
    pub remote_host: String,
    pub remote_port: i32,
    pub remote_user: String,
    pub remote_pass: String,
    pub remote_name: String,
    pub preferred_mode: Option<String>,
}
