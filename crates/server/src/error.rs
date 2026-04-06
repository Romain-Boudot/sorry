use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Application error type — converts cleanly into HTTP responses.
#[derive(Debug)]
pub enum AppError {
    /// 400
    BadRequest(String),
    /// 401
    Unauthorized,
    /// 403
    Forbidden,
    /// 404
    NotFound,
    /// 409
    Conflict,
    /// 410
    Gone,
    /// 413
    PayloadTooLarge,
    /// 415
    UnsupportedMediaType,
    /// 429
    TooManyRequests,
    /// 500 with context
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            AppError::BadRequest(m) => (StatusCode::BAD_REQUEST, m.as_str()),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "Forbidden"),
            AppError::NotFound => (StatusCode::NOT_FOUND, "Not found"),
            AppError::Conflict => (StatusCode::CONFLICT, "Conflict"),
            AppError::Gone => (StatusCode::GONE, "Gone"),
            AppError::PayloadTooLarge => (StatusCode::PAYLOAD_TOO_LARGE, "Payload too large"),
            AppError::UnsupportedMediaType => (StatusCode::UNSUPPORTED_MEDIA_TYPE, "Unsupported media type"),
            AppError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, "Too many requests"),
            AppError::Internal(m) => {
                tracing::error!("Internal error: {m}");
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };
        (status, msg.to_string()).into_response()
    }
}

/// Convert sqlx errors into AppError::Internal
impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Internal(e.to_string())
    }
}

