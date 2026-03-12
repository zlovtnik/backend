use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use serde_json::json;
use serde_json::to_vec;
use tonic::Code;

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
    #[error(transparent)]
    ServiceError(#[from] crate::error::ServiceError),
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("internal: {0}")]
    Internal(#[from] anyhow::Error),
}

fn code_to_grpc_code(status_code: StatusCode) -> Code {
    match status_code {
        StatusCode::BAD_REQUEST => Code::InvalidArgument,
        StatusCode::UNAUTHORIZED => Code::Unauthenticated,
        StatusCode::NOT_FOUND => Code::NotFound,
        StatusCode::CONFLICT => Code::AlreadyExists,
        StatusCode::UNPROCESSABLE_ENTITY => Code::InvalidArgument,
        StatusCode::FORBIDDEN => Code::PermissionDenied,
        StatusCode::INTERNAL_SERVER_ERROR => Code::Internal,
        _ => Code::Internal,
    }
}

impl ResponseError for AppError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::NotFound(_) => StatusCode::NOT_FOUND,
            Self::Validation(_) => StatusCode::UNPROCESSABLE_ENTITY,
            Self::Unauthorized => StatusCode::UNAUTHORIZED,
            Self::ServiceError(err) => err.http_status(),
            Self::Conflict(_) => StatusCode::CONFLICT,
            Self::Database(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn error_response(&self) -> HttpResponse {
        tracing::error!(error = %self);
        if let Self::ServiceError(err) = self {
            return err.error_response();
        }

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
            AppError::ServiceError(service_error) => {
                let status_code = service_error.http_status();
                let envelope = crate::error::ErrorEnvelope::from_error(&service_error);
                let details = to_vec(&envelope).unwrap_or_else(|_| {
                    serde_json::to_vec(&serde_json::json!({
                        "message": envelope.message,
                    }))
                    .unwrap_or_default()
                });

                tonic::Status::with_details(
                    code_to_grpc_code(status_code),
                    envelope.message,
                    details.into(),
                )
            }
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
