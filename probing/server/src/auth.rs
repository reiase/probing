use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use once_cell::sync::Lazy;
use std::env;

// Auth token environment variable name
pub const AUTH_TOKEN_ENV: &str = "PROBING_AUTH_TOKEN";

// Static variable to hold the configured token
pub static AUTH_TOKEN: Lazy<Option<String>> = Lazy::new(|| {
    env::var(AUTH_TOKEN_ENV).ok().filter(|s| !s.is_empty())
});

/// Check if authentication is required
pub fn is_auth_required() -> bool {
    AUTH_TOKEN.is_some()
}

/// Get the auth token from the request
fn get_token_from_request(headers: &HeaderMap) -> Option<String> {
    headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            if value.starts_with("Bearer ") {
                Some(value[7..].to_string())
            } else {
                None
            }
        })
        .or_else(|| {
            headers
                .get("X-Probing-Token")
                .and_then(|value| value.to_str().ok())
                .map(|s| s.to_string())
        })
}

/// Authentication middleware
pub async fn auth_middleware(request: Request, next: Next) -> Result<Response, StatusCode> {
    // Skip authentication if no token is configured
    if !is_auth_required() {
        return Ok(next.run(request).await);
    }

    // Get the configured token
    let configured_token = AUTH_TOKEN.as_ref().unwrap();

    // Extract token from the request
    let provided_token = get_token_from_request(request.headers());

    // Check if token matches
    match provided_token {
        Some(token) if token == *configured_token => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
