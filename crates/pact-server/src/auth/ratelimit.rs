use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct RateLimiter {
    attempts: DashMap<String, Vec<Instant>>,
    max_attempts: usize,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_attempts: usize, window_secs: u64) -> Self {
        Self {
            attempts: DashMap::new(),
            max_attempts,
            window: Duration::from_secs(window_secs),
        }
    }

    pub fn check(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut entry = self.attempts.entry(key.to_string()).or_default();

        entry.retain(|t| now.duration_since(*t) < self.window);

        if entry.len() >= self.max_attempts {
            return false;
        }

        entry.push(now);
        true
    }

    pub fn remaining(&self, key: &str) -> usize {
        let now = Instant::now();
        if let Some(entry) = self.attempts.get(key) {
            let active = entry.iter().filter(|t| now.duration_since(**t) < self.window).count();
            self.max_attempts.saturating_sub(active)
        } else {
            self.max_attempts
        }
    }

    pub fn cleanup_expired(&self) {
        let now = Instant::now();
        self.attempts.retain(|_, entries| {
            entries.retain(|t| now.duration_since(*t) < self.window);
            !entries.is_empty()
        });
    }
}

pub type SharedRateLimiter = Arc<RateLimiter>;

pub fn auth_rate_limiter() -> SharedRateLimiter {
    Arc::new(RateLimiter::new(5, 60))
}

/// Limiter para endpoints pre-login menos sensibles (salt, listado de usuarios).
/// Más permisivo que el de autenticación para no romper la carga normal de la UI.
pub fn public_rate_limiter() -> SharedRateLimiter {
    Arc::new(RateLimiter::new(60, 60))
}
