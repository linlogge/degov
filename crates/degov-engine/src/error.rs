use thiserror::Error;

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),

    #[error("Instance not found: {0}")]
    InstanceNotFound(String),

    #[error("Invalid state transition: from {from} to {to}")]
    InvalidTransition { from: String, to: String },

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task already claimed: {0}")]
    TaskAlreadyClaimed(String),

    #[error("Task lease expired: {0}")]
    TaskLeaseExpired(String),

    #[error("Worker not found: {0}")]
    WorkerNotFound(String),

    #[error("Optimistic lock conflict")]
    OptimisticLockConflict,

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Transaction conflict")]
    TransactionConflict,

    #[error("Script execution error: {0}")]
    ScriptError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Instance already exists: {0}")]
    InstanceAlreadyExists(String),

    #[error("Dead letter queue: {task_id} - {reason}")]
    DeadLetterQueue { task_id: String, reason: String },
}

pub type Result<T> = std::result::Result<T, EngineError>;

impl From<foundationdb::FdbError> for EngineError {
    fn from(err: foundationdb::FdbError) -> Self {
        EngineError::DatabaseError(format!("{:?}", err))
    }
}

impl From<foundationdb::TransactionCommitError> for EngineError {
    fn from(err: foundationdb::TransactionCommitError) -> Self {
        EngineError::DatabaseError(format!("{:?}", err))
    }
}

impl From<serde_json::Error> for EngineError {
    fn from(err: serde_json::Error) -> Self {
        EngineError::SerializationError(err.to_string())
    }
}
