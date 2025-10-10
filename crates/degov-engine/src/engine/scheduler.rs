//! Task scheduler with round-robin worker selection

use crate::persistence::PersistenceLayer;
use crate::types::{WorkerInfo, WorkerId};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

/// Task scheduler for round-robin worker assignment
pub struct TaskScheduler {
    persistence: Arc<PersistenceLayer>,
    workers: Arc<RwLock<Vec<WorkerInfo>>>,
    next_worker_idx: AtomicUsize,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(persistence: Arc<PersistenceLayer>) -> Self {
        Self {
            persistence,
            workers: Arc::new(RwLock::new(Vec::new())),
            next_worker_idx: AtomicUsize::new(0),
        }
    }

    /// Register a worker
    pub fn register_worker(&self, worker: WorkerInfo) {
        let mut workers = self.workers.write();
        
        // Remove if already exists (re-registration)
        workers.retain(|w| w.id != worker.id);
        
        workers.push(worker);
        tracing::info!("Registered worker, total workers: {}", workers.len());
    }

    /// Unregister a worker
    pub fn unregister_worker(&self, worker_id: &WorkerId) {
        let mut workers = self.workers.write();
        workers.retain(|w| w.id != *worker_id);
        tracing::info!("Unregistered worker, total workers: {}", workers.len());
    }

    /// Get next worker using round-robin
    pub fn get_next_worker(&self) -> Option<WorkerId> {
        let workers = self.workers.read();
        
        if workers.is_empty() {
            return None;
        }

        let idx = self.next_worker_idx.fetch_add(1, Ordering::Relaxed) % workers.len();
        Some(workers[idx].id.clone())
    }

    /// Get worker count
    pub fn worker_count(&self) -> usize {
        self.workers.read().len()
    }

    /// Get all workers
    pub fn list_workers(&self) -> Vec<WorkerInfo> {
        self.workers.read().clone()
    }

    /// Check if a worker is registered
    pub fn is_worker_registered(&self, worker_id: &WorkerId) -> bool {
        self.workers.read().iter().any(|w| w.id == *worker_id)
    }

    /// Update worker statistics
    pub fn update_worker_stats(
        &self,
        worker_id: &WorkerId,
        active_tasks: u32,
        total_completed: u64,
        total_failed: u64,
    ) {
        let mut workers = self.workers.write();
        if let Some(worker) = workers.iter_mut().find(|w| w.id == *worker_id) {
            worker.stats.active_tasks = active_tasks;
            worker.stats.total_tasks_completed = total_completed;
            worker.stats.total_tasks_failed = total_failed;
        }
    }
}


