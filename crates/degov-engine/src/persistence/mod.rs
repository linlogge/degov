//! Persistence layer using FoundationDB

mod task;
mod worker;
mod workflow;

pub use task::TaskStore;
pub use worker::WorkerStore;
pub use workflow::WorkflowStore;

use crate::error::PersistenceResult;
use foundationdb::Database;
use std::sync::Arc;

/// Main persistence layer coordinator
#[derive(Clone)]
pub struct PersistenceLayer {
    db: Arc<Database>,
    workflow_store: WorkflowStore,
    task_store: TaskStore,
    worker_store: WorkerStore,
}

impl PersistenceLayer {
    /// Create a new persistence layer
    pub fn new(db: Database) -> Self {
        let db = Arc::new(db);
        Self {
            workflow_store: WorkflowStore::new(db.clone()),
            task_store: TaskStore::new(db.clone()),
            worker_store: WorkerStore::new(db.clone()),
            db,
        }
    }

    /// Get the workflow store
    pub fn workflows(&self) -> &WorkflowStore {
        &self.workflow_store
    }

    /// Get the task store
    pub fn tasks(&self) -> &TaskStore {
        &self.task_store
    }

    /// Get the worker store
    pub fn workers(&self) -> &WorkerStore {
        &self.worker_store
    }

    /// Get the underlying database
    pub fn db(&self) -> &Database {
        &self.db
    }

    /// Run a health check
    pub async fn health_check(&self) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        
        // Set transaction timeout to 2 seconds
        tx.set_option(foundationdb::options::TransactionOption::Timeout(2000))?;
        
        // Simple read to verify database connection
        let _result = tx.get(b"health_check", false).await?;
        tx.cancel();
        Ok(())
    }
}

/// Key prefix constants
pub(crate) mod keys {
    pub const WORKFLOW_PREFIX: &[u8] = b"wf:";
    pub const WORKFLOW_DEF_PREFIX: &[u8] = b"wfd:";
    pub const TASK_PREFIX: &[u8] = b"tk:";
    pub const TASK_QUEUE_PREFIX: &[u8] = b"tq:";
    pub const WORKER_PREFIX: &[u8] = b"wr:";
    pub const WORKER_HEARTBEAT_PREFIX: &[u8] = b"wh:";
}

/// Helper to build FDB keys
pub(crate) fn build_key(prefix: &[u8], id: &str) -> Vec<u8> {
    let mut key = Vec::with_capacity(prefix.len() + id.len());
    key.extend_from_slice(prefix);
    key.extend_from_slice(id.as_bytes());
    key
}


