use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("No autorizado")]
    Unauthorized,

    #[error("No encontrado")]
    NotFound,

    #[error("Error de base de datos: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Error interno: {0}")]
    Internal(#[from] anyhow::Error),

    #[error("{0}")]
    BadRequest(String),
}

#[derive(Serialize)]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::DatabaseError(ref e) => {
                tracing::error!("Error de base de datos: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Error en la base de datos".to_string())
            }
            AppError::Internal(ref e) => {
                // El detalle se registra pero no se expone al cliente.
                tracing::error!("Error interno: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, "Error interno del servidor".to_string())
            }
            AppError::BadRequest(ref s) => (StatusCode::BAD_REQUEST, s.clone()),
        };

        let body = Json(ErrorResponse {
            error: error_message,
        });

        (status, body).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
