use axum::{
    extract::{FromRequestParts, FromRef},
    http::request::Parts,
    async_trait,
};
use crate::error::AppError;
use crate::auth::session::{SharedSessionManager, Session};

pub struct AuthenticatedUser(pub Session);

#[async_trait]
impl<S> FromRequestParts<S> for AuthenticatedUser
where
    SharedSessionManager: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session_mgr = SharedSessionManager::from_ref(state);
        
        let auth_header = parts.headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        if !auth_header.starts_with("Bearer ") {
            return Err(AppError::Unauthorized);
        }

        let token = &auth_header[7..];
        let session = session_mgr.get_session(token).ok_or(AppError::Unauthorized)?;

        Ok(AuthenticatedUser(session))
    }
}
