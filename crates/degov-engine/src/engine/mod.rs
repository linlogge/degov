//! Workflow engine implementation

mod registry;
mod scheduler;
mod server;

pub use registry::WorkflowRegistry;
pub use scheduler::TaskScheduler;
pub use server::run_server;

use crate::error::{EngineError, Result};
use crate::persistence::PersistenceLayer;
use crate::state_machine::Context;
use crate::types::{
    TaskDefinition, TaskExecution, TaskId, TaskStatus, WorkflowDefinition, WorkflowId,
    WorkflowInstance, WorkflowStatus,
};
use chrono::Utc;
use foundationdb::Database;
use parking_lot::RwLock;
use std::net::SocketAddr;
use std::sync::Arc;

/// Main workflow engine
pub struct WorkflowEngine {
    persistence: Arc<PersistenceLayer>,
    registry: Arc<RwLock<WorkflowRegistry>>,
    scheduler: Arc<TaskScheduler>,
    bind_addr: SocketAddr,
}

impl WorkflowEngine {
    /// Create a new workflow engine
    pub async fn new(db: Database, bind_addr: SocketAddr) -> Result<Self> {
        let persistence = Arc::new(PersistenceLayer::new(db));
        let scheduler = Arc::new(TaskScheduler::new(persistence.clone()));
        let registry = Arc::new(RwLock::new(WorkflowRegistry::new()));

        // Perform health check
        persistence
            .health_check()
            .await
            .map_err(|e| EngineError::Internal(format!("Database health check failed: {}", e)))?;

        Ok(Self {
            persistence,
            registry,
            scheduler,
            bind_addr,
        })
    }

    /// Register a workflow definition
    pub async fn register_workflow(&self, definition: WorkflowDefinition) -> Result<WorkflowId> {
        // Validate the state machine
        definition
            .state_machine
            .validate()
            .map_err(EngineError::Workflow)?;

        // Save to persistence
        self.persistence
            .workflows()
            .save_definition(&definition)
            .await
            .map_err(EngineError::Persistence)?;

        // Add to registry
        let id = definition.id;
        self.registry.write().register(definition);

        tracing::info!("Registered workflow: {}", id);
        Ok(id)
    }

    /// Start a workflow instance
    pub async fn start_workflow(
        &self,
        definition_id: &WorkflowId,
        input: serde_json::Value,
    ) -> Result<WorkflowInstance> {
        // Get workflow definition
        let definition = self
            .registry
            .read()
            .get(definition_id)
            .ok_or_else(|| EngineError::Workflow(crate::error::WorkflowError::NotFound(definition_id.to_string())))?
            .clone();

        // Create workflow instance
        let instance = WorkflowInstance {
            id: WorkflowId::new(),
            definition_id: *definition_id,
            current_state: definition.state_machine.initial_state().to_string(),
            context: input,
            status: WorkflowStatus::Running,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            completed_at: None,
        };

        // Save instance
        self.persistence
            .workflows()
            .save_instance(&instance)
            .await
            .map_err(EngineError::Persistence)?;

        // Execute initial state actions
        self.execute_state_actions(&instance, &definition).await?;

        tracing::info!("Started workflow instance: {}", instance.id);
        Ok(instance)
    }

    /// Transition a workflow to a new state
    pub async fn transition_workflow(
        &self,
        workflow_id: &WorkflowId,
        event: &str,
    ) -> Result<String> {
        // Get workflow instance
        let instance = self
            .persistence
            .workflows()
            .get_instance(workflow_id)
            .await
            .map_err(EngineError::Persistence)?
            .ok_or_else(|| EngineError::Workflow(crate::error::WorkflowError::NotFound(workflow_id.to_string())))?;

        // Get workflow definition
        let definition = self
            .persistence
            .workflows()
            .get_definition(&instance.definition_id)
            .await
            .map_err(EngineError::Persistence)?
            .ok_or_else(|| EngineError::Workflow(crate::error::WorkflowError::NotFound(instance.definition_id.to_string())))?;

        // Create context
        let mut ctx = Context::with_data(
            *workflow_id,
            instance.current_state.clone(),
            instance.context.clone(),
        );

        // Perform transition
        let new_state = definition
            .state_machine
            .transition(&mut ctx, event)
            .await
            .map_err(EngineError::Workflow)?;

        // Update workflow instance
        self.persistence
            .workflows()
            .update_state(workflow_id, &new_state, WorkflowStatus::Running)
            .await
            .map_err(EngineError::Persistence)?;

        self.persistence
            .workflows()
            .update_context(workflow_id, ctx.data().clone())
            .await
            .map_err(EngineError::Persistence)?;

        tracing::info!("Workflow {} transitioned to state: {}", workflow_id, new_state);
        Ok(new_state)
    }

    /// Execute state actions (enqueue tasks)
    async fn execute_state_actions(
        &self,
        instance: &WorkflowInstance,
        definition: &WorkflowDefinition,
    ) -> Result<()> {
        let state = definition
            .state_machine
            .get_state(&instance.current_state)
            .ok_or_else(|| {
                EngineError::Workflow(crate::error::WorkflowError::InvalidState(
                    instance.current_state.clone(),
                ))
            })?;

        // Enqueue tasks from on_enter actions
        for action in state.on_enter_actions() {
            if let crate::state_machine::Action::ExecuteTask(task_def) = action {
                self.enqueue_task(instance.id, task_def.clone()).await?;
            }
        }

        Ok(())
    }

    /// Enqueue a task for execution
    async fn enqueue_task(&self, workflow_id: WorkflowId, definition: TaskDefinition) -> Result<TaskId> {
        let task = TaskExecution {
            id: TaskId::new(),
            workflow_id,
            definition,
            input: Vec::new(), // TODO: Get from context
            status: TaskStatus::Pending,
            assigned_worker: None,
            attempt: 0,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
        };

        let task_id = task.id;
        self.persistence
            .tasks()
            .enqueue(task)
            .await
            .map_err(EngineError::Persistence)?;

        tracing::info!("Enqueued task: {}", task_id);
        Ok(task_id)
    }

    /// Get the scheduler
    pub fn scheduler(&self) -> &TaskScheduler {
        &self.scheduler
    }

    /// Get the persistence layer
    pub fn persistence(&self) -> &PersistenceLayer {
        &self.persistence
    }

    /// Run the engine (start RPC server)
    pub async fn run(self: Arc<Self>) -> Result<()> {
        let bind_addr = self.bind_addr;
        tracing::info!("Starting workflow engine on {}", bind_addr);
        
        // Start the RPC server
        server::run_server(self, bind_addr).await
    }

    /// Recover from crashes (reschedule orphaned tasks)
    pub async fn recover(&self) -> Result<()> {
        tracing::info!("Starting recovery process");
        
        // TODO: Implement recovery logic
        // 1. Find tasks with status Assigned but worker is dead
        // 2. Reschedule them
        // 3. Find workflows in Running state and verify consistency
        
        tracing::info!("Recovery complete");
        Ok(())
    }
}

