use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a workflow definition
pub type WorkflowId = String;

/// Unique identifier for a workflow instance
pub type InstanceId = String;

/// Unique identifier for a task
pub type TaskId = String;

/// Unique identifier for a worker
pub type WorkerId = String;

/// Workflow definition with states and transitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDefinition {
    pub id: WorkflowId,
    pub name: String,
    pub version: u32,
    pub initial_state: String,
    pub states: HashMap<String, StateDefinition>,
    pub transitions: Vec<Transition>,
    pub created_at: i64,
}

/// Definition of a state in the workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDefinition {
    pub name: String,
    pub is_terminal: bool,
    pub on_enter: Option<Action>,
    pub on_exit: Option<Action>,
    pub timeout_seconds: Option<u64>,
}

/// Transition between states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from: String,
    pub to: String,
    pub event: String,
    pub condition: Option<String>, // JavaScript expression
    pub action: Option<Action>,
    pub compensation: Option<Action>, // For rollback
}

/// Action to execute during state transition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Action {
    Script {
        code: String,
        language: String, // "javascript" or "typescript"
    },
    Task {
        task_type: String,
        payload: serde_json::Value,
    },
    Http {
        url: String,
        method: String,
        headers: HashMap<String, String>,
        body: Option<serde_json::Value>,
    },
    Delay {
        seconds: u64,
    },
}

/// Current state of a workflow instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceState {
    pub instance_id: InstanceId,
    pub workflow_id: WorkflowId,
    pub workflow_version: u32,
    pub current_state: String,
    pub context: serde_json::Value,
    pub status: InstanceStatus,
    pub created_at: i64,
    pub updated_at: i64,
    pub versionstamp: Vec<u8>, // For optimistic locking
}

/// Status of a workflow instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InstanceStatus {
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// A task to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub task_id: TaskId,
    pub instance_id: InstanceId,
    pub workflow_id: WorkflowId,
    pub action: Action,
    pub idempotency_key: String,
    pub priority: i32,
    pub created_at: i64,
    pub scheduled_at: i64, // For delayed tasks
    pub status: TaskStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub lease: Option<TaskLease>,
}

/// Task execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Claimed,
    Running,
    Completed,
    Failed,
    DeadLetter,
}

/// Lease information for task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskLease {
    pub worker_id: WorkerId,
    pub claimed_at: i64,
    pub expires_at: i64,
    pub heartbeat_at: i64,
}

/// Worker registration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub worker_id: WorkerId,
    pub hostname: String,
    pub process_id: u32,
    pub capabilities: Vec<String>,
    pub registered_at: i64,
    pub heartbeat_at: i64,
    pub status: WorkerStatus,
}

/// Worker status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorkerStatus {
    Active,
    Idle,
    Stopping,
    Stopped,
}

/// Event log entry for audit trail
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventLog {
    pub event_id: String,
    pub instance_id: InstanceId,
    pub event_type: EventType,
    pub timestamp: i64,
    pub data: serde_json::Value,
}

/// Types of events that can occur
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    InstanceCreated,
    StateTransition,
    TaskCreated,
    TaskClaimed,
    TaskCompleted,
    TaskFailed,
    ActionExecuted,
    InstancePaused,
    InstanceResumed,
    InstanceCancelled,
    InstanceCompleted,
    CompensationStarted,
    CompensationCompleted,
}

/// Result of task execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: TaskId,
    pub success: bool,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}
