use axum::Router;

use super::{cluster, extension_handler, file_api, profiling, system};

/// Main router for all API endpoints
pub fn apis_route() -> Router {
    Router::new()
        .route("/overview", axum::routing::get(system::get_overview_json))
        .route("/files", axum::routing::get(file_api::read_file))
        .route(
            "/nodes",
            axum::routing::get(cluster::get_nodes).put(cluster::put_node),
        )
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
