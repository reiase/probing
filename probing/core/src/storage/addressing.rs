use std::collections::hash_map::DefaultHasher;
use std::fmt::{self, Display};
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

use super::topology::TopologyView;
use crate::core::cluster_model::{NodeId, WorkerId};

/// Errors that can occur during addressing operations.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum AddressError {
    /// Error indicating an invalid address format.
    #[error("Invalid address format: {0}")]
    InvalidFormat(String),

    /// Error indicating an invalid URI format.
    #[error("Invalid URI format: {0}")]
    InvalidUri(String),

    /// Error indicating that an object ID cannot be empty.
    #[error("Object ID cannot be empty")]
    EmptyObject,

    /// Error indicating that the topology view is insufficient for allocation.
    /// This can happen if the known part of the cluster is too small or too outdated.
    #[error("Topology view is insufficient for allocation")]
    InsufficientTopology,

    /// Error indicating that no nodes are available for address allocation.
    #[error("No available nodes for address allocation")]
    NoAvailableNodes,

    /// Error indicating that no workers are available on a specific node.
    #[error("No workers available on node {node}")]
    NoWorkersOnNode { node: String },

    /// Error indicating an unsupported URI scheme.
    #[error("Unsupported URI scheme: {scheme}")]
    UnsupportedScheme { scheme: String },

    /// Error for other internal issues.
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// A specialized `Result` type for addressing operations.
pub type Result<T> = std::result::Result<T, AddressError>;

/// Represents a unique address for an object within the distributed system.
///
/// An `Address` typically includes the node, worker, and a specific object identifier.
/// It provides mechanisms for URI conversion and shard key generation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address {
    /// The ID of the worker process on the node responsible for the object.
    /// `None` if the address is not yet fully resolved or worker assignment is not applicable.
    pub worker: Option<WorkerId>,
    /// The unique identifier for the object itself (e.g., filename, task ID).
    pub object: String,
}

impl Address {
    /// Creates a new `Address` with specified worker and object ID.
    ///
    /// # Arguments
    /// * `worker`: The worker ID.
    /// * `object`: The object identifier.
    pub fn new<W: Into<WorkerId>>(worker: W, object: String) -> Self {
        Self {
            worker: Some(worker.into()),
            object,
        }
    }

    /// Create address from URI format.
    /// Supports the simplified routing pattern:
    ///
    /// **URI Format: scheme://{worker_id}/objects/{object_id}**
    /// - `probing://worker-1/objects/report.pdf`
    /// - `http://compute-worker-alpha/objects/data/input.csv`
    /// - `https://gpu-worker-zeta:8080/objects/models/bert.bin` (port is allowed by Url::parse but currently ignored by Address)
    ///
    /// The `worker_id` is taken from the host part of the URI.
    /// The `object_id` is taken from the path part, after `/objects/`.
    ///
    /// # Supported URI Schemes:
    /// - `probing` (primary)
    /// - `http`
    /// - `https`
    ///
    /// # Examples:
    /// - `probing://compute-worker-1/objects/report.pdf`
    /// - `http://gpu-worker/objects/models/bert.bin`
    /// - `https://cache-worker/objects/temp/data.json`
    ///
    /// # Errors
    /// Returns `AddressError` if the URI is malformed, uses an unsupported scheme,
    /// is missing the worker_id (host), or does not follow the expected path pattern.
    pub fn from_uri(uri: &str) -> Result<Self> {
        let parsed_url = Url::parse(uri).map_err(|e| {
            AddressError::InvalidUri(format!("Failed to parse URI '{}': {}", uri, e))
        })?;

        match parsed_url.scheme() {
            "probing" | "http" | "https" => (),
            scheme => {
                return Err(AddressError::UnsupportedScheme {
                    scheme: scheme.to_string(),
                })
            }
        }

        let worker_id_str = parsed_url
            .host_str()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| {
                AddressError::InvalidUri(
                    "Missing worker ID in URI (host part is empty or not present)".to_string(),
                )
            })?
            .to_string();

        let path = parsed_url.path();

        if let Some(object_id_part) = path.strip_prefix("/objects/") {
            if object_id_part.is_empty() {
                return Err(AddressError::InvalidUri(format!(
                    "Missing object ID in URI path after /objects/: {}",
                    path
                )));
            }
            let object_id_str = object_id_part.to_string();

            Ok(Self {
                worker: Some(worker_id_str),
                object: object_id_str,
            })
        } else {
            Err(AddressError::InvalidUri(format!(
                "Unsupported URI path pattern '{}'. Expected path like /objects/{{object_id...}} (worker ID should be in the host part of the URI)",
                path
            )))
        }
    }

    /// Converts the `Address` to a URI string with the specified scheme.
    /// The format will be `scheme://{worker_id}/objects/{object_id}`.
    ///
    /// # Arguments
    /// * `scheme`: The URI scheme to use (e.g., "probing", "http").
    ///
    /// # Returns
    /// A URI string representation of the address.
    /// If `worker` is `None`, "UNKNOWN_WORKER" will be used as the host (worker_id part).
    pub fn to_uri(&self, scheme: &str) -> String {
        format!(
            "{}://{}/objects/{}",
            scheme,
            self.worker.as_deref().unwrap_or("UNKNOWN_WORKER"), // Worker ID as host
            self.object
        )
    }

    /// Get shard key for data distribution.
    /// The shard key is based on the worker ID.
    ///
    /// # Returns
    /// An `Option<String>` containing the worker ID as the shard key if the worker is set,
    /// otherwise `None`.
    pub fn shard_key(&self) -> Option<String> {
        self.worker.as_ref().map(|w| w.to_string())
    }

    /// Check if address is local to the given worker.
    ///
    /// # Arguments
    /// * `current_worker_id`: The ID of the current worker.
    ///
    /// # Returns
    /// `true` if the address's worker matches the provided ID, `false` otherwise.
    pub fn is_local<W: Into<WorkerId>>(&self, current_worker_id: W) -> bool {
        let c_worker_id_val: WorkerId = current_worker_id.into();
        self.worker
            .as_ref()
            .map_or(false, |w| w == &c_worker_id_val)
    }
}

impl Into<String> for Address {
    /// Converts an `Address` into its string representation (URI with "probing" scheme).
    fn into(self) -> String {
        self.to_uri("probing")
    }
}

impl Display for Address {
    /// Formats the `Address` as a string, defaulting to the "probing" URI scheme.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_uri("probing"))
    }
}

impl Into<Address> for String {
    /// Converts a `String` into an `Address`.
    /// Attempts to parse the string as a URI. If parsing fails,
    /// creates an `Address` with `None` for worker, and the string as the object ID.
    fn into(self) -> Address {
        self.as_str().into()
    }
}

impl Into<Address> for &str {
    /// Converts a string slice (`&str`) into an `Address`.
    /// Attempts to parse the string as a URI. If parsing fails,
    /// creates an `Address` with `None` for worker, and the string as the object ID.
    fn into(self) -> Address {
        match Address::from_uri(self) {
            Ok(addr) => addr,
            Err(_) => Address {
                worker: None,
                object: self.to_string(),
            },
        }
    }
}

/// `AddressAllocator` is responsible for assigning addresses to objects,
/// including a primary address and optional replica addresses.
///
/// It employs a consistent hashing strategy (similar to Rendezvous Hashing)
/// to determine the placement of objects on nodes and workers. This approach
/// ensures that even with a partial or incomplete view of the cluster topology,
/// the allocator can still make reasonable and balanced placement decisions.
/// The hashing mechanism minimizes remapping of objects when nodes are added or
/// removed, contributing to system stability and efficient load distribution.
///
/// The allocator considers the current `TopologyView`, a desired number of replicas,
/// and parameters for topology sufficiency (minimum knowledge ratio and TTL).
pub struct AddressAllocator {
    topology: TopologyView,
    replica_count: usize, // This is the number of *additional* replicas, not total instances.
    min_knowledge_ratio: f64,
    topology_ttl: u64,
}

impl AddressAllocator {
    /// Creates a new `AddressAllocator`.
    ///
    /// # Arguments
    /// * `topology`: The current view of the cluster topology.
    /// * `replica_count`: The number of *additional* replicas desired for each object,
    ///   excluding the primary. So, `total_addresses = 1 (primary) + replica_count`.
    pub fn new(topology: TopologyView, replica_count: usize) -> Self {
        Self {
            topology,
            replica_count,
            min_knowledge_ratio: 0.7,
            topology_ttl: 300,
        }
    }

    /// Sets topology parameters for the allocator using a builder pattern.
    ///
    /// # Arguments
    /// * `min_knowledge_ratio`: The minimum ratio of known nodes to total estimated nodes
    ///   for the topology to be considered sufficient. Clamped between 0.0 and 1.0.
    /// * `ttl_seconds`: The time-to-live for topology information, in seconds.
    ///
    /// # Returns
    /// The `AddressAllocator` instance with updated parameters.
    pub fn with_topology_params(mut self, min_knowledge_ratio: f64, ttl_seconds: u64) -> Self {
        self.min_knowledge_ratio = min_knowledge_ratio.clamp(0.0, 1.0);
        self.topology_ttl = ttl_seconds;
        self
    }

    /// Returns a reference to the current topology view used by the allocator.
    pub fn topology(&self) -> &TopologyView {
        &self.topology
    }

    /// Allocates the primary address for an object.
    ///
    /// If the input `Address` already specifies a worker, it's assumed to be
    /// partially or fully pre-assigned. The current logic will return it directly
    /// if a worker is present. Otherwise, it uses a consistent hashing approach.
    ///
    /// # Arguments
    /// * `addr`: An `Address` or a type convertible to `Address` (e.g., `String` for object ID).
    ///   If `addr.worker` is `Some`, this address might be considered pre-assigned.
    ///
    /// # Errors
    /// Returns `AddressError::InsufficientTopology` if the topology view is not sufficient.
    /// Returns `AddressError::NoAvailableNodes` if no suitable node/worker can be found.
    fn allocate_primary_address<A: Into<Address>>(&self, addr: A) -> Result<Address> {
        let addr = addr.into();
        // If worker is specified, consider it (partially) assigned.
        // The original check also included node. Now only worker matters for this check.
        if addr.worker.is_some() {
            // If the intention is that a pre-specified worker means the address is fully resolved,
            // then this direct return is correct.
            // If a pre-specified worker still needs node assignment via topology, this logic would need adjustment.
            // For now, assume worker presence implies it's sufficiently specified for this stage.
            return Ok(addr);
        }

        if !self.is_topology_sufficient() {
            return Err(AddressError::InsufficientTopology);
        }
        let object_id = &addr.object;
        let best_pair = self
            .topology
            .workers_per_node
            .iter()
            .flat_map(|(node_id, workers)| {
                workers.iter().map(move |worker_id| (node_id, worker_id))
            })
            .max_by_key(|&(_node_id, worker_id)| {
                // node_id is available from iteration
                self.calculate_assignment_score(object_id, _node_id, worker_id)
            });

        match best_pair {
            Some((_chosen_node_id, chosen_worker_id)) => Ok(Address::new(
                // Pass only worker to Address::new
                // chosen_node_id.clone(), // No longer passed to Address::new
                chosen_worker_id.clone(),
                object_id.to_string(),
            )),
            None => Err(AddressError::NoAvailableNodes),
        }
    }

    /// Allocates all addresses for an object, including the primary and any configured replicas.
    /// The first address in the returned vector is always the primary.
    /// Replicas are chosen to be on distinct nodes from the primary and from each other,
    /// using the same consistent hashing strategy to select diverse placements.
    ///
    /// # Arguments
    /// * `addr`: An `Address` or a type convertible to `Address` (e.g., `String` for object ID).
    ///
    /// # Returns
    /// A `Result` containing a `Vec<Address>` where the first element is the primary,
    /// followed by replicas. The number of replicas will be up to `self.replica_count`,
    /// limited by the number of available distinct nodes.
    ///
    /// # Errors
    /// Propagates errors from `allocate_primary_address` or `allocate_replica_addresses`.
    pub fn allocate_addresses<A: Into<Address>>(&self, addr: A) -> Result<Vec<Address>> {
        let addr = addr.into();
        let primary = self.allocate_primary_address(addr)?;
        let mut all_addresses = vec![primary.clone()]; // Start with primary

        if self.replica_count == 0 {
            return Ok(all_addresses);
        }

        let replicas = self.allocate_replica_addresses(&primary, self.replica_count)?;
        all_addresses.extend(replicas);

        Ok(all_addresses)
    }

    /// Checks if the current topology view is sufficient for making allocation decisions.
    /// Sufficiency is determined by `min_knowledge_ratio` and `topology_ttl`.
    fn is_topology_sufficient(&self) -> bool {
        self.topology
            .is_sufficient(self.min_knowledge_ratio, self.topology_ttl)
    }

    /// Allocates replica addresses for a given primary address.
    ///
    /// This method attempts to find `num_replicas_to_find` distinct nodes (different from the primary\'s node)
    /// to host the replicas. The selection of replica nodes and their workers also uses the
    /// consistent hashing score (`calculate_assignment_score`) to ensure deterministic and
    /// distributed placement.
    ///
    /// # Arguments
    /// * `primary`: The primary `Address` for which replicas are needed.
    /// * `num_replicas_to_find`: The desired number of replica addresses.
    ///
    /// # Returns
    /// A `Result` containing a `Vec<Address>` of replica addresses. The vector may contain
    /// fewer addresses than `num_replicas_to_find` if not enough distinct suitable nodes
    /// are available. Returns an empty vector if `num_replicas_to_find` is 0.
    ///
    /// # Errors
    /// Returns `AddressError::InternalError` if the primary address has no worker ID,
    /// or if the primary worker's node cannot be found in the topology.
    pub fn allocate_replica_addresses(
        &self,
        primary: &Address,
        num_replicas_to_find: usize,
    ) -> Result<Vec<Address>> {
        if num_replicas_to_find == 0 {
            return Ok(Vec::new());
        }

        let primary_worker_id = primary.worker.as_ref().ok_or_else(|| {
            AddressError::InternalError("Primary address is missing a worker ID".to_string())
        })?;

        // Find the node of the primary worker
        let mut primary_node_id_opt: Option<NodeId> = None;
        for (node_id, worker_list) in &self.topology.workers_per_node {
            if worker_list.contains(primary_worker_id) {
                primary_node_id_opt = Some(node_id.clone());
                break;
            }
        }
        let primary_node_id = primary_node_id_opt.ok_or_else(|| {
            AddressError::InternalError(format!(
                "Node for primary worker '{}' not found in topology",
                primary_worker_id
            ))
        })?;

        // Generate all potential (node, worker) pairs for replicas, score them, and sort.
        let mut potential_replicas: Vec<_> = self
            .topology
            .workers_per_node
            .iter()
            // Exclude primary node. The node_id here is the key from workers_per_node.
            .filter(|(node_id, _workers)| *node_id != &primary_node_id)
            .flat_map(|(node_id, workers)| {
                // node_id is the actual NodeId
                workers
                    .iter()
                    .map(move |worker_id| (node_id.clone(), worker_id.clone())) // Clone to own
            })
            .map(|(node_id, worker_id)| {
                // node_id is NodeId, worker_id is WorkerId
                let score = self.calculate_assignment_score(&primary.object, &node_id, &worker_id);
                (score, node_id, worker_id) // Store NodeId along with WorkerId
            })
            .collect();

        // Sort by score in descending order
        potential_replicas.sort_unstable_by_key(|k| std::cmp::Reverse(k.0));

        let mut replicas = Vec::with_capacity(num_replicas_to_find);
        let mut used_replica_nodes = std::collections::HashSet::new();
        // Add primary's node to prevent choosing it for a replica, though already filtered.
        // This is more of a safeguard if the filter logic changes.
        used_replica_nodes.insert(primary_node_id);

        for (_score, candidate_node_id, candidate_worker_id) in potential_replicas {
            if replicas.len() >= num_replicas_to_find {
                break;
            }

            // Ensure replica is on a distinct node.
            // candidate_node_id is the NodeId of the candidate_worker_id.
            if !used_replica_nodes.contains(&candidate_node_id) {
                replicas.push(Address::new(
                    // candidate_node_id.clone(), // No longer passed to Address::new
                    candidate_worker_id, // Already cloned from map iteration
                    primary.object.clone(),
                ));
                used_replica_nodes.insert(candidate_node_id); // candidate_node_id is already NodeId
            }
        }

        Ok(replicas)
    }

    /// Calculates an assignment score for a given object, node, and worker combination.
    /// This score is used in the consistent hashing algorithm to determine the preferred
    /// placement for objects (both primary and replicas).
    ///
    /// The score is generated by hashing a string composed of the `object_id_seed`,
    /// `node_id`, and `worker_id`. This ensures that for a given object, the scores
    /// for different (node, worker) pairs are consistent, allowing for stable assignments
    /// even as the topology changes. This is a key component of the Rendezvous Hashing-like
    /// behavior, enabling load balancing with potentially incomplete topology information.
    ///
    /// # Arguments
    /// * `object_id_seed`: A string seed, typically the object's unique identifier,
    ///   used to ensure that the same object consistently scores potential locations.
    /// * `node_id`: The ID of the candidate node.
    /// * `worker_id`: The ID of the candidate worker on the node.
    ///
    /// # Returns
    /// A `u64` score. Higher scores are generally preferred during allocation.
    fn calculate_assignment_score(
        &self,
        object_id_seed: &str,
        node_id: &NodeId,
        worker_id: &WorkerId,
    ) -> u64 {
        let mut hasher = DefaultHasher::new();
        format!("{node_id}::{worker_id}:{object_id_seed}").hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    // Helper to get node for a worker from topology, useful for tests.
    // Assuming TopologyView might have or need such a helper.
    // If it's defined elsewhere, this is just for conceptual clarity in test logic.
    trait TopologyViewTestExt {
        fn find_node_for_worker(&self, worker_id: &WorkerId) -> Option<NodeId>;
    }

    impl TopologyViewTestExt for TopologyView {
        fn find_node_for_worker(&self, worker_id_to_find: &WorkerId) -> Option<NodeId> {
            self.workers_per_node.iter().find_map(|(node_id, workers)| {
                if workers.contains(worker_id_to_find) {
                    Some(node_id.clone())
                } else {
                    None
                }
            })
        }
    }

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
        let addr_full = Address::new("worker1", "obj123".to_string());
        let addr_string = addr_full.to_string(); // Uses to_uri("probing")
        assert_eq!(addr_string, "probing://worker1/objects/obj123");
    }

    #[test]
    fn test_to_uri_and_from_uri() {
        let addr = Address::new("worker1", "task_123".to_string());

        // Test with "probing" scheme
        let probing_uri = addr.to_uri("probing");
        assert_eq!(probing_uri, "probing://worker1/objects/task_123");
        let parsed_probing = Address::from_uri(&probing_uri).unwrap();
        assert_eq!(addr, parsed_probing); // Roundtrip check

        // Test with "http" scheme
        let http_uri = addr.to_uri("http");
        assert_eq!(http_uri, "http://worker1/objects/task_123");
        let parsed_http = Address::from_uri(&http_uri).unwrap();
        assert_eq!(addr.worker, parsed_http.worker); // Check components
        assert_eq!(addr.object, parsed_http.object);
        assert_eq!(parsed_http.to_uri("http"), http_uri);

        // Test with "https" scheme
        let https_uri = addr.to_uri("https");
        assert_eq!(https_uri, "https://worker1/objects/task_123");
        let parsed_https = Address::from_uri(&https_uri).unwrap();
        assert_eq!(addr.worker, parsed_https.worker);
        assert_eq!(addr.object, parsed_https.object);
        assert_eq!(parsed_https.to_uri("https"), https_uri);

        // Test with nested object paths
        let addr_nested = Address::new("worker1", "data/user/profile_456".to_string());
        let nested_uri = addr_nested.to_uri("probing");
        assert_eq!(
            nested_uri,
            "probing://worker1/objects/data/user/profile_456"
        );
        let parsed_nested = Address::from_uri(&nested_uri).unwrap();
        assert_eq!(addr_nested, parsed_nested); // Roundtrip check

        // Test URI with a port (port should be ignored by Address parsing but valid for Url::parse)
        let uri_with_port = "probing://worker-port:8080/objects/obj_with_port";
        let parsed_with_port = Address::from_uri(uri_with_port).unwrap();
        assert_eq!(parsed_with_port.worker, Some("worker-port".to_string())); // Hostname is worker-port
        assert_eq!(parsed_with_port.object, "obj_with_port");
        assert_eq!(
            parsed_with_port.to_uri("probing"),
            "probing://worker-port/objects/obj_with_port"
        );
    }

    #[test]
    fn test_uri_scheme_validation() {
        // Test unsupported scheme
        let result = Address::from_uri("ftp://worker1/objects/test"); // Valid structure, invalid scheme
        assert!(result.is_err());
        assert!(matches!(
            result.err().unwrap(),
            AddressError::UnsupportedScheme { scheme } if scheme == "ftp"
        ));

        // Test invalid URI (overall)
        let result_invalid_uri = Address::from_uri("invalid-uri-format");
        assert!(result_invalid_uri.is_err());
        assert!(matches!(
            result_invalid_uri.err().unwrap(),
            AddressError::InvalidUri(_)
        ));

        // Test missing worker (host)
        let result_missing_host = Address::from_uri("probing:///objects/test");
        assert!(result_missing_host.is_err());
        assert!(
            matches!(result_missing_host.err().unwrap(), AddressError::InvalidUri(msg) if msg.contains("Missing worker ID"))
        );

        // Test missing /objects/ prefix
        let result_missing_objects_prefix = Address::from_uri("probing://worker1/test");
        assert!(result_missing_objects_prefix.is_err());
        assert!(
            matches!(result_missing_objects_prefix.err().unwrap(), AddressError::InvalidUri(msg) if msg.contains("Unsupported URI path pattern"))
        );

        // Test empty object_id after /objects/
        let result_empty_object = Address::from_uri("probing://worker1/objects/");
        assert!(result_empty_object.is_err());
        assert!(
            matches!(result_empty_object.err().unwrap(), AddressError::InvalidUri(msg) if msg.contains("Missing object ID in URI path after /objects/"))
        );
    }

    #[test]
    fn test_fromstr_with_uri() {
        // Test URI format parsing
        let uri_addr: Address = "probing://worker1/objects/test_obj".into();
        assert_eq!(uri_addr.worker, Some("worker1".to_string()));
        assert_eq!(uri_addr.object, "test_obj");

        // Test display format (should use URI when possible)
        let display_str = format!("{}", uri_addr);
        assert_eq!(display_str, "probing://worker1/objects/test_obj");
    }

    #[test]
    fn test_shard_key() {
        let addr_full = Address::new("worker1", "obj123".to_string());
        assert_eq!(addr_full.shard_key(), Some("worker1".to_string()));
    }

    #[test]
    fn test_primary_address_allocation() {
        // Renamed from test_address_allocation
        let topology = create_basic_topology(2, 2); // node1 (w1,w2), node2 (w3,w4)
        let allocator = AddressAllocator::new(topology.clone(), 0); // No replicas needed for this test

        let addr_res = allocator.allocate_primary_address("test_object");
        assert!(addr_res.is_ok());
        let addr = addr_res.unwrap();

        // assert!(addr.node.is_some()); // Node not stored in Address
        assert!(addr.worker.is_some());
        // let node_id = addr.node.clone().unwrap(); // Cannot get node_id directly from Address
        let worker_id = addr.worker.clone().unwrap();

        // To verify node, we need to find the worker in the topology
        let mut found_in_topology = false;
        for (_node, workers) in &topology.workers_per_node {
            if workers.contains(&worker_id) {
                found_in_topology = true;
                break;
            }
        }
        assert!(
            found_in_topology,
            "Allocated worker should exist in the topology"
        );
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

        let mut distinct_workers = std::collections::HashSet::new();
        distinct_workers.insert(primary.worker.as_ref().unwrap().clone());

        let mut distinct_nodes_from_topology = std::collections::HashSet::new();
        let primary_worker_node = allocator
            .topology
            .find_node_for_worker(primary.worker.as_ref().unwrap())
            .unwrap();
        distinct_nodes_from_topology.insert(primary_worker_node.clone());

        for i in 1..=replica_count {
            let replica = &all_addrs[i];
            assert_eq!(replica.object, "obj_with_replicas");

            let replica_worker_node = allocator
                .topology
                .find_node_for_worker(replica.worker.as_ref().unwrap())
                .unwrap();
            assert_ne!(
                replica_worker_node, primary_worker_node,
                "Replica should be on a different node than primary"
            );
            // Worker IDs should be distinct if they are on different nodes,
            // or if multiple workers on the same node are chosen (though current replica logic aims for distinct nodes first)
            assert!(
                distinct_workers.insert(replica.worker.as_ref().unwrap().clone()),
                "Replica workers should be distinct (or on distinct nodes)"
            );
            assert!(
                distinct_nodes_from_topology.insert(replica_worker_node),
                "Replica nodes should be distinct based on topology lookup"
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

        // Find nodes for these workers using the topology
        let primary_node = allocator
            .topology
            .find_node_for_worker(primary.worker.as_ref().unwrap())
            .unwrap();
        let replica1_node = allocator
            .topology
            .find_node_for_worker(replica1.worker.as_ref().unwrap())
            .unwrap();
        let replica2_node = allocator
            .topology
            .find_node_for_worker(replica2.worker.as_ref().unwrap())
            .unwrap();

        assert_ne!(primary_node, replica1_node);
        assert_ne!(primary_node, replica2_node);
        assert_ne!(replica1_node, replica2_node);

        for addr in &all_addresses {
            // assert!(addr.node.is_some()); // Node not stored
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

        let primary_node = allocator
            .topology
            .find_node_for_worker(primary.worker.as_ref().unwrap())
            .unwrap();
        let replica1_node = allocator
            .topology
            .find_node_for_worker(replica1.worker.as_ref().unwrap())
            .unwrap();

        assert_ne!(primary_node, replica1_node);
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
        let addr = Address::new("worker1", "obj1".into());
        assert!(addr.is_local("worker1"));
        assert!(!addr.is_local("worker2"));
        // Old tests for node are no longer applicable directly
        // assert!(!addr.is_local("node2", "worker1"));
        // assert!(!addr.is_local("node1", "worker2"));
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

        let topology = TopologyView::new(workers_per_node.clone(), 1); // version 1
        let replica_count = 2;
        let allocator = AddressAllocator::new(topology, replica_count);

        let all_addresses = allocator.allocate_addresses("test-object-functional".to_string())?;

        assert_eq!(all_addresses.len(), 1 + replica_count);

        let primary = &all_addresses[0];
        assert_eq!(primary.object, "test-object-functional");
        assert!(primary.worker.is_some());

        let primary_worker_id_str = primary.worker.as_deref().unwrap();
        let primary_node_id = workers_per_node
            .iter()
            .find_map(|(node, workers)| {
                if workers.contains(&primary_worker_id_str.to_string()) {
                    Some(node.as_str())
                } else {
                    None
                }
            })
            .unwrap_or("UNKNOWN");
        assert!(["node-1", "node-2", "node-3"].contains(&primary_node_id));

        let mut distinct_nodes_from_topology = std::collections::HashSet::new();
        let primary_worker_node_found = allocator
            .topology
            .find_node_for_worker(primary.worker.as_ref().unwrap())
            .expect("Primary worker must be in topology");
        distinct_nodes_from_topology.insert(primary_worker_node_found.clone());

        for i in 0..replica_count {
            let replica = &all_addresses[i + 1];
            assert_eq!(replica.object, "test-object-functional");
            let replica_worker_node = allocator
                .topology
                .find_node_for_worker(replica.worker.as_ref().unwrap())
                .expect("Replica worker must be in topology");

            assert_ne!(
                replica_worker_node, primary_worker_node_found,
                "Replica node should differ from primary"
            );
            assert!(
                distinct_nodes_from_topology.insert(replica_worker_node.clone()),
                "Replica nodes should be distinct among themselves and from primary"
            );
        }

        println!("✓ Functional AddressAllocator test passed.");
        // Display trait will use the new URI format
        println!("Primary Address: {}", primary);
        for (i, replica) in all_addresses.iter().skip(1).enumerate() {
            // If you want to also print the node, you'd look it up via topology
            // let replica_node = allocator.topology.find_node_for_worker(replica.worker.as_ref().unwrap()).unwrap_or_else(|| "NODE_NOT_FOUND".to_string());
            // println!("Replica {}: {} on Node: {}", i + 1, replica, replica_node);
            println!("Replica {}: {}", i + 1, replica);
        }

        Ok(())
    }

    #[test]
    fn test_invalid_uri_patterns() {
        // Test path not starting with /objects/
        let invalid_uri = "probing://worker1/unsupported/pattern";
        let result = Address::from_uri(invalid_uri);
        assert!(result.is_err());
        assert!(
            matches!(result.err().unwrap(), AddressError::InvalidUri(msg) if msg.contains("Unsupported URI path pattern"))
        );

        // Test empty worker_id (host)
        let invalid_uri_empty_host = "probing:///objects/some_object";
        let result_empty_host = Address::from_uri(invalid_uri_empty_host);
        assert!(result_empty_host.is_err());
        assert!(
            matches!(result_empty_host.err().unwrap(), AddressError::InvalidUri(msg) if msg.contains("Missing worker ID"))
        );
    }

    #[test]
    fn test_display_trait() {
        let addr = Address::new("worker1", "obj1".into());
        let display_str = format!("{}", addr);
        assert_eq!(display_str, "probing://worker1/objects/obj1");
    }
}
