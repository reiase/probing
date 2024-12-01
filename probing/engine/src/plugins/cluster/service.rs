use std::sync::{Arc, LazyLock, RwLock};

use arrow::array::{ArrayRef, Int32Array, StringArray};
use probing_dpp::protocol::cluster;

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

// Implementation for String
impl IntoArrow for Option<String> {
    fn into_arrow_array(values: Vec<Self>) -> ArrayRef {
        Arc::new(StringArray::from(values))
    }
}

// Implementation for i32
impl IntoArrow for Option<i32> {
    fn into_arrow_array(values: Vec<Self>) -> ArrayRef {
        Arc::new(Int32Array::from(values))
    }
}

pub fn extract_array<T, V, F>(nodes: &Vec<T>, f: F) -> ArrayRef
where
    F: FnMut(&T) -> V,
    V: IntoArrow,
{
    let values: Vec<V> = nodes.iter().map(f).collect();
    V::into_arrow_array(values)
}

pub static CLUSTER: LazyLock<RwLock<cluster::Cluster>> =
    LazyLock::new(|| RwLock::new(cluster::Cluster::default()));

pub fn update_node(node: cluster::Node) {
    CLUSTER.write().unwrap().put(node);
}

pub fn update_nodes(nodes: Vec<cluster::Node>) {
    let mut cluster = CLUSTER.write().unwrap();

    for node in nodes {
        cluster.put(node);
    }
}

pub fn get_nodes() -> Vec<cluster::Node> {
    CLUSTER.read().unwrap().list()
}
