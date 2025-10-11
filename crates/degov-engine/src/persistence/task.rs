//! Task persistence

use super::{build_key, keys};
use crate::error::{PersistenceError, PersistenceResult};
use crate::types::{TaskExecution, TaskId, TaskResult, TaskStatus, WorkerId};
use chrono::Utc;
use foundationdb::{Database, RangeOption, Transaction};
use std::sync::Arc;

/// Task storage operations
#[derive(Clone)]
pub struct TaskStore {
    db: Arc<Database>,
}

impl TaskStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Enqueue a task for execution
    pub async fn enqueue(&self, task: TaskExecution) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        
        // Set transaction timeout to 2 seconds
        tx.set_option(foundationdb::options::TransactionOption::Timeout(2000))?;
        tx.set_option(foundationdb::options::TransactionOption::RetryLimit(5))?;
        
        self.enqueue_tx(&tx, task).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Enqueue a task within a transaction
    pub async fn enqueue_tx(&self, tx: &Transaction, task: TaskExecution) -> PersistenceResult<()> {
        // Save task data
        let task_key = build_key(keys::TASK_PREFIX, &task.id.to_string());
        let task_value = serde_json::to_vec(&task)?;
        tx.set(&task_key, &task_value);

        // Add to pending queue with timestamp for ordering
        let queue_key = self.build_queue_key(&task.id);
        tx.set(&queue_key, &task.id.to_string().as_bytes());

        Ok(())
    }

    /// Dequeue next pending task (atomic operation)
    pub async fn dequeue(&self, worker_id: &WorkerId) -> PersistenceResult<Option<TaskExecution>> {
        let tx = self.db.create_trx()?;
        
        // Set transaction timeout to 2 seconds
        tx.set_option(foundationdb::options::TransactionOption::Timeout(2000))?;
        tx.set_option(foundationdb::options::TransactionOption::RetryLimit(5))?;
        
        let result = self.dequeue_tx(&tx, worker_id).await?;
        tx.commit().await?;
        Ok(result)
    }

    /// Dequeue next pending task within a transaction
    pub async fn dequeue_tx(
        &self,
        tx: &Transaction,
        worker_id: &WorkerId,
    ) -> PersistenceResult<Option<TaskExecution>> {
        // Get first pending task from queue
        let end_key = self.queue_end_key();
        let range = RangeOption {
            begin: foundationdb::KeySelector::first_greater_or_equal(keys::TASK_QUEUE_PREFIX),
            end: foundationdb::KeySelector::first_greater_or_equal(&end_key),
            mode: foundationdb::options::StreamingMode::Small,
            limit: Some(1),
            reverse: false,
            ..Default::default()
        };

        let results = tx.get_range(&range, 1, false).await?;
        
        if results.is_empty() {
            return Ok(None);
        }

        let queue_key = &results[0].key();
        let task_id_bytes = results[0].value();
        let task_id_str = String::from_utf8_lossy(task_id_bytes.as_ref());
        let task_id = TaskId::from_uuid(
            uuid::Uuid::parse_str(&task_id_str)
                .map_err(|e| PersistenceError::Corruption(format!("Invalid task ID: {}", e)))?
        );

        // Get task data
        let task_key = build_key(keys::TASK_PREFIX, &task_id.to_string());
        let task_bytes = tx.get(&task_key, false).await?
            .ok_or_else(|| PersistenceError::Corruption("Task data not found".to_string()))?;
        
        let mut task: TaskExecution = serde_json::from_slice(task_bytes.as_ref())?;

        // Update task status
        task.status = TaskStatus::Assigned;
        task.assigned_worker = Some(worker_id.clone());
        task.started_at = Some(Utc::now());

        // Save updated task
        let updated_value = serde_json::to_vec(&task)?;
        tx.set(&task_key, &updated_value);

        // Remove from pending queue
        tx.clear(queue_key);

        Ok(Some(task))
    }

    /// Mark task as completed
    pub async fn complete(
        &self,
        task_id: &TaskId,
        result: TaskResult,
    ) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        
        // Set transaction timeout to 2 seconds
        tx.set_option(foundationdb::options::TransactionOption::Timeout(2000))?;
        tx.set_option(foundationdb::options::TransactionOption::RetryLimit(5))?;
        
        self.complete_tx(&tx, task_id, result).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Mark task as completed within a transaction
    pub async fn complete_tx(
        &self,
        tx: &Transaction,
        task_id: &TaskId,
        result: TaskResult,
    ) -> PersistenceResult<()> {
        let task_key = build_key(keys::TASK_PREFIX, &task_id.to_string());
        let task_bytes = tx.get(&task_key, false).await?
            .ok_or_else(|| PersistenceError::NotFound(task_id.to_string()))?;
        
        let mut task: TaskExecution = serde_json::from_slice(task_bytes.as_ref())?;

        task.status = if result.success {
            TaskStatus::Completed
        } else {
            TaskStatus::Failed
        };
        task.completed_at = Some(Utc::now());
        task.result = Some(result);

        let updated_value = serde_json::to_vec(&task)?;
        tx.set(&task_key, &updated_value);

        Ok(())
    }

    /// Get a task by ID
    pub async fn get(&self, task_id: &TaskId) -> PersistenceResult<Option<TaskExecution>> {
        let tx = self.db.create_trx()?;
        let result = self.get_tx(&tx, task_id).await?;
        tx.cancel();
        Ok(result)
    }

    /// Get a task by ID within a transaction
    pub async fn get_tx(
        &self,
        tx: &Transaction,
        task_id: &TaskId,
    ) -> PersistenceResult<Option<TaskExecution>> {
        let task_key = build_key(keys::TASK_PREFIX, &task_id.to_string());
        let bytes = tx.get(&task_key, false).await?;
        
        match bytes {
            Some(data) => {
                let task = serde_json::from_slice(data.as_ref())?;
                Ok(Some(task))
            }
            None => Ok(None),
        }
    }

    /// Reschedule a failed task for retry
    pub async fn reschedule(&self, task_id: &TaskId) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        
        // Set transaction timeout to 2 seconds
        tx.set_option(foundationdb::options::TransactionOption::Timeout(2000))?;
        tx.set_option(foundationdb::options::TransactionOption::RetryLimit(5))?;
        
        let task_key = build_key(keys::TASK_PREFIX, &task_id.to_string());
        let task_bytes = tx.get(&task_key, false).await?
            .ok_or_else(|| PersistenceError::NotFound(task_id.to_string()))?;
        
        let mut task: TaskExecution = serde_json::from_slice(task_bytes.as_ref())?;

        task.status = TaskStatus::Pending;
        task.assigned_worker = None;
        task.attempt += 1;

        let updated_value = serde_json::to_vec(&task)?;
        tx.set(&task_key, &updated_value);

        // Re-add to queue
        let queue_key = self.build_queue_key(task_id);
        tx.set(&queue_key, &task_id.to_string().as_bytes());

        tx.commit().await?;
        Ok(())
    }

    /// Build queue key with timestamp for ordering
    fn build_queue_key(&self, task_id: &TaskId) -> Vec<u8> {
        let timestamp = Utc::now().timestamp_millis();
        let mut key = Vec::new();
        key.extend_from_slice(keys::TASK_QUEUE_PREFIX);
        key.extend_from_slice(&timestamp.to_be_bytes());
        key.extend_from_slice(task_id.to_string().as_bytes());
        key
    }

    /// Get the end key for queue range scans
    fn queue_end_key(&self) -> Vec<u8> {
        let mut key = keys::TASK_QUEUE_PREFIX.to_vec();
        key.push(0xff);
        key
    }
}

