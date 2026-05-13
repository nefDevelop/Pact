use serde::{Serialize, Deserialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct Folder {
    pub id: i32,
    pub uid: String, 
    pub user_id: i32,
    pub name: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CreateFolder {
    pub user_id: i32,
    pub name: String,
    pub color: Option<String>,
    pub icon: Option<String>,
    pub uid: Option<String>,
}

impl From<Folder> for CreateFolder {
    fn from(f: Folder) -> Self {
        Self {
            user_id: f.user_id,
            name: f.name,
            color: f.color,
            icon: f.icon,
            uid: Some(f.uid),
        }
    }
}
