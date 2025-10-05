//! Main workflow engine that orchestrates workflow execution

use crate::error::{EngineError, Result};
use crate::models::*;
use crate::persistence::WorkflowPersistence;
use crate::runtime::DenoRuntime;
use crate::state_machine::{WorkflowStateMachine, TaskCreation};
use crate::schema::SchemaUtils;
use chrono::Utc;
use foundationdb::Database;
use std::sync::Arc;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Main workflow engine
pub struct WorkflowEngine {
    persistence: Arc<WorkflowPersistence>,
    runtime: Arc<DenoRuntime>,
}

impl WorkflowEngine {
    /// Create a new workflow engine
    pub async fn new(db: Database, runtime_pool_size: usize) -> Result<Self> {
        let persistence = Arc::new(WorkflowPersistence::new(db));
        let runtime = Arc::new(DenoRuntime::new(runtime_pool_size).await?);

        Ok(Self {
            persistence,
            runtime,
        })
    }

    // ========== Workflow Definition Management ==========

    /// Register a new workflow definition
    pub async fn register_workflow_definition(
        &self,
        definition: &WorkflowDefinition,
    ) -> Result<()> {
        // Validate the workflow definition
        WorkflowStateMachine::validate_workflow_definition(definition)?;

        // Store the definition
        self.persistence.store_workflow_definition(definition).await?;

        info!("Registered workflow definition: {} (version {})", definition.name, definition.version);
        Ok(())
    }

    /// Get a workflow definition
    pub async fn get_workflow_definition(&self, id: &Uuid) -> Result<Option<WorkflowDefinition>> {
        self.persistence.get_workflow_definition(id).await
    }

    // ========== Workflow Instance Management ==========

    /// Start a new workflow instance
    pub async fn start_workflow(
        &self,
        workflow_definition_id: &Uuid,
        input: serde_json::Value,
        metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<WorkflowInstanceId> {
        // Get the workflow definition
        let definition = self.persistence.get_workflow_definition(workflow_definition_id).await?
            .ok_or_else(|| EngineError::NotFound(
                format!("Workflow definition not found: {}", workflow_definition_id)
            ))?;

        // Create workflow instance
        let instance = WorkflowInstance {
            id: Uuid::new_v4(),
            workflow_definition_id: *workflow_definition_id,
            current_state: definition.initial_state.clone(),
            status: WorkflowStatus::Running,
            input: input.clone(),
            output: None,
            state_data: std::collections::HashMap::new(),
            error: None,
            metadata: metadata.unwrap_or_default(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            version: 1,
        };

        // Store the instance
        self.persistence.create_workflow_instance(&instance).await?;

        // Determine initial tasks
        let state_context = self.build_state_context(&instance, &definition)?;
        let task_creations = WorkflowStateMachine::determine_next_tasks(
            &definition,
            &instance,
            &state_context,
        )?;

        // Create initial tasks
        for task_creation in task_creations {
            self.create_task(&instance, &definition, &task_creation).await?;
        }

        info!("Started workflow instance: {} for definition: {}", instance.id, workflow_definition_id);
        Ok(instance.id)
    }

    /// Execute a state transition for a workflow instance
    pub async fn execute_transition(
        &self,
        workflow_instance_id: &Uuid,
        transition_name: &str,
        context: Option<serde_json::Value>,
    ) -> Result<()> {
        // Get workflow instance and definition
        let mut instance = self.persistence.get_workflow_instance(workflow_instance_id).await?
            .ok_or_else(|| EngineError::NotFound(
                format!("Workflow instance not found: {}", workflow_instance_id)
            ))?;

        if instance.status != WorkflowStatus::Running {
            return Err(EngineError::TransitionError(
                format!("Cannot execute transition on workflow in state: {:?}", instance.status)
            ));
        }

        let definition = self.persistence.get_workflow_definition(&instance.workflow_definition_id).await?
            .ok_or_else(|| EngineError::NotFound(
                format!("Workflow definition not found: {}", instance.workflow_definition_id)
            ))?;

        // Build execution context
        let execution_context = self.build_state_context(&instance, &definition)?;
        let transition_context = context.unwrap_or(execution_context);

        // Execute state transition
        let state_transition = WorkflowStateMachine::execute_transition(
            &definition,
            &instance,
            transition_name,
            &transition_context,
        ).await?;

        // Update workflow instance state
        let from_state = instance.current_state.clone();
        instance.current_state = state_transition.to_state.clone();
        instance.updated_at = Utc::now();

        // Store state change
        if let Some(state_data) = transition_context.as_object() {
            for (key, value) in state_data {
                instance.state_data.insert(format!("{}.{}", from_state, key), value.clone());
            }
        }

        // Update instance in database
        self.persistence.update_workflow_instance(&instance).await?;

        // Log state change event
        self.persistence.log_workflow_event(&WorkflowEvent {
            id: Uuid::new_v4(),
            workflow_instance_id: *workflow_instance_id,
            task_id: None,
            event_type: EventType::StateChanged {
                from: from_state.clone(),
                to: state_transition.to_state.clone(),
            },
            data: serde_json::json!({
                "transition": transition_name,
                "context": transition_context,
            }),
            timestamp: Utc::now(),
        }).await?;

        // Determine next tasks for new state
        let new_state_context = self.build_state_context(&instance, &definition)?;
        if let Ok(task_creations) = WorkflowStateMachine::determine_next_tasks(
            &definition,
            &instance,
            &new_state_context,
        ) {
            for task_creation in task_creations {
                self.create_task(&instance, &definition, &task_creation).await?;
            }
        }

        // Check if we've reached a terminal state
        if let Some(current_state_def) = definition.states.get(&instance.current_state) {
            if current_state_def.is_terminal {
                self.complete_workflow(&mut instance).await?;
            }
        }

        info!("Executed transition '{}' on workflow {}: {} -> {}",
              transition_name, workflow_instance_id, from_state, state_transition.to_state);

        Ok(())
    }

    // ========== Task Execution ==========

    /// Execute a task
    pub async fn execute_task(
        &self,
        task_id: &Uuid,
        worker_id: &Uuid,
    ) -> Result<serde_json::Value> {
        // Claim the task
        let task = self.persistence.claim_task(task_id, worker_id, 30000).await? // 30 second lease
            .ok_or_else(|| EngineError::TaskError(
                format!("Failed to claim task: {}", task_id)
            ))?;

        debug!("Executing task: {} for worker: {}", task_id, worker_id);

        // Get workflow definition and instance
        let instance = self.persistence.get_workflow_instance(&task.workflow_instance_id).await?
            .ok_or_else(|| EngineError::NotFound(
                format!("Workflow instance not found: {}", task.workflow_instance_id)
            ))?;

        let definition = self.persistence.get_workflow_definition(&instance.workflow_definition_id).await?
            .ok_or_else(|| EngineError::NotFound(
                format!("Workflow definition not found: {}", instance.workflow_definition_id)
            ))?;

        // Build execution context
        let context = self.build_task_context(&task, &instance, &definition)?;

        // Execute the action
        let result = match &task.action.action_type {
            ActionType::JavaScript { code } => {
                self.execute_javascript_action(code, &context).await?
            },
            ActionType::HttpRequest { method, url, headers, body: _ } => {
                self.execute_http_action(method, url, headers, &context).await?
            },
            ActionType::Delay { duration_ms } => {
                tokio::time::sleep(tokio::time::Duration::from_millis(*duration_ms)).await;
                serde_json::json!({"delayed_ms": duration_ms})
            },
            ActionType::Custom { type_name, data } => {
                self.execute_custom_action(type_name, data, &context).await?
            },
        };

        // Complete the task
        self.persistence.complete_task(task_id, &result).await?;

        // Check if this task enables any state transitions
        self.check_workflow_progress(&instance, &definition, &task, &result).await?;

        debug!("Completed task: {} with result: {:?}", task_id, result);
        Ok(result)
    }

    /// Get next available tasks for a worker
    pub async fn get_next_tasks(&self, _worker_capabilities: &[String], limit: usize) -> Result<Vec<Task>> {
        let tasks = self.persistence.get_next_tasks(limit).await?;

        // Filter tasks by worker capabilities
        let filtered_tasks = tasks.into_iter()
            .filter(|_task| {
                // For now, accept all tasks. In a real implementation, you'd match task types to worker capabilities
                true
            })
            .collect();

        Ok(filtered_tasks)
    }

    // ========== Worker Management ==========

    /// Register a worker
    pub async fn register_worker(
        &self,
        name: &str,
        capabilities: Vec<String>,
        metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
    ) -> Result<WorkerId> {
        let worker = Worker {
            id: Uuid::new_v4(),
            name: name.to_string(),
            status: WorkerStatus::Active,
            capabilities,
            last_heartbeat: Utc::now(),
            metadata: metadata.unwrap_or_default(),
            created_at: Utc::now(),
        };

        self.persistence.register_worker(&worker).await?;
        info!("Registered worker: {} ({})", worker.id, worker.name);
        Ok(worker.id)
    }

    /// Update worker heartbeat
    pub async fn update_worker_heartbeat(&self, worker_id: &WorkerId) -> Result<()> {
        self.persistence.update_worker_heartbeat(worker_id).await?;
        Ok(())
    }

    // ========== Private Helper Methods ==========

    /// Create a task
    async fn create_task(
        &self,
        instance: &WorkflowInstance,
        _definition: &WorkflowDefinition,
        task_creation: &TaskCreation,
    ) -> Result<()> {
        let idempotency_key = SchemaUtils::generate_idempotency_key(
            &instance.id,
            &format!("{:?}", task_creation.task_type),
            &serde_json::to_string(&task_creation.context).unwrap_or_default(),
        );

        let task = Task {
            id: Uuid::new_v4(),
            workflow_instance_id: instance.id,
            workflow_definition_id: instance.workflow_definition_id,
            task_type: task_creation.task_type.clone(),
            action: task_creation.action.clone(),
            input_data: task_creation.context.clone(),
            status: TaskStatus::Queued,
            worker_id: None,
            idempotency_key,
            retry_count: 0,
            max_retries: task_creation.action.retry_policy.as_ref()
                .map(|p| p.max_attempts)
                .unwrap_or(3),
            lease_expires_at: None,
            scheduled_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        self.persistence.create_task(&task).await?;
        Ok(())
    }

    /// Execute JavaScript action
    async fn execute_javascript_action(
        &self,
        code: &str,
        context: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        let result = self.runtime.execute_script(code, context.clone()).await?;

        if result.success {
            Ok(result.output.unwrap_or(serde_json::Value::Null))
        } else {
            Err(EngineError::TaskError(
                result.error.unwrap_or("JavaScript execution failed".to_string())
            ))
        }
    }

    /// Execute HTTP action
    async fn execute_http_action(
        &self,
        method: &str,
        url: &str,
        headers: &std::collections::HashMap<String, String>,
        _context: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        // TODO: Implement HTTP client action
        // For now, return a mock response
        warn!("HTTP action not yet implemented: {} {}", method, url);
        Ok(serde_json::json!({
            "status": "not_implemented",
            "method": method,
            "url": url,
            "headers": headers,
        }))
    }

    /// Execute custom action
    async fn execute_custom_action(
        &self,
        type_name: &str,
        data: &serde_json::Value,
        _context: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        // TODO: Implement custom action system
        warn!("Custom action not yet implemented: {} with data: {:?}", type_name, data);
        Ok(serde_json::json!({
            "status": "not_implemented",
            "type": type_name,
            "data": data,
        }))
    }

    /// Build execution context for a state
    fn build_state_context(
        &self,
        instance: &WorkflowInstance,
        _definition: &WorkflowDefinition,
    ) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "workflow_instance_id": instance.id,
            "current_state": instance.current_state,
            "input": instance.input,
            "state_data": instance.state_data,
            "metadata": instance.metadata,
        }))
    }

    /// Build execution context for a task
    fn build_task_context(
        &self,
        task: &Task,
        instance: &WorkflowInstance,
        _definition: &WorkflowDefinition,
    ) -> Result<serde_json::Value> {
        Ok(serde_json::json!({
            "task_id": task.id,
            "task_type": task.task_type,
            "workflow_instance_id": instance.id,
            "current_state": instance.current_state,
            "input_data": task.input_data,
            "workflow_input": instance.input,
            "state_data": instance.state_data,
            "action_parameters": task.action.parameters,
        }))
    }

    /// Check if workflow can progress after task completion
    async fn check_workflow_progress(
        &self,
        instance: &WorkflowInstance,
        definition: &WorkflowDefinition,
        task: &Task,
        result: &serde_json::Value,
    ) -> Result<()> {
        // Update instance with task result
        let mut updated_instance = instance.clone();
        updated_instance.state_data.insert(
            format!("task.{}.result", task.id),
            result.clone()
        );

        // Determine next tasks based on current state and new data
        let state_context = self.build_state_context(&updated_instance, definition)?;
        if let Ok(task_creations) = WorkflowStateMachine::determine_next_tasks(
            definition,
            &updated_instance,
            &state_context,
        ) {
            for task_creation in task_creations {
                self.create_task(&updated_instance, definition, &task_creation).await?;
            }
        }

        Ok(())
    }

    /// Complete a workflow instance
    async fn complete_workflow(&self, instance: &mut WorkflowInstance) -> Result<()> {
        instance.status = WorkflowStatus::Completed;
        instance.output = Some(serde_json::json!({
            "final_state": instance.current_state,
            "state_data": instance.state_data,
        }));
        instance.updated_at = Utc::now();

        self.persistence.update_workflow_instance(instance).await?;

        // Log completion event
        self.persistence.log_workflow_event(&WorkflowEvent {
            id: Uuid::new_v4(),
            workflow_instance_id: instance.id,
            task_id: None,
            event_type: EventType::WorkflowCompleted,
            data: serde_json::json!({
                "final_state": instance.current_state,
                "output": instance.output,
            }),
            timestamp: Utc::now(),
        }).await?;

        info!("Completed workflow instance: {}", instance.id);
        Ok(())
    }
}