use axum::{
    extract::Request,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use once_cell::sync::Lazy;
use std::env;

// Auth token environment variable name
pub const AUTH_TOKEN_ENV: &str = "PROBING_AUTH_TOKEN";
pub const AUTH_USERNAME_ENV: &str = "PROBING_AUTH_USERNAME"; // Optional, default is "admin"
pub const AUTH_REALM_ENV: &str = "PROBING_AUTH_REALM"; // Optional, default is "Probe Server"

// Static variable to hold the configured token
pub static AUTH_TOKEN: Lazy<Option<String>> =
    Lazy::new(|| env::var(AUTH_TOKEN_ENV).ok().filter(|s| !s.is_empty()));

pub static AUTH_USERNAME: Lazy<String> =
    Lazy::new(|| env::var(AUTH_USERNAME_ENV).unwrap_or_else(|_| "admin".to_string()));

pub static AUTH_REALM: Lazy<String> =
    Lazy::new(|| env::var(AUTH_REALM_ENV).unwrap_or_else(|_| "Probe Server".to_string()));

/// Check if authentication is required
pub fn is_auth_required() -> bool {
    AUTH_TOKEN.is_some()
}

/// Get the auth token from the request
fn get_token_from_request(headers: &HeaderMap) -> Option<String> {
    // Try Bearer token first
    let bearer_token = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            if value.starts_with("Bearer ") {
                Some(value[7..].to_string())
            } else {
                None
            }
        });

    if bearer_token.is_some() {
        return bearer_token;
    }

    // Try Basic Auth
    let basic_auth = headers
        .get("Authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| {
            if value.starts_with("Basic ") {
                Some(value[6..].to_string())
            } else {
                None
            }
        })
        .and_then(|base64_value| BASE64.decode(base64_value).ok())
        .and_then(|decoded| String::from_utf8(decoded).ok())
        .and_then(|credentials| {
            // Basic auth format is "username:password"
            let parts: Vec<&str> = credentials.splitn(2, ':').collect();
            if parts.len() == 2 && parts[0] == AUTH_USERNAME.as_str() {
                Some(parts[1].to_string())
            } else {
                None
            }
        });

    if basic_auth.is_some() {
        return basic_auth;
    }

    // Finally try custom header
    headers
        .get("X-Probing-Token")
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string())
}

/// Create a response that prompts the browser to show a login dialog
fn unauthorized_response() -> Response {
    let realm = format!("Basic realm=\"{}\"", AUTH_REALM.as_str());

    let response = (
        StatusCode::UNAUTHORIZED,
        [
            (
                header::WWW_AUTHENTICATE,
                HeaderValue::from_str(&realm).unwrap(),
            ),
            (header::CONTENT_TYPE, HeaderValue::from_static("text/plain")),
        ],
        "Unauthorized: Please login to access this resource",
    )
        .into_response();

    response
}

/// Authentication middleware
pub async fn auth_middleware(request: Request, next: Next) -> Result<Response, impl IntoResponse> {
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
        _ => Err(unauthorized_response()),
    }
}

// Path prefixes that should bypass authentication
pub fn is_public_path(path: &str) -> bool {
    // Allow static assets without authentication
    path.starts_with("/static/")
        || path == "/"
        || path == "/index.html"
        || path.starts_with("/favicon")
}

/// Selective auth middleware that skips authentication for specific paths
pub async fn selective_auth_middleware(
    request: Request,
    next: Next,
) -> Result<Response, impl IntoResponse> {
    let path = request.uri().path();

    // Skip authentication for public paths
    if is_public_path(path) {
        return Ok(next.run(request).await);
    }

    // Apply authentication for all other paths
    auth_middleware(request, next).await
}
