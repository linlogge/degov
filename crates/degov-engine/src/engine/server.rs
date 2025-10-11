//! RPC server for worker communication

use crate::engine::WorkflowEngine;
use crate::error::Result;
use crate::types::{RuntimeType, WorkerHealthStatus, WorkerInfo, WorkerId, WorkerStats};
use axum::Router;
use chrono::Utc;
use degov_rpc::prelude::*;
use std::net::SocketAddr;
use std::sync::Arc;

// Include generated protobuf code
mod proto {
    include!(concat!(env!("OUT_DIR"), "/workflow.rs"));
}

use proto::*;

/// Run the RPC server using the generated service handlers
pub async fn run_server(engine: Arc<WorkflowEngine>, bind_addr: SocketAddr) -> Result<()> {
    // Use the generated RPC service methods
    let app = Router::new()
        .rpc(WorkflowService::register_worker(register_worker_handler))
        .rpc(WorkflowService::poll_task(poll_task_handler))
        .rpc(WorkflowService::complete_task(complete_task_handler))
        .rpc(WorkflowService::heartbeat(heartbeat_handler))
        .with_state(engine);

    let listener = tokio::net::TcpListener::bind(bind_addr).await
        .map_err(|e| crate::error::EngineError::Internal(format!("Failed to bind: {}", e)))?;

    tracing::info!("🚀 Workflow engine server started on {}", bind_addr);

    axum::serve(listener, app).await
        .map_err(|e| crate::error::EngineError::Internal(format!("Server error: {}", e)))?;

    Ok(())
}

async fn register_worker_handler(
    axum::extract::State(engine): axum::extract::State<Arc<WorkflowEngine>>,
    request: RegisterWorkerRequest,
) -> RegisterWorkerResponse {

    let worker_id = WorkerId::from_string(request.worker_id.clone());
    let capabilities: Vec<RuntimeType> = request
        .capabilities
        .iter()
        .filter_map(|c| match c.as_str() {
            "javascript" => Some(RuntimeType::JavaScript),
            "wasm" => Some(RuntimeType::Wasm),
            _ => None,
        })
        .collect();

    let worker = WorkerInfo {
        id: worker_id.clone(),
        capabilities,
        hostname: request.hostname,
        registered_at: Utc::now(),
        last_heartbeat: Utc::now(),
        status: WorkerHealthStatus::Healthy,
        stats: WorkerStats::default(),
    };

    // Register in scheduler
    engine.scheduler().register_worker(worker.clone());

    // Persist to database
    if let Err(e) = engine.persistence().workers().register(worker).await {
        tracing::error!("Failed to persist worker: {}", e);
        return RegisterWorkerResponse {
            success: false,
            message: format!("Failed to register: {}", e),
        };
    }

    tracing::info!("Registered worker: {}", worker_id);

    RegisterWorkerResponse {
        success: true,
        message: "Worker registered successfully".to_string(),
    }
}

async fn poll_task_handler(
    axum::extract::State(engine): axum::extract::State<Arc<WorkflowEngine>>,
    request: PollTaskRequest,
) -> PollTaskResponse {
    let worker_id = WorkerId::from_string(request.worker_id);

    // Try to dequeue a task
    match engine.persistence().tasks().dequeue(&worker_id).await {
        Ok(Some(task)) => {
            let payload = TaskPayload {
                task_id: task.id.to_string(),
                workflow_id: task.workflow_id.to_string(),
                task_type: task.definition.runtime_type.as_str().to_string(),
                code: task.definition.code,
                input: task.input,
                timeout_ms: task.definition.timeout_ms as i64,
                metadata: std::collections::HashMap::new(),
            };

            PollTaskResponse {
                task: Some(payload),
                no_task_reason: None,
            }
        }
        Ok(None) => {
            PollTaskResponse {
                task: None,
                no_task_reason: Some("no_pending_tasks".to_string()),
            }
        }
        Err(e) => {
            tracing::error!("Failed to dequeue task: {}", e);
            PollTaskResponse {
                task: None,
                no_task_reason: Some(format!("error: {}", e)),
            }
        }
    }
}

async fn complete_task_handler(
    axum::extract::State(engine): axum::extract::State<Arc<WorkflowEngine>>,
    request: CompleteTaskRequest,
) -> CompleteTaskResponse {
    let task_id = match uuid::Uuid::parse_str(&request.task_id) {
        Ok(id) => crate::types::TaskId::from_uuid(id),
        Err(e) => {
            tracing::error!("Invalid task ID: {}", e);
            return CompleteTaskResponse {
                acknowledged: false,
            };
        }
    };

    let result_proto = request.result.unwrap_or_default();
    let result = crate::types::TaskResult {
        success: result_proto.success,
        output: result_proto.output,
        error: result_proto.error,
        execution_time_ms: result_proto.execution_time_ms.max(0) as u64,
    };

    if let Err(e) = engine.persistence().tasks().complete(&task_id, result).await {
        tracing::error!("Failed to complete task: {}", e);
        return CompleteTaskResponse {
            acknowledged: false,
        };
    }

    tracing::info!("Task {} completed", task_id);

    CompleteTaskResponse {
        acknowledged: true,
    }
}

async fn heartbeat_handler(
    axum::extract::State(engine): axum::extract::State<Arc<WorkflowEngine>>,
    request: HeartbeatRequest,
) -> HeartbeatResponse {
    let worker_id = WorkerId::from_string(request.worker_id.clone());

    // Update heartbeat in persistence
    if let Err(e) = engine.persistence().workers().heartbeat(&worker_id).await {
        tracing::error!("Failed to update heartbeat: {}", e);
    }

    // Update stats in scheduler
    if let Some(status) = request.status {
        engine.scheduler().update_worker_stats(
            &worker_id,
            status.active_tasks as u32,
            status.total_tasks_completed as u64,
            status.total_tasks_failed as u64,
        );
    }

    HeartbeatResponse {
        active: true,
        message: Some("Heartbeat received".to_string()),
    }
}
