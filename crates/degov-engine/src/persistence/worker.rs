//! Worker persistence

use super::{build_key, keys};
use crate::error::PersistenceResult;
use crate::types::{WorkerHealthStatus, WorkerInfo, WorkerId};
use chrono::Utc;
use foundationdb::{Database, Transaction};
use std::sync::Arc;

/// Worker storage operations
#[derive(Clone)]
pub struct WorkerStore {
    db: Arc<Database>,
}

impl WorkerStore {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Register a worker
    pub async fn register(&self, worker: WorkerInfo) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        
        // Set transaction timeout to 2 seconds
        tx.set_option(foundationdb::options::TransactionOption::Timeout(2000))?;
        
        // Set retry limit
        tx.set_option(foundationdb::options::TransactionOption::RetryLimit(5))?;
        
        self.register_tx(&tx, worker).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Register a worker within a transaction
    pub async fn register_tx(&self, tx: &Transaction, worker: WorkerInfo) -> PersistenceResult<()> {
        let key = build_key(keys::WORKER_PREFIX, worker.id.as_str());
        let value = serde_json::to_vec(&worker)?;
        tx.set(&key, &value);

        // Set heartbeat timestamp
        let heartbeat_key = build_key(keys::WORKER_HEARTBEAT_PREFIX, worker.id.as_str());
        let timestamp = Utc::now().timestamp_millis().to_be_bytes();
        tx.set(&heartbeat_key, &timestamp);

        Ok(())
    }

    /// Get a worker by ID
    pub async fn get(&self, worker_id: &WorkerId) -> PersistenceResult<Option<WorkerInfo>> {
        let tx = self.db.create_trx()?;
        let result = self.get_tx(&tx, worker_id).await?;
        tx.cancel();
        Ok(result)
    }

    /// Get a worker by ID within a transaction
    pub async fn get_tx(
        &self,
        tx: &Transaction,
        worker_id: &WorkerId,
    ) -> PersistenceResult<Option<WorkerInfo>> {
        let key = build_key(keys::WORKER_PREFIX, worker_id.as_str());
        let bytes = tx.get(&key, false).await?;
        
        match bytes {
            Some(data) => {
                let worker = serde_json::from_slice(data.as_ref())?;
                Ok(Some(worker))
            }
            None => Ok(None),
        }
    }

    /// Update worker heartbeat
    pub async fn heartbeat(&self, worker_id: &WorkerId) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        
        // Set transaction timeout to 2 seconds
        tx.set_option(foundationdb::options::TransactionOption::Timeout(2000))?;
        tx.set_option(foundationdb::options::TransactionOption::RetryLimit(3))?;
        
        let heartbeat_key = build_key(keys::WORKER_HEARTBEAT_PREFIX, worker_id.as_str());
        let timestamp = Utc::now().timestamp_millis().to_be_bytes();
        tx.set(&heartbeat_key, &timestamp);

        // Update worker record
        let worker_key = build_key(keys::WORKER_PREFIX, worker_id.as_str());
        if let Some(worker_bytes) = tx.get(&worker_key, false).await? {
            let mut worker: WorkerInfo = serde_json::from_slice(worker_bytes.as_ref())?;
            worker.last_heartbeat = Utc::now();
            worker.status = WorkerHealthStatus::Healthy;
            
            let updated_value = serde_json::to_vec(&worker)?;
            tx.set(&worker_key, &updated_value);
        }

        tx.commit().await?;
        Ok(())
    }

    /// Update worker statistics
    pub async fn update_stats(
        &self,
        worker_id: &WorkerId,
        active_tasks: u32,
        total_completed: u64,
        total_failed: u64,
    ) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        
        // Set transaction timeout to 2 seconds
        tx.set_option(foundationdb::options::TransactionOption::Timeout(2000))?;
        tx.set_option(foundationdb::options::TransactionOption::RetryLimit(3))?;
        
        let worker_key = build_key(keys::WORKER_PREFIX, worker_id.as_str());
        if let Some(worker_bytes) = tx.get(&worker_key, false).await? {
            let mut worker: WorkerInfo = serde_json::from_slice(worker_bytes.as_ref())?;
            worker.stats.active_tasks = active_tasks;
            worker.stats.total_tasks_completed = total_completed;
            worker.stats.total_tasks_failed = total_failed;
            
            let updated_value = serde_json::to_vec(&worker)?;
            tx.set(&worker_key, &updated_value);
        }

        tx.commit().await?;
        Ok(())
    }

    /// Unregister a worker
    pub async fn unregister(&self, worker_id: &WorkerId) -> PersistenceResult<()> {
        let tx = self.db.create_trx()?;
        
        // Set transaction timeout to 2 seconds
        tx.set_option(foundationdb::options::TransactionOption::Timeout(2000))?;
        tx.set_option(foundationdb::options::TransactionOption::RetryLimit(3))?;
        
        let worker_key = build_key(keys::WORKER_PREFIX, worker_id.as_str());
        tx.clear(&worker_key);
        
        let heartbeat_key = build_key(keys::WORKER_HEARTBEAT_PREFIX, worker_id.as_str());
        tx.clear(&heartbeat_key);

        tx.commit().await?;
        Ok(())
    }
}


