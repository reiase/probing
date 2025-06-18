use super::distributed::RemoteStoreClient;
use super::mem_store::MemoryStore; // Add this line
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc; // Add this line

/// In-memory remote client for testing
#[derive(Clone)] // Add Clone
pub struct MemoryRemoteClient {
    store: Arc<MemoryStore>, // Change data to store
}

impl MemoryRemoteClient {
    pub fn new(store: Arc<MemoryStore>) -> Self { // Modify constructor
        Self { store }
    }
}

#[async_trait]
impl RemoteStoreClient for MemoryRemoteClient {
    async fn put(&self, key: &str, data: &[u8]) -> Result<()> {
        self.store.raw_entities_save(key.to_string(), data.to_vec()).await
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.store.raw_entities_get(key).await
    }

    async fn del(&self, key: &str) -> Result<()> {
        self.store.raw_entities_delete(key).await
    }

    async fn is_healthy(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::mem_store::MemoryStore; // Add this line
    use std::sync::Arc; // Add this line

    #[tokio::test]
    async fn test_memory_remote_client() {
        let store = Arc::new(MemoryStore::new()); // Add this line
        let client = MemoryRemoteClient::new(store.clone()); // Modify this line
        let key = "test_key";
        let data = b"test_data";

        // Test save
        client.put(key, data).await.unwrap();

        // Test get
        let retrieved = client.get(key).await.unwrap();
        assert_eq!(retrieved, Some(data.to_vec()));

        // Test delete
        client.del(key).await.unwrap();
        let retrieved = client.get(key).await.unwrap();
        assert_eq!(retrieved, None);

        // Test health
        assert!(client.is_healthy().await);
    }
}
