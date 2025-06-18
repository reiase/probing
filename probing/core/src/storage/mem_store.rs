// probing/core/src/storage/sled_store.rs
use super::entity::{EntityId, EntityStore, PersistentEntity};
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

/// 内存存储实现，用于测试
#[derive(Default, Clone)]
pub struct MemoryStore {
    entities: Arc<tokio::sync::RwLock<HashMap<String, Vec<u8>>>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    // New methods for raw access to the 'entities' map, used by MemoryRemoteClient
    pub async fn raw_entities_save(&self, key: String, data: Vec<u8>) -> Result<()> {
        self.entities.write().await.insert(key, data);
        Ok(())
    }

    pub async fn raw_entities_get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        Ok(self.entities.read().await.get(key).cloned())
    }

    pub async fn raw_entities_delete(&self, key: &str) -> Result<()> {
        self.entities.write().await.remove(key);
        Ok(())
    }

    pub async fn raw_entities_contains(&self, key: &str) -> bool {
        self.entities.read().await.contains_key(key)
    }
}

#[async_trait]
impl EntityStore for MemoryStore {
    async fn put<T: PersistentEntity>(&self, entity: &T) -> Result<()> {
        let key = format!("{}::{}", T::entity_type(), entity.id().as_str());
        let value = bincode::serialize(entity)?;

        self.entities.write().await.insert(key, value);
        Ok(())
    }

    async fn get<T: PersistentEntity>(&self, id: &T::Id) -> Result<Option<T>> {
        let key = format!("{}::{}", T::entity_type(), id.as_str());
        let entities = self.entities.read().await;

        if let Some(bytes) = entities.get(&key) {
            Ok(Some(bincode::deserialize(bytes)?))
        } else {
            Ok(None)
        }
    }

    async fn del<T: PersistentEntity>(&self, id: &T::Id) -> Result<()> {
        let key = format!("{}::{}", T::entity_type(), id.as_str());
        self.entities.write().await.remove(&key);
        Ok(())
    }

    async fn list_all<T: PersistentEntity>(&self) -> Result<Vec<T>> {
        let entities = self.entities.read().await;
        let prefix = format!("{}::", T::entity_type());

        let mut result = Vec::new();

        for (key, value) in entities.iter() {
            if key.starts_with(&prefix) {
                if let Ok(entity) = bincode::deserialize::<T>(value) {
                    result.push(entity);
                }
            }
        }

        Ok(result)
    }

    async fn list_paginated<T: PersistentEntity>(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<T>, bool)> {
        let all_entities = self.list_all::<T>().await?;
        let total = all_entities.len();

        let start = offset.min(total);
        let end = (offset + limit).min(total);
        let has_more = end < total;

        Ok((all_entities[start..end].to_vec(), has_more))
    }
}
