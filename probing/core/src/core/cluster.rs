use std::sync::{Arc, LazyLock, RwLock};

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

pub fn update_node(node: Node) {
    let mut node = node.clone();
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

pub fn get_nodes() -> Vec<Node> {
    CLUSTER.read().unwrap().list()
}
