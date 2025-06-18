use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use super::addressing::{Address, AddressAllocator};
use super::entity::{EntityId, EntityStore, PersistentEntity};
use super::mem_store::MemoryStore;
use super::topology::TopologyView;
use crate::core::cluster_model::{NodeId, WorkerId};

#[async_trait]
pub trait RemoteStoreClient: Send + Sync {
    async fn put(&self, key: &str, data: &[u8]) -> Result<()>;
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;
    async fn del(&self, key: &str) -> Result<()>;
    async fn is_healthy(&self) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    Primary,
    Quorum,
    All,
}

pub struct DistributedStoreCoordinator {
    node_id: NodeId,
    worker_id: WorkerId,
    topology: Arc<RwLock<TopologyView>>,
    address_allocator: Arc<RwLock<AddressAllocator>>,
    local_store: Arc<MemoryStore>,
    remote_clients: Arc<RwLock<HashMap<String, Arc<dyn RemoteStoreClient>>>>,
    default_replica_count: usize,
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

    pub async fn add_remote_client(&self, address: String, client: Arc<dyn RemoteStoreClient>) {
        self.remote_clients.write().await.insert(address, client);
    }

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

    async fn allocate_addresses<T: PersistentEntity>(&self, entity: &T) -> Result<Vec<Address>> {
        Ok(self
            .address_allocator
            .read()
            .await
            .allocate_addresses(Address::new(
                self.worker_id.clone(),
                entity.id().as_str().to_string(),
            ))?)
    }

    fn select_write_locations<'a>(
        &self,
        locations: &'a [Address],
        consistency: &ConsistencyLevel,
    ) -> Vec<&'a Address> {
        match consistency {
            ConsistencyLevel::Primary => {
                vec![&locations[0]]
            }
            ConsistencyLevel::Quorum => locations.iter().take(1 + locations.len() / 2).collect(),
            ConsistencyLevel::All => locations.iter().collect(),
        }
    }

    async fn delete<T: PersistentEntity>(&self, id: &T::Id, locations: &[&Address]) -> Result<()> {
        let key = format!("{}::{}", T::entity_type(), id.as_str());

        let mut results = Vec::new();

        for location in locations {
            let result = if location.is_local(&self.worker_id) {
                self.local_store.delete::<T>(id).await
            } else {
                let shard_key = location
                    .shard_key()
                    .ok_or_else(|| anyhow!("Invalid address for remote write"))?;

                let clients = self.remote_clients.read().await;
                if let Some(client) = clients.get(&shard_key) {
                    client.del(&key).await
                } else {
                    Err(anyhow!("No remote client for shard: {}", shard_key))
                }
            };

            results.push(result);
        }

        Ok(())
    }

    async fn write<T: PersistentEntity>(
        &self,
        entity: &T,
        locations: &[&Address],
    ) -> Result<Vec<Result<()>>> {
        let serialized = bincode::serialize(entity)?;
        let key = format!("{}::{}", T::entity_type(), entity.id().as_str());

        let mut results = Vec::new();

        for location in locations {
            let result = if location.is_local(&self.worker_id) {
                self.local_store.save(entity).await
            } else {
                let shard_key = location
                    .shard_key()
                    .ok_or_else(|| anyhow!("Invalid address for remote write"))?;

                let clients = self.remote_clients.read().await;
                if let Some(client) = clients.get(&shard_key) {
                    client.put(&key, &serialized).await
                } else {
                    Err(anyhow!("No remote client for shard: {}", shard_key))
                }
            };

            results.push(result);
        }

        Ok(results)
    }

    async fn read<T: PersistentEntity>(&self, id: &T::Id, location: &Address) -> Result<Option<T>> {
        let key = format!("{}::{}", T::entity_type(), id.as_str());

        if location.is_local(&self.worker_id) {
            self.local_store.get::<T>(id).await
        } else {
            let shard_key = location
                .shard_key()
                .ok_or_else(|| anyhow!("Invalid address for remote read"))?;

            let clients = self.remote_clients.read().await;
            if let Some(client) = clients.get(&shard_key) {
                if let Some(data) = client.get(&key).await? {
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
}

impl DistributedStoreCoordinator {
    pub fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    pub fn worker_id(&self) -> &WorkerId {
        &self.worker_id
    }

    pub fn local_store(&self) -> &Arc<MemoryStore> {
        &self.local_store
    }
}

pub struct DistributedEntityStore {
    coordinator: Arc<DistributedStoreCoordinator>,
}

impl DistributedEntityStore {
    pub fn new(coordinator: Arc<DistributedStoreCoordinator>) -> Self {
        Self { coordinator }
    }
}

#[async_trait]
impl EntityStore for DistributedEntityStore {
    async fn save<T: PersistentEntity>(&self, entity: &T) -> Result<()> {
        let locations = self.coordinator.allocate_addresses(entity).await?;

        let write_locations = self
            .coordinator
            .select_write_locations(&locations, &self.coordinator.default_consistency);

        if write_locations.is_empty() && !locations.is_empty() {
            return Err(anyhow!(
                "No suitable write locations found, though addresses were allocated."
            ));
        }
        if write_locations.is_empty() && locations.is_empty() {
            return Err(anyhow!("No addresses allocated for the entity."));
        }

        let write_results = self.coordinator.write(entity, &write_locations).await?;

        let _writes = write_results.iter().filter(|r| r.is_ok()).count();

        Ok(())
    }

    async fn get<T: PersistentEntity>(&self, id: &T::Id) -> Result<Option<T>> {
        if self
            .coordinator
            .local_store
            .raw_entities_contains(id.as_str())
            .await
        {
            if let Some(entity) = self.coordinator.local_store.get::<T>(id).await? {
                return Ok(Some(entity));
            }
        }

        let addr = Address {
            worker: None,
            object: id.to_string(),
        };

        let replicas = self
            .coordinator
            .address_allocator
            .read()
            .await
            .allocate_replica_addresses(&addr, 3)?;

        for location in replicas {
            if let Ok(Some(entity)) = self.coordinator.read::<T>(id, &location).await {
                return Ok(Some(entity));
            }
        }

        Ok(None)
    }

    async fn delete<T: PersistentEntity>(&self, id: &T::Id) -> Result<()> {
        if self
            .coordinator
            .local_store
            .raw_entities_contains(id.as_str())
            .await
        {
            self.coordinator.local_store.delete::<T>(id).await?;
        }

        let addr = Address {
            worker: None,
            object: id.to_string(),
        };

        let replicas = self
            .coordinator
            .address_allocator
            .read()
            .await
            .allocate_replica_addresses(&addr, 3)?;
        self.coordinator
            .delete::<T>(
                &id,
                replicas.iter().map(|a| a).collect::<Vec<_>>().as_slice(),
            )
            .await?;
        Ok(())
    }

    async fn find_by_index<T: PersistentEntity>(
        &self,
        index_name: &str,
        index_value: &str,
    ) -> Result<Vec<T>> {
        self.coordinator
            .local_store()
            .find_by_index(index_name, index_value)
            .await
    }

    async fn list_all<T: PersistentEntity>(&self) -> Result<Vec<T>> {
        self.coordinator.local_store().list_all().await
    }

    async fn list_paginated<T: PersistentEntity>(
        &self,
        limit: usize,
        offset: usize,
    ) -> Result<(Vec<T>, bool)> {
        self.coordinator
            .local_store()
            .list_paginated(limit, offset)
            .await
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
    use crate::core::cluster_model::{NodeId, WorkerId};
    use crate::storage::mem_store::MemoryStore;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
    struct ClusterJob {
        id: String,
        name: String,
        tasks_count: u32,
        status: String,
    }

    impl PersistentEntity for ClusterJob {
        type Id = String;

        fn entity_type() -> &'static str {
            "cluster_job"
        }

        fn id(&self) -> &Self::Id {
            &self.id
        }

        fn version(&self) -> Option<u64> {
            Some(1)
        }
    }

    fn setup_coordinator(
        node_id: NodeId,
        worker_id: WorkerId,
        workers_on_node: Vec<WorkerId>,
        replica_count: usize,
    ) -> Arc<DistributedStoreCoordinator> {
        let mut workers_map = HashMap::new();
        workers_map.insert(node_id.clone(), workers_on_node);
        let topology = TopologyView::new(workers_map, replica_count);
        let local_store = Arc::new(MemoryStore::new());

        Arc::new(DistributedStoreCoordinator::new(
            node_id,
            worker_id,
            topology,
            local_store,
            replica_count,
        ))
    }

    #[tokio::test]
    async fn test_save_and_get_entity_locally() {
        let node_id = "node1".to_string();
        let worker_id = "worker1".to_string();
        let coordinator = setup_coordinator(
            node_id.clone(),
            worker_id.clone(),
            vec![worker_id.clone()],
            1,
        );

        let store = DistributedEntityStore::new(coordinator.clone());

        let job = ClusterJob {
            id: "job123".to_string(),
            name: "Test Job".to_string(),
            tasks_count: 5,
            status: "Pending".to_string(),
        };

        store.save(&job).await.expect("Failed to save job");

        let retrieved_job: Option<ClusterJob> =
            store.get(&job.id).await.expect("Failed to get job");

        assert_eq!(retrieved_job, Some(job));
    }

    #[tokio::test]
    async fn test_entity_not_found() {
        let node_id = "node1".to_string();
        let worker_id = "worker1".to_string();
        let coordinator = setup_coordinator(
            node_id.clone(),
            worker_id.clone(),
            vec![worker_id.clone()],
            1,
        );
        let store = DistributedEntityStore::new(coordinator.clone());

        let retrieved_job: Option<ClusterJob> = store
            .get(&"nonexistentjob".to_string())
            .await
            .expect("Get failed");
        assert!(retrieved_job.is_none());
    }

    #[tokio::test]
    async fn test_save_and_delete_entity() {
        let node_id = "node1".to_string();
        let worker_id = "worker1".to_string();
        let coordinator = setup_coordinator(
            node_id.clone(),
            worker_id.clone(),
            vec![worker_id.clone()],
            1,
        );
        let store = DistributedEntityStore::new(coordinator.clone());

        let job = ClusterJob {
            id: "job456".to_string(),
            name: "Job to Delete".to_string(),
            tasks_count: 3,
            status: "Running".to_string(),
        };

        store
            .save(&job)
            .await
            .expect("Failed to save job for deletion test");
        let retrieved_job: Option<ClusterJob> = store
            .get(&job.id)
            .await
            .expect("Failed to get job before delete");
        assert!(retrieved_job.is_some(), "Job should exist before deletion");

        store
            .delete::<ClusterJob>(&job.id)
            .await
            .expect("Failed to delete job");
        let retrieved_job_after_delete: Option<ClusterJob> = store
            .get(&job.id)
            .await
            .expect("Failed to get job after delete");
        assert!(
            retrieved_job_after_delete.is_none(),
            "Job should not exist after deletion"
        );
    }

    struct MemoryRemoteClient {
        remote_store: Arc<MemoryStore>,
        is_healthy_state: Arc<RwLock<bool>>,
    }

    impl MemoryRemoteClient {
        fn new(store: Arc<MemoryStore>) -> Self {
            Self {
                remote_store: store,
                is_healthy_state: Arc::new(RwLock::new(true)),
            }
        }
        #[allow(dead_code)]
        async fn set_healthy(&self, healthy: bool) {
            *self.is_healthy_state.write().await = healthy;
        }
    }

    #[async_trait]
    impl RemoteStoreClient for MemoryRemoteClient {
        async fn put(&self, key: &str, data: &[u8]) -> Result<()> {
            if !*self.is_healthy_state.read().await {
                return Err(anyhow!("Simulated network error: remote client unhealthy"));
            }
            let parts: Vec<&str> = key.splitn(2, "::").collect();
            if parts.len() == 2 {
                #[derive(Serialize, Deserialize, Clone, Debug)]
                struct RemoteStoredData(Vec<u8>);
                impl PersistentEntity for RemoteStoredData {
                    type Id = String;
                    fn entity_type() -> &'static str {
                        "__remote_bytes__"
                    }
                    fn id(&self) -> &Self::Id {
                        panic!("MemoryRemoteClient save_entity mock needs better ID handling for PersistentEntity");
                    }
                }
                let _ = self
                    .remote_store
                    .raw_entities_save(key.to_string(), data.to_vec())
                    .await;
                Ok(())
            } else {
                Err(anyhow!(
                    "Invalid key format for MemoryRemoteClient: {}",
                    key
                ))
            }
        }

        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
            if !*self.is_healthy_state.read().await {
                return Err(anyhow!("Simulated network error: remote client unhealthy"));
            }
            let parts: Vec<&str> = key.splitn(2, "::").collect();
            if parts.len() == 2 {
                Ok(self.remote_store.raw_entities_get(key).await?)
            } else {
                Err(anyhow!(
                    "Invalid key format for MemoryRemoteClient: {}",
                    key
                ))
            }
        }

        async fn del(&self, key: &str) -> Result<()> {
            if !*self.is_healthy_state.read().await {
                return Err(anyhow!("Simulated network error: remote client unhealthy"));
            }
            let parts: Vec<&str> = key.splitn(2, "::").collect();
            if parts.len() == 2 {
                let _ = self.remote_store.raw_entities_delete(key).await;
                Ok(())
            } else {
                Err(anyhow!(
                    "Invalid key format for MemoryRemoteClient: {}",
                    key
                ))
            }
        }

        async fn is_healthy(&self) -> bool {
            *self.is_healthy_state.read().await
        }
    }
}
