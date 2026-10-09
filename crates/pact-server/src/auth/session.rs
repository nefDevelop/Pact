use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};

const MAX_SESSIONS_PER_USER: usize = 5;
const MAX_TOTAL_SESSIONS: usize = 1000;

#[derive(Debug, Clone)]
pub struct Session {
    pub username: String,
    pub last_activity: DateTime<Utc>,
}

pub struct SessionManager {
    // K: Session Token (UUID), V: Session data
    sessions: DashMap<String, Session>,
    session_timeout: Duration,
}

impl SessionManager {
    pub fn new() -> Self {
        Self::new_with_timeout(Duration::hours(1))
    }

    pub fn new_with_timeout(timeout: Duration) -> Self {
        Self {
            sessions: DashMap::new(),
            session_timeout: timeout,
        }
    }

    /// Crea una nueva sesión y retorna el token.
    /// Limita a MAX_SESSIONS_PER_USER por usuario y MAX_TOTAL_SESSIONS global.
    pub fn create_session(&self, username: String) -> Option<String> {
        self.cleanup_expired();

        if self.sessions.len() >= MAX_TOTAL_SESSIONS {
            tracing::warn!("Límite global de sesiones alcanzado ({})", MAX_TOTAL_SESSIONS);
            return None;
        }

        let user_count = self.sessions.iter().filter(|e| e.value().username == username).count();
        if user_count >= MAX_SESSIONS_PER_USER {
            tracing::warn!("Usuario '{}' alcanzó límite de sesiones ({})", username, MAX_SESSIONS_PER_USER);
            return None;
        }

        let token = Uuid::new_v4().to_string();
        self.sessions.insert(token.clone(), Session { 
            username,
            last_activity: Utc::now(),
        });
        Some(token)
    }

    /// Obtiene los datos de una sesión activa, validando expiración y actualizando actividad.
    pub fn get_session(&self, token: &str) -> Option<Session> {
        let now = Utc::now();
        
        let mut session_to_remove = false;
        let session = if let Some(mut entry) = self.sessions.get_mut(token) {
            if now.signed_duration_since(entry.last_activity) > self.session_timeout {
                session_to_remove = true;
                None
            } else {
                entry.last_activity = now;
                Some(entry.clone())
            }
        } else {
            None
        };

        if session_to_remove {
            self.sessions.remove(token);
        }

        session
    }

    /// Elimina una sesión (Bloqueo de bóveda).
    pub fn remove_session(&self, token: &str) {
        self.sessions.remove(token);
    }

    /// Invalida todas las sesiones de un usuario (útil al borrar usuario).
    pub fn invalidate_user_sessions(&self, username: &str) {
        self.sessions.retain(|_, session| session.username != username);
    }

    /// Retorna el número de sesiones activas.
    pub fn active_sessions(&self) -> usize {
        self.cleanup_expired();
        self.sessions.len()
    }

    /// Limpia sesiones que han expirado.
    pub fn cleanup_expired(&self) {
        let now = Utc::now();
        self.sessions.retain(|_, session| {
            now.signed_duration_since(session.last_activity) <= self.session_timeout
        });
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

pub type SharedSessionManager = Arc<SessionManager>;
