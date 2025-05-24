use axum::Router;

use super::{cluster, error::ApiResult, extension_handler, file_api, profiling, system};

/// API handler for getting system overview
async fn api_get_overview() -> ApiResult<String> {
    Ok(serde_json::to_string(&system::get_overview()?)?)
}

/// API handler for putting nodes (delegates to cluster module)
async fn api_put_nodes(axum::Json(payload): axum::Json<probing_proto::prelude::Node>) -> ApiResult<()> {
    cluster::put_node(payload).await
}

/// API handler for getting nodes (delegates to cluster module)
async fn api_get_nodes() -> ApiResult<String> {
    cluster::get_nodes().await
}

/// Main router for all API endpoints
pub fn apis_route() -> Router {
    Router::new()
        .route("/overview", axum::routing::get(api_get_overview))
        .route("/files", axum::routing::get(file_api::read_file))
        .route("/nodes", axum::routing::get(api_get_nodes).put(api_put_nodes))
        .route(
            "/flamegraph/torch",
            axum::routing::get(profiling::get_torch_flamegraph),
        )
        .route(
            "/flamegraph/pprof",
            axum::routing::get(profiling::get_pprof_flamegraph),
        )
        .fallback(extension_handler::handle_extension_call)
}