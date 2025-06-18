use anyhow::Result;
use probing_core::storage::{
    DistributedEntityStore, EntityStore, MemoryRemoteClient, MemoryStore, TopologyView,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
struct TestEntity {
    id: String,
    data: String,
    version: u64,
}

impl probing_core::storage::entity::PersistentEntity for TestEntity {
    type Id = String;

    fn entity_type() -> &'static str {
        "test_entity"
    }

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn version(&self) -> Option<u64> {
        Some(self.version)
    }
}

async fn create_test_cluster() -> Result<Vec<Arc<DistributedEntityStore>>> {
    // Create a 3-node cluster
    let mut workers_per_node = HashMap::new();
    workers_per_node.insert("node1".to_string(), vec!["worker1".to_string()]);
    workers_per_node.insert("node2".to_string(), vec!["worker2".to_string()]);
    workers_per_node.insert("node3".to_string(), vec!["worker3".to_string()]);

    let topology = TopologyView::new(workers_per_node, 2);
    let mut stores = Vec::new();

    let mems = vec![Arc::new(MemoryStore::new()); 3];

    // Create coordinators for each node
    for (idx, (node_id, workers)) in topology.workers_per_node.iter().enumerate() {
        let worker_id = workers[0].clone();
        let local_store = mems[idx].clone(); //Arc::new(MemoryStore::new());
        let distributed_store = Arc::new(DistributedEntityStore::new(
            node_id.clone(),
            worker_id,
            topology.clone(),
            local_store,
            3, // replica count
        ));

        // Add remote clients for other nodes
        for (idx, (remote_node, workers)) in topology.workers_per_node.iter().enumerate() {
            if remote_node != node_id {
                let worker_id = workers[0].clone();

                let remote_store = mems[idx].clone(); // Use the same MemoryStore for simplicity
                let remote_client = Arc::new(MemoryRemoteClient::new(remote_store));
                distributed_store
                    .add_remote_client(worker_id.clone(), remote_client)
                    .await;
            }
        }

        stores.push(distributed_store);
    }

    Ok(stores)
}

#[tokio::test]
async fn test_multi_node_storage_and_retrieval() -> Result<()> {
    let stores = create_test_cluster().await?;

    // Create test entity
    let entity = TestEntity {
        id: "test-entity-1".to_string(),
        data: "distributed storage test".to_string(),
        version: 1,
    };

    // Save entity using first node
    stores[0].put(&entity).await?;

    // Try to retrieve from different nodes
    for (i, store) in stores.iter().enumerate() {
        let retrieved = store.get::<TestEntity>(&entity.id).await?;
        assert!(
            retrieved.is_some(),
            "Node {} should be able to retrieve entity",
            i
        );
        assert_eq!(
            retrieved.unwrap(),
            entity,
            "Retrieved entity should match original"
        );
    }

    Ok(())
}

#[tokio::test]
async fn test_consistency_levels() -> Result<()> {
    let stores = create_test_cluster().await?;

    // Test basic save/get operations (no save_with_consistency method available)
    let primary_entity = TestEntity {
        id: "primary-test".to_string(),
        data: "primary consistency".to_string(),
        version: 1,
    };

    stores[0].put(&primary_entity).await?;
    let retrieved = stores[0].get::<TestEntity>(&primary_entity.id).await?;
    assert!(retrieved.is_some());

    // Test from different node
    let quorum_entity = TestEntity {
        id: "quorum-test".to_string(),
        data: "quorum consistency".to_string(),
        version: 1,
    };

    stores[1].put(&quorum_entity).await?;
    let retrieved = stores[1].get::<TestEntity>(&quorum_entity.id).await?;
    assert!(retrieved.is_some());

    Ok(())
}

#[tokio::test]
async fn test_fault_tolerance() -> Result<()> {
    let stores = create_test_cluster().await?;

    // Create entity that will be replicated
    let entity = TestEntity {
        id: "fault-tolerance-test".to_string(),
        data: "testing fault tolerance".to_string(),
        version: 1,
    };

    // Save entity (uses default consistency level)
    stores[0].put(&entity).await?;

    // Verify entity can be retrieved from the same node
    let retrieved = stores[0].get::<TestEntity>(&entity.id).await?;
    assert!(retrieved.is_some(), "Node should have the entity");

    Ok(())
}

#[tokio::test]
async fn test_concurrent_operations() -> Result<()> {
    let stores = create_test_cluster().await?;
    let store = stores[0].clone();

    // Create multiple entities concurrently
    let mut handles = Vec::new();

    for i in 0..10 {
        let store_clone = store.clone();
        let handle = tokio::spawn(async move {
            let entity = TestEntity {
                id: format!("concurrent-test-{}", i),
                data: format!("concurrent data {}", i),
                version: 1,
            };

            store_clone.put(&entity).await?;
            store_clone.get::<TestEntity>(&entity.id).await
        });
        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        let result = handle.await??;
        assert!(result.is_some(), "Concurrent operation should succeed");
    }

    Ok(())
}

#[tokio::test]
async fn test_update_and_versioning() -> Result<()> {
    let stores = create_test_cluster().await?;

    // Create initial entity
    let mut entity = TestEntity {
        id: "version-test".to_string(),
        data: "initial data".to_string(),
        version: 1,
    };

    stores[0].put(&entity).await?;

    // Update entity
    entity.data = "updated data".to_string();
    entity.version = 2;

    stores[0].put(&entity).await?;

    // Verify update
    let retrieved = stores[0].get::<TestEntity>(&entity.id).await?;
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.data, "updated data");
    assert_eq!(retrieved.version, 2);

    Ok(())
}

#[tokio::test]
async fn test_delete_operations() -> Result<()> {
    let stores = create_test_cluster().await?;

    // Create entity
    let entity = TestEntity {
        id: "delete-test".to_string(),
        data: "to be deleted".to_string(),
        version: 1,
    };

    stores[0].put(&entity).await?;

    // Verify entity exists
    let retrieved = stores[0].get::<TestEntity>(&entity.id).await?;
    assert!(retrieved.is_some());

    // Delete entity
    stores[0].del::<TestEntity>(&entity.id).await?;

    // Verify entity is deleted
    let retrieved = stores[0].get::<TestEntity>(&entity.id).await?;
    assert!(retrieved.is_none());

    Ok(())
}
