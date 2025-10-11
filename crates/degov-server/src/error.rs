/// Server-level errors for business logic operations
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Engine error: {0}")]
    EngineError(String),

    #[error("Permission denied")]
    PermissionDenied,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Internal error: {0}")]
    Internal(String),
}


