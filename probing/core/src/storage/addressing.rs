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

        match parsed_url.scheme() {
            "probing" | "http" | "https" => (), // Scheme is supported
            scheme => {
                return Err(AddressError::UnsupportedScheme {
                    scheme: scheme.to_string(),
                })
            }
        }

        let node = parsed_url
            .host_str()
            .ok_or_else(|| AddressError::InvalidUri("Missing host in URI".to_string()))?
            .to_string();

        let path = parsed_url.path();

        // Expect path like "/objects/{worker_id}/{object_id}" or "/objects/{worker_id}/path/to/{object_id}"
        if let Some(remaining_path) = path.strip_prefix("/objects/") {
            let mut segments = remaining_path.splitn(2, '/');
            let worker_str = segments.next().filter(|s| !s.is_empty()).ok_or_else(|| {
                AddressError::InvalidUri(format!("Missing worker ID in URI path: {}", path))
            })?;
            let object_str = segments.next().filter(|s| !s.is_empty()).ok_or_else(|| {
                AddressError::InvalidUri(format!("Missing object ID in URI path: {}", path))
            })?;

            Ok(Self {
                node: Some(node),
                worker: Some(worker_str.to_string()),
                object: object_str.to_string(),
            })
        } else {
            Err(AddressError::InvalidUri(format!(
                "Unsupported URI pattern '{}'. Expected format: /objects/{{worker}}/{{object...}}",
                uri
            )))
        }
    }

    pub fn to_uri(&self, scheme: &str) -> String {
        format!(
            "{}://{}/objects/{}/{}",
            scheme,
            self.node.as_deref().unwrap_or("None"),
            self.worker.as_deref().unwrap_or("None"),
            self.object
        )
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
        self.node
            .as_ref()
            .zip(self.worker.as_ref())
            .map_or(false, |(n, w)| n == &c_node_id_val && w == &c_worker_id_val)
    }

    /// 检查地址是否在同一节点上（忽略worker差异）
    pub fn is_same_node<N: Into<NodeId>>(&self, current_node_id: N) -> bool {
        let c_node_id_val: NodeId = current_node_id.into();
        self.node.as_ref().map_or(false, |n| n == &c_node_id_val)
    }
}

impl Into<String> for Address {
    fn into(self) -> String {
        // Default to "probing" scheme when converting to String
        self.to_uri("probing")
    }
}

impl Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Default to "probing" scheme for display
        write!(f, "{}", self.to_uri("probing"))
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

        let best_pair = self
            .topology
            .workers_per_node
            .iter()
            .flat_map(|(node_id, workers)| {
                workers.iter().map(move |worker_id| (node_id, worker_id))
            })
            .max_by_key(|&(node_id, worker_id)| {
                self.calculate_assignment_score(object_id, node_id, worker_id)
            });

        match best_pair {
            Some((chosen_node_id, chosen_worker_id)) => Ok(Address::new(
                chosen_node_id.clone(),
                chosen_worker_id.clone(),
                object_id.to_string(),
            )),
            None => Err(AddressError::NoAvailableNodes), // Or more specific if all nodes have 0 workers but nodes exist
        }
    }

    /// 为对象分配所有地址（主地址和副本地址）
    /// Returns a Vec<Address> where the first element is the primary.
    pub fn allocate_addresses(&self, object_id: String) -> Result<Vec<Address>> {
        let primary = self.allocate_primary_address(&object_id)?;
        let mut all_addresses = vec![primary.clone()]; // Start with primary

        if self.replica_count == 0 {
            return Ok(all_addresses);
        }

        let replicas = self.allocate_replica_addresses(&primary, self.replica_count)?;
        all_addresses.extend(replicas);

        Ok(all_addresses)
    }

    /// 检查当前拓扑是否足够进行分配
    fn is_topology_sufficient(&self) -> bool {
        self.topology
            .is_sufficient(self.min_knowledge_ratio, self.topology_ttl)
    }

    pub fn allocate_replica_addresses(
        &self,
        primary: &Address,
        num_replicas_to_find: usize,
    ) -> Result<Vec<Address>> {
        if num_replicas_to_find == 0 {
            return Ok(Vec::new());
        }

        let primary_node_id = primary.node.as_ref();

        // Generate all potential (node, worker) pairs for replicas, score them, and sort.
        let mut potential_replicas: Vec<_> = self
            .topology
            .workers_per_node
            .iter()
            .filter(|(node_id, _workers)| Some(*node_id) != primary_node_id) // Exclude primary node
            .flat_map(|(node_id, workers)| {
                workers.iter().map(move |worker_id| (node_id, worker_id))
            })
            .map(|(node_id, worker_id)| {
                // Use primary.object as the seed for consistent scoring context.
                // The specific node and worker will differentiate the scores.
                let score = self.calculate_assignment_score(&primary.object, node_id, worker_id);
                (score, node_id, worker_id)
            })
            .collect();

        // Sort by score in descending order
        potential_replicas.sort_unstable_by_key(|k| std::cmp::Reverse(k.0));

        let mut replicas = Vec::with_capacity(num_replicas_to_find);
        let mut used_replica_nodes = std::collections::HashSet::new();

        for (_score, candidate_node_id, candidate_worker_id) in potential_replicas {
            if replicas.len() >= num_replicas_to_find {
                break;
            }

            // Ensure replica is on a distinct node
            if !used_replica_nodes.contains(candidate_node_id) {
                replicas.push(Address::new(
                    candidate_node_id.clone(),
                    candidate_worker_id.clone(),
                    primary.object.clone(),
                ));
                used_replica_nodes.insert(candidate_node_id.clone());
            }
        }

        Ok(replicas)
    }

    /// Generates a hash for selecting a worker on a node or a (node, worker) pair.
    fn calculate_assignment_score(
        &self,
        object_id_seed: &str,
        node_id: &NodeId,
        worker_id: &WorkerId,
    ) -> u64 {
        let mut hasher = DefaultHasher::new();
        format!("{}:{}:{}", object_id_seed, node_id, worker_id).hash(&mut hasher);
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
    fn test_address_creation_and_to_string() {
        let addr_full = Address::new("node1", "worker1", "obj123".to_string());
        let addr_string = addr_full.to_string(); // Use to_string()
        assert_eq!(addr_string, "probing://node1/objects/worker1/obj123");
    }

    #[test]
    fn test_to_uri_and_from_uri() {
        let addr = Address::new("node1", "worker1", "task_123".to_string());

        // Test with "probing" scheme
        let probing_uri = addr.to_uri("probing");
        assert_eq!(probing_uri, "probing://node1/objects/worker1/task_123");
        let parsed_probing = Address::from_uri(&probing_uri).unwrap();
        assert_eq!(parsed_probing.node, Some("node1".to_string()));
        assert_eq!(parsed_probing.worker, Some("worker1".to_string()));
        assert_eq!(parsed_probing.object, "task_123");
        assert_eq!(addr, parsed_probing); // Roundtrip check

        // Test with "http" scheme
        let http_uri = addr.to_uri("http");
        assert_eq!(http_uri, "http://node1/objects/worker1/task_123");
        let parsed_http = Address::from_uri(&http_uri).unwrap();
        assert_eq!(parsed_http.node, Some("node1".to_string()));
        assert_eq!(parsed_http.worker, Some("worker1".to_string()));
        assert_eq!(parsed_http.object, "task_123");

        // Test with "https" scheme
        let https_uri = addr.to_uri("https");
        assert_eq!(https_uri, "https://node1/objects/worker1/task_123");
        let parsed_https = Address::from_uri(&https_uri).unwrap();
        assert_eq!(parsed_https.node, Some("node1".to_string()));
        assert_eq!(parsed_https.worker, Some("worker1".to_string()));
        assert_eq!(parsed_https.object, "task_123");


        // Test with nested object paths
        let addr_nested = Address::new("node1", "worker1", "data/user/profile_456".to_string());
        let nested_uri = addr_nested.to_uri("probing");
        assert_eq!(nested_uri, "probing://node1/objects/worker1/data/user/profile_456");
        let parsed_nested = Address::from_uri(&nested_uri).unwrap();
        assert_eq!(parsed_nested.object, "data/user/profile_456");
        assert_eq!(addr_nested, parsed_nested); // Roundtrip check
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
    fn test_fromstr_with_uri() {
        // Test URI format parsing
        let uri_addr = Address::from_str("probing://node1/objects/worker1/test_obj").unwrap();
        assert_eq!(uri_addr.node, Some("node1".to_string()));
        assert_eq!(uri_addr.worker, Some("worker1".to_string()));
        assert_eq!(uri_addr.object, "test_obj");

        // Test display format (should use URI when possible)
        let display_str = format!("{}", uri_addr);
        assert_eq!(display_str, "probing://node1/objects/worker1/test_obj"); // Check full string
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
    fn test_invalid_uri_patterns() {
        // Test unsupported pattern
        let invalid_uri = "probing://node1/unsupported/pattern";
        let result = Address::from_uri(invalid_uri);
        assert!(result.is_err());
    }

    #[test]
    fn test_display_trait() {
        let addr = Address::new("node1", "worker1", "obj1".into());
        let display_str = format!("{}", addr);
        assert_eq!(display_str, "probing://node1/objects/worker1/obj1");
    }
}
