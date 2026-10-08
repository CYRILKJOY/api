use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

pub enum ApiError {
    Internal(anyhow::Error),
    Database(sqlx::Error),
    NotFound(String),
    Unauthorized,
    Forbidden,
    InvalidRequest(String),
    SeatsUnavailable,
    HoldExpired,
    HoldNotFound,
    IdempotencyConflict,
    PaymentFailed,
}

// Rule #63: API Error Model with stable codes. Never expose raw database errors.
impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHENTICATED", "Authentication required".into()),
            ApiError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", "Insufficient permissions".into()),
            ApiError::InvalidRequest(msg) => (StatusCode::BAD_REQUEST, "INVALID_REQUEST", msg),
            ApiError::SeatsUnavailable => (StatusCode::CONFLICT, "SEATS_UNAVAILABLE", "Requested seats are no longer available".into()),
            ApiError::HoldExpired => (StatusCode::BAD_REQUEST, "HOLD_EXPIRED", "The seat hold has expired".into()),
            ApiError::HoldNotFound => (StatusCode::NOT_FOUND, "HOLD_NOT_FOUND", "Hold record not found".into()),
            ApiError::IdempotencyConflict => (StatusCode::CONFLICT, "IDEMPOTENCY_CONFLICT", "Conflict with previous idempotent request".into()),
            ApiError::PaymentFailed => (StatusCode::PAYMENT_REQUIRED, "PAYMENT_FAILED", "Payment processing failed".into()),
            ApiError::Database(err) => {
                // Log the real error securely, but return a generic internal error to the client
                tracing::error!("Database error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "An internal error occurred".into())
            }
            ApiError::Internal(err) => {
                tracing::error!("Internal error: {}", err);
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "An internal error occurred".into())
            }
        };

        let body = Json(json!({
            "error": {
                "code": error_code,
                "message": message
            }
        }));

        (status, body).into_response()
    }
}

impl From<sqlx::Error> for ApiError {
    fn from(err: sqlx::Error) -> Self {
        ApiError::Database(err)
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::Internal(err)
    }
}
