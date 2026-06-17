pub mod crypto;
pub mod db;
pub mod error;
pub mod models;
pub mod auth;
pub mod vault;
pub mod storage;
pub mod schemas;
pub mod sync;

use std::sync::Arc;
use crate::db::Database;
use crate::auth::session::SharedSessionManager;
use crate::storage::StorageManager;
use axum::extract::FromRef;

// Estado global de la aplicación
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<dyn Database>,
    pub session_mgr: SharedSessionManager,
    pub storage: Arc<StorageManager>,
    pub machine_id: String,
    pub sync_crypto: Arc<crate::crypto::SyncCrypto>,
    pub auth_limiter: crate::auth::ratelimit::SharedRateLimiter,
    pub db_type: String,
}

// Implementación de FromRef para permitir extraer partes del estado individualmente en los handlers
impl FromRef<AppState> for Arc<dyn Database> {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}

impl FromRef<AppState> for SharedSessionManager {
    fn from_ref(state: &AppState) -> Self {
        state.session_mgr.clone()
    }
}

impl FromRef<AppState> for Arc<StorageManager> {
    fn from_ref(state: &AppState) -> Self {
        state.storage.clone()
    }
}

impl FromRef<AppState> for Arc<crate::crypto::SyncCrypto> {
    fn from_ref(state: &AppState) -> Self {
        state.sync_crypto.clone()
    }
}

impl FromRef<AppState> for crate::auth::ratelimit::SharedRateLimiter {
    fn from_ref(state: &AppState) -> Self {
        state.auth_limiter.clone()
    }
}
