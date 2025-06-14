// Rust-native addressing system using functional programming patterns
use std::fmt::{self, Display};
use std::str::FromStr;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};

use crate::core::cluster_model::{NodeId, WorkerId};
use super::topology::TopologyView;

/// Simple error type for addressing operations
#[derive(Debug, Clone, PartialEq)]
pub enum AddressError {
    InvalidFormat(String),
    EmptyObject,
    InsufficientTopology,
    NoAvailableNodes,
    NoWorkersOnNode(String),
    // Added for unexpected internal errors, though we aim to avoid these.
    InternalError(String), 
}

impl std::fmt::Display for AddressError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat(msg) => write!(f, "Invalid address format: {}", msg),
            Self::EmptyObject => write!(f, "Object ID cannot be empty"),
            Self::InsufficientTopology => write!(f, "Topology view is insufficient for allocation"),
            Self::NoAvailableNodes => write!(f, "No available nodes for address allocation"),
            Self::NoWorkersOnNode(node) => write!(f, "No workers available on node {}", node),
            Self::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AddressError {}

pub type Result<T> = std::result::Result<T, AddressError>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address {
    pub node: Option<NodeId>,
    pub worker: Option<WorkerId>,
    pub object: String,
}

impl Address {
    pub fn new<N: Into<NodeId>, W: Into<WorkerId>>(node: N, worker: W, object: String) -> Self {
        Self {
            node: Some(node.into()),
            worker: Some(worker.into()),
            object,
        }
    }

    /// Get shard key for data distribution
    pub fn shard_key(&self) -> Option<String> {
        match (&self.node, &self.worker) {
            (Some(n), Some(w)) => Some(format!("{}::{}", n, w)),
            _ => None,
        }
    }

    /// Check if address is local to given node and worker
    pub fn is_local<N: Into<NodeId>, W: Into<WorkerId>>(
        &self,
        current_node_id: N,
        current_worker_id: W,
    ) -> bool {
        let c_node_id_val: NodeId = current_node_id.into();
        let c_worker_id_val: WorkerId = current_worker_id.into();
        match (&self.node, &self.worker) {
            (Some(n), Some(w)) => n == &c_node_id_val && w == &c_worker_id_val,
            _ => false,
        }
    }

    /// 检查地址是否在同一节点上（忽略worker差异）
    pub fn is_same_node<N: Into<NodeId>>(&self, current_node_id: N) -> bool {
        let c_node_id_val: NodeId = current_node_id.into();
        match &self.node {
            Some(n) => n == &c_node_id_val,
            None => false,
        }
    }
}

impl Into<String> for Address {
    fn into(self) -> String {
        format!(
            "{}::{}::{}",
            self.node.as_deref().unwrap_or(""),
            self.worker.as_deref().unwrap_or(""),
            self.object
        )
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s: String = self.clone().into();
        write!(f, "{}", s)
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self> {
        let (node_str, rest) = s.split_once("::").ok_or_else(|| 
            AddressError::InvalidFormat(format!("expected 'node::worker::object', missing first '::' in '{}'", s)))?;
        let (worker_str, object_str) = rest.split_once("::").ok_or_else(|| 
            AddressError::InvalidFormat(format!("expected 'node::worker::object', missing second '::' in '{}'", s)))?;

        if object_str.is_empty() {
            return Err(AddressError::EmptyObject);
        }
        if object_str.contains("::") {
            return Err(AddressError::InvalidFormat(format!("Object ID contains '::', which is not allowed: '{}'", s)));
        }

        let node = if node_str.is_empty() { None } else { Some(node_str.to_string()) };
        let worker = if worker_str.is_empty() { None } else { Some(worker_str.to_string()) };
        let object = object_str.to_string();

        Ok(Self { node, worker, object })
    }
}

/// 地址分配器 - 使用函数式设计而非面向对象
pub struct AddressAllocator {
    topology: TopologyView,
    replica_count: usize, // This is the number of *additional* replicas, not total instances.
    min_knowledge_ratio: f64,
    topology_ttl: u64,
}

impl AddressAllocator {
    /// 创建新的地址分配器
    /// replica_count is the number of desired replicas, EXCLUDING the primary.
    /// So, total_addresses = 1 (primary) + replica_count.
    pub fn new(topology: TopologyView, replica_count: usize) -> Self {
        Self {
            topology,
            replica_count,
            min_knowledge_ratio: 0.7, 
            topology_ttl: 300,       
        }
    }
    
    /// 设置拓扑参数（Builder 模式）
    pub fn with_topology_params(mut self, min_knowledge_ratio: f64, ttl_seconds: u64) -> Self {
        self.min_knowledge_ratio = min_knowledge_ratio.clamp(0.0, 1.0);
        self.topology_ttl = ttl_seconds;
        self
    }

    /// 获取当前拓扑视图
    pub fn topology(&self) -> &TopologyView {
        &self.topology
    }

    /// 为对象分配主地址（带安全检查）
    fn allocate_primary_address(&self, object_id: &str) -> Result<Address> {
        if !self.is_topology_sufficient() {
            return Err(AddressError::InsufficientTopology);
        }
        
        let available_nodes: Vec<NodeId> = self.topology.workers_per_node.keys().cloned().collect();
        if available_nodes.is_empty() {
            return Err(AddressError::NoAvailableNodes);
        }

        let mut max_score = 0;
        let mut chosen_node_id: Option<NodeId> = None;

        for current_node_id_candidate in available_nodes {
            let score = self.hash_string_with_version(object_id, &current_node_id_candidate, self.topology.version);
            if chosen_node_id.is_none() || score > max_score {
                max_score = score;
                chosen_node_id = Some(current_node_id_candidate.clone());
            }
        }

        let node_id = chosen_node_id.ok_or_else(|| AddressError::InternalError("Failed to select a primary node despite available nodes.".to_string()))?;

        let workers = self.topology.workers_per_node
            .get(&node_id)
            .ok_or_else(|| AddressError::NoWorkersOnNode(node_id.clone()))?;

        if workers.is_empty() {
            return Err(AddressError::NoWorkersOnNode(node_id.clone()));
        }

        let worker_hash = self.hash_string_with_version(object_id, &node_id, self.topology.version);
        let worker_index = (worker_hash % workers.len() as u64) as usize;
        let worker_id = workers[worker_index].clone();

        Ok(Address::new(node_id, worker_id, object_id.to_string()))
    }

    /// 为对象分配所有地址（主地址和副本地址）
    /// Returns a Vec<Address> where the first element is the primary.
    pub fn allocate_addresses(&self, object_id: String) -> Result<Vec<Address>> {
        let primary = self.allocate_primary_address(&object_id)?;
        let mut all_addresses = vec![primary.clone()]; // Start with primary

        if self.replica_count == 0 {
            return Ok(all_addresses);
        }

        let replicas = self.allocate_replica_addresses_internal(&primary, self.replica_count)?;
        all_addresses.extend(replicas);
        
        Ok(all_addresses)
    }

    /// 检查当前拓扑是否足够进行分配
    fn is_topology_sufficient(&self) -> bool {
        self.topology.is_sufficient(self.min_knowledge_ratio, self.topology_ttl)
    }

    /// 为已有主地址分配副本地址 (internal helper)
    fn allocate_replica_addresses_internal(&self, primary: &Address, num_replicas_to_find: usize) -> Result<Vec<Address>> {
        let mut replicas = Vec::new();
        if num_replicas_to_find == 0 {
            return Ok(replicas);
        }

        let available_nodes: Vec<NodeId> = self.topology.workers_per_node.keys().cloned().collect();

        if available_nodes.len() <= 1 && primary.node.is_some() { // Not enough distinct nodes for replicas
             // If only one node exists and primary is on it, no replicas possible on other nodes.
            if available_nodes.len() == 1 && available_nodes.first() == primary.node.as_ref() {
                 return Ok(replicas); // No other nodes to pick from
            }
            // If no nodes, or primary is not set (should not happen here), let it proceed to score.
        }
        if available_nodes.is_empty() {
             return Ok(replicas); // No nodes at all
        }


        let mut node_scores: Vec<(u64, NodeId)> = available_nodes
            .iter()
            .filter(|&n_id| Some(n_id) != primary.node.as_ref()) // Exclude primary node from candidates
            .map(|n_id| {
                let score = self.hash_string_with_version(&primary.object, n_id, self.topology.version);
                (score, n_id.clone())
            })
            .collect();

        node_scores.sort_unstable_by(|a, b| b.0.cmp(&a.0)); // Sort by score descending

        for (_score, candidate_node_id) in node_scores {
            if replicas.len() >= num_replicas_to_find {
                break;
            }

            // This check is now implicitly handled by filtering `primary.node` before scoring,
            // but double-checking doesn't hurt if logic changes.
            // if Some(&candidate_node_id) == primary.node.as_ref() {
            //     continue;
            // }

            // Ensure replica node is not already in the replicas list (for distinct nodes)
            if replicas.iter().any(|r: &Address| r.node.as_ref() == Some(&candidate_node_id)) {
                continue;
            }

            if let Some(workers) = self.topology.workers_per_node.get(&candidate_node_id) {
                if !workers.is_empty() {
                    // Use a slightly different seed for replica worker selection to avoid collision if object_id is the same
                    let replica_seed_object_id = format!("{}:replica:{}", primary.object, replicas.len());
                    let worker_hash = self.hash_string_with_version(&replica_seed_object_id, &candidate_node_id, self.topology.version);
                    let worker_index = (worker_hash % workers.len() as u64) as usize;
                    let worker_id_candidate = workers[worker_index].clone();
                    
                    // Primary's (node, worker) pair for comparison
                    let primary_node_worker = primary.node.as_ref().zip(primary.worker.as_ref());
                    // Candidate replica's (node, worker) pair
                    let candidate_node_worker = (Some(&candidate_node_id), Some(&worker_id_candidate));
                    // Convert tuple of Options into an Option of tuple for comparison
                    let candidate_node_worker_opt = candidate_node_worker.0.zip(candidate_node_worker.1);

                    // Ensure replica (node, worker) is different from primary's (node, worker)
                    // This is mainly for the case where primary node was None, or replica is on same node (if allowed by future logic)
                    if primary_node_worker != candidate_node_worker_opt {
                         replicas.push(Address::new(
                            candidate_node_id.clone(),
                            worker_id_candidate,
                            primary.object.clone(),
                        ));
                    } else if primary.node.as_ref() != Some(&candidate_node_id) {
                        // If on a different node, (node,worker) collision is not an issue with primary
                        // This path is taken if primary_node_worker was None, or nodes are different
                         replicas.push(Address::new(
                            candidate_node_id.clone(),
                            worker_id_candidate,
                            primary.object.clone(),
                        ));
                    }
                }
            }
        }
        Ok(replicas)
    }

    /// 带版本的哈希函数，确保拓扑变化时的一致性
    fn hash_string_with_version(&self, object_id: &str, node_id: &str, version: u64) -> u64 {
        let mut hasher = DefaultHasher::new();
        format!("{}:{}:v{}", object_id, node_id, version).hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_basic_topology(num_nodes: usize, workers_per_node_count: usize) -> TopologyView {
        let mut workers_map = HashMap::new();
        for i in 0..num_nodes {
            let node_id = format!("node{}", i + 1);
            let mut worker_ids = Vec::new();
            for j in 0..workers_per_node_count {
                worker_ids.push(format!("worker{}", (i * workers_per_node_count) + j + 1));
            }
            workers_map.insert(node_id, worker_ids);
        }
        TopologyView::new(workers_map, 1) // version 1
    }
    
    #[test]
    fn test_address_creation_and_into_string() {
        let addr_full = Address::new("node1", "worker1", "obj123".to_string());
        let addr_string: String = addr_full.clone().into();
        assert_eq!(addr_string, "node1::worker1::obj123");
    }

    #[test]
    fn test_shard_key() {
        let addr_full = Address::new("node1", "worker1", "obj123".to_string());
        assert_eq!(addr_full.shard_key(), Some("node1::worker1".to_string()));
    }

    #[test]
    fn test_address_parsing() {
        let addr_str_full = "node1::worker1::obj123";
        let addr = Address::from_str(addr_str_full).unwrap();
        assert_eq!(addr.node, Some("node1".to_string()));
        assert_eq!(addr.worker, Some("worker1".to_string()));
        assert_eq!(addr.object, "obj123".to_string());

        let addr_str_no_node = "::worker1::obj123";
        let addr = Address::from_str(addr_str_no_node).unwrap();
        assert_eq!(addr.node, None);
        assert_eq!(addr.worker, Some("worker1".to_string()));
        assert_eq!(addr.object, "obj123".to_string());

        let addr_str_no_worker = "node1::::obj123";
        let addr = Address::from_str(addr_str_no_worker).unwrap();
        assert_eq!(addr.node, Some("node1".to_string()));
        assert_eq!(addr.worker, None);
        assert_eq!(addr.object, "obj123".to_string());

        let addr_str_none = "::::obj123";
        let addr = Address::from_str(addr_str_none).unwrap();
        assert_eq!(addr.node, None);
        assert_eq!(addr.worker, None);
        assert_eq!(addr.object, "obj123".to_string());

        assert!(Address::from_str("node1::worker1").is_err());
        assert!(Address::from_str("node1::worker1::").is_err());
        assert!(Address::from_str("node1::worker1::obj::extra").is_err());
        assert!(Address::from_str("node1").is_err());
    }

    #[test]
    fn test_primary_address_allocation() { // Renamed from test_address_allocation
        let topology = create_basic_topology(2, 2); // node1 (w1,w2), node2 (w3,w4)
        let allocator = AddressAllocator::new(topology.clone(), 0); // No replicas needed for this test
        
        let addr_res = allocator.allocate_primary_address("test_object");
        assert!(addr_res.is_ok());
        let addr = addr_res.unwrap();

        assert!(addr.node.is_some());
        assert!(addr.worker.is_some());
        let node_id = addr.node.clone().unwrap();
        let worker_id = addr.worker.clone().unwrap();

        assert!(topology.workers_per_node.contains_key(&node_id));
        assert!(topology.workers_per_node.get(&node_id).unwrap().contains(&worker_id));
    }
    
    #[test]
    fn test_allocate_addresses_no_replicas() {
        let topology = create_basic_topology(3, 1);
        let allocator = AddressAllocator::new(topology, 0); // 0 replicas
        let addresses = allocator.allocate_addresses("obj_no_replica".to_string()).unwrap();
        
        assert_eq!(addresses.len(), 1); // Only primary
        assert_eq!(addresses[0].object, "obj_no_replica");
    }

    #[test]
    fn test_allocate_addresses_with_replicas() {
        let topology = create_basic_topology(3, 1); // node1(w1), node2(w2), node3(w3)
        let replica_count = 2;
        let allocator = AddressAllocator::new(topology.clone(), replica_count);
        let all_addrs = allocator.allocate_addresses("obj_with_replicas".to_string()).unwrap();

        assert_eq!(all_addrs.len(), 1 + replica_count); // Primary + 2 replicas
        let primary = &all_addrs[0];
        assert_eq!(primary.object, "obj_with_replicas");

        let mut distinct_nodes = std::collections::HashSet::new();
        distinct_nodes.insert(primary.node.as_ref().unwrap().clone());

        for i in 1..=replica_count {
            let replica = &all_addrs[i];
            assert_eq!(replica.object, "obj_with_replicas");
            assert_ne!(replica.node, primary.node, "Replica should be on a different node than primary");
            assert!(distinct_nodes.insert(replica.node.as_ref().unwrap().clone()), "Replica nodes should be distinct");
        }
    }
    
    #[test]
    fn test_replica_generation_sufficient_nodes() { // Adapted from old test_replica_generation
        let topology = create_basic_topology(3, 1); // node1(w1), node2(w2), node3(w3)
        let allocator = AddressAllocator::new(topology.clone(), 2); // Request 2 replicas
        
        let all_addresses = allocator.allocate_addresses("obj123".to_string()).unwrap();
        
        assert_eq!(all_addresses.len(), 3); // Primary + 2 replicas
        let primary = &all_addresses[0];
        let replica1 = &all_addresses[1];
        let replica2 = &all_addresses[2];

        assert_ne!(primary.node, replica1.node);
        assert_ne!(primary.node, replica2.node);
        assert_ne!(replica1.node, replica2.node);

        for addr in &all_addresses {
            assert!(addr.node.is_some());
            assert!(addr.worker.is_some());
            assert_eq!(addr.object, "obj123");
        }
    }

    #[test]
    fn test_replica_generation_insufficient_nodes() {
        let topology = create_basic_topology(2, 1); // node1(w1), node2(w2)
        let allocator = AddressAllocator::new(topology.clone(), 2); // Request 2 replicas, but only 1 other node available
        
        let all_addresses = allocator.allocate_addresses("obj_few_nodes".to_string()).unwrap();
        
        assert_eq!(all_addresses.len(), 2); // Primary + 1 possible replica
        let primary = &all_addresses[0];
        let replica1 = &all_addresses[1];
        
        assert_ne!(primary.node, replica1.node);
    }
    
    #[test]
    fn test_replica_generation_single_node_no_replicas_possible() {
        let topology = create_basic_topology(1, 1); // node1(w1)
        let allocator = AddressAllocator::new(topology.clone(), 1); // Request 1 replica
        
        let all_addresses = allocator.allocate_addresses("obj_single_node".to_string()).unwrap();
        
        assert_eq!(all_addresses.len(), 1); // Only primary, no other nodes for replicas
    }


    #[test]
    fn test_empty_topology_allocation_fails() { // Adapted from old test
        let empty_workers: HashMap<NodeId, Vec<WorkerId>> = HashMap::new();
        let empty_topology = TopologyView::new(empty_workers, 0); // version 0
        let allocator = AddressAllocator::new(empty_topology, 2);
        
        let result = allocator.allocate_addresses("test_empty".to_string());
        assert!(result.is_err());
        assert_eq!(result.err(), Some(AddressError::InsufficientTopology)); // Or NoAvailableNodes depending on checks
    }

    #[test]
    fn test_is_local() {
        let addr = Address::new("node1", "worker1", "obj1".into());
        assert!(addr.is_local("node1", "worker1"));
        assert!(!addr.is_local("node2", "worker1"));
        assert!(!addr.is_local("node1", "worker2"));
    }

    #[tokio::test]
    async fn test_address_allocator_functional() -> std::result::Result<(), AddressError> { // Renamed and updated
        let mut workers_per_node = HashMap::new();
        workers_per_node.insert("node-1".to_string(), vec!["worker-1".to_string(), "worker-2".to_string()]);
        workers_per_node.insert("node-2".to_string(), vec!["worker-3".to_string()]);
        workers_per_node.insert("node-3".to_string(), vec!["worker-4".to_string(), "worker-5".to_string()]);
        
        let topology = TopologyView::new(workers_per_node, 1); // version 1
        let replica_count = 2;
        let allocator = AddressAllocator::new(topology, replica_count);

        let all_addresses = allocator.allocate_addresses("test-object-functional".to_string())?;

        assert_eq!(all_addresses.len(), 1 + replica_count);
        
        let primary = &all_addresses[0];
        assert_eq!(primary.object, "test-object-functional");
        assert!(primary.node.is_some());
        assert!(["node-1", "node-2", "node-3"].contains(&primary.node.as_deref().unwrap()));

        let mut distinct_nodes = std::collections::HashSet::new();
        distinct_nodes.insert(primary.node.as_ref().unwrap().clone());

        for i in 0..replica_count {
            let replica = &all_addresses[i+1];
            assert_eq!(replica.object, "test-object-functional");
            assert_ne!(replica.node, primary.node, "Replica node should differ from primary");
            assert!(distinct_nodes.insert(replica.node.as_ref().unwrap().clone()), "Replica nodes should be distinct among themselves and from primary");
        }
        
        println!("✓ Functional AddressAllocator test passed.");
        println!("Primary Address: {}", primary);
        for (i, replica) in all_addresses.iter().skip(1).enumerate() {
            println!("Replica {}: {}", i + 1, replica);
        }

        Ok(())
    }
}
