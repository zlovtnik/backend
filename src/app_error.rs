use actix_web::{
    http::StatusCode,
    HttpResponse, ResponseError,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid input: {0}")]
    Validation(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("internal: {0}")]
    Internal(#[from] anyhow::Error),
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Database(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        tracing::error!(error = %self);
        let public_message = match self {
            Self::Database(_) => "database error".to_string(),
            Self::Internal(_) => "internal server error".to_string(),
            _ => self.to_string(),
        };

        HttpResponse::build(self.status_code()).json(json!({ "error": public_message }))
    }
}

impl From<AppError> for tonic::Status {
    fn from(value: AppError) -> Self {
        match value {
            AppError::NotFound(m) => tonic::Status::not_found(m),
            AppError::Validation(m) => tonic::Status::invalid_argument(m),
            AppError::Unauthorized => tonic::Status::unauthenticated("unauthorized"),
            AppError::Conflict(m) => tonic::Status::already_exists(m),
            AppError::Database(e) => {
                tracing::error!(error = %e, "database error mapped to grpc internal");
                tonic::Status::internal("internal server error")
            }
            AppError::Internal(e) => {
                tracing::error!(error = %e, "internal error mapped to grpc internal");
                tonic::Status::internal("internal server error")
            }
        }
    }
}

impl From<crate::error::ServiceError> for AppError {
    fn from(value: crate::error::ServiceError) -> Self {
        match value {
            crate::error::ServiceError::Unauthorized { .. } => Self::Unauthorized,
            crate::error::ServiceError::BadRequest { error_message, .. } => {
                Self::Validation(error_message)
            }
            crate::error::ServiceError::NotFound { error_message, .. } => {
                Self::NotFound(error_message)
            }
            crate::error::ServiceError::Conflict { error_message, .. } => {
                Self::Conflict(error_message)
            }
            crate::error::ServiceError::InternalServerError { error_message, .. } => {
                Self::Internal(anyhow::anyhow!(error_message))
            }
        }
    }
}
