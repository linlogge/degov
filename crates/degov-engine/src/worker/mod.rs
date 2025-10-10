//! Worker implementation

mod executor;

pub use executor::TaskExecutor;

use crate::error::{EngineError, Result, RpcError};
use crate::runtime::{JavaScriptRuntime, WasmRuntime};
use crate::types::{RuntimeType, TaskResult, WorkerId, WorkerStats};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{interval, sleep};

// Include generated protobuf code
include!(concat!(env!("OUT_DIR"), "/workflow.rs"));

/// Worker that executes tasks
pub struct Worker {
    id: WorkerId,
    engine_url: String,
    rpc_client: reqwest::Client,
    executor: TaskExecutor,
    poll_interval: Duration,
    heartbeat_interval: Duration,
    hostname: String,
    stats: Arc<parking_lot::RwLock<WorkerStats>>,
}

impl Worker {
    /// Create a new worker
    pub async fn new(engine_url: &str) -> Result<Self> {
        let rpc_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| EngineError::Rpc(RpcError::Connection(format!("Failed to create client: {}", e))))?;

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
            engine_url: engine_url.to_string(),
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

        // Spawn heartbeat task
        let heartbeat_handle = {
            let worker = self.clone_for_heartbeat();
            tokio::spawn(async move {
                worker.heartbeat_loop().await;
            })
        };

        // Main polling loop
        let mut poll_timer = interval(self.poll_interval);
        loop {
            poll_timer.tick().await;

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

        // Note: This is unreachable but included for completeness
        #[allow(unreachable_code)]
        {
            let _handle = heartbeat_handle;
            Ok(())
        }
    }

    /// Register worker with engine
    async fn register(&self) -> Result<()> {
        use prost::Message;

        let capabilities = self.executor.supported_runtimes()
            .iter()
            .map(|rt| rt.as_str().to_string())
            .collect();

        let request = RegisterWorkerRequest {
            worker_id: self.id.to_string(),
            capabilities,
            hostname: self.hostname.clone(),
        };

        let mut buf = Vec::new();
        request.encode(&mut buf).map_err(|e| {
            EngineError::Rpc(RpcError::Protocol(format!("Failed to encode request: {}", e)))
        })?;

        let url = format!("{}/workflow.WorkflowService/RegisterWorker", self.engine_url);
        let response = self
            .rpc_client
            .post(&url)
            .header("Content-Type", "application/proto")
            .body(buf)
            .send()
            .await
            .map_err(|e| EngineError::Rpc(RpcError::Connection(format!("Request failed: {}", e))))?;

        let response_bytes = response.bytes().await
            .map_err(|e| EngineError::Rpc(RpcError::Connection(format!("Failed to read response: {}", e))))?;

        let response = RegisterWorkerResponse::decode(response_bytes.as_ref())
            .map_err(|e| EngineError::Rpc(RpcError::Protocol(format!("Failed to decode response: {}", e))))?;

        if !response.success {
            return Err(EngineError::Rpc(RpcError::Connection(
                response.message,
            )));
        }

        tracing::info!("Worker registered successfully");
        Ok(())
    }

    /// Poll for a task and execute it
    async fn poll_and_execute(&self) -> Result<bool> {
        use prost::Message;

        let request = PollTaskRequest {
            worker_id: self.id.to_string(),
        };

        let mut buf = Vec::new();
        request.encode(&mut buf).map_err(|e| {
            EngineError::Rpc(RpcError::Protocol(format!("Failed to encode request: {}", e)))
        })?;

        let url = format!("{}/workflow.WorkflowService/PollTask", self.engine_url);
        let response = self
            .rpc_client
            .post(&url)
            .header("Content-Type", "application/proto")
            .body(buf)
            .send()
            .await
            .map_err(|e| EngineError::Rpc(RpcError::Connection(format!("Request failed: {}", e))))?;

        let response_bytes = response.bytes().await
            .map_err(|e| EngineError::Rpc(RpcError::Connection(format!("Failed to read response: {}", e))))?;

        let response = PollTaskResponse::decode(response_bytes.as_ref())
            .map_err(|e| EngineError::Rpc(RpcError::Protocol(format!("Failed to decode response: {}", e))))?;

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
            Ok(output) =>                 TaskExecutionResult {
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
    async fn report_completion(&self, task_id: &str, result: crate::worker::TaskResult) -> Result<()> {
        use prost::Message;

        let result_proto = result;

        let request = CompleteTaskRequest {
            worker_id: self.id.to_string(),
            task_id: task_id.to_string(),
            result: Some(result_proto),
        };

        let mut buf = Vec::new();
        request.encode(&mut buf).map_err(|e| {
            EngineError::Rpc(RpcError::Protocol(format!("Failed to encode request: {}", e)))
        })?;

        let url = format!("{}/workflow.WorkflowService/CompleteTask", self.engine_url);
        let _response = self
            .rpc_client
            .post(&url)
            .header("Content-Type", "application/proto")
            .body(buf)
            .send()
            .await
            .map_err(|e| EngineError::Rpc(RpcError::Connection(format!("Request failed: {}", e))))?;

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
        use prost::Message;

        let stats = self.stats.read().clone();

        let status = crate::worker::WorkerStatus {
            active_tasks: stats.active_tasks as i32,
            total_tasks_completed: stats.total_tasks_completed as i64,
            total_tasks_failed: stats.total_tasks_failed as i64,
        };

        let request = HeartbeatRequest {
            worker_id: self.id.to_string(),
            status: Some(status),
        };

        let mut buf = Vec::new();
        request.encode(&mut buf).map_err(|e| {
            EngineError::Rpc(RpcError::Protocol(format!("Failed to encode request: {}", e)))
        })?;

        let url = format!("{}/workflow.WorkflowService/Heartbeat", self.engine_url);
        let _response = self
            .rpc_client
            .post(&url)
            .header("Content-Type", "application/proto")
            .body(buf)
            .send()
            .await
            .map_err(|e| EngineError::Rpc(RpcError::Connection(format!("Request failed: {}", e))))?;

        Ok(())
    }

    /// Clone for heartbeat task
    fn clone_for_heartbeat(&self) -> Self {
        Self {
            id: self.id.clone(),
            engine_url: self.engine_url.clone(),
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
    result: crate::worker::TaskResult,
}

