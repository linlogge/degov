use crate::error::{EngineError, Result};
use crate::executers::deno::DenoRuntime;
use crate::model::*;
use crate::storage::WorkflowStorage;
use foundationdb::Database;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

pub struct WorkflowEngine {
    storage: Arc<WorkflowStorage>,
    runtime: Arc<DenoRuntime>,
    worker_id: WorkerId,
    worker_handle: Arc<RwLock<Option<JoinHandle<()>>>>,
    shutdown: Arc<tokio::sync::Notify>,
}

impl WorkflowEngine {
    pub async fn new(db: Database, pool_size: usize) -> Result<Self> {
        let storage = Arc::new(WorkflowStorage::new(db));
        let runtime = Arc::new(DenoRuntime::new(pool_size).await?);
        let worker_id = format!("worker-{}", Uuid::new_v4());
        let shutdown = Arc::new(tokio::sync::Notify::new());

        Ok(Self {
            storage,
            runtime,
            worker_id,
            worker_handle: Arc::new(RwLock::new(None)),
            shutdown,
        })
    }

    // ==================== Workflow Definition Management ====================

    pub async fn register_workflow(&self, workflow: WorkflowDefinition) -> Result<()> {
        // Validate workflow
        self.validate_workflow(&workflow)?;
        self.storage.save_workflow(&workflow).await?;
        info!("Registered workflow: {} v{}", workflow.id, workflow.version);
        Ok(())
    }

    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Option<WorkflowDefinition>> {
        self.storage.get_workflow(workflow_id).await
    }

    fn validate_workflow(&self, workflow: &WorkflowDefinition) -> Result<()> {
        // Check initial state exists
        if !workflow.states.contains_key(&workflow.initial_state) {
            return Err(EngineError::ValidationError(format!(
                "Initial state '{}' not found in states",
                workflow.initial_state
            )));
        }

        // Validate all transitions reference valid states
        for transition in &workflow.transitions {
            if !workflow.states.contains_key(&transition.from) {
                return Err(EngineError::ValidationError(format!(
                    "Transition from unknown state: {}",
                    transition.from
                )));
            }
            if !workflow.states.contains_key(&transition.to) {
                return Err(EngineError::ValidationError(format!(
                    "Transition to unknown state: {}",
                    transition.to
                )));
            }
        }

        Ok(())
    }

    // ==================== Instance Lifecycle Management ====================

    pub async fn create_instance(
        &self,
        workflow_id: &str,
        instance_id: Option<String>,
        initial_context: serde_json::Value,
    ) -> Result<InstanceId> {
        let workflow = self
            .storage
            .get_workflow(workflow_id)
            .await?
            .ok_or_else(|| EngineError::WorkflowNotFound(workflow_id.to_string()))?;

        let instance_id = instance_id.unwrap_or_else(|| format!("inst-{}", Uuid::new_v4()));
        let now = chrono::Utc::now().timestamp_millis();

        let instance = InstanceState {
            instance_id: instance_id.clone(),
            workflow_id: workflow_id.to_string(),
            workflow_version: workflow.version,
            current_state: workflow.initial_state.clone(),
            context: initial_context,
            status: InstanceStatus::Running,
            created_at: now,
            updated_at: now,
            versionstamp: Vec::new(),
        };

        self.storage.create_instance(&instance).await?;

        // Log event
        let event = EventLog {
            event_id: Uuid::new_v4().to_string(),
            instance_id: instance_id.clone(),
            event_type: EventType::InstanceCreated,
            timestamp: now,
            data: serde_json::json!({
                "workflow_id": workflow_id,
                "initial_state": workflow.initial_state,
            }),
        };
        self.storage.append_event(&event).await?;

        // Execute on_enter action for initial state if any
        if let Some(state_def) = workflow.states.get(&workflow.initial_state) {
            if let Some(action) = &state_def.on_enter {
                info!("Creating task for on_enter action in state {}", workflow.initial_state);
                self.create_task_for_action(&instance_id, action.clone())
                    .await?;
            } else {
                info!("No on_enter action for initial state {}", workflow.initial_state);
            }
        }

        info!(
            "Created instance {} for workflow {}",
            instance_id, workflow_id
        );
        Ok(instance_id)
    }

    pub async fn get_instance(&self, instance_id: &str) -> Result<Option<InstanceState>> {
        self.storage.get_instance(instance_id).await
    }

    pub async fn pause_instance(&self, instance_id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        self.storage
            .update_instance_state(instance_id, |instance| {
                if instance.status != InstanceStatus::Running {
                    return Err(EngineError::ValidationError(
                        "Can only pause running instances".to_string(),
                    ));
                }
                instance.status = InstanceStatus::Paused;
                Ok(())
            })
            .await?;

        let event = EventLog {
            event_id: Uuid::new_v4().to_string(),
            instance_id: instance_id.to_string(),
            event_type: EventType::InstancePaused,
            timestamp: now,
            data: serde_json::json!({}),
        };
        self.storage.append_event(&event).await?;

        info!("Paused instance {}", instance_id);
        Ok(())
    }

    pub async fn resume_instance(&self, instance_id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        self.storage
            .update_instance_state(instance_id, |instance| {
                if instance.status != InstanceStatus::Paused {
                    return Err(EngineError::ValidationError(
                        "Can only resume paused instances".to_string(),
                    ));
                }
                instance.status = InstanceStatus::Running;
                Ok(())
            })
            .await?;

        let event = EventLog {
            event_id: Uuid::new_v4().to_string(),
            instance_id: instance_id.to_string(),
            event_type: EventType::InstanceResumed,
            timestamp: now,
            data: serde_json::json!({}),
        };
        self.storage.append_event(&event).await?;

        info!("Resumed instance {}", instance_id);
        Ok(())
    }

    pub async fn cancel_instance(&self, instance_id: &str) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        self.storage
            .update_instance_state(instance_id, |instance| {
                instance.status = InstanceStatus::Cancelled;
                Ok(())
            })
            .await?;

        let event = EventLog {
            event_id: Uuid::new_v4().to_string(),
            instance_id: instance_id.to_string(),
            event_type: EventType::InstanceCancelled,
            timestamp: now,
            data: serde_json::json!({}),
        };
        self.storage.append_event(&event).await?;

        info!("Cancelled instance {}", instance_id);
        Ok(())
    }

    // ==================== State Transitions ====================

    pub async fn trigger_event(
        &self,
        instance_id: &str,
        event: &str,
        event_data: Option<serde_json::Value>,
    ) -> Result<()> {
        let instance = self
            .storage
            .get_instance(instance_id)
            .await?
            .ok_or_else(|| EngineError::InstanceNotFound(instance_id.to_string()))?;

        if instance.status != InstanceStatus::Running {
            return Err(EngineError::ValidationError(format!(
                "Instance is not running: {:?}",
                instance.status
            )));
        }

        let workflow = self
            .storage
            .get_workflow(&instance.workflow_id)
            .await?
            .ok_or_else(|| EngineError::WorkflowNotFound(instance.workflow_id.clone()))?;

        // Find matching transition
        let transition = workflow
            .transitions
            .iter()
            .find(|t| t.from == instance.current_state && t.event == event)
            .cloned()
            .ok_or_else(|| EngineError::InvalidTransition {
                from: instance.current_state.clone(),
                to: event.to_string(),
            })?;

        // Evaluate condition if present
        if let Some(condition) = &transition.condition {
            let should_transition = self
                .evaluate_condition(condition, &instance.context)
                .await?;
            if !should_transition {
                debug!("Transition condition not met for instance {}", instance_id);
                return Ok(());
            }
        }

        // Perform transition atomically
        self.perform_transition(instance_id, &transition, event_data)
            .await?;

        Ok(())
    }

    async fn perform_transition(
        &self,
        instance_id: &str,
        transition: &Transition,
        event_data: Option<serde_json::Value>,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        let workflow_id = {
            let instance = self.storage.get_instance(instance_id).await?
                .ok_or_else(|| EngineError::InstanceNotFound(instance_id.to_string()))?;
            instance.workflow_id.clone()
        };

        let workflow = self
            .storage
            .get_workflow(&workflow_id)
            .await?
            .ok_or_else(|| EngineError::WorkflowNotFound(workflow_id.clone()))?;

        // Get state definitions
        let from_state = workflow
            .states
            .get(&transition.from)
            .ok_or_else(|| EngineError::ValidationError(format!("State not found: {}", transition.from)))?;
        let to_state = workflow
            .states
            .get(&transition.to)
            .ok_or_else(|| EngineError::ValidationError(format!("State not found: {}", transition.to)))?;

        // Execute on_exit action
        if let Some(action) = &from_state.on_exit {
            self.create_task_for_action(instance_id, action.clone())
                .await?;
        }

        // Execute transition action
        if let Some(action) = &transition.action {
            self.create_task_for_action(instance_id, action.clone())
                .await?;
        }

        // Update state
        self.storage
            .update_instance_state(instance_id, |instance| {
                instance.current_state = transition.to.clone();

                // Merge event data into context
                if let Some(data) = event_data {
                    if let Some(context_obj) = instance.context.as_object_mut() {
                        if let Some(data_obj) = data.as_object() {
                            for (k, v) in data_obj {
                                context_obj.insert(k.clone(), v.clone());
                            }
                        }
                    }
                }

                // Mark as completed if terminal state
                if to_state.is_terminal {
                    instance.status = InstanceStatus::Completed;
                }

                Ok(())
            })
            .await?;

        // Log transition event
        let event = EventLog {
            event_id: Uuid::new_v4().to_string(),
            instance_id: instance_id.to_string(),
            event_type: EventType::StateTransition,
            timestamp: now,
            data: serde_json::json!({
                "from": transition.from,
                "to": transition.to,
                "event": transition.event,
            }),
        };
        self.storage.append_event(&event).await?;

        // Execute on_enter action
        if let Some(action) = &to_state.on_enter {
            self.create_task_for_action(instance_id, action.clone())
                .await?;
        }

        info!(
            "Instance {} transitioned from {} to {}",
            instance_id, transition.from, transition.to
        );

        Ok(())
    }

    async fn evaluate_condition(
        &self,
        condition: &str,
        context: &serde_json::Value,
    ) -> Result<bool> {
        // Execute condition as JavaScript expression
        let code = format!(
            "export default function(context) {{ return ({}); }}",
            condition
        );

        let result = self.runtime.execute_script(&code, context.clone()).await?;

        if result.success {
            if let Some(output) = result.output {
                if let Some(bool_val) = output.as_bool() {
                    return Ok(bool_val);
                }
            }
        }

        Ok(false)
    }

    // ==================== Task Management ====================

    async fn create_task_for_action(&self, instance_id: &str, action: Action) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();
        let task_id = format!("task-{}", Uuid::new_v4());
        let idempotency_key = format!("{}-{}", instance_id, Uuid::new_v4());

        let instance = self
            .storage
            .get_instance(instance_id)
            .await?
            .ok_or_else(|| EngineError::InstanceNotFound(instance_id.to_string()))?;

        let scheduled_at = match &action {
            Action::Delay { seconds } => now + (*seconds as i64 * 1000),
            _ => now,
        };

        let task = Task {
            task_id: task_id.clone(),
            instance_id: instance_id.to_string(),
            workflow_id: instance.workflow_id.clone(),
            action: action.clone(),
            idempotency_key: idempotency_key.clone(),
            priority: 0,
            created_at: now,
            scheduled_at,
            status: TaskStatus::Pending,
            retry_count: 0,
            max_retries: 3,
            lease: None,
        };

        info!("Creating task {} for instance {} (scheduled_at: {}, now: {})", task_id, instance_id, scheduled_at, now);
        self.storage.create_task(&task).await?;

        let event = EventLog {
            event_id: Uuid::new_v4().to_string(),
            instance_id: instance_id.to_string(),
            event_type: EventType::TaskCreated,
            timestamp: now,
            data: serde_json::json!({
                "task_id": task_id,
            }),
        };
        self.storage.append_event(&event).await?;

        info!("Task {} created and stored in queue for instance {}", task_id, instance_id);
        Ok(())
    }

    // ==================== Worker Operations ====================

    pub async fn start_worker(&self) -> Result<()> {
        let worker = Worker {
            worker_id: self.worker_id.clone(),
            hostname: hostname::get()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            process_id: std::process::id(),
            capabilities: vec!["script".to_string(), "http".to_string()],
            registered_at: chrono::Utc::now().timestamp_millis(),
            heartbeat_at: chrono::Utc::now().timestamp_millis(),
            status: WorkerStatus::Active,
        };

        self.storage.register_worker(&worker).await?;

        let storage = self.storage.clone();
        let runtime = self.runtime.clone();
        let worker_id = self.worker_id.clone();
        let shutdown = self.shutdown.clone();

        let handle = tokio::spawn(async move {
            info!("Worker {} started", worker_id);

            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        info!("Worker {} shutting down", worker_id);
                        break;
                    }
                    _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                        info!("Worker {} polling for tasks...", worker_id);
                        // Poll for tasks
                        match Self::worker_tick(&storage, &runtime, &worker_id).await {
                            Ok(_) => {},
                            Err(e) => {
                                error!("Worker tick error: {:?}", e);
                            }
                        }

                        // Update heartbeat
                        let now = chrono::Utc::now().timestamp_millis();
                        if let Err(e) = storage.update_worker_heartbeat(&worker_id, now).await {
                            error!("Failed to update heartbeat: {:?}", e);
                        }
                    }
                }
            }
        });

        *self.worker_handle.write().await = Some(handle);
        Ok(())
    }

    async fn worker_tick(
        storage: &Arc<WorkflowStorage>,
        runtime: &Arc<DenoRuntime>,
        worker_id: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().timestamp_millis();

        // Try to claim a task
        let task = match storage.claim_task(worker_id, now).await? {
            Some(t) => t,
            None => {
                debug!("Worker {} found no tasks to claim", worker_id);
                return Ok(());
            }
        };

        info!("Worker {} claimed and processing task {}", worker_id, task.task_id);

        // Check idempotency
        if let Some(_existing_result) = storage.get_task_result(&task.idempotency_key).await? {
            info!("Task {} already executed (idempotent), skipping", task.task_id);
            return Ok(());
        }

        // Execute the task
        let start = std::time::Instant::now();
        let result = Self::execute_task(storage, runtime, &task).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        let task_result = match result {
            Ok(output) => TaskResult {
                task_id: task.task_id.clone(),
                success: true,
                output: Some(output),
                error: None,
                duration_ms,
            },
            Err(e) => TaskResult {
                task_id: task.task_id.clone(),
                success: false,
                output: None,
                error: Some(e.to_string()),
                duration_ms,
            },
        };

        // Store result
        storage.store_task_result(&task_result).await?;

        // Update task status
        let mut updated_task = task.clone();
        if task_result.success {
            updated_task.status = TaskStatus::Completed;
        } else {
            updated_task.retry_count += 1;
            if updated_task.retry_count >= updated_task.max_retries {
                updated_task.status = TaskStatus::DeadLetter;
                warn!(
                    "Task {} moved to dead letter queue after {} retries",
                    updated_task.task_id, updated_task.max_retries
                );
            } else {
                updated_task.status = TaskStatus::Pending;
                updated_task.lease = None;
                info!(
                    "Task {} failed, retry {}/{}",
                    updated_task.task_id, updated_task.retry_count, updated_task.max_retries
                );
            }
        }

        storage.update_task(&updated_task).await?;

        // Log event
        let event = EventLog {
            event_id: Uuid::new_v4().to_string(),
            instance_id: task.instance_id.clone(),
            event_type: if task_result.success {
                EventType::TaskCompleted
            } else {
                EventType::TaskFailed
            },
            timestamp: now,
            data: serde_json::json!({
                "task_id": task.task_id,
                "duration_ms": duration_ms,
                "success": task_result.success,
            }),
        };
        storage.append_event(&event).await?;

        Ok(())
    }

    async fn execute_task(
        storage: &Arc<WorkflowStorage>,
        runtime: &Arc<DenoRuntime>,
        task: &Task,
    ) -> Result<serde_json::Value> {
        let instance = storage
            .get_instance(&task.instance_id)
            .await?
            .ok_or_else(|| EngineError::InstanceNotFound(task.instance_id.clone()))?;

        match &task.action {
            Action::Script { code, language: _ } => {
                info!("Executing script for task {}: {}", task.task_id, code.chars().take(50).collect::<String>());
                let result = runtime.execute_script(code, instance.context).await?;
                if result.success {
                    info!("Script completed successfully for task {}", task.task_id);
                    Ok(result.output.unwrap_or(serde_json::json!(null)))
                } else {
                    error!("Script failed for task {}: {:?}", task.task_id, result.error);
                    Err(EngineError::ScriptError(
                        result.error.unwrap_or_else(|| "Unknown error".to_string()),
                    ))
                }
            }
            Action::Task { task_type, payload } => {
                // Placeholder for custom task handlers
                Ok(serde_json::json!({
                    "task_type": task_type,
                    "payload": payload,
                }))
            }
            Action::Http {
                url,
                method,
                headers: _,
                body,
            } => {
                // Placeholder for HTTP requests
                Ok(serde_json::json!({
                    "url": url,
                    "method": method,
                    "body": body,
                }))
            }
            Action::Delay { seconds } => {
                // Delay is handled in scheduling
                Ok(serde_json::json!({
                    "delayed": seconds,
                }))
            }
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!("Shutting down workflow engine");

        // Signal shutdown
        self.shutdown.notify_waiters();

        // Wait for worker to stop
        if let Some(handle) = self.worker_handle.write().await.take() {
            let _ = handle.await;
        }

        // Shutdown runtime
        self.runtime.shutdown().await;

        info!("Workflow engine shut down");
        Ok(())
    }

    pub fn worker_id(&self) -> &str {
        &self.worker_id
    }

    pub async fn get_events(&self, instance_id: &str) -> Result<Vec<EventLog>> {
        self.storage.get_events(instance_id).await
    }
}
