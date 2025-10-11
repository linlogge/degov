//! Worker implementation

mod executor;

pub use executor::TaskExecutor;

use crate::error::{EngineError, Result};
use crate::runtime::{JavaScriptRuntime, WasmRuntime};
use crate::types::{RuntimeType, WorkerId, WorkerStats};
use degov_rpc::client::{RpcClient, RpcClientConfig};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::{interval, sleep};

// Import the generated proto code
mod proto {
    include!(concat!(env!("OUT_DIR"), "/workflow.rs"));
}

use proto::*;

/// Worker that executes tasks
pub struct Worker {
    id: WorkerId,
    rpc_client: WorkflowServiceClient,
    executor: TaskExecutor,
    poll_interval: Duration,
    heartbeat_interval: Duration,
    hostname: String,
    stats: Arc<parking_lot::RwLock<WorkerStats>>,
}

impl Worker {
    /// Create a new worker
    pub async fn new(engine_url: &str) -> Result<Self> {
        let client_config = RpcClientConfig::new(engine_url)
            .map_err(|e| EngineError::Internal(format!("Failed to create RPC config: {}", e)))?;
        let rpc_client = WorkflowServiceClient::new(RpcClient::new(client_config));

        let mut executor = TaskExecutor::new();
        executor.register_runtime(RuntimeType::JavaScript, Box::new(JavaScriptRuntime::new()));
        executor.register_runtime(
            RuntimeType::Wasm,
            Box::new(WasmRuntime::new().map_err(|e| EngineError::Runtime(e))?),
        );

        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(Self {
            id: WorkerId::new(),
            rpc_client,
            executor,
            poll_interval: Duration::from_millis(500),
            heartbeat_interval: Duration::from_secs(10),
            hostname,
            stats: Arc::new(parking_lot::RwLock::new(WorkerStats::default())),
        })
    }

    /// Get the worker ID
    pub fn id(&self) -> &WorkerId {
        &self.id
    }

    /// Set poll interval
    pub fn with_poll_interval(mut self, duration: Duration) -> Self {
        self.poll_interval = duration;
        self
    }

    /// Set heartbeat interval
    pub fn with_heartbeat_interval(mut self, duration: Duration) -> Self {
        self.heartbeat_interval = duration;
        self
    }

    /// Run the worker
    pub async fn run(&self) -> Result<()> {
        // Register with engine
        self.register().await?;

        tracing::info!("Worker {} started", self.id);

        // Create shutdown channel
        let (shutdown_tx, mut shutdown_rx) = broadcast::channel::<()>(1);

        // Spawn heartbeat task
        let heartbeat_handle = {
            let worker = self.clone_for_heartbeat();
            let mut heartbeat_shutdown = shutdown_tx.subscribe();
            tokio::spawn(async move {
                tokio::select! {
                    _ = worker.heartbeat_loop() => {},
                    _ = heartbeat_shutdown.recv() => {
                        tracing::info!("Heartbeat loop shutting down");
                    }
                }
            })
        };

        // Spawn shutdown signal handler
        let shutdown_handle = tokio::spawn(async move {
            wait_for_shutdown_signal().await;
            let _ = shutdown_tx.send(());
        });

        // Main polling loop
        let mut poll_timer = interval(self.poll_interval);
        let mut graceful_shutdown = false;
        
        loop {
            tokio::select! {
                _ = poll_timer.tick() => {
                    if graceful_shutdown {
                        break;
                    }
                    
                    match self.poll_and_execute().await {
                        Ok(true) => {
                            // Task executed, continue immediately
                        }
                        Ok(false) => {
                            // No task available
                        }
                        Err(e) => {
                            tracing::error!("Error polling/executing task: {}", e);
                            sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    tracing::info!("Shutdown signal received");
                    graceful_shutdown = true;
                    
                    // Check if there's an active task
                    let active_tasks = self.stats.read().active_tasks;
                    if active_tasks > 0 {
                        tracing::info!("Waiting for {} active task(s) to complete...", active_tasks);
                        // Continue the loop to finish the current task
                    } else {
                        break;
                    }
                }
            }
            
            // If shutting down and no active tasks, exit
            if graceful_shutdown && self.stats.read().active_tasks == 0 {
                break;
            }
        }

        tracing::info!("Worker shutting down gracefully...");
        
        // Abort heartbeat task
        heartbeat_handle.abort();
        let _ = shutdown_handle.await;

        tracing::info!("Worker {} stopped", self.id);
        
        Ok(())
    }

    /// Register worker with engine
    async fn register(&self) -> Result<()> {
        let capabilities = self.executor.supported_runtimes()
            .iter()
            .map(|rt| rt.as_str().to_string())
            .collect();

        let request = RegisterWorkerRequest {
            worker_id: self.id.to_string(),
            capabilities,
            hostname: self.hostname.clone(),
        };

        let response = self
            .rpc_client
            .register_worker(request)
            .await
            .map_err(|e| EngineError::Internal(format!("Registration failed: {}", e)))?;

        if !response.success {
            return Err(EngineError::Internal(format!(
                "Registration failed: {}",
                response.message
            )));
        }

        tracing::info!("Worker registered successfully");
        Ok(())
    }

    /// Poll for a task and execute it
    async fn poll_and_execute(&self) -> Result<bool> {
        let request = PollTaskRequest {
            worker_id: self.id.to_string(),
        };

        let response = self
            .rpc_client
            .poll_task(request)
            .await
            .map_err(|e| EngineError::Internal(format!("Poll failed: {}", e)))?;

        match response.task {
            Some(task_payload) => {
                tracing::info!("Received task: {}", task_payload.task_id);
                
                // Increment active tasks
                {
                    let mut stats = self.stats.write();
                    stats.active_tasks += 1;
                }

                // Execute task
                let result = self.execute_task(task_payload).await;

                // Update stats
                {
                    let mut stats = self.stats.write();
                    stats.active_tasks = stats.active_tasks.saturating_sub(1);
                    if result.result.success {
                        stats.total_tasks_completed += 1;
                    } else {
                        stats.total_tasks_failed += 1;
                    }
                }

                // Report completion
                self.report_completion(&result.task_id, result.result).await?;

                Ok(true)
            }
            None => Ok(false),
        }
    }

    /// Execute a task
    async fn execute_task(&self, payload: TaskPayload) -> TaskExecutionResult {
        let start = std::time::Instant::now();

        let runtime_type = match payload.task_type.as_str() {
            "javascript" => RuntimeType::JavaScript,
            "wasm" => RuntimeType::Wasm,
            _ => {
                return TaskExecutionResult {
                    task_id: payload.task_id,
                    result: TaskResult {
                        success: false,
                        output: Vec::new(),
                        error: Some(format!("Unknown runtime type: {}", payload.task_type)),
                        execution_time_ms: 0,
                    },
                };
            }
        };

        let task_def = crate::types::TaskDefinition {
            name: "task".to_string(),
            runtime_type,
            code: payload.code,
            timeout_ms: payload.timeout_ms as u64,
            retry_policy: None,
        };

        match self.executor.execute(&task_def, &payload.input).await {
            Ok(output) => TaskExecutionResult {
                    task_id: payload.task_id,
                    result: TaskResult {
                        success: true,
                        output,
                        error: None,
                        execution_time_ms: start.elapsed().as_millis() as i64,
                    },
                },
            Err(e) => TaskExecutionResult {
                task_id: payload.task_id,
                result: TaskResult {
                    success: false,
                    output: Vec::new(),
                    error: Some(e.to_string()),
                    execution_time_ms: start.elapsed().as_millis() as i64,
                },
            },
        }
    }

    /// Report task completion
    async fn report_completion(&self, task_id: &str, result: TaskResult) -> Result<()> {
        let request = CompleteTaskRequest {
            worker_id: self.id.to_string(),
            task_id: task_id.to_string(),
            result: Some(result),
        };

        let _response = self
            .rpc_client
            .complete_task(request)
            .await
            .map_err(|e| EngineError::Internal(format!("Complete task failed: {}", e)))?;

        tracing::info!("Task {} completion reported", task_id);
        Ok(())
    }

    /// Heartbeat loop
    async fn heartbeat_loop(&self) {
        let mut timer = interval(self.heartbeat_interval);
        loop {
            timer.tick().await;

            if let Err(e) = self.send_heartbeat().await {
                tracing::error!("Failed to send heartbeat: {}", e);
            }
        }
    }

    /// Send heartbeat
    async fn send_heartbeat(&self) -> Result<()> {
        let stats = self.stats.read().clone();

        let status = WorkerStatus {
            active_tasks: stats.active_tasks as i32,
            total_tasks_completed: stats.total_tasks_completed as i64,
            total_tasks_failed: stats.total_tasks_failed as i64,
        };

        let request = HeartbeatRequest {
            worker_id: self.id.to_string(),
            status: Some(status),
        };

        let _response = self
            .rpc_client
            .heartbeat(request)
            .await
            .map_err(|e| EngineError::Internal(format!("Heartbeat failed: {}", e)))?;

        Ok(())
    }

    /// Clone for heartbeat task
    fn clone_for_heartbeat(&self) -> Self {
        Self {
            id: self.id.clone(),
            rpc_client: self.rpc_client.clone(),
            executor: TaskExecutor::new(), // Empty executor for heartbeat
            poll_interval: self.poll_interval,
            heartbeat_interval: self.heartbeat_interval,
            hostname: self.hostname.clone(),
            stats: self.stats.clone(),
        }
    }
}

struct TaskExecutionResult {
    task_id: String,
    result: TaskResult,
}

async fn wait_for_shutdown_signal() {
    use tokio::signal;
    
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C signal");
        },
        _ = terminate => {
            tracing::info!("Received terminate signal");
        },
    }
}

