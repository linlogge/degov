//! FoundationDB key-value schema for workflow engine

/// Key prefixes for different data types in FoundationDB
pub struct KeyPrefix;

impl KeyPrefix {
    pub const WORKFLOW_DEFINITION: &'static [u8] = b"wfdef";
    pub const WORKFLOW_INSTANCE: &'static [u8] = b"wfinst";
    pub const TASK_QUEUE: &'static [u8] = b"taskq";
    pub const TASK: &'static [u8] = b"task";
    pub const WORKER: &'static [u8] = b"worker";
    pub const WORKFLOW_EVENT: &'static [u8] = b"wflevt";
    pub const DEAD_LETTER: &'static [u8] = b"dlq";
    pub const WORKFLOW_LOCK: &'static [u8] = b"wflk";
    pub const TASK_LEASE: &'static [u8] = b"taskls";
    pub const WORKER_HEARTBEAT: &'static [u8] = b"workerhb";
}

/// Schema for workflow definitions
pub struct WorkflowDefSchema;

impl WorkflowDefSchema {
    /// Key for a specific workflow definition
    pub fn key(id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::WORKFLOW_DEFINITION.to_vec();
        key.extend_from_slice(id.as_bytes());
        key.extend_from_slice(b":def");
        key
    }

    /// Key prefix for listing all workflow definitions
    pub fn list_prefix() -> Vec<u8> {
        KeyPrefix::WORKFLOW_DEFINITION.to_vec()
    }

    /// Key for workflow definition by name and version
    pub fn name_version_key(name: &str, version: u32) -> Vec<u8> {
        let mut key = KeyPrefix::WORKFLOW_DEFINITION.to_vec();
        key.extend_from_slice(name.as_bytes());
        key.push(0); // separator
        key.extend_from_slice(&version.to_be_bytes());
        key
    }
}

/// Schema for workflow instances
pub struct WorkflowInstanceSchema;

impl WorkflowInstanceSchema {
    /// Key for a specific workflow instance
    pub fn key(id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::WORKFLOW_INSTANCE.to_vec();
        key.extend_from_slice(id.as_bytes());
        key.extend_from_slice(b":inst");
        key
    }

    /// Key prefix for instances of a specific workflow definition
    pub fn workflow_prefix(workflow_def_id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::WORKFLOW_INSTANCE.to_vec();
        key.extend_from_slice(workflow_def_id.as_bytes());
        key.push(0); // separator
        key
    }

    /// Key prefix for all instances
    pub fn list_prefix() -> Vec<u8> {
        KeyPrefix::WORKFLOW_INSTANCE.to_vec()
    }

    /// Key for workflow lock (ensures only one worker processes an instance at a time)
    pub fn lock_key(id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::WORKFLOW_LOCK.to_vec();
        key.extend_from_slice(id.as_bytes());
        key
    }
}

/// Schema for task queue (ordered by scheduled time for priority)
pub struct TaskQueueSchema;

impl TaskQueueSchema {
    /// Key for task queue entry (ordered by scheduled_at, then task_id for uniqueness)
    pub fn queue_key(scheduled_at: &chrono::DateTime<chrono::Utc>, task_id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::TASK_QUEUE.to_vec();
        key.extend_from_slice(&scheduled_at.timestamp_nanos_opt().unwrap_or(0).to_be_bytes());
        key.extend_from_slice(task_id.as_bytes());
        key
    }

    /// Key prefix for task queue
    pub fn prefix() -> Vec<u8> {
        KeyPrefix::TASK_QUEUE.to_vec()
    }

    /// Key for tasks ready to be processed (current timestamp or earlier)
    pub fn ready_prefix() -> Vec<u8> {
        KeyPrefix::TASK_QUEUE.to_vec()
    }
}

/// Schema for individual tasks
pub struct TaskSchema;

impl TaskSchema {
    /// Key for a specific task
    pub fn key(id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::TASK.to_vec();
        key.extend_from_slice(id.as_bytes());
        key
    }

    /// Key prefix for tasks of a specific workflow instance
    pub fn workflow_prefix(workflow_instance_id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::TASK.to_vec();
        key.extend_from_slice(workflow_instance_id.as_bytes());
        key.push(0); // separator
        key
    }

    /// Key prefix for tasks claimed by a specific worker
    pub fn worker_prefix(worker_id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::TASK.to_vec();
        key.extend_from_slice(b"worker:");
        key.extend_from_slice(worker_id.as_bytes());
        key.push(0); // separator
        key
    }

    /// Key for task lease (prevents multiple workers from claiming same task)
    pub fn lease_key(id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::TASK_LEASE.to_vec();
        key.extend_from_slice(id.as_bytes());
        key
    }
}

/// Schema for workers
pub struct WorkerSchema;

impl WorkerSchema {
    /// Key for a specific worker
    pub fn key(id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::WORKER.to_vec();
        key.extend_from_slice(id.as_bytes());
        key
    }

    /// Key prefix for all workers
    pub fn list_prefix() -> Vec<u8> {
        KeyPrefix::WORKER.to_vec()
    }

    /// Key for worker heartbeat (with TTL for automatic cleanup)
    pub fn heartbeat_key(id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::WORKER_HEARTBEAT.to_vec();
        key.extend_from_slice(id.as_bytes());
        key
    }

    /// Key prefix for workers by capability
    pub fn capability_prefix(capability: &str) -> Vec<u8> {
        let mut key = KeyPrefix::WORKER.to_vec();
        key.extend_from_slice(b"capability:");
        key.extend_from_slice(capability.as_bytes());
        key.push(0); // separator
        key
    }
}

/// Schema for workflow events (audit log)
pub struct WorkflowEventSchema;

impl WorkflowEventSchema {
    /// Key for a specific event (ordered by timestamp for append-only log)
    pub fn key(timestamp: &chrono::DateTime<chrono::Utc>, event_id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::WORKFLOW_EVENT.to_vec();
        key.extend_from_slice(&timestamp.timestamp_nanos_opt().unwrap_or(0).to_be_bytes());
        key.extend_from_slice(event_id.as_bytes());
        key
    }

    /// Key prefix for events of a specific workflow instance
    pub fn workflow_prefix(workflow_instance_id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::WORKFLOW_EVENT.to_vec();
        key.extend_from_slice(workflow_instance_id.as_bytes());
        key.push(0); // separator
        key
    }

    /// Key prefix for all events
    pub fn list_prefix() -> Vec<u8> {
        KeyPrefix::WORKFLOW_EVENT.to_vec()
    }
}

/// Schema for dead letter queue
pub struct DeadLetterSchema;

impl DeadLetterSchema {
    /// Key for dead letter task (ordered by dead_lettered_at)
    pub fn key(dead_lettered_at: &chrono::DateTime<chrono::Utc>, task_id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::DEAD_LETTER.to_vec();
        key.extend_from_slice(&dead_lettered_at.timestamp_nanos_opt().unwrap_or(0).to_be_bytes());
        key.extend_from_slice(task_id.as_bytes());
        key
    }

    /// Key prefix for all dead letter tasks
    pub fn list_prefix() -> Vec<u8> {
        KeyPrefix::DEAD_LETTER.to_vec()
    }

    /// Key prefix for dead letter tasks by original workflow
    pub fn workflow_prefix(workflow_instance_id: &uuid::Uuid) -> Vec<u8> {
        let mut key = KeyPrefix::DEAD_LETTER.to_vec();
        key.extend_from_slice(workflow_instance_id.as_bytes());
        key.push(0); // separator
        key
    }
}

/// Utility functions for working with the schema
pub struct SchemaUtils;

impl SchemaUtils {
    /// Generate a unique idempotency key for a task
    pub fn generate_idempotency_key(
        workflow_instance_id: &uuid::Uuid,
        task_type: &str,
        context: &str,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        workflow_instance_id.hash(&mut hasher);
        task_type.hash(&mut hasher);
        context.hash(&mut hasher);

        format!("idem_{}_{}_{}",
                workflow_instance_id,
                task_type,
                hasher.finish())
    }

    /// Calculate lease expiration time
    pub fn lease_expiration_time(duration_ms: u64) -> chrono::DateTime<chrono::Utc> {
        chrono::Utc::now() + chrono::Duration::milliseconds(duration_ms as i64)
    }

    /// Check if a lease is still valid
    pub fn is_lease_valid(expires_at: &chrono::DateTime<chrono::Utc>) -> bool {
        chrono::Utc::now() < *expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_workflow_definition_keys() {
        let id = Uuid::new_v4();
        let key = WorkflowDefSchema::key(&id);

        // Basic check that key is generated
        assert!(!key.is_empty());
        assert!(key.starts_with(KeyPrefix::WORKFLOW_DEFINITION));
    }

    #[test]
    fn test_task_queue_ordering() {
        use chrono::Utc;

        let task1_id = Uuid::new_v4();
        let task2_id = Uuid::new_v4();

        let time1 = Utc::now();
        let time2 = time1 + chrono::Duration::seconds(1);

        let key1 = TaskQueueSchema::queue_key(&time1, &task1_id);
        let key2 = TaskQueueSchema::queue_key(&time2, &task2_id);

        // Earlier tasks should come first in lexicographic ordering
        assert!(key1 < key2);
    }

    #[test]
    fn test_idempotency_key_generation() {
        let workflow_id = Uuid::new_v4();
        let key1 = SchemaUtils::generate_idempotency_key(&workflow_id, "test", "context1");
        let key2 = SchemaUtils::generate_idempotency_key(&workflow_id, "test", "context2");
        let key3 = SchemaUtils::generate_idempotency_key(&workflow_id, "test", "context1");

        assert_ne!(key1, key2);
        assert_eq!(key1, key3);
    }
}