/// Error types for the workflow engine
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Script execution error: {0}")]
    ScriptError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Worker pool error: {0}")]
    WorkerPoolError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Invalid workflow: {0}")]
    InvalidWorkflow(String),

    #[error("Rustyscript error: {0}")]
    RustyscriptError(#[from] rustyscript::Error),
}

pub type Result<T> = std::result::Result<T, EngineError>;

