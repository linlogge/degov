use std::sync::Arc;
use degov_engine::{WorkflowId, WorkflowInstance};

use crate::{AppState, ServerError};

/// Main service coordinator for workflow operations
/// This is where business logic and orchestration lives
pub struct WorkflowService {
    state: Arc<AppState>,
}

impl WorkflowService {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }

    /// Start a workflow instance
    pub async fn start_workflow(
        &self,
        workflow_def_id: &str,
        input_json: &str,
    ) -> Result<WorkflowInstance, ServerError> {
        let workflow_id = uuid::Uuid::parse_str(workflow_def_id)
            .map_err(|e| ServerError::InvalidInput(format!("Invalid workflow ID: {}", e)))?;
        let workflow_id = WorkflowId::from_uuid(workflow_id);

        let input: serde_json::Value = serde_json::from_str(input_json)
            .map_err(|e| ServerError::InvalidInput(format!("Invalid input JSON: {}", e)))?;

        let instance = self.state.engine
            .start_workflow(&workflow_id, input)
            .await
            .map_err(|e| ServerError::EngineError(e.to_string()))?;

        Ok(instance)
    }

    /// Get a workflow instance
    pub async fn get_workflow_instance(
        &self,
        instance_id: &str,
    ) -> Result<WorkflowInstance, ServerError> {
        let instance_id = uuid::Uuid::parse_str(instance_id)
            .map_err(|e| ServerError::InvalidInput(format!("Invalid instance ID: {}", e)))?;
        let instance_id = WorkflowId::from_uuid(instance_id);

        let instance = self.state.engine
            .persistence()
            .workflows()
            .get_instance(&instance_id)
            .await
            .map_err(|e| ServerError::Internal(format!("Database error: {}", e)))?
            .ok_or_else(|| ServerError::NotFound(format!("Workflow instance not found: {}", instance_id)))?;

        Ok(instance)
    }

    /// Trigger a workflow state transition
    pub async fn trigger_transition(
        &self,
        instance_id: &str,
        event: &str,
    ) -> Result<String, ServerError> {
        let instance_id = uuid::Uuid::parse_str(instance_id)
            .map_err(|e| ServerError::InvalidInput(format!("Invalid instance ID: {}", e)))?;
        let instance_id = WorkflowId::from_uuid(instance_id);

        let new_state = self.state.engine
            .transition_workflow(&instance_id, event)
            .await
            .map_err(|e| ServerError::EngineError(e.to_string()))?;

        Ok(new_state)
    }

    /// Cancel a workflow instance
    pub async fn cancel_workflow(
        &self,
        instance_id: &str,
    ) -> Result<(), ServerError> {
        let instance_id = uuid::Uuid::parse_str(instance_id)
            .map_err(|e| ServerError::InvalidInput(format!("Invalid instance ID: {}", e)))?;
        let instance_id = WorkflowId::from_uuid(instance_id);

        self.state.engine
            .persistence()
            .workflows()
            .update_state(&instance_id, "cancelled", degov_engine::WorkflowStatus::Cancelled)
            .await
            .map_err(|e| ServerError::Internal(format!("Failed to cancel workflow: {}", e)))?;

        Ok(())
    }

    /// List all registered workers
    pub async fn list_workers(&self) -> Result<Vec<degov_engine::WorkerInfo>, ServerError> {
        let workers = self.state.engine.scheduler().list_workers();
        Ok(workers)
    }

    /// Get a specific worker's status
    pub async fn get_worker_status(
        &self,
        worker_id: &str,
    ) -> Result<degov_engine::WorkerInfo, ServerError> {
        let worker_id = degov_engine::WorkerId::from_string(worker_id.to_string());

        let worker = self.state.engine
            .persistence()
            .workers()
            .get(&worker_id)
            .await
            .map_err(|e| ServerError::Internal(format!("Database error: {}", e)))?
            .ok_or_else(|| ServerError::NotFound(format!("Worker not found: {}", worker_id)))?;

        Ok(worker)
    }

    /// Get a task's status
    pub async fn get_task_status(
        &self,
        task_id: &str,
    ) -> Result<degov_engine::TaskExecution, ServerError> {
        let task_id = uuid::Uuid::parse_str(task_id)
            .map_err(|e| ServerError::InvalidInput(format!("Invalid task ID: {}", e)))?;
        let task_id = degov_engine::TaskId::from_uuid(task_id);

        let task = self.state.engine
            .persistence()
            .tasks()
            .get(&task_id)
            .await
            .map_err(|e| ServerError::Internal(format!("Database error: {}", e)))?
            .ok_or_else(|| ServerError::NotFound(format!("Task not found: {}", task_id)))?;

        Ok(task)
    }
}

