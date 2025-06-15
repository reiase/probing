use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Display};
use std::hash::{Hash, Hasher};
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use super::topology::TopologyView;
use crate::core::cluster_model::{NodeId, WorkerId};

/// Addressing operation errors
#[derive(Debug, Clone, PartialEq, Error)]
pub enum AddressError {
    #[error("Invalid address format: {0}")]
    InvalidFormat(String),

    #[error("Invalid URI format: {0}")]
    InvalidUri(String),

    #[error("Object ID cannot be empty")]
    EmptyObject,

    #[error("Topology view is insufficient for allocation")]
    InsufficientTopology,

    #[error("No available nodes for address allocation")]
    NoAvailableNodes,

    #[error("No workers available on node {node}")]
    NoWorkersOnNode { node: String },

    #[error("Unsupported URI scheme: {scheme}")]
    UnsupportedScheme { scheme: String },

    #[error("Internal error: {0}")]
    InternalError(String),
}

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

    /// Create address from URI format
    /// Supports the simplified routing pattern:
    ///
    /// **URI Format: /objects/{worker}/{object}**
    /// - probing://node/objects/worker_id/object_id
    /// - http://node/objects/worker_id/object_id
    /// - https://node/objects/worker_id/object_id
    ///
    /// This format is web framework friendly and provides a clean separation
    /// between the routing prefix (/objects) and the resource identifiers.
    ///
    /// Examples:
    /// - `probing://storage-node-1/objects/compute-worker-1/report.pdf`
    /// - `http://api.example.com:8080/objects/gpu-worker/models/bert.bin`
    /// - `https://cluster.internal/objects/cache-worker/temp/data.json`
    pub fn from_uri(uri: &str) -> Result<Self> {
        let parsed_url = Url::parse(uri).map_err(|e| {
            AddressError::InvalidUri(format!("Failed to parse URI '{}': {}", uri, e))
        })?;

        // Validate scheme
        match parsed_url.scheme() {
            "probing" | "http" | "https" => {}
            scheme => {
                return Err(AddressError::UnsupportedScheme {
                    scheme: scheme.to_string(),
                })
            }
        }

        // Extract node from host
        let node = parsed_url
            .host_str()
            .ok_or_else(|| AddressError::InvalidUri("Missing host in URI".to_string()))?
            .to_string();

        let path = parsed_url.path();
        let path_segments: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        match path_segments.as_slice() {
            ["objects", worker, object @ ..] if !object.is_empty() => Ok(Self {
                node: Some(node),
                worker: Some(worker.to_string()),
                object: object.join("/"),
            }),
            _ => Err(AddressError::InvalidUri(format!(
                "Unsupported URI pattern '{}'. Expected format: /objects/{{worker}}/{{object}}",
                uri
            ))),
        }
    }

    pub fn to_uri(&self, scheme: &str) -> String {
        format!(
            "{}://{}/objects/{}/{}",
            scheme,
            self.node.clone().unwrap_or("None".to_string()),
            self.worker.clone().unwrap_or("None".to_string()),
            self.object
        )
    }

    /// Convert to probing:// URI
    pub fn to_probing_uri(&self) -> String {
        self.to_uri("probing")
    }

    /// Convert to HTTP URI
    pub fn to_http_uri(&self) -> String {
        self.to_uri("http")
    }

    /// Convert to HTTPS URI
    pub fn to_https_uri(&self) -> String {
        self.to_uri("https")
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
        // Default to legacy format for backward compatibility
        self.to_probing_uri()
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_probing_uri())
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self> {
        // Try URI format first
        if s.contains("://") {
            return Self::from_uri(s);
        }

        Err(AddressError::InvalidFormat(format!(
            "Invalid address format: '{}'. Expected URI format.",
            s
        )))
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
            let score = self.hash_string_with_version(
                object_id,
                &current_node_id_candidate,
                self.topology.version,
            );
            if chosen_node_id.is_none() || score > max_score {
                max_score = score;
                chosen_node_id = Some(current_node_id_candidate.clone());
            }
        }

        let node_id = chosen_node_id.ok_or_else(|| {
            AddressError::InternalError(
                "Failed to select a primary node despite available nodes.".to_string(),
            )
        })?;

        let workers = self
            .topology
            .workers_per_node
            .get(&node_id)
            .ok_or_else(|| AddressError::NoWorkersOnNode {
                node: node_id.clone(),
            })?;

        if workers.is_empty() {
            return Err(AddressError::NoWorkersOnNode {
                node: node_id.clone(),
            });
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
        self.topology
            .is_sufficient(self.min_knowledge_ratio, self.topology_ttl)
    }

    /// 为已有主地址分配副本地址 (internal helper)
    fn allocate_replica_addresses_internal(
        &self,
        primary: &Address,
        num_replicas_to_find: usize,
    ) -> Result<Vec<Address>> {
        let mut replicas = Vec::new();
        if num_replicas_to_find == 0 {
            return Ok(replicas);
        }

        let available_nodes: Vec<NodeId> = self.topology.workers_per_node.keys().cloned().collect();

        if available_nodes.len() <= 1 && primary.node.is_some() {
            // Not enough distinct nodes for replicas
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
                let score =
                    self.hash_string_with_version(&primary.object, n_id, self.topology.version);
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
            if replicas
                .iter()
                .any(|r: &Address| r.node.as_ref() == Some(&candidate_node_id))
            {
                continue;
            }

            if let Some(workers) = self.topology.workers_per_node.get(&candidate_node_id) {
                if !workers.is_empty() {
                    // Use a slightly different seed for replica worker selection to avoid collision if object_id is the same
                    let replica_seed_object_id =
                        format!("{}:replica:{}", primary.object, replicas.len());
                    let worker_hash = self.hash_string_with_version(
                        &replica_seed_object_id,
                        &candidate_node_id,
                        self.topology.version,
                    );
                    let worker_index = (worker_hash % workers.len() as u64) as usize;
                    let worker_id_candidate = workers[worker_index].clone();

                    // Primary's (node, worker) pair for comparison
                    let primary_node_worker = primary.node.as_ref().zip(primary.worker.as_ref());
                    // Candidate replica's (node, worker) pair
                    let candidate_node_worker =
                        (Some(&candidate_node_id), Some(&worker_id_candidate));
                    // Convert tuple of Options into an Option of tuple for comparison
                    let candidate_node_worker_opt =
                        candidate_node_worker.0.zip(candidate_node_worker.1);

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
        assert_eq!(addr_string, "probing://node1/objects/worker1/obj123");
    }

    #[test]
    fn test_uri_creation_and_parsing() {
        // Test probing:// scheme with namespace format
        let addr = Address::new("node1", "worker1", "task_123".to_string());
        let uri = addr.to_probing_uri();
        assert_eq!(uri, "probing://node1/objects/worker1/task_123");

        let parsed = Address::from_uri(&uri).unwrap();
        assert_eq!(parsed.node, Some("node1".to_string()));
        assert_eq!(parsed.worker, Some("worker1".to_string()));
        assert_eq!(parsed.object, "task_123");

        // Test HTTP scheme with port and namespace format
        let http_uri = addr.to_http_uri();
        assert_eq!(http_uri, "http://node1/objects/worker1/task_123");

        let parsed_http = Address::from_uri(&http_uri).unwrap();
        assert_eq!(parsed_http.node, Some("node1".to_string()));
        assert_eq!(parsed_http.worker, Some("worker1".to_string()));
        assert_eq!(parsed_http.object, "task_123");
    }

    #[test]
    fn test_uri_with_nested_object_paths() {
        let addr = Address::new("node1", "worker1", "data/user/profile_456".to_string());
        let uri = addr.to_probing_uri();
        assert_eq!(uri, "probing://node1/objects/worker1/data/user/profile_456");

        let parsed = Address::from_uri(&uri).unwrap();
        assert_eq!(parsed.object, "data/user/profile_456");
    }

    #[test]
    fn test_uri_scheme_validation() {
        // Test unsupported scheme
        let result = Address::from_uri("ftp://node1/objects/worker1/test");
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            AddressError::UnsupportedScheme { .. }
        ));

        // Test invalid URI
        let result = Address::from_uri("invalid-uri");
        assert!(result.is_err());
        assert!(matches!(result.err().unwrap(), AddressError::InvalidUri(_)));
    }

    #[test]
    fn test_fromstr_with_uri_and_legacy_formats() {
        // Test URI format parsing
        let uri_addr = Address::from_str("probing://node1/objects/worker1/test_obj").unwrap();
        assert_eq!(uri_addr.node, Some("node1".to_string()));
        assert_eq!(uri_addr.worker, Some("worker1".to_string()));
        assert_eq!(uri_addr.object, "test_obj");

        // Test display format (should use URI when possible)
        let display_str = format!("{}", uri_addr);
        assert!(display_str.starts_with("probing://"));
    }

    #[test]
    fn test_shard_key() {
        let addr_full = Address::new("node1", "worker1", "obj123".to_string());
        assert_eq!(addr_full.shard_key(), Some("node1::worker1".to_string()));
    }

    #[test]
    fn test_primary_address_allocation() {
        // Renamed from test_address_allocation
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
        assert!(topology
            .workers_per_node
            .get(&node_id)
            .unwrap()
            .contains(&worker_id));
    }

    #[test]
    fn test_allocate_addresses_no_replicas() {
        let topology = create_basic_topology(3, 1);
        let allocator = AddressAllocator::new(topology, 0); // 0 replicas
        let addresses = allocator
            .allocate_addresses("obj_no_replica".to_string())
            .unwrap();

        assert_eq!(addresses.len(), 1); // Only primary
        assert_eq!(addresses[0].object, "obj_no_replica");
    }

    #[test]
    fn test_allocate_addresses_with_replicas() {
        let topology = create_basic_topology(3, 1); // node1(w1), node2(w2), node3(w3)
        let replica_count = 2;
        let allocator = AddressAllocator::new(topology.clone(), replica_count);
        let all_addrs = allocator
            .allocate_addresses("obj_with_replicas".to_string())
            .unwrap();

        assert_eq!(all_addrs.len(), 1 + replica_count); // Primary + 2 replicas
        let primary = &all_addrs[0];
        assert_eq!(primary.object, "obj_with_replicas");

        let mut distinct_nodes = std::collections::HashSet::new();
        distinct_nodes.insert(primary.node.as_ref().unwrap().clone());

        for i in 1..=replica_count {
            let replica = &all_addrs[i];
            assert_eq!(replica.object, "obj_with_replicas");
            assert_ne!(
                replica.node, primary.node,
                "Replica should be on a different node than primary"
            );
            assert!(
                distinct_nodes.insert(replica.node.as_ref().unwrap().clone()),
                "Replica nodes should be distinct"
            );
        }
    }

    #[test]
    fn test_replica_generation_sufficient_nodes() {
        // Adapted from old test_replica_generation
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

        let all_addresses = allocator
            .allocate_addresses("obj_few_nodes".to_string())
            .unwrap();

        assert_eq!(all_addresses.len(), 2); // Primary + 1 possible replica
        let primary = &all_addresses[0];
        let replica1 = &all_addresses[1];

        assert_ne!(primary.node, replica1.node);
    }

    #[test]
    fn test_replica_generation_single_node_no_replicas_possible() {
        let topology = create_basic_topology(1, 1); // node1(w1)
        let allocator = AddressAllocator::new(topology.clone(), 1); // Request 1 replica

        let all_addresses = allocator
            .allocate_addresses("obj_single_node".to_string())
            .unwrap();

        assert_eq!(all_addresses.len(), 1); // Only primary, no other nodes for replicas
    }

    #[test]
    fn test_empty_topology_allocation_fails() {
        // Adapted from old test
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
    async fn test_address_allocator_functional() -> std::result::Result<(), AddressError> {
        // Renamed and updated
        let mut workers_per_node = HashMap::new();
        workers_per_node.insert(
            "node-1".to_string(),
            vec!["worker-1".to_string(), "worker-2".to_string()],
        );
        workers_per_node.insert("node-2".to_string(), vec!["worker-3".to_string()]);
        workers_per_node.insert(
            "node-3".to_string(),
            vec!["worker-4".to_string(), "worker-5".to_string()],
        );

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
            let replica = &all_addresses[i + 1];
            assert_eq!(replica.object, "test-object-functional");
            assert_ne!(
                replica.node, primary.node,
                "Replica node should differ from primary"
            );
            assert!(
                distinct_nodes.insert(replica.node.as_ref().unwrap().clone()),
                "Replica nodes should be distinct among themselves and from primary"
            );
        }

        println!("✓ Functional AddressAllocator test passed.");
        println!("Primary Address: {}", primary);
        for (i, replica) in all_addresses.iter().skip(1).enumerate() {
            println!("Replica {}: {}", i + 1, replica);
        }

        Ok(())
    }

    #[test]
    fn test_uri_generation() {
        let addr = Address::new("node1", "worker1", "obj1".into());

        // Test legacy URI generation
        let uri = addr.to_probing_uri();
        assert_eq!(uri, "probing://node1/objects/worker1/obj1");

        let http_uri = addr.to_http_uri();
        assert_eq!(http_uri, "http://node1/objects/worker1/obj1");
    }

    #[test]
    fn test_uri_parsing() {
        // Test legacy pattern parsing (backward compatibility)
        let uri = "probing://node1/objects/worker1/obj1";
        let addr = Address::from_uri(uri).unwrap();
        assert_eq!(addr.node, Some("node1".to_string()));
        assert_eq!(addr.worker, Some("worker1".to_string()));
        assert_eq!(addr.object, "obj1");
    }

    #[test]
    fn test_uri_roundtrip() {
        let original = Address::new("node1", "worker1", "nested/obj/path".into());

        let uri = original.to_probing_uri();
        let parsed = Address::from_uri(&uri).unwrap();
        assert_eq!(original, parsed);

        let uri = original.to_http_uri();
        let parsed = Address::from_uri(&uri).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_invalid_uri_patterns() {
        // Test unsupported pattern
        let invalid_uri = "probing://node1/unsupported/pattern";
        let result = Address::from_uri(invalid_uri);
        assert!(result.is_err());
    }

    #[test]
    fn test_uri_format_display() {
        let addr = Address::new("node1", "worker1", "obj1".into());

        // Display should use namespace-based URI format by default
        let display_str = format!("{}", addr);
        assert_eq!(display_str, "probing://node1/objects/worker1/obj1");
    }
}
