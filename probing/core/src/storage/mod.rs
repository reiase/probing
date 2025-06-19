pub mod addressing;
pub mod distributed;
pub mod entity;
pub mod mem_store;
pub mod remote_client;
pub mod topology;

// Re-export the main interfaces for easier access
pub use entity::{EntityId, EntityStore, PersistentEntity};
pub use mem_store::MemoryStore;

// Distributed storage exports
pub use addressing::{Address, AddressAllocator};
pub use distributed::{ConsistencyLevel, DistributedEntityStore, RemoteStoreClient};
pub use remote_client::MemoryRemoteClient;
pub use topology::{TopologyStats, TopologyView};
