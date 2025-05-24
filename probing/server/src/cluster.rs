use probing_core::core::cluster::{get_nodes as core_get_nodes, update_node};
use probing_proto::prelude::*;

use crate::error::ApiResult;

/// Update a node in the cluster
pub async fn put_node(node: Node) -> ApiResult<()> {
    update_node(node);
    Ok(())
}

/// Get all nodes in the cluster
pub async fn get_nodes() -> ApiResult<String> {
    let nodes = core_get_nodes();
    Ok(serde_json::to_string(&nodes)?)
}
