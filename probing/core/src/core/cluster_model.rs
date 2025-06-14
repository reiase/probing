use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Import the entity traits
use crate::storage::entity::PersistentEntity;

// Potentially use Uuid for IDs if preferred, for now using String aliases
pub type ClusterId = String;
pub type NodeId = String;
pub type WorkerId = String;
pub type DeviceId = String;
pub type JobId = String;
pub type TaskDefinitionId = String;
pub type TaskExecutionId = String;
pub type SpanId = String;
pub type TraceId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum NodeStatus {
    Pending,
    Initializing,
    Running,
    Healthy,
    Unhealthy,
    Unreachable,
    Stopped,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ClusterStatus {
    Initializing,
    Active,
    Degraded,
    Inactive,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum WorkerStatus {
    Pending,
    Starting,
    Running,
    Completed,
    Failed,
    Stopped,
    Lost,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum JobStatus {
    Pending,
    Submitted,
    Running,
    Succeeded,
    Failed,
    Stopped,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum TaskExecutionStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ClusterFrameworkType {
    TorchRun,
    Ray,
    Kubernetes,
    Slurm,
    Standalone, // For single-node or non-managed clusters
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    pub id: DeviceId,        // Unique ID within the node (e.g., "GPU-0", "TPU-chip-1")
    pub device_type: String, // e.g., "GPU", "TPU", "FPGA"
    pub model_name: Option<String>,
    pub total_memory_bytes: Option<u64>,
    pub node_id: NodeId, // Back-reference to the node it belongs to
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: ClusterId,
    pub name: Option<String>,
    pub cluster_type: ClusterFrameworkType,
    pub status: ClusterStatus,
    pub nodes: HashMap<NodeId, Node>,
    pub metadata: Option<HashMap<String, String>>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,            // Unique identifier (e.g., hostname, IP, or internal ID)
    pub cluster_id: ClusterId, // Back-reference
    pub hostname: Option<String>,
    pub addresses: Vec<String>, // List of IP addresses or other network identifiers
    pub status: NodeStatus,
    pub devices: HashMap<DeviceId, Device>, // Devices present on this node
    pub roles: Option<Vec<String>>,         // e.g., ["master", "worker"], ["head_node"]
    pub last_heartbeat: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub id: WorkerId,     // Globally unique or unique within a job
    pub node_id: NodeId,  // Node where this worker is running
    pub pid: Option<u32>, // OS Process ID, if applicable
    pub status: WorkerStatus,
    pub role: Option<String>, // e.g., "trainer", "ps", "rank_0"
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub platform_specific_info: Option<serde_json::Value>, // e.g., TorchRun rank, Ray Actor ID
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    // The "type" or "template" of a repeatable task
    pub id: TaskDefinitionId, // Unique within a job or globally for task types
    pub job_id: JobId,
    pub name: String, // e.g., "training_step", "data_loading_batch"
    pub description: Option<String>,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    pub name: Option<String>,
    pub cluster_id: ClusterId,
    pub status: JobStatus,
    pub submit_time: Option<DateTime<Utc>>,
    pub start_time: Option<DateTime<Utc>>,
    pub end_time: Option<DateTime<Utc>>,
    pub workers: Vec<WorkerId>,
    pub tasks: Option<HashMap<TaskDefinitionId, Task>>,
    pub metadata: Option<HashMap<String, String>>,
    pub platform_specific_info: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    // An instance of a TaskDefinition being executed
    pub execution_id: TaskExecutionId,
    pub task_definition_id: TaskDefinitionId,
    pub job_id: JobId,
    pub worker_id: Option<WorkerId>,
    pub status: TaskExecutionStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub inputs: Option<serde_json::Value>,
    pub outputs: Option<serde_json::Value>,
    pub error_message: Option<String>,
    pub span_ids: Vec<SpanId>,
}

// Implement PersistentEntity for cluster model structs

#[async_trait::async_trait]
impl PersistentEntity for Cluster {
    type Id = ClusterId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "cluster"
    }

    fn last_updated(&self) -> Option<DateTime<Utc>> {
        Some(self.last_updated)
    }

    fn set_last_updated(&mut self, timestamp: DateTime<Utc>) {
        self.last_updated = timestamp;
    }
}

#[async_trait::async_trait]
impl PersistentEntity for Node {
    type Id = NodeId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "node"
    }

    fn last_updated(&self) -> Option<DateTime<Utc>> {
        self.last_heartbeat
    }

    fn index_keys(&self) -> Vec<(String, String)> {
        vec![
            ("cluster_id".to_string(), self.cluster_id.clone()),
            ("status".to_string(), format!("{:?}", self.status)),
        ]
    }
}

#[async_trait::async_trait]
impl PersistentEntity for Job {
    type Id = JobId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "job"
    }

    fn index_keys(&self) -> Vec<(String, String)> {
        vec![
            ("cluster_id".to_string(), self.cluster_id.clone()),
            ("status".to_string(), format!("{:?}", self.status)),
        ]
    }
}

#[async_trait::async_trait]
impl PersistentEntity for Worker {
    type Id = WorkerId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "worker"
    }

    fn index_keys(&self) -> Vec<(String, String)> {
        vec![
            ("node_id".to_string(), self.node_id.clone()),
            ("status".to_string(), format!("{:?}", self.status)),
        ]
    }
}

#[async_trait::async_trait]
impl PersistentEntity for Task {
    type Id = TaskDefinitionId;

    fn id(&self) -> &Self::Id {
        &self.id
    }

    fn entity_type() -> &'static str {
        "task_definition"
    }

    fn index_keys(&self) -> Vec<(String, String)> {
        vec![("job_id".to_string(), self.job_id.clone())]
    }
}

#[async_trait::async_trait]
impl PersistentEntity for TaskExecution {
    type Id = TaskExecutionId;

    fn id(&self) -> &Self::Id {
        &self.execution_id
    }

    fn entity_type() -> &'static str {
        "task_execution"
    }

    fn index_keys(&self) -> Vec<(String, String)> {
        vec![
            ("job_id".to_string(), self.job_id.clone()),
            (
                "task_definition_id".to_string(),
                self.task_definition_id.clone(),
            ),
            ("status".to_string(), format!("{:?}", self.status)),
        ]
    }
}
