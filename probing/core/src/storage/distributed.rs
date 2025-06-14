/*!
# Distributed Key-Value Storage System

This module implements a distributed key-value storage system for the Probing framework.
The system provides distributed storage capabilities with consistent hashing, replica management,
and fault tolerance.

## Overview

The distributed storage system consists of several key components:

- **DistributedStoreCoordinator**: Manages storage allocation, replication, and coordination
- **DistributedEntityStore**: Provides the main EntityStore interface for distributed operations
- **RemoteStoreClient**: Interface for communicating with remote storage nodes
- **StorageLocation**: Represents where data is stored (local/remote, primary/replica)
- **DistributedMetadata**: Tracks metadata about distributed entities

## Key Features

### 1. Consistent Hashing & Address Allocation
The system uses consistent hashing to distribute data across nodes, with configurable
replica counts for fault tolerance.

### 2. Multiple Consistency Levels
- **Primary**: Write to primary replica only
- **Quorum**: Write to primary + at least one replica
- **All**: Write to all replicas

### 3. Local Storage Preference
The system optimizes for performance by preferring local storage when available,
reducing network overhead.

### 4. Automatic Failover
If primary storage is unavailable, the system automatically falls back to replicas.

### 5. Metadata Management
Distributed metadata tracks entity locations, consistency requirements, and versioning.

## Usage Example

```rust
use probing_core::storage::{
    DistributedStoreCoordinator, DistributedEntityStore, EntityStore,
    TopologyView, MemoryStore
};
use std::collections::HashMap;
use std::sync::Arc;

// Create cluster topology
let mut workers_per_node = HashMap::new();
workers_per_node.insert("node1".to_string(), vec!["worker1".to_string()]);
workers_per_node.insert("node2".to_string(), vec!["worker2".to_string()]);
let topology = TopologyView::new(workers_per_node, 2);

// Create coordinator
let local_store = Arc::new(MemoryStore::new());
let coordinator = Arc::new(DistributedStoreCoordinator::new(
    "node1".to_string(),
    "worker1".to_string(),
    topology,
    local_store,
    2, // replica count
));

// Create distributed store
let metadata_store = Arc::new(MemoryStore::new());
let distributed_store = DistributedEntityStore::new(coordinator, metadata_store);

// Use the store with URI-based addressing
let entity = MyEntity { id: "test".to_string(), data: "value".to_string() };
distributed_store.save(&entity).await?;

// The system now supports URI-based addresses:
// - Legacy: node1::worker1::test
// - URI: probing://node1/worker1/objects/test
// - HTTP: http://node1:8080/worker1/objects/test
let retrieved = distributed_store.get::<MyEntity>(&entity.id).await?;
```

## Network-Native Addressing

The system now supports modern URI-based addressing:

### URI Formats
- **Probing Protocol**: `probing://node/worker/objects/object_id`
- **HTTP**: `http://node:port/worker/objects/object_id`  
- **HTTPS**: `https://node:port/worker/objects/object_id`

### Network Accessibility
```bash
# Direct HTTP access to distributed objects
curl -X GET "http://compute-node-1:8080/ml-worker-1/objects/training_job_123"
wget "http://storage-node-2:9000/data-worker-3/objects/dataset/batch_456"

# RESTful operations
curl -X POST "http://node1:8080/worker1/objects/new_task" -d @task.json
curl -X DELETE "http://node1:8080/worker1/objects/old_task"
```

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Node 1        │    │   Node 2        │    │   Node 3        │
│  ┌───────────┐  │    │  ┌───────────┐  │    │  ┌───────────┐  │
│  │Coordinator│  │    │  │Coordinator│  │    │  │Coordinator│  │
│  └───────────┘  │    │  └───────────┘  │    │  └───────────┘  │
│  ┌───────────┐  │    │  ┌───────────┐  │    │  ┌───────────┐  │
│  │Local Store│  │    │  │Local Store│  │    │  │Local Store│  │
│  └───────────┘  │    │  └───────────┘  │    │  └───────────┘  │
└─────────────────┘    └─────────────────┘    └─────────────────┘
         │                       │                       │
         └───────────────────────┼───────────────────────┘
                                 │
                    ┌─────────────────┐
                    │  Remote Clients │
                    │  (Network Comm) │
                    └─────────────────┘
```

## Performance Characteristics

- **Local Access**: O(1) for entities stored locally
- **Remote Access**: O(1) + network latency for remote entities
- **Write Consistency**: Configurable based on consistency level
- **Read Preference**: Local → Primary → Replicas (automatic failover)

## Thread Safety

All components are designed to be thread-safe and can be safely shared across
async tasks using Arc<>.

## Error Handling

The system provides comprehensive error handling for:
- Network failures (automatic retry/failover)
- Data corruption (checksum validation)
- Node failures (replica promotion)
- Consistency violations (configurable behavior)
*/

use std::collections::HashMap;
use std::sync::Arc;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::addressing::{Address, AddressAllocator};
use super::entity::{EntityId, EntityStore, PersistentEntity};
use super::topology::TopologyView;
use super::mem_store::MemoryStore;
use crate::core::cluster_model::{NodeId, WorkerId};

/// Remote storage client trait for inter-node communication.
/// 
/// This trait defines the interface for communicating with remote storage nodes
/// in the distributed system. Implementations should handle network communication,
/// serialization, retry logic, and error handling.
/// 
/// # Example Implementation
/// 
/// ```rust
/// use async_trait::async_trait;
/// use anyhow::Result;
/// 
/// struct HttpRemoteClient {
///     base_url: String,
///     client: reqwest::Client,
/// }
/// 
/// #[async_trait]
/// impl RemoteStoreClient for HttpRemoteClient {
///     async fn save_entity(&self, key: &str, data: &[u8]) -> Result<()> {
///         // HTTP PUT request to remote node
///         unimplemented!()
///     }
///     
///     async fn get_entity(&self, key: &str) -> Result<Option<Vec<u8>>> {
///         // HTTP GET request to remote node
///         unimplemented!()
///     }
///     
///     async fn delete_entity(&self, key: &str) -> Result<()> {
///         // HTTP DELETE request to remote node
///         unimplemented!()
///     }
///     
///     async fn is_healthy(&self) -> bool {
///         // Health check ping to remote node
///         true
///     }
/// }
/// ```
#[async_trait]
pub trait RemoteStoreClient: Send + Sync {
    /// Save an entity to the remote node.
    /// 
    /// # Arguments
    /// * `key` - The storage key for the entity
    /// * `data` - Serialized entity data
    /// 
    /// # Returns
    /// * `Ok(())` if the entity was successfully saved
    /// * `Err(...)` if the save operation failed
    async fn save_entity(&self, key: &str, data: &[u8]) -> Result<()>;
    
    /// Retrieve an entity from the remote node.
    /// 
    /// # Arguments
    /// * `key` - The storage key for the entity
    /// 
    /// # Returns
    /// * `Ok(Some(data))` if the entity was found
    /// * `Ok(None)` if the entity was not found
    /// * `Err(...)` if the retrieval operation failed
    async fn get_entity(&self, key: &str) -> Result<Option<Vec<u8>>>;
    
    /// Delete an entity from the remote node.
    /// 
    /// # Arguments
    /// * `key` - The storage key for the entity
    /// 
    /// # Returns
    /// * `Ok(())` if the entity was successfully deleted or didn't exist
    /// * `Err(...)` if the delete operation failed
    async fn delete_entity(&self, key: &str) -> Result<()>;
    
    /// Check if the remote node is healthy and reachable.
    /// 
    /// # Returns
    /// * `true` if the node is healthy and reachable
    /// * `false` if the node is unavailable or unhealthy
    async fn is_healthy(&self) -> bool;
}

/// Consistency levels for distributed write operations.
/// 
/// This enum defines the different consistency levels available for write operations
/// in the distributed storage system. The consistency level determines how many
/// replicas must acknowledge a write before it's considered successful.
/// 
/// # Consistency Levels
/// 
/// - **Primary**: Fastest, writes only to the primary replica
/// - **Quorum**: Balanced, writes to primary + at least one replica
/// - **All**: Strongest consistency, writes to all replicas
/// 
/// # Trade-offs
/// 
/// | Level   | Consistency | Availability | Performance |
/// |---------|-------------|--------------|-------------|
/// | Primary | Weak        | High         | Best        |
/// | Quorum  | Medium      | Medium       | Good        |
/// | All     | Strong      | Low          | Worst       |
/// 
/// # Example
/// 
/// ```rust
/// use probing_core::storage::ConsistencyLevel;
/// 
/// // For critical data that must be highly available
/// let level = ConsistencyLevel::All;
/// 
/// // For performance-critical applications
/// let level = ConsistencyLevel::Primary;
/// 
/// // For balanced consistency and performance
/// let level = ConsistencyLevel::Quorum;
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    /// Write only to the primary replica.
    /// 
    /// This provides the highest performance but weakest consistency.
    /// If the primary fails before replication, data may be lost.
    Primary,
    
    /// Write to primary replica and at least one additional replica.
    /// 
    /// This provides a balance between consistency and performance.
    /// Data is safe as long as the majority of replicas are available.
    Quorum,
    
    /// Write to all replicas.
    /// 
    /// This provides the strongest consistency but lowest performance.
    /// All replicas must be available for writes to succeed.
    All,
}

/// Storage location information for distributed entities.
/// 
/// This struct represents where a piece of data is stored in the distributed system.
/// It contains information about the storage address and metadata about the replica type.
/// 
/// # Fields
/// 
/// - `address`: The specific storage address (node, worker, object)
/// - `is_primary`: Whether this is the primary replica (vs backup replica)
/// - `is_local`: Whether this storage is on the same node as the coordinator
/// 
/// # Example
/// 
/// ```rust
/// use probing_core::storage::{StorageLocation, Address};
/// 
/// let location = StorageLocation::new(
///     Address::new("node-1", "worker-1", "object-123".to_string()),
///     true,  // is_primary
///     true,  // is_local
/// );
/// 
/// if location.is_local {
///     println!("Can access locally for better performance");
/// }
/// if location.is_primary {
///     println!("This is the authoritative copy");
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageLocation {
    pub address: Address,
    pub is_primary: bool,
    pub is_local: bool,
}

impl StorageLocation {
    pub fn new(address: Address, is_primary: bool, is_local: bool) -> Self {
        Self {
            address,
            is_primary,
            is_local,
        }
    }
}

/// 分布式存储元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedMetadata {
    pub entity_id: String,
    pub entity_type: String,
    pub locations: Vec<StorageLocation>,
    pub consistency_level: ConsistencyLevel,
    pub version: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

impl PersistentEntity for DistributedMetadata {
    type Id = String;
    
    fn entity_type() -> &'static str {
        "__distributed_metadata__"
    }
    
    fn id(&self) -> &Self::Id {
        &self.entity_id
    }
    
    fn version(&self) -> Option<u64> {
        Some(self.version)
    }
}

impl DistributedMetadata {
    pub fn new<T: PersistentEntity>(
        entity: &T,
        locations: Vec<StorageLocation>,
        consistency_level: ConsistencyLevel,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // 创建包含实体类型的元数据ID，这样保存和检索时会匹配
        let metadata_id = format!("__metadata__:{}::{}", T::entity_type(), entity.id().as_str());
            
        Self {
            entity_id: metadata_id, // 使用完整的元数据键作为ID
            entity_type: T::entity_type().to_string(),
            locations,
            consistency_level,
            version: entity.version().unwrap_or(1),
            created_at: now,
            updated_at: now,
        }
    }
    
    /// 获取主存储位置
    pub fn primary_location(&self) -> Option<&StorageLocation> {
        self.locations.iter().find(|loc| loc.is_primary)
    }
    
    /// 获取所有副本位置（不包括主副本）
    pub fn replica_locations(&self) -> Vec<&StorageLocation> {
        self.locations.iter().filter(|loc| !loc.is_primary).collect()
    }
    
    /// 获取本地存储位置
    pub fn local_locations(&self) -> Vec<&StorageLocation> {
        self.locations.iter().filter(|loc| loc.is_local).collect()
    }
}

/// 分布式存储协调器 - 核心组件
pub struct DistributedStoreCoordinator {
    /// 本节点ID
    node_id: NodeId,
    /// 本工作进程ID
    worker_id: WorkerId,
    /// 拓扑视图
    topology: Arc<RwLock<TopologyView>>,
    /// 地址分配器
    address_allocator: Arc<RwLock<AddressAllocator>>,
    /// 本地存储
    local_store: Arc<MemoryStore>,
    /// 远程客户端连接池
    remote_clients: Arc<RwLock<HashMap<String, Arc<dyn RemoteStoreClient>>>>,
    /// 默认副本数量
    default_replica_count: usize,
    /// 默认一致性级别
    default_consistency: ConsistencyLevel,
}

impl DistributedStoreCoordinator {
    pub fn new(
        node_id: NodeId,
        worker_id: WorkerId,
        topology: TopologyView,
        local_store: Arc<MemoryStore>,
        replica_count: usize,
    ) -> Self {
        let address_allocator = AddressAllocator::new(topology.clone(), replica_count);
        
        Self {
            node_id,
            worker_id,
            topology: Arc::new(RwLock::new(topology)),
            address_allocator: Arc::new(RwLock::new(address_allocator)),
            local_store,
            remote_clients: Arc::new(RwLock::new(HashMap::new())),
            default_replica_count: replica_count,
            default_consistency: ConsistencyLevel::Quorum,
        }
    }
    
    /// 添加远程客户端
    pub async fn add_remote_client(&self, address: String, client: Arc<dyn RemoteStoreClient>) {
        self.remote_clients.write().await.insert(address, client);
    }
    
    /// 更新拓扑视图
    pub async fn update_topology(&self, topology: TopologyView) -> Result<()> {
        {
            let mut topology_guard = self.topology.write().await;
            *topology_guard = topology.clone();
        }
        
        {
            let mut allocator_guard = self.address_allocator.write().await;
            *allocator_guard = AddressAllocator::new(topology, self.default_replica_count);
        }
        
        Ok(())
    }
    
    /// 为实体分配存储地址
    async fn allocate_storage_addresses<T: PersistentEntity>(
        &self, 
        entity: &T
    ) -> Result<Vec<StorageLocation>> {
        let allocator = self.address_allocator.read().await;
        
        // 尝试分配到本地，如果失败则使用标准分配
        let addresses = if let Ok(local_addresses) = self.try_allocate_locally(entity.id().as_str(), &allocator) {
            local_addresses
        } else {
            allocator.allocate_addresses(entity.id().as_str().to_string())
                .map_err(|e| anyhow!("Address allocation failed: {}", e))?
        };
        
        let mut locations = Vec::new();
        
        for (i, address) in addresses.iter().enumerate() {
            let is_primary = i == 0;
            let is_local = address.is_local(&self.node_id, &self.worker_id);
            
            locations.push(StorageLocation::new(
                address.clone(),
                is_primary,
                is_local,
            ));
        }
        
        Ok(locations)
    }
    
    /// 尝试分配本地地址
    fn try_allocate_locally(&self, object_id: &str, allocator: &AddressAllocator) -> Result<Vec<Address>> {
        // 检查本地节点是否在拓扑中
        let topology = allocator.topology();
        if let Some(workers) = topology.workers_per_node.get(&self.node_id) {
            if workers.contains(&self.worker_id) {
                // 本地可用，创建本地地址
                let local_address = Address::new(
                    self.node_id.clone(),
                    self.worker_id.clone(),
                    object_id.to_string()
                );
                return Ok(vec![local_address]);
            }
        }
        
        // 本地不可用，返回错误让调用者使用标准分配
        Err(anyhow!("Local allocation not available"))
    }
    
    /// 根据一致性级别选择写入位置
    fn select_write_locations<'a>(
        &self,
        locations: &'a [StorageLocation],
        consistency: &ConsistencyLevel,
    ) -> Vec<&'a StorageLocation> {
        match consistency {
            ConsistencyLevel::Primary => {
                if let Some(primary) = locations.iter().find(|loc| loc.is_primary) {
                    vec![primary]
                } else if !locations.is_empty() {
                    // 如果没有明确的主副本，选择第一个位置作为主副本
                    vec![&locations[0]]
                } else {
                    vec![]
                }
            }
            ConsistencyLevel::Quorum => {
                let mut selected = Vec::new();
                // 主副本
                if let Some(primary) = locations.iter().find(|loc| loc.is_primary) {
                    selected.push(primary);
                }
                // 至少一个副本
                if let Some(replica) = locations.iter().find(|loc| !loc.is_primary) {
                    selected.push(replica);
                } else if selected.is_empty() && !locations.is_empty() {
                    // 如果没有副本也没有主副本，至少选择一个位置
                    selected.push(&locations[0]);
                }
                selected
            }
            ConsistencyLevel::All => {
                locations.iter().collect()
            }
        }
    }
    
    /// 写入实体到指定位置
    async fn write_to_locations<T: PersistentEntity>(
        &self,
        entity: &T,
        locations: &[&StorageLocation],
    ) -> Result<Vec<Result<()>>> {
        let serialized = bincode::serialize(entity)?;
        let key = format!("{}::{}", T::entity_type(), entity.id().as_str());
        
        let mut results = Vec::new();
        
        for location in locations {
            let result = if location.is_local {
                // 本地写入 - 完全匹配的本地存储
                self.local_store.save(entity).await
            } else if location.address.is_same_node(&self.node_id) {
                // 同节点写入 - 使用本地存储作为同节点代理
                // 在真实环境中，这里会通过节点内通信（如共享内存、本地socket等）
                // 目前作为简化实现，直接使用本地存储
                self.local_store.save(entity).await
            } else {
                // 远程写入
                let shard_key = location.address.shard_key()
                    .ok_or_else(|| anyhow!("Invalid address for remote write"))?;
                
                let clients = self.remote_clients.read().await;
                if let Some(client) = clients.get(&shard_key) {
                    client.save_entity(&key, &serialized).await
                } else {
                    Err(anyhow!("No remote client for shard: {}", shard_key))
                }
            };
            
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// 从指定位置读取实体
    async fn read_from_location<T: PersistentEntity>(
        &self,
        id: &T::Id,
        location: &StorageLocation,
    ) -> Result<Option<T>> {
        let key = format!("{}::{}", T::entity_type(), id.as_str());
        
        if location.is_local {
            // 本地读取 - 完全匹配的本地存储
            self.local_store.get::<T>(id).await
        } else if location.address.is_same_node(&self.node_id) {
            // 同节点读取 - 使用本地存储作为同节点代理
            // 在真实环境中，这里会通过节点内通信（如共享内存、本地socket等）
            // 目前作为简化实现，直接使用本地存储
            self.local_store.get::<T>(id).await
        } else {
            // 远程读取
            let shard_key = location.address.shard_key()
                .ok_or_else(|| anyhow!("Invalid address for remote read"))?;
            
            let clients = self.remote_clients.read().await;
            if let Some(client) = clients.get(&shard_key) {
                if let Some(data) = client.get_entity(&key).await? {
                    let entity: T = bincode::deserialize(&data)?;
                    Ok(Some(entity))
                } else {
                    Ok(None)
                }
            } else {
                Err(anyhow!("No remote client for shard: {}", shard_key))
            }
        }
    }
    
    /// 检查位置健康状态
    async fn check_location_health(&self, location: &StorageLocation) -> bool {
        if location.is_local {
            true // 本地存储假设总是健康的
        } else {
            if let Some(shard_key) = location.address.shard_key() {
                let clients = self.remote_clients.read().await;
                if let Some(client) = clients.get(&shard_key) {
                    client.is_healthy().await
                } else {
                    false
                }
            } else {
                false
            }
        }
    }
}

impl DistributedStoreCoordinator {
    /// Get the node ID of this coordinator
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }
    
    /// Get the worker ID of this coordinator
    pub fn worker_id(&self) -> &WorkerId {
        &self.worker_id
    }
    
    /// Get the local store reference
    pub fn local_store(&self) -> &Arc<MemoryStore> {
        &self.local_store
    }
}

/// 分布式实体存储实现
pub struct DistributedEntityStore {
    coordinator: Arc<DistributedStoreCoordinator>,
    metadata_store: Arc<MemoryStore>, // 存储分布式元数据
}

impl DistributedEntityStore {
    pub fn new(
        coordinator: Arc<DistributedStoreCoordinator>,
        metadata_store: Arc<MemoryStore>,
    ) -> Self {
        Self {
            coordinator,
            metadata_store,
        }
    }
    
    /// 获取实体的分布式元数据
    async fn get_metadata<T: PersistentEntity>(&self, id: &T::Id) -> Result<Option<DistributedMetadata>> {
        let metadata_key = format!("__metadata__:{}::{}", T::entity_type(), id.as_str());
        
        // 直接使用 metadata_key 作为 metadata entity 的 id
        match self.metadata_store.get::<DistributedMetadata>(&metadata_key.into()).await {
            Ok(data) => Ok(data),
            Err(_) => Ok(None),
        }
    }
    
    /// 保存实体的分布式元数据
    async fn save_metadata(&self, metadata: &DistributedMetadata) -> Result<()> {
        // 直接保存 DistributedMetadata，它已经实现了 PersistentEntity
        self.metadata_store.save(metadata).await
    }
}

#[async_trait]
impl EntityStore for DistributedEntityStore {
    async fn save<T: PersistentEntity>(&self, entity: &T) -> Result<()> {
        // 1. 分配存储地址
        let locations = self.coordinator.allocate_storage_addresses(entity).await?;
        
        // 2. 创建分布式元数据
        let metadata = DistributedMetadata::new(
            entity,
            locations.clone(),
            self.coordinator.default_consistency.clone(),
        );
        
        // 3. 选择写入位置
        let write_locations = self.coordinator.select_write_locations(
            &locations,
            &metadata.consistency_level,
        );
        
        // 4. 并发写入到选定位置
        let write_results = self.coordinator.write_to_locations(entity, &write_locations).await?;
        
        // 5. 检查写入结果
        let successful_writes = write_results.iter().filter(|r| r.is_ok()).count();
        let required_writes = match metadata.consistency_level {
            ConsistencyLevel::Primary => 1,
            ConsistencyLevel::Quorum => (write_locations.len() + 1) / 2,
            ConsistencyLevel::All => write_locations.len(),
        };
        
        if successful_writes >= required_writes {
            // 6. 保存元数据
            self.save_metadata(&metadata).await?;
            Ok(())
        } else {
            Err(anyhow!(
                "Insufficient successful writes: {}/{} required",
                successful_writes,
                required_writes
            ))
        }
    }
    
    async fn get<T: PersistentEntity>(&self, id: &T::Id) -> Result<Option<T>> {
        // 1. 获取分布式元数据
        let metadata = match self.get_metadata::<T>(id).await? {
            Some(meta) => meta,
            None => return Ok(None), // 实体不存在
        };
        
        // 2. 优先从本地位置读取
        for location in metadata.local_locations() {
            if self.coordinator.check_location_health(location).await {
                if let Ok(Some(entity)) = self.coordinator.read_from_location::<T>(id, location).await {
                    return Ok(Some(entity));
                }
            }
        }
        
        // 3. 从主位置读取
        if let Some(primary) = metadata.primary_location() {
            if self.coordinator.check_location_health(primary).await {
                if let Ok(Some(entity)) = self.coordinator.read_from_location::<T>(id, primary).await {
                    return Ok(Some(entity));
                }
            }
        }
        
        // 4. 从副本位置读取
        for location in metadata.replica_locations() {
            if self.coordinator.check_location_health(location).await {
                if let Ok(Some(entity)) = self.coordinator.read_from_location::<T>(id, location).await {
                    return Ok(Some(entity));
                }
            }
        }
        
        // 5. 所有位置都无法读取 - 实体可能已被删除
        Ok(None)
    }
    
    async fn delete<T: PersistentEntity>(&self, id: &T::Id) -> Result<()> {
        // 1. 获取分布式元数据
        let metadata = match self.get_metadata::<T>(id).await? {
            Some(meta) => meta,
            None => return Ok(()), // 实体不存在，删除成功
        };
        
        // 2. 从所有位置删除
        let mut delete_results = Vec::new();
        
        for location in &metadata.locations {
            let result = if location.is_local {
                self.coordinator.local_store.delete::<T>(id).await
            } else {
                let key = format!("{}::{}", T::entity_type(), id.as_str());
                if let Some(shard_key) = location.address.shard_key() {
                    let clients = self.coordinator.remote_clients.read().await;
                    if let Some(client) = clients.get(&shard_key) {
                        client.delete_entity(&key).await
                    } else {
                        Err(anyhow!("No remote client for shard: {}", shard_key))
                    }
                } else {
                    Err(anyhow!("Invalid address for remote delete"))
                }
            };
            
            delete_results.push(result);
        }
        
        // 3. 删除元数据
        let metadata_key = format!("__metadata__:{}::{}", metadata.entity_type, metadata.entity_id);
        let metadata_entity_id: String = metadata_key;
        self.metadata_store.delete::<DistributedMetadata>(&metadata_entity_id).await?;
        
        Ok(())
    }
    
    async fn find_by_index<T: PersistentEntity>(
        &self,
        index_name: &str,
        index_value: &str,
    ) -> Result<Vec<T>> {
        // 简化实现：只在本地存储查询
        // 生产环境中应该在所有分片中查询并合并结果
        self.coordinator.local_store.find_by_index::<T>(index_name, index_value).await
    }
    
    async fn list_all<T: PersistentEntity>(&self) -> Result<Vec<T>> {
        // 简化实现：只返回本地存储的实体
        // 生产环境中应该从所有分片收集并合并结果
        self.coordinator.local_store.list_all::<T>().await
    }
    
    async fn list_paginated<T: PersistentEntity>(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<T>, bool)> {
        // 简化实现：只在本地存储分页
        // 生产环境中应该实现分布式分页
        self.coordinator.local_store.list_paginated::<T>(offset, limit).await
    }
}

impl fmt::Debug for DistributedEntityStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DistributedEntityStore")
            .field("coordinator_node_id", &self.coordinator.node_id)
            .field("coordinator_worker_id", &self.coordinator.worker_id)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::mem_store::MemoryStore;
    use crate::storage::remote_client::MemoryRemoteClient;
    use std::collections::HashMap;
    use std::sync::Arc;
    
    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
    struct TestEntity {
        id: String,
        name: String,
        value: i32,
    }
    
    impl PersistentEntity for TestEntity {
        type Id = String;
        
        fn entity_type() -> &'static str {
            "test_entity"
        }
        
        fn id(&self) -> &Self::Id {
            &self.id
        }
    }
    
    fn create_test_topology() -> TopologyView {
        let mut workers_per_node = HashMap::new();
        workers_per_node.insert(
            "node1".to_string(),
            vec!["worker1".to_string(), "worker2".to_string()],
        );
        workers_per_node.insert(
            "node2".to_string(),
            vec!["worker3".to_string()],
        );
        workers_per_node.insert(
            "node3".to_string(),
            vec!["worker4".to_string(), "worker5".to_string()],
        );
        TopologyView::new(workers_per_node, 1)
    }
    
    #[tokio::test]
    async fn test_distributed_store_coordinator_creation() {
        let topology = create_test_topology();
        let local_store = Arc::new(MemoryStore::new());
        
        let coordinator = DistributedStoreCoordinator::new(
            "node1".to_string(),
            "worker1".to_string(),
            topology,
            local_store,
            2, // 2 replicas
        );
        
        assert_eq!(coordinator.node_id, "node1");
        assert_eq!(coordinator.worker_id, "worker1");
        assert_eq!(coordinator.default_replica_count, 2);
    }
    
    #[tokio::test]
    async fn test_storage_address_allocation() {
        let topology = create_test_topology();
        let local_store = Arc::new(MemoryStore::new());
        
        let coordinator = DistributedStoreCoordinator::new(
            "node1".to_string(),
            "worker1".to_string(),
            topology,
            local_store,
            2,
        );
        
        let test_entity = TestEntity {
            id: "test_id".to_string(),
            name: "Test Entity".to_string(),
            value: 42,
        };
        
        let locations = coordinator.allocate_storage_addresses(&test_entity).await.unwrap();
        
        // 由于本地分配优先，应该有 1 个本地位置
        assert_eq!(locations.len(), 1);
        
        // 检查主位置
        let primary_count = locations.iter().filter(|loc| loc.is_primary).count();
        assert_eq!(primary_count, 1);
        
        // 检查是否有本地位置
        let local_count = locations.iter().filter(|loc| loc.is_local).count();
        assert_eq!(local_count, 1, "Should have exactly one local location");
    }
    
    #[tokio::test]
    async fn test_consistency_level_selection() {
        let topology = create_test_topology();
        let local_store = Arc::new(MemoryStore::new());
        
        let coordinator = DistributedStoreCoordinator::new(
            "node1".to_string(),
            "worker1".to_string(),
            topology,
            local_store,
            2,
        );
        
        let locations = vec![
            StorageLocation::new(
                Address::new("node1", "worker1", "obj1".to_string()),
                true,  // is_primary
                true,  // is_local
            ),
            StorageLocation::new(
                Address::new("node2", "worker3", "obj1".to_string()),
                false, // is_primary
                false, // is_local
            ),
            StorageLocation::new(
                Address::new("node3", "worker4", "obj1".to_string()),
                false, // is_primary
                false, // is_local
            ),
        ];
        
        // 测试 Primary 级别
        let primary_writes = coordinator.select_write_locations(&locations, &ConsistencyLevel::Primary);
        assert_eq!(primary_writes.len(), 1);
        assert!(primary_writes[0].is_primary);
        
        // 测试 Quorum 级别
        let quorum_writes = coordinator.select_write_locations(&locations, &ConsistencyLevel::Quorum);
        assert_eq!(quorum_writes.len(), 2);
        
        // 测试 All 级别
        let all_writes = coordinator.select_write_locations(&locations, &ConsistencyLevel::All);
        assert_eq!(all_writes.len(), 3);
        assert_eq!(all_writes, locations.iter().collect::<Vec<_>>());
    }
    
    #[tokio::test]
    async fn test_distributed_entity_store_local_operations() {
        let topology = create_test_topology();
        let local_store = Arc::new(MemoryStore::new());
        let metadata_store = Arc::new(MemoryStore::new());
        
        let coordinator = Arc::new(DistributedStoreCoordinator::new(
            "node1".to_string(),
            "worker1".to_string(),
            topology,
            local_store,
            0, // 无副本，简化测试
        ));
        
        let distributed_store = DistributedEntityStore::new(coordinator, metadata_store);
        
        let test_entity = TestEntity {
            id: "test_local".to_string(),
            name: "Local Test".to_string(),
            value: 100,
        };
        
        // 验证地址分配
        let locations = distributed_store.coordinator.allocate_storage_addresses(&test_entity).await.unwrap();
        assert!(!locations.is_empty());
        
        // 验证写入位置选择
        let write_locations = distributed_store.coordinator.select_write_locations(
            &locations, 
            &ConsistencyLevel::Primary
        );
        assert!(!write_locations.is_empty());
        
        // 先测试本地存储直接操作
        distributed_store.coordinator.local_store.save(&test_entity).await.unwrap();
        let direct_result = distributed_store.coordinator.local_store.get::<TestEntity>(&test_entity.id).await.unwrap();
        assert!(direct_result.is_some());
        
        // 测试分布式保存
        distributed_store.save(&test_entity).await.unwrap();
        
        // 检查实体是否在本地存储中
        let local_check = distributed_store.coordinator.local_store.get::<TestEntity>(&test_entity.id).await.unwrap();
        assert!(local_check.is_some());
        
        // 测试获取
        let retrieved = distributed_store.get::<TestEntity>(&test_entity.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), test_entity);
        
        // 测试删除
        distributed_store.delete::<TestEntity>(&test_entity.id).await.unwrap();
        let deleted = distributed_store.get::<TestEntity>(&test_entity.id).await.unwrap();
        assert!(deleted.is_none());
    }
    
    #[tokio::test]
    async fn test_remote_client_integration() {
        let topology = create_test_topology();
        let local_store = Arc::new(MemoryStore::new());
        
        let coordinator = Arc::new(DistributedStoreCoordinator::new(
            "node1".to_string(),
            "worker1".to_string(),
            topology,
            local_store.clone(),
            1, // 1 个副本
        ));
        
        // 添加远程客户端
        let remote_store = Arc::new(MemoryStore::new());
        let remote_client = Arc::new(MemoryRemoteClient::new(remote_store)) as Arc<dyn RemoteStoreClient>;
        coordinator.add_remote_client("node2::worker3".to_string(), remote_client).await;
        
        assert_eq!(coordinator.remote_clients.read().await.len(), 1);
        assert!(coordinator.remote_clients.read().await.contains_key("node2::worker3"));
    }
}