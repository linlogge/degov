//! Error types for the workflow engine

use thiserror::Error;

/// Main error type for the workflow engine
#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Workflow error: {0}")]
    Workflow(#[from] WorkflowError),
    
    #[error("Persistence error: {0}")]
    Persistence(#[from] PersistenceError),
    
    #[error("Runtime error: {0}")]
    Runtime(#[from] RuntimeError),
    
    #[error("RPC error: {0}")]
    Rpc(#[from] RpcError),
    
    #[error("Scheduler error: {0}")]
    Scheduler(String),
    
    #[error("Worker not found: {0}")]
    WorkerNotFound(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Workflow-specific errors
#[derive(Error, Debug)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    NotFound(String),
    
    #[error("Invalid workflow state: {0}")]
    InvalidState(String),
    
    #[error("State machine error: {0}")]
    StateMachine(String),
    
    #[error("Transition not allowed: from '{from}' with event '{event}'")]
    TransitionNotAllowed { from: String, event: String },
    
    #[error("Invalid workflow definition: {0}")]
    InvalidDefinition(String),
    
    #[error("Task execution failed: {0}")]
    TaskFailed(String),
}

/// Persistence layer errors
#[derive(Error, Debug)]
pub enum PersistenceError {
    #[error("FoundationDB error: {0}")]
    Fdb(#[from] foundationdb::FdbError),
    
    #[error("FoundationDB commit error: {0}")]
    FdbCommit(#[from] foundationdb::TransactionCommitError),
    
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    #[error("Data not found: {0}")]
    NotFound(String),
    
    #[error("Data corruption: {0}")]
    Corruption(String),
    
    #[error("Transaction conflict")]
    Conflict,
}

/// Runtime execution errors
#[derive(Error, Debug)]
pub enum RuntimeError {
    #[error("JavaScript execution error: {0}")]
    JavaScript(String),
    
    #[error("WASM execution error: {0}")]
    Wasm(String),
    
    #[error("Timeout exceeded: {0}ms")]
    Timeout(u64),
    
    #[error("Invalid code: {0}")]
    InvalidCode(String),
    
    #[error("Runtime not available: {0}")]
    RuntimeNotAvailable(String),
    
    #[error("Execution error: {0}")]
    Execution(String),
}

/// RPC communication errors
#[derive(Error, Debug)]
pub enum RpcError {
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Request error: {0}")]
    Request(#[from] connectare::error::RpcError),
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    
    #[error("Protocol error: {0}")]
    Protocol(String),
}

/// Convenience result types
pub type Result<T> = std::result::Result<T, EngineError>;
pub type WorkflowResult<T> = std::result::Result<T, WorkflowError>;
pub type PersistenceResult<T> = std::result::Result<T, PersistenceError>;
pub type RuntimeResult<T> = std::result::Result<T, RuntimeError>;
pub type RpcResult<T> = std::result::Result<T, RpcError>;

