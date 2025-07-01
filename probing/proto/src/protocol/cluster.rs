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
    pub nodes: HashMap<String, Node>,     // 使用host:addr作为key
    pub rank_index: HashMap<i32, String>, // rank到节点key的映射
}

impl Cluster {
    pub fn put(&mut self, node: Node) {
        let key = format!("{}:{}", node.host, node.addr);

        // 如果有rank，维护rank索引
        if let Some(rank) = node.rank {
            self.rank_index.insert(rank, key.clone());
        }

        self.nodes.insert(key, node);
    }

    pub fn get(&self, rank: i32) -> Option<&Node> {
        self.rank_index
            .get(&rank)
            .and_then(|key| self.nodes.get(key))
    }

    pub fn get_by_addr(&self, host: &str, addr: &str) -> Option<&Node> {
        let key = format!("{host}:{addr}");
        self.nodes.get(&key)
    }

    pub fn remove(&mut self, rank: i32) -> Option<Node> {
        if let Some(key) = self.rank_index.remove(&rank) {
            self.nodes.remove(&key)
        } else {
            None
        }
    }

    pub fn remove_by_addr(&mut self, host: &str, addr: &str) -> Option<Node> {
        let key = format!("{host}:{addr}");
        if let Some(node) = self.nodes.remove(&key) {
            // 同时移除rank索引
            if let Some(rank) = node.rank {
                self.rank_index.remove(&rank);
            }
            Some(node)
        } else {
            None
        }
    }

    pub fn list(&self) -> Vec<Node> {
        self.nodes.values().cloned().collect()
    }
}
