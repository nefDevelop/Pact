use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc, Duration};

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
        Self {
            sessions: DashMap::new(),
            session_timeout: Duration::hours(1),
        }
    }

    /// Crea una nueva sesión y retorna el token.
    pub fn create_session(&self, username: String) -> String {
        let token = Uuid::new_v4().to_string();
        self.sessions.insert(token.clone(), Session { 
            username,
            last_activity: Utc::now(),
        });
        token
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

    /// Retorna el número de sesiones activas.
    pub fn active_sessions(&self) -> usize {
        // Limpiamos sesiones expiradas antes de contar
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

pub type SharedSessionManager = Arc<SessionManager>;
