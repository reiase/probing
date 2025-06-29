// probing/core/src/storage/entity.rs
use anyhow::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A generic entity ID trait that all ID types should implement.
pub trait EntityId:
    Clone
    + Send
    + Sync
    + std::fmt::Debug
    + std::fmt::Display
    + PartialEq
    + Eq
    + std::hash::Hash
    + 'static
{
    fn as_str(&self) -> &str;
    fn from_string(s: String) -> Self;
}

// Implement EntityId for our ID type
impl EntityId for String {
    fn as_str(&self) -> &str {
        self // Corrected from self.as_str()
    }

    fn from_string(s: String) -> Self {
        s
    }
}

/// Base trait for persistent entities.
/// All objects that need to be persisted should implement this trait.
#[async_trait]
pub trait PersistentEntity:
    Clone + Send + Sync + Serialize + for<'de> Deserialize<'de> + std::fmt::Debug + 'static
{
    /// The ID type of the entity.
    type Id: EntityId;

    /// Gets the unique identifier of the entity.
    fn id(&self) -> &Self::Id;

    /// The entity type name, used for storage paths/table names, etc.
    fn entity_type() -> &'static str;

    /// Gets the last updated time (if any).
    fn last_updated(&self) -> Option<DateTime<Utc>> {
        None
    }

    /// Sets the last updated time (if supported).
    fn set_last_updated(&mut self, _timestamp: DateTime<Utc>) {
        // Default empty implementation, entities can implement it selectively.
    }

    /// Gets the version of the entity (for optimistic concurrency control, optional).
    fn version(&self) -> Option<u64> {
        None
    }

    /// Gets related index keys for secondary indexing.
    /// Returns in the format: (index_name, index_key)
    fn index_keys(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}

/// A generic storage interface that supports any type that implements PersistentEntity.
#[async_trait]
pub trait EntityStore: Send + Sync + 'static {
    /// Saves an entity.
    async fn put<T: PersistentEntity>(&self, entity: &T) -> Result<()>;

    /// Gets an entity by its ID.
    async fn get<T: PersistentEntity>(&self, id: &T::Id) -> Result<Option<T>>;

    /// Deletes an entity.
    async fn del<T: PersistentEntity>(&self, id: &T::Id) -> Result<()>;

    /// Lists all entities of a certain type (use with caution, may return a large amount of data).
    async fn list_all<T: PersistentEntity>(&self) -> Result<Vec<T>>;

    /// Lists entities with pagination.
    async fn list_paginated<T: PersistentEntity>(
        &self,
        offset: usize,
        limit: usize,
    ) -> Result<(Vec<T>, bool)>; // (entities, has_more)
}
