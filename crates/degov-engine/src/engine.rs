/// Core workflow engine
use crate::{
    error::{EngineError, Result},
    runtime::DenoRuntime,
    workflow::{ExecutionState, StepResult, StepType, Workflow, WorkflowExecution},
};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Main workflow engine
pub struct WorkflowEngine {
    /// Deno runtime for script execution
    runtime: Arc<DenoRuntime>,
    /// Registered workflows
    workflows: Arc<RwLock<HashMap<String, Workflow>>>,
    /// Active executions
    executions: Arc<RwLock<HashMap<String, WorkflowExecution>>>,
    /// Event channel for workflow events
    event_tx: mpsc::UnboundedSender<WorkflowEvent>,
}

/// Events emitted by the workflow engine
#[derive(Debug, Clone)]
pub enum WorkflowEvent {
    WorkflowStarted {
        workflow_id: String,
        execution_id: String,
    },
    StepStarted {
        execution_id: String,
        step_id: String,
    },
    StepCompleted {
        execution_id: String,
        step_id: String,
        result: StepResult,
    },
    WorkflowCompleted {
        execution_id: String,
        success: bool,
    },
    WorkflowFailed {
        execution_id: String,
        error: String,
    },
}

impl WorkflowEngine {
    /// Create a new workflow engine
    pub async fn new(worker_pool_size: usize) -> Result<Self> {
        let runtime = Arc::new(DenoRuntime::new(worker_pool_size).await?);
        let (event_tx, _event_rx) = mpsc::unbounded_channel();

        Ok(Self {
            runtime,
            workflows: Arc::new(RwLock::new(HashMap::new())),
            executions: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        })
    }

    /// Register a workflow
    pub fn register_workflow(&self, workflow: Workflow) -> Result<()> {
        let workflow_id = workflow.id.clone();
        info!("Registering workflow: {}", workflow_id);
        
        self.workflows.write().insert(workflow_id, workflow);
        Ok(())
    }

    /// Get a workflow by ID
    pub fn get_workflow(&self, workflow_id: &str) -> Option<Workflow> {
        self.workflows.read().get(workflow_id).cloned()
    }

    /// Start a workflow execution
    pub async fn start_workflow(
        &self,
        workflow_id: &str,
        execution_id: String,
        inputs: HashMap<String, serde_json::Value>,
    ) -> Result<String> {
        let workflow = self
            .get_workflow(workflow_id)
            .ok_or_else(|| EngineError::WorkflowNotFound(workflow_id.to_string()))?;

        let mut execution = WorkflowExecution::new(workflow_id.to_string(), execution_id.clone());
        execution.state = ExecutionState::Running;
        
        // Set input variables
        for (key, value) in inputs {
            execution.set_variable(key, value);
        }

        // Also set workflow inputs
        for (key, value) in workflow.inputs.iter() {
            if !execution.variables.contains_key(key) {
                execution.set_variable(key.clone(), value.clone());
            }
        }

        self.executions.write().insert(execution_id.clone(), execution);

        // Emit event
        let _ = self.event_tx.send(WorkflowEvent::WorkflowStarted {
            workflow_id: workflow_id.to_string(),
            execution_id: execution_id.clone(),
        });

        // Start execution in background
        let engine = self.clone();
        let exec_id = execution_id.clone();
        tokio::spawn(async move {
            if let Err(e) = engine.execute_workflow(&exec_id).await {
                error!("Workflow execution failed: {:?}", e);
            }
        });

        Ok(execution_id)
    }

    /// Execute a workflow
    async fn execute_workflow(&self, execution_id: &str) -> Result<()> {
        let workflow = {
            let executions = self.executions.read();
            let execution = executions
                .get(execution_id)
                .ok_or_else(|| EngineError::ExecutionFailed("Execution not found".to_string()))?;
            
            self.get_workflow(&execution.workflow_id)
                .ok_or_else(|| EngineError::WorkflowNotFound(execution.workflow_id.clone()))?
        };

        info!("Executing workflow: {} (execution: {})", workflow.id, execution_id);

        for step in &workflow.steps {
            // Update current step
            {
                let mut executions = self.executions.write();
                if let Some(exec) = executions.get_mut(execution_id) {
                    exec.current_step += 1;
                }
            }

            // Emit step started event
            let _ = self.event_tx.send(WorkflowEvent::StepStarted {
                execution_id: execution_id.to_string(),
                step_id: step.id.clone(),
            });

            debug!("Executing step: {}", step.name);
            
            let result = self.execute_step(execution_id, step).await?;

            // Emit step completed event
            let _ = self.event_tx.send(WorkflowEvent::StepCompleted {
                execution_id: execution_id.to_string(),
                step_id: step.id.clone(),
                result: result.clone(),
            });

            if !result.success {
                // Mark execution as failed
                {
                    let mut executions = self.executions.write();
                    if let Some(exec) = executions.get_mut(execution_id) {
                        exec.state = ExecutionState::Failed;
                    }
                }

                let error_msg = result.error.unwrap_or_else(|| "Unknown error".to_string());
                
                let _ = self.event_tx.send(WorkflowEvent::WorkflowFailed {
                    execution_id: execution_id.to_string(),
                    error: error_msg.clone(),
                });

                return Err(EngineError::ExecutionFailed(error_msg));
            }

            // Update variables with step output
            if let Some(output) = result.output {
                let mut executions = self.executions.write();
                if let Some(exec) = executions.get_mut(execution_id) {
                    if let serde_json::Value::Object(map) = output {
                        for (key, value) in map {
                            exec.set_variable(key, value);
                        }
                    }
                }
            }
        }

        // Mark execution as completed
        {
            let mut executions = self.executions.write();
            if let Some(exec) = executions.get_mut(execution_id) {
                exec.state = ExecutionState::Completed;
            }
        }

        let _ = self.event_tx.send(WorkflowEvent::WorkflowCompleted {
            execution_id: execution_id.to_string(),
            success: true,
        });

        info!("Workflow execution completed: {}", execution_id);
        Ok(())
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        execution_id: &str,
        step: &crate::workflow::Step,
    ) -> Result<StepResult> {
        match &step.step_type {
            StepType::Script { code } => {
                // Get current context
                let context = {
                    let executions = self.executions.read();
                    let execution = executions
                        .get(execution_id)
                        .ok_or_else(|| EngineError::ExecutionFailed("Execution not found".to_string()))?;
                    
                    serde_json::to_value(&execution.variables)
                        .map_err(|e| EngineError::SerializationError(e))?
                };

                // Execute script via Deno runtime
                let script_result = self.runtime.execute_script(code, context).await?;

                Ok(StepResult {
                    step_id: step.id.clone(),
                    success: script_result.success,
                    output: script_result.output,
                    error: script_result.error,
                })
            }
            StepType::Set => {
                // Set variables from params
                let mut executions = self.executions.write();
                if let Some(exec) = executions.get_mut(execution_id) {
                    for (key, value) in &step.params {
                        exec.set_variable(key.clone(), value.clone());
                    }
                }

                Ok(StepResult {
                    step_id: step.id.clone(),
                    success: true,
                    output: Some(serde_json::json!(step.params)),
                    error: None,
                })
            }
            StepType::Log => {
                // Log variables
                let executions = self.executions.read();
                if let Some(exec) = executions.get(execution_id) {
                    info!("Log step: {:?}", exec.variables);
                }

                Ok(StepResult {
                    step_id: step.id.clone(),
                    success: true,
                    output: None,
                    error: None,
                })
            }
        }
    }

    /// Get execution status
    pub fn get_execution(&self, execution_id: &str) -> Option<WorkflowExecution> {
        self.executions.read().get(execution_id).cloned()
    }

    /// Subscribe to workflow events
    pub fn subscribe(&self) -> mpsc::UnboundedReceiver<WorkflowEvent> {
        let (_tx, rx) = mpsc::unbounded_channel();
        // In a full implementation, we'd manage multiple subscribers
        // For now, this is a simplified version
        rx
    }

    /// Shutdown the engine
    pub async fn shutdown(&self) {
        info!("Shutting down workflow engine");
        self.runtime.shutdown().await;
    }
}

impl Clone for WorkflowEngine {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.clone(),
            workflows: self.workflows.clone(),
            executions: self.executions.clone(),
            event_tx: self.event_tx.clone(),
        }
    }
}

