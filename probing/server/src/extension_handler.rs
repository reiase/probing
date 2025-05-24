use std::collections::HashMap;

use axum::{
    http::StatusCode,
    response::{AppendHeaders, IntoResponse, Response},
};
use http_body_util::BodyExt;

use probing_core::core::EngineExtensionManager;

use crate::engine_handler::ENGINE;
use crate::error::ApiResult;

/// Handle extension API calls
pub async fn handle_extension_call(req: axum::extract::Request) -> ApiResult<Response> {
    let (parts, body) = req.into_parts();
    let path = parts.uri.path();
    let params_str = parts.uri.query().unwrap_or_default();
    let params: HashMap<String, String> =
        serde_urlencoded::from_str(params_str).unwrap_or_default();
    let body = body.collect().await?.to_bytes().clone();

    log::info!(
        "API Call[{}]: params = {:?}, body = {:?}",
        path,
        params,
        body
    );

    let engine = ENGINE.write().await;
    let state = engine.context.state();
    let eem = state
        .config()
        .options()
        .extensions
        .get::<EngineExtensionManager>();
    
    if let Some(eem) = eem {
        match eem.call(path, &params, body.to_vec().as_slice()) {
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
                return Err(anyhow::anyhow!("Extension call failed: {}", e).into());
            }
        }
    }

    // Return 404 if no extension manager is available
    Ok((StatusCode::NOT_FOUND, "Extension not found").into_response())
}
