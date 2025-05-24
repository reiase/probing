use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use http_body_util::BodyExt;
use bytes::Bytes;
use super::config::get_max_request_body_size;

/// Middleware to limit request body size
pub async fn request_size_limit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, Response> {
    let max_size = get_max_request_body_size();
    
    // Get the content-length header if present
    let content_length = request
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<usize>().ok());

    // If content-length is present and exceeds limit, reject immediately
    if let Some(length) = content_length {
        if length > max_size {
            log::warn!(
                "Request rejected: Content-Length {} exceeds limit {}",
                length,
                max_size
            );
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("Request body too large (max {} bytes allowed)", max_size),
            )
                .into_response());
        }
    }

    // For requests without content-length or with acceptable content-length,
    // we need to check the actual body size
    let (parts, body) = request.into_parts();
    
    // Collect body with size limit
    let body_bytes = match collect_body_with_limit(body, max_size).await {
        Ok(bytes) => bytes,
        Err(e) => {
            log::warn!("Request body collection failed: {}", e);
            return Err((
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("Request body too large (max {} bytes allowed)", max_size),
            )
                .into_response());
        }
    };

    // Reconstruct the request with the limited body
    let new_body = Body::from(body_bytes);
    let new_request = Request::from_parts(parts, new_body);

    // Continue to the next middleware/handler
    Ok(next.run(new_request).await)
}

/// Collect body bytes with a size limit using BodyExt::collect()
async fn collect_body_with_limit(
    body: Body,
    limit: usize,
) -> Result<Bytes, &'static str> {
    // Use BodyExt::collect() which is already available
    let collected = body.collect().await
        .map_err(|_| "Failed to collect body")?;
    
    let bytes = collected.to_bytes();
    
    // Check size limit
    if bytes.len() > limit {
        return Err("Request body size limit exceeded");
    }
    
    Ok(bytes)
}

/// Middleware for logging requests (optional - for debugging)
pub async fn request_logging_middleware(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = std::time::Instant::now();

    log::debug!("Incoming request: {} {}", method, uri);

    let response = next.run(request).await;
    let duration = start.elapsed();

    log::debug!(
        "Request completed: {} {} - {} in {:?}",
        method,
        uri,
        response.status(),
        duration
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use bytes::Bytes;

    #[tokio::test]
    async fn test_collect_body_with_limit_success() {
        let body = Body::from("Hello, World!");
        let result = collect_body_with_limit(body, 100).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Bytes::from("Hello, World!"));
    }

    #[tokio::test]
    async fn test_collect_body_with_limit_exceeded() {
        let large_data = "x".repeat(1000);
        let body = Body::from(large_data);
        let result = collect_body_with_limit(body, 100).await;
        assert!(result.is_err());
    }
}
