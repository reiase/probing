use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]

pub struct Node {
    pub host: String,
    pub addr: String,

    pub local_rank: Option<i32>,

    pub rank: Option<i32>,
    pub world_size: Option<i32>,

    pub group_rank: Option<i32>,
    pub group_world_size: Option<i32>,

    pub role_name: Option<String>,
    pub role_rank: Option<i32>,
    pub role_world_size: Option<i32>,

    pub status: Option<String>,
    pub timestamp: u64,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Cluster {
    pub nodes: HashMap<String, Node>,
}

impl Cluster {
    pub fn put(&mut self, node: Node) {
        self.nodes.insert(node.addr.clone(), node);
    }

    pub fn get(&self, addr: &str) -> Option<&Node> {
        self.nodes.get(addr)
    }

    pub fn remove(&mut self, addr: &str) -> Option<Node> {
        self.nodes.remove(addr)
    }

    pub fn list(&self) -> Vec<Node> {
        self.nodes.values().cloned().collect()
    }
}