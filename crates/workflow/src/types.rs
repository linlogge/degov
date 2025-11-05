//! Core domain types for the workflow engine

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a workflow definition
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowId(pub Uuid);

impl WorkflowId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
    
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for WorkflowId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a task
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub Uuid);

impl TaskId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
    
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }
    
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a worker
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkerId(pub String);

impl WorkerId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }
    
    pub fn from_string(s: String) -> Self {
        Self(s)
    }
    
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for WorkerId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Workflow definition created by the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: WorkflowId,
    pub name: String,
    pub description: Option<String>,
    pub state_machine: crate::state_machine::StateMachine,
    pub created_at: DateTime<Utc>,
}

/// Running instance of a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInstance {
    pub id: WorkflowId,
    pub definition_id: WorkflowId,
    pub current_state: String,
    pub context: serde_json::Value,
    pub status: WorkflowStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Status of a workflow instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Task definition within a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDefinition {
    pub name: String,
    pub runtime_type: RuntimeType,
    pub code: Vec<u8>,
    pub timeout_ms: u64,
    pub retry_policy: Option<RetryPolicy>,
}

/// Type of runtime for task execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuntimeType {
    JavaScript,
    Wasm,
}

impl RuntimeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RuntimeType::JavaScript => "javascript",
            RuntimeType::Wasm => "wasm",
        }
    }
}

/// Retry policy for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            max_delay_ms: 60000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Task execution record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskExecution {
    pub id: TaskId,
    pub workflow_id: WorkflowId,
    pub definition: TaskDefinition,
    pub input: Vec<u8>,
    pub status: TaskStatus,
    pub assigned_worker: Option<WorkerId>,
    pub attempt: u32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<TaskResult>,
}

/// Status of a task execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
    Retrying,
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub success: bool,
    pub output: Vec<u8>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Worker information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerInfo {
    pub id: WorkerId,
    pub capabilities: Vec<RuntimeType>,
    pub hostname: String,
    pub registered_at: DateTime<Utc>,
    pub last_heartbeat: DateTime<Utc>,
    pub status: WorkerHealthStatus,
    pub stats: WorkerStats,
}

/// Worker health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkerHealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Worker statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkerStats {
    pub active_tasks: u32,
    pub total_tasks_completed: u64,
    pub total_tasks_failed: u64,
}

