//! Error types for MST operations

use foundationdb::TransactionCommitError;
use thiserror::Error;

/// Errors that can occur during MST operations
#[derive(Error, Debug)]
pub enum MstError {
    #[error("FoundationDB error: {0}")]
    FdbError(#[from] foundationdb::FdbError),
    #[error("FoundationDB commit error: {0}")]
    FdbCommitError(#[from] TransactionCommitError),
	#[error("Serialization error: {0}")]
	SerdeError(#[from] serde_json::Error),
	#[error("DAG-CBOR error: {0}")]
	DagCbor(String),
    #[error("Invalid layer")]
    InvalidLayer,
    #[error("Node not found")]
    NodeNotFound,
    #[error("Conflict: {0}")]
    Conflict(String),
}
