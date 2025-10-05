use crate::error::{EngineError, Result};
use crate::model::*;
use foundationdb::{Database, Transaction};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// FDB Schema using manual key construction:
/// /degov/workflow/workflows/{workflow_id} -> WorkflowDefinition
/// /degov/workflow/instances/{instance_id} -> InstanceState
/// /degov/workflow/instance_index/{workflow_id}/{instance_id} -> empty
/// /degov/workflow/tasks/{priority}/{scheduled_at}/{task_id} -> Task
/// /degov/workflow/task_by_id/{task_id} -> Task
/// /degov/workflow/task_idempotency/{idempotency_key} -> TaskResult
/// /degov/workflow/workers/{worker_id} -> Worker
/// /degov/workflow/events/{instance_id}/{timestamp}/{event_id} -> EventLog
/// /degov/workflow/locks/{instance_id} -> (WorkerId, expires_at)

#[allow(dead_code)]
pub struct WorkflowStorage {
    db: Arc<Database>,
    prefix: Vec<u8>,
}

impl WorkflowStorage {
    pub fn new(db: Database) -> Self {
        Self {
            db: Arc::new(db),
            prefix: b"/degov/workflow/".to_vec(),
        }
    }

    fn key(&self, parts: &[&[u8]]) -> Vec<u8> {
        let mut k = self.prefix.clone();
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                k.push(b'/');
            }
            k.extend_from_slice(part);
        }
        k
    }

    // ==================== Workflow Operations ====================

    pub async fn save_workflow(&self, workflow: &WorkflowDefinition) -> Result<()> {
        let trx = self.db.create_trx()?;
        let key = self.key(&[b"workflows", workflow.id.as_bytes()]);
        let value = serde_json::to_vec(workflow)?;
        trx.set(&key, &value);
        trx.commit().await?;
        debug!("Saved workflow: {}", workflow.id);
        Ok(())
    }

    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>> {
        let trx = self.db.create_trx()?;
        let key = self.key(&[b"workflows", workflow_id.as_bytes()]);

        let value = trx.get(&key, false).await?;
        let result = match value {
            Some(bytes) => {
                let workflow: WorkflowDefinition = serde_json::from_slice(&bytes)?;
                Some(workflow)
            }
            None => None,
        };
        trx.cancel();
        Ok(result)
    }

    // ==================== Instance Operations ====================

    pub async fn create_instance(&self, instance: &InstanceState) -> Result<()> {
        let trx = self.db.create_trx()?;

        // Check if instance already exists
        let instance_key = self.key(&[b"instances", instance.instance_id.as_bytes()]);
        let existing = trx.get(&instance_key, false).await?;
        if existing.is_some() {
            return Err(EngineError::InstanceAlreadyExists(
                instance.instance_id.clone(),
            ));
        }

        // Save instance
        let value = serde_json::to_vec(instance)?;
        trx.set(&instance_key, &value);

        // Create index entry
        let index_key = self.key(&[
            b"instance_index",
            instance.workflow_id.as_bytes(),
            instance.instance_id.as_bytes(),
        ]);
        trx.set(&index_key, &[]);

        trx.commit().await?;
        debug!("Created instance: {}", instance.instance_id);
        Ok(())
    }

    pub async fn get_instance(&self, instance_id: &str) -> Result<Option<InstanceState>> {
        let trx = self.db.create_trx()?;
        let key = self.key(&[b"instances", instance_id.as_bytes()]);

        let value = trx.get(&key, false).await?;
        let result = match value {
            Some(bytes) => {
                let instance: InstanceState = serde_json::from_slice(&bytes)?;
                Some(instance)
            }
            None => None,
        };
        trx.cancel();
        Ok(result)
    }

    /// Atomically update instance state with optimistic locking
    pub async fn update_instance_state<F>(
        &self,
        instance_id: &str,
        update_fn: F,
    ) -> Result<InstanceState>
    where
        F: FnOnce(&mut InstanceState) -> Result<()>,
    {
        let trx = self.db.create_trx()?;
        self.update_instance_state_in_trx(&trx, instance_id, update_fn)
            .await?;
        trx.commit().await?;
        self.get_instance(instance_id)
            .await?
            .ok_or_else(|| EngineError::InstanceNotFound(instance_id.to_string()))
    }

    /// Update instance state within an existing transaction
    pub async fn update_instance_state_in_trx<F>(
        &self,
        trx: &Transaction,
        instance_id: &str,
        update_fn: F,
    ) -> Result<()>
    where
        F: FnOnce(&mut InstanceState) -> Result<()>,
    {
        let key = self.key(&[b"instances", instance_id.as_bytes()]);

        // Get current state
        let value = trx
            .get(&key, false)
            .await?
            .ok_or_else(|| EngineError::InstanceNotFound(instance_id.to_string()))?;

        let mut instance: InstanceState = serde_json::from_slice(&value)?;
        let old_versionstamp = instance.versionstamp.clone();

        // Apply update
        update_fn(&mut instance)?;

        // Update timestamp
        instance.updated_at = chrono::Utc::now().timestamp_millis();

        // Check for optimistic lock conflict
        let current = trx.get(&key, false).await?;
        if let Some(current_bytes) = current {
            let current_instance: InstanceState = serde_json::from_slice(&current_bytes)?;
            if current_instance.versionstamp != old_versionstamp {
                return Err(EngineError::OptimisticLockConflict);
            }
        }

        // Generate new versionstamp (simplified - just use timestamp)
        instance.versionstamp = chrono::Utc::now().timestamp_nanos_opt()
            .unwrap_or(0)
            .to_be_bytes()
            .to_vec();

        // Set the new value
        let new_value = serde_json::to_vec(&instance)?;
        trx.set(&key, &new_value);

        Ok(())
    }

    pub async fn list_instances(&self, workflow_id: &str) -> Result<Vec<InstanceId>> {
        let trx = self.db.create_trx()?;
        let prefix = self.key(&[b"instance_index", workflow_id.as_bytes()]);

        // Create range that covers all keys starting with prefix
        let mut end_prefix = prefix.clone();
        if let Some(last) = end_prefix.last_mut() {
            *last = last.wrapping_add(1);
        }
        let range = foundationdb::RangeOption::from(prefix.clone()..end_prefix);
        let results = trx.get_range(&range, 1_000, false).await?;

        let mut instances = Vec::new();
        for kv in results.iter() {
            // Extract instance_id from key
            let key_bytes = kv.key();
            if let Some(rest) = key_bytes.strip_prefix(prefix.as_slice()) {
                if rest.starts_with(&[b'/']) {
                    let instance_id = String::from_utf8_lossy(&rest[1..]).to_string();
                    instances.push(instance_id);
                }
            }
        }

        trx.cancel();
        Ok(instances)
    }

    // ==================== Task Operations ====================

    pub async fn create_task(&self, task: &Task) -> Result<()> {
        let trx = self.db.create_trx()?;
        self.create_task_in_trx(&trx, task).await?;
        trx.commit().await?;
        Ok(())
    }

    pub async fn create_task_in_trx(&self, trx: &Transaction, task: &Task) -> Result<()> {
        // Store in priority queue: negative priority for high-priority-first ordering
        let priority_bytes = (-task.priority).to_be_bytes();
        let scheduled_bytes = task.scheduled_at.to_be_bytes();
        let queue_key = self.key(&[
            b"tasks",
            &priority_bytes,
            &scheduled_bytes,
            task.task_id.as_bytes(),
        ]);
        let value = serde_json::to_vec(task)?;
        trx.set(&queue_key, &value);

        // Store by ID for direct lookup
        let id_key = self.key(&[b"task_by_id", task.task_id.as_bytes()]);
        trx.set(&id_key, &value);

        debug!("Created task: {} with priority {}", task.task_id, task.priority);
        Ok(())
    }

    /// Claim next available task (for workers)
    pub async fn claim_task(&self, worker_id: &str, now: i64) -> Result<Option<Task>> {
        let trx = self.db.create_trx()?;

        // Scan task queue - create a range that covers all keys starting with prefix
        let prefix = self.key(&[b"tasks"]);
        let mut end_prefix = prefix.clone();
        // Increment the last byte to create exclusive end bound
        if let Some(last) = end_prefix.last_mut() {
            *last = last.wrapping_add(1);
        }
        info!("Scanning tasks with prefix: {:?}", String::from_utf8_lossy(&prefix));
        let range = foundationdb::RangeOption::from(prefix.clone()..end_prefix);

        // Get first 10 tasks to check
        let results = trx.get_range(&range, 10, false).await?;
        info!("Found {} potential tasks in queue for worker {}", results.len(), worker_id);

        // Convert to owned data immediately to avoid Send issues
        let tasks_data: Vec<(Vec<u8>, Vec<u8>)> = results
            .iter()
            .map(|kv| (kv.key().to_vec(), kv.value().to_vec()))
            .collect();
        
        debug!("Processing {} tasks for worker {}", tasks_data.len(), worker_id);

        for (queue_key, task_value) in tasks_data {
            let mut task: Task = match serde_json::from_slice(&task_value) {
                Ok(t) => t,
                Err(_) => continue,
            };

            // Skip if not ready yet
            if task.scheduled_at > now {
                continue;
            }

            // Skip if already claimed and lease hasn't expired
            if task.status == TaskStatus::Claimed || task.status == TaskStatus::Running {
                if let Some(lease) = &task.lease {
                    if lease.expires_at > now {
                        continue; // Lease still valid
                    }
                    warn!(
                        "Task {} lease expired, reclaiming from worker {}",
                        task.task_id, lease.worker_id
                    );
                }
            }

            // Skip completed or dead letter tasks
            if task.status == TaskStatus::Completed || task.status == TaskStatus::DeadLetter {
                continue;
            }

            // Claim the task
            task.status = TaskStatus::Claimed;
            task.lease = Some(TaskLease {
                worker_id: worker_id.to_string(),
                claimed_at: now,
                expires_at: now + 30_000, // 30 second lease
                heartbeat_at: now,
            });

            // Update in database
            let id_key = self.key(&[b"task_by_id", task.task_id.as_bytes()]);
            let value = serde_json::to_vec(&task)?;

            trx.set(&queue_key, &value);
            trx.set(&id_key, &value);

            trx.commit().await?;
            debug!("Worker {} claimed task {}", worker_id, task.task_id);
            return Ok(Some(task));
        }

        trx.cancel();
        Ok(None)
    }

    /// Update task status
    pub async fn update_task(&self, task: &Task) -> Result<()> {
        let trx = self.db.create_trx()?;

        let id_key = self.key(&[b"task_by_id", task.task_id.as_bytes()]);
        let value = serde_json::to_vec(task)?;
        trx.set(&id_key, &value);

        // Also update in queue
        let priority_bytes = (-task.priority).to_be_bytes();
        let scheduled_bytes = task.scheduled_at.to_be_bytes();
        let queue_key = self.key(&[
            b"tasks",
            &priority_bytes,
            &scheduled_bytes,
            task.task_id.as_bytes(),
        ]);
        trx.set(&queue_key, &value);

        trx.commit().await?;
        Ok(())
    }

    /// Store task result with idempotency key
    pub async fn store_task_result(&self, result: &TaskResult) -> Result<()> {
        let trx = self.db.create_trx()?;

        // Get task to get idempotency key
        let task = self.get_task(&result.task_id).await?.ok_or_else(|| {
            EngineError::TaskNotFound(result.task_id.clone())
        })?;

        let key = self.key(&[b"task_idempotency", task.idempotency_key.as_bytes()]);
        let value = serde_json::to_vec(result)?;
        trx.set(&key, &value);

        trx.commit().await?;
        debug!("Stored result for task: {}", result.task_id);
        Ok(())
    }

    /// Check if task was already executed (idempotency)
    pub async fn get_task_result(&self, idempotency_key: &str) -> Result<Option<TaskResult>> {
        let trx = self.db.create_trx()?;
        let key = self.key(&[b"task_idempotency", idempotency_key.as_bytes()]);

        let value = trx.get(&key, false).await?;
        let result = match value {
            Some(bytes) => {
                let result: TaskResult = serde_json::from_slice(&bytes)?;
                Some(result)
            }
            None => None,
        };
        trx.cancel();
        Ok(result)
    }

    pub async fn get_task(&self, task_id: &str) -> Result<Option<Task>> {
        let trx = self.db.create_trx()?;
        let key = self.key(&[b"task_by_id", task_id.as_bytes()]);

        let value = trx.get(&key, false).await?;
        let result = match value {
            Some(bytes) => {
                let task: Task = serde_json::from_slice(&bytes)?;
                Some(task)
            }
            None => None,
        };
        trx.cancel();
        Ok(result)
    }

    // ==================== Worker Operations ====================

    pub async fn register_worker(&self, worker: &Worker) -> Result<()> {
        let trx = self.db.create_trx()?;
        let key = self.key(&[b"workers", worker.worker_id.as_bytes()]);
        let value = serde_json::to_vec(worker)?;
        trx.set(&key, &value);
        trx.commit().await?;
        debug!("Registered worker: {}", worker.worker_id);
        Ok(())
    }

    pub async fn update_worker_heartbeat(&self, worker_id: &str, now: i64) -> Result<()> {
        let trx = self.db.create_trx()?;
        let key = self.key(&[b"workers", worker_id.as_bytes()]);

        let value = trx
            .get(&key, false)
            .await?
            .ok_or_else(|| EngineError::WorkerNotFound(worker_id.to_string()))?;

        let mut worker: Worker = serde_json::from_slice(&value)?;
        worker.heartbeat_at = now;

        let new_value = serde_json::to_vec(&worker)?;
        trx.set(&key, &new_value);
        trx.commit().await?;

        Ok(())
    }

    pub async fn get_worker(&self, worker_id: &str) -> Result<Option<Worker>> {
        let trx = self.db.create_trx()?;
        let key = self.key(&[b"workers", worker_id.as_bytes()]);

        let value = trx.get(&key, false).await?;
        let result = match value {
            Some(bytes) => {
                let worker: Worker = serde_json::from_slice(&bytes)?;
                Some(worker)
            }
            None => None,
        };
        trx.cancel();
        Ok(result)
    }

    pub async fn list_workers(&self) -> Result<Vec<Worker>> {
        let trx = self.db.create_trx()?;
        let prefix = self.key(&[b"workers"]);
        
        // Create range that covers all keys starting with prefix
        let mut end_prefix = prefix.clone();
        if let Some(last) = end_prefix.last_mut() {
            *last = last.wrapping_add(1);
        }
        let range = foundationdb::RangeOption::from(prefix.clone()..end_prefix);

        let results = trx.get_range(&range, 1_000, false).await?;
        let mut workers = Vec::new();

        for kv in results.iter() {
            if let Ok(worker) = serde_json::from_slice::<Worker>(kv.value()) {
                workers.push(worker);
            }
        }

        trx.cancel();
        Ok(workers)
    }

    // ==================== Event Logging ====================

    pub async fn append_event(&self, event: &EventLog) -> Result<()> {
        let trx = self.db.create_trx()?;
        self.append_event_in_trx(&trx, event).await?;
        trx.commit().await?;
        Ok(())
    }

    pub async fn append_event_in_trx(&self, trx: &Transaction, event: &EventLog) -> Result<()> {
        let timestamp_bytes = event.timestamp.to_be_bytes();
        let key = self.key(&[
            b"events",
            event.instance_id.as_bytes(),
            &timestamp_bytes,
            event.event_id.as_bytes(),
        ]);
        let value = serde_json::to_vec(event)?;
        trx.set(&key, &value);
        debug!("Appended event: {:?} for instance {}", event.event_type, event.instance_id);
        Ok(())
    }

    pub async fn get_events(&self, instance_id: &str) -> Result<Vec<EventLog>> {
        let trx = self.db.create_trx()?;
        let prefix = self.key(&[b"events", instance_id.as_bytes()]);

        // Create range that covers all keys starting with prefix
        let mut end_prefix = prefix.clone();
        if let Some(last) = end_prefix.last_mut() {
            *last = last.wrapping_add(1);
        }
        let range = foundationdb::RangeOption::from(prefix.clone()..end_prefix);
        let results = trx.get_range(&range, 1_000, false).await?;

        let mut events = Vec::new();
        for kv in results.iter() {
            if let Ok(event) = serde_json::from_slice::<EventLog>(kv.value()) {
                events.push(event);
            }
        }

        trx.cancel();
        Ok(events)
    }

    // ==================== Distributed Locking ====================

    /// Try to acquire a lock for a workflow instance
    pub async fn try_lock_instance(&self, instance_id: &str, worker_id: &str, ttl_ms: i64) -> Result<bool> {
        let trx = self.db.create_trx()?;
        let key = self.key(&[b"locks", instance_id.as_bytes()]);
        let now = chrono::Utc::now().timestamp_millis();

        let existing = trx.get(&key, false).await?;

        match existing {
            Some(bytes) => {
                // Check if lock expired
                let lock_data: (String, i64) = serde_json::from_slice(&bytes)?;
                let (existing_worker, expires_at) = lock_data;

                if expires_at > now {
                    // Lock still valid
                    if existing_worker == worker_id {
                        // Same worker, renew the lock
                        let new_expires = now + ttl_ms;
                        let new_value = serde_json::to_vec(&(worker_id, new_expires))?;
                        trx.set(&key, &new_value);
                        trx.commit().await?;
                        return Ok(true);
                    } else {
                        // Different worker holds lock
                        trx.cancel();
                        return Ok(false);
                    }
                }
                // Lock expired, can acquire
            }
            None => {
                // No lock exists
            }
        }

        // Acquire lock
        let expires_at = now + ttl_ms;
        let value = serde_json::to_vec(&(worker_id, expires_at))?;
        trx.set(&key, &value);
        trx.commit().await?;

        debug!("Worker {} acquired lock for instance {}", worker_id, instance_id);
        Ok(true)
    }

    /// Release lock for a workflow instance
    pub async fn unlock_instance(&self, instance_id: &str, worker_id: &str) -> Result<()> {
        let trx = self.db.create_trx()?;
        let key = self.key(&[b"locks", instance_id.as_bytes()]);

        let existing = trx.get(&key, false).await?;
        if let Some(bytes) = existing {
            let lock_data: (String, i64) = serde_json::from_slice(&bytes)?;
            let (existing_worker, _) = lock_data;

            if existing_worker == worker_id {
                trx.clear(&key);
                trx.commit().await?;
                debug!("Worker {} released lock for instance {}", worker_id, instance_id);
                return Ok(());
            }
        }

        trx.cancel();
        Ok(())
    }
}