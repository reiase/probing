use std::{collections::HashMap, fmt::Display};

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

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Node {{ host: {}, addr: {}, local_rank: {:?}, rank: {:?}, world_size: {:?}, group_rank: {:?}, group_world_size: {:?}, role_name: {:?}, role_rank: {:?}, role_world_size: {:?}, status: {:?}, timestamp: {} }}",
            self.host,
            self.addr,
            self.local_rank,
            self.rank,
            self.world_size,
            self.group_rank,
            self.group_world_size,
            self.role_name,
            self.role_rank,
            self.role_world_size,
            self.status,
            self.timestamp
        )
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq, Clone)]
pub struct Cluster {
    pub nodes: HashMap<i32, Node>,
}

impl Cluster {
    pub fn put(&mut self, node: Node) {
        self.nodes.insert(node.rank.unwrap_or(-1), node);
    }

    pub fn get(&self, rank: i32) -> Option<&Node> {
        self.nodes.get(&rank)
    }

    pub fn remove(&mut self, rank: i32) -> Option<Node> {
        self.nodes.remove(&rank)
    }

    pub fn list(&self) -> Vec<Node> {
        self.nodes.values().cloned().collect()
    }
}
