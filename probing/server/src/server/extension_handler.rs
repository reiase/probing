use std::collections::HashMap;

use axum::{
    http::StatusCode,
    response::{AppendHeaders, IntoResponse, Response},
};
use http_body_util::BodyExt;

use probing_core::core::EngineExtensionManager;

use super::error::ApiResult;
use crate::engine::ENGINE;

/// Handle extension API calls
#[axum::debug_handler]
pub async fn handle_extension_call(req: axum::extract::Request) -> ApiResult<Response> {
    let (parts, body) = req.into_parts();
    let path = parts.uri.path();
    let params_str = parts.uri.query().unwrap_or_default();
    let params: HashMap<String, String> =
        serde_urlencoded::from_str(params_str).unwrap_or_default();

    // Body size is already limited by middleware, so we can safely collect it
    let body_bytes = body.collect().await?.to_bytes();

    // Only log request details in debug mode to avoid log spam
    log::debug!(
        "Extension API Call[{}]: params = {:?}, body_size = {} bytes",
        path,
        params,
        body_bytes.len()
    );

    let eem = {
        let engine = ENGINE.write().await;
        let state = engine.context.state();
        state
            .config()
            .options()
            .extensions
            .get::<EngineExtensionManager>()
            .cloned()
    };

    if let Some(eem) = eem {
        match eem.call(path, &params, &body_bytes).await {
            Ok(response) => {
                // If response is a string, return it as plain text
                return Ok((
                    StatusCode::OK,
                    AppendHeaders([("Content-Type", "text/plain")]),
                    response,
                )
                    .into_response());
            }
            Err(e) => {
                log::warn!("Extension call failed for path '{path}': {e}");
                return Err(anyhow::anyhow!("Extension call failed: {}", e).into());
            }
        }
    }

    // Return 404 if no extension manager is available
    Ok((StatusCode::NOT_FOUND, "Extension not found").into_response())
}
