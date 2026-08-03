use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, LazyLock, RwLock,
};
use std::time::Duration;

use arrow::array::{ArrayRef, Int32Array, StringArray, TimestampMicrosecondArray};
use probing_proto::prelude::{Cluster, Node};

pub trait IntoArrow {
    fn into_arrow_array(values: Vec<Self>) -> ArrayRef
    where
        Self: Sized;
}

// Implementation for String
impl IntoArrow for String {
    fn into_arrow_array(values: Vec<Self>) -> ArrayRef {
        Arc::new(StringArray::from(values))
    }
}

impl IntoArrow for Option<String> {
    fn into_arrow_array(values: Vec<Self>) -> ArrayRef {
        Arc::new(StringArray::from(values))
    }
}

impl IntoArrow for Option<i32> {
    fn into_arrow_array(values: Vec<Self>) -> ArrayRef {
        Arc::new(Int32Array::from(values))
    }
}

impl IntoArrow for std::time::Duration {
    fn into_arrow_array(values: Vec<Self>) -> ArrayRef {
        Arc::new(TimestampMicrosecondArray::from(
            values
                .iter()
                .map(|v| Some(v.as_micros() as i64))
                .collect::<Vec<_>>(),
        ))
    }
}

pub fn extract_array<T, V, F>(nodes: &[T], f: F) -> ArrayRef
where
    F: FnMut(&T) -> V,
    V: IntoArrow,
{
    let values: Vec<V> = nodes.iter().map(f).collect();
    V::into_arrow_array(values)
}

pub static CLUSTER: LazyLock<RwLock<Cluster>> = LazyLock::new(|| RwLock::new(Cluster::default()));
static NODE_STALE_TIMEOUT_MICROS: AtomicU64 = AtomicU64::new(30_000_000);

pub fn update_node(mut node: Node) {
    node.timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64;
    CLUSTER.write().unwrap().put(node);
}

pub fn update_nodes(nodes: Vec<Node>) {
    let mut cluster = CLUSTER.write().unwrap();

    for node in nodes {
        cluster.put(node);
    }
}

pub fn set_node_stale_timeout(timeout: Duration) {
    NODE_STALE_TIMEOUT_MICROS.store(timeout.as_micros() as u64, Ordering::Relaxed);
}

pub fn get_nodes() -> Vec<Node> {
    let mut nodes = CLUSTER.read().unwrap().list();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_micros() as u64;
    let stale = NODE_STALE_TIMEOUT_MICROS.load(Ordering::Relaxed);
    apply_node_status(&mut nodes, now, stale);
    nodes.sort_by(|left, right| {
        (
            left.rank.is_none(),
            left.rank.unwrap_or_default(),
            &left.host,
            &left.addr,
        )
            .cmp(&(
                right.rank.is_none(),
                right.rank.unwrap_or_default(),
                &right.host,
                &right.addr,
            ))
    });
    nodes
}

fn apply_node_status(nodes: &mut [Node], now: u64, stale: u64) {
    for node in nodes {
        let status = if now.saturating_sub(node.timestamp) > stale {
            "offline"
        } else {
            "running"
        };
        node.status = Some(status.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_status_from_heartbeat_age() {
        let mut nodes = vec![
            Node {
                timestamp: 80,
                ..Node::default()
            },
            Node {
                timestamp: 69,
                ..Node::default()
            },
        ];
        apply_node_status(&mut nodes, 100, 30);
        assert_eq!(nodes[0].status.as_deref(), Some("running"));
        assert_eq!(nodes[1].status.as_deref(), Some("offline"));
    }
}
