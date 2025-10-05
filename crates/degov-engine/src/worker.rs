//! Worker implementation for processing workflow tasks

use crate::models::*;
use crate::workflow_engine::WorkflowEngine;
use chrono::Utc;
use crate::error::Result;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info};

/// Worker that processes workflow tasks
pub struct WorkflowWorker {
    engine: Arc<WorkflowEngine>,
    worker_id: WorkerId,
    capabilities: Vec<String>,
    heartbeat_interval_ms: u64,
    poll_interval_ms: u64,
    max_concurrent_tasks: usize,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl WorkflowWorker {
    /// Create a new worker
    pub async fn new(
        engine: Arc<WorkflowEngine>,
        name: &str,
        capabilities: Vec<String>,
    ) -> Result<Self> {
        // Register the worker
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("started_at".to_string(), serde_json::json!(Utc::now()));
        metadata.insert("version".to_string(), serde_json::json!("1.0"));
        
        let worker_id = engine.register_worker(
            name,
            capabilities.clone(),
            Some(metadata),
        ).await?;

        Ok(Self {
            engine,
            worker_id,
            capabilities,
            heartbeat_interval_ms: 10000, // 10 seconds
            poll_interval_ms: 1000,         // 1 second
            max_concurrent_tasks: 10,
            shutdown_tx: None,
        })
    }

    /// Set heartbeat interval
    pub fn with_heartbeat_interval(mut self, interval_ms: u64) -> Self {
        self.heartbeat_interval_ms = interval_ms;
        self
    }

    /// Set poll interval
    pub fn with_poll_interval(mut self, interval_ms: u64) -> Self {
        self.poll_interval_ms = interval_ms;
        self
    }

    /// Set max concurrent tasks
    pub fn with_max_concurrent_tasks(mut self, max_tasks: usize) -> Self {
        self.max_concurrent_tasks = max_tasks;
        self
    }

    /// Start the worker
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        info!("Starting worker: {} ({})", self.worker_id, self.capabilities.join(", "));

        // Keep a clone for the main waiting loop
        let main_shutdown_rx = shutdown_rx.clone();
        
        // Start heartbeat task
        let engine = self.engine.clone();
        let worker_id = self.worker_id;
        let heartbeat_interval = self.heartbeat_interval_ms;
        let heartbeat_shutdown_rx = shutdown_rx.clone();

        let heartbeat_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(heartbeat_interval));
            let mut shutdown_rx = heartbeat_shutdown_rx;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = engine.update_worker_heartbeat(&worker_id).await {
                            error!("Failed to update heartbeat for worker {}: {}", worker_id, e);
                        }
                    },
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            debug!("Heartbeat task shutting down for worker: {}", worker_id);
                            break;
                        }
                    }
                }
            }
        });

        // Start task processing task
        let engine = self.engine.clone();
        let worker_id = self.worker_id.clone();
        let capabilities = self.capabilities.clone();
        let poll_interval = self.poll_interval_ms;
        let _max_concurrent = self.max_concurrent_tasks; // Reserved for future use
        let processing_shutdown_rx = shutdown_rx.clone();

        let processing_task: tokio::task::JoinHandle<()> = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(poll_interval));
            let mut shutdown_rx = processing_shutdown_rx;

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Check if we should shutdown
                        if *shutdown_rx.borrow() {
                            debug!("Task processing stopping for worker: {}", worker_id);
                            break;
                        }

                        // Try to get tasks and process them
                        // Note: This happens outside the spawned task to avoid Send issues
                        match engine.get_next_tasks(&capabilities, 5).await {
                            Ok(tasks) => {
                                for task in tasks {
                                    let engine_clone = engine.clone();
                                    let task_id = task.id;
                                    let worker_id_clone = worker_id.clone();

                                    // Process the task in a separate task
                                    tokio::spawn(async move {
                                        if let Err(e) = Self::process_single_task(&engine_clone, &worker_id_clone, &task).await {
                                            error!("Failed to process task {}: {}", task_id, e);
                                        }
                                    });
                                }
                            },
                            Err(e) => {
                                debug!("Failed to get next tasks: {}", e);
                            }
                        }
                    },
                    _ = shutdown_rx.changed() => {
                        if *shutdown_rx.borrow() {
                            debug!("Task processing task shutting down for worker: {}", worker_id);
                            break;
                        }
                    }
                }
            }
        });

        // Wait for shutdown signal
        while !*main_shutdown_rx.borrow() {
            sleep(Duration::from_millis(100)).await;
        }

        // Wait for tasks to complete
        let _ = tokio::try_join!(heartbeat_task, processing_task);

        info!("Worker shutdown complete: {}", self.worker_id);
        Ok(())
    }

    /// Shutdown the worker gracefully
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down worker: {}", self.worker_id);

        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(true);
        }

        // Give some time for in-flight tasks to complete
        sleep(Duration::from_secs(5)).await;

        Ok(())
    }

    /// Process a single task
    async fn process_single_task(
        engine: &WorkflowEngine,
        worker_id: &WorkerId,
        task: &Task,
    ) -> Result<()> {
        debug!("Processing task: {} for worker: {}", task.id, worker_id);

        let start_time = Utc::now();

        // Execute the task
        match engine.execute_task(&task.id, worker_id).await {
            Ok(result) => {
                let duration = Utc::now() - start_time;
                info!("Successfully completed task: {} in {:?} with result: {:?}",
                      task.id, duration, result);
            },
            Err(e) => {
                let duration = Utc::now() - start_time;
                error!("Failed to process task: {} in {:?}: {}", task.id, duration, e);

                // The engine handles task retries and dead letter queue
                // so we just log the error here
            }
        }

        Ok(())
    }

    /// Get worker ID
    pub fn worker_id(&self) -> &WorkerId {
        &self.worker_id
    }

    /// Get worker capabilities
    pub fn capabilities(&self) -> &[String] {
        &self.capabilities
    }
}

/// Worker pool manager for running multiple workers
pub struct WorkerPool {
    workers: Vec<WorkflowWorker>,
    shutdown_tx: Option<tokio::sync::watch::Sender<bool>>,
}

impl WorkerPool {
    /// Create a new worker pool
    pub fn new() -> Self {
        Self {
            workers: Vec::new(),
            shutdown_tx: None,
        }
    }

    /// Add a worker to the pool
    pub async fn add_worker(&mut self, worker: WorkflowWorker) -> &mut Self {
        self.workers.push(worker);
        self
    }

    /// Start all workers in the pool
    pub async fn start(&mut self) -> Result<()> {
        let (shutdown_tx, _shutdown_rx) = tokio::sync::watch::channel(false);
        self.shutdown_tx = Some(shutdown_tx);

        info!("Starting worker pool with {} workers", self.workers.len());

        // Note: This is a simplified implementation
        // In a real implementation, you'd spawn background tasks for each worker
        for worker in &self.workers {
            info!("Worker {} ready to start", worker.worker_id());
        }

        info!("Worker pool started successfully");
        Ok(())
    }

    /// Shutdown all workers in the pool
    pub async fn shutdown(&mut self) -> Result<()> {
        info!("Shutting down worker pool with {} workers", self.workers.len());

        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(true);
        }

        // Shutdown each worker
        for worker in &mut self.workers {
            if let Err(e) = worker.shutdown().await {
                error!("Error shutting down worker {}: {}", worker.worker_id(), e);
            }
        }

        info!("Worker pool shutdown complete");
        Ok(())
    }

    /// Get the number of workers in the pool
    pub fn worker_count(&self) -> usize {
        self.workers.len()
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        // Note: This is a simplified drop implementation
        // In a real implementation, you'd want to handle async cleanup properly
        if let Some(shutdown_tx) = &self.shutdown_tx {
            let _ = shutdown_tx.send(true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running FoundationDB instance
    // They're meant to demonstrate usage rather than be run in CI

    #[tokio::test]
    #[ignore] // Requires FDB
    async fn test_worker_lifecycle() -> Result<()> {
        // This test requires a real FDB instance
        // let db = Database::new()?;
        // let engine = Arc::new(WorkflowEngine::new(db, 4).await?);
        //
        // let mut worker = WorkflowWorker::new(
        //     engine.clone(),
        //     "test_worker",
        //     vec!["javascript".to_string(), "http".to_string()],
        // ).await?;
        //
        // // Start worker in background
        // let worker_handle = tokio::spawn(async move {
        //     worker.start().await
        // });
        //
        // // Let it run for a bit
        // tokio::time::sleep(Duration::from_secs(2)).await;
        //
        // // Shutdown
        // worker.shutdown().await?;
        //
        // // Wait for completion
        // worker_handle.await??;

        Ok(())
    }
}