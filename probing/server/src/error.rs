use anyhow::Result;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

/// Shared error type for HTTP API responses
#[derive(Debug)]
pub struct ApiError(pub anyhow::Error);

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0),
        )
            .into_response()
    }
}

impl<E> From<E> for ApiError
where
    E: Into<anyhow::Error>,
{
    fn from(err: E) -> Self {
        Self(err.into())
    }
}

/// Alias for convenience
pub type ApiResult<T> = Result<T, ApiError>;
