use probing_core::core::cluster::{get_nodes as core_get_nodes, update_node};
use probing_proto::prelude::*;

use super::error::ApiResult;

/// Update a node in the cluster (HTTP handler)
pub async fn put_node(axum::Json(node): axum::Json<Node>) -> ApiResult<()> {
    update_node(node);
    Ok(())
}

/// Get all nodes in the cluster as JSON
pub async fn get_nodes() -> ApiResult<axum::Json<Vec<Node>>> {
    Ok(axum::Json(core_get_nodes()))
}
