//! RPC server for worker communication

use crate::engine::WorkflowEngine;
use crate::error::Result;
use crate::types::{RuntimeType, WorkerHealthStatus, WorkerInfo, WorkerId, WorkerStats};
use axum::{extract::State, routing::post, Router};
use chrono::Utc;
use std::net::SocketAddr;
use std::sync::Arc;

// Include generated protobuf code
include!(concat!(env!("OUT_DIR"), "/workflow.rs"));

/// Run the RPC server
pub async fn run_server(engine: Arc<WorkflowEngine>, bind_addr: SocketAddr) -> Result<()> {
    let app = Router::new()
        .route(
            "/workflow.WorkflowService/RegisterWorker",
            post(register_worker_handler),
        )
        .route(
            "/workflow.WorkflowService/PollTask",
            post(poll_task_handler),
        )
        .route(
            "/workflow.WorkflowService/CompleteTask",
            post(complete_task_handler),
        )
        .route(
            "/workflow.WorkflowService/Heartbeat",
            post(heartbeat_handler),
        )
        .with_state(engine);

    let listener = tokio::net::TcpListener::bind(bind_addr).await
        .map_err(|e| crate::error::EngineError::Internal(format!("Failed to bind: {}", e)))?;

    tracing::info!("🚀 Workflow engine server started on {}", bind_addr);

    axum::serve(listener, app).await
        .map_err(|e| crate::error::EngineError::Internal(format!("Server error: {}", e)))?;

    Ok(())
}

async fn register_worker_handler(
    State(engine): State<Arc<WorkflowEngine>>,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use prost::Message;
    
    let request = match RegisterWorkerRequest::decode(body) {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("Failed to decode register request: {}", e);
            return axum::response::Response::builder()
                .status(400)
                .body(axum::body::Body::from("Invalid request"))
                .unwrap();
        }
    };

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
        let response = RegisterWorkerResponse {
            success: false,
            message: format!("Failed to register: {}", e),
        };
        let mut buf = bytes::BytesMut::new();
        response.encode(&mut buf).unwrap();
        return axum::response::Response::builder()
            .status(500)
            .body(axum::body::Body::from(buf.freeze()))
            .unwrap();
    }

    tracing::info!("Registered worker: {}", worker_id);

    let response = RegisterWorkerResponse {
        success: true,
        message: "Worker registered successfully".to_string(),
    };

    let mut buf = bytes::BytesMut::new();
    response.encode(&mut buf).unwrap();
    axum::response::Response::builder()
        .status(200)
        .body(axum::body::Body::from(buf.freeze()))
        .unwrap()
}

async fn poll_task_handler(
    State(engine): State<Arc<WorkflowEngine>>,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use prost::Message;
    
    let request = match PollTaskRequest::decode(body) {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("Failed to decode poll request: {}", e);
            return axum::response::Response::builder()
                .status(400)
                .body(axum::body::Body::from("Invalid request"))
                .unwrap();
        }
    };

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

            let response = PollTaskResponse {
                task: Some(payload),
                no_task_reason: None,
            };

            let mut buf = bytes::BytesMut::new();
            response.encode(&mut buf).unwrap();
            axum::response::Response::builder()
                .status(200)
                .body(axum::body::Body::from(buf.freeze()))
                .unwrap()
        }
        Ok(None) => {
            let response = PollTaskResponse {
                task: None,
                no_task_reason: Some("no_pending_tasks".to_string()),
            };

            let mut buf = bytes::BytesMut::new();
            response.encode(&mut buf).unwrap();
            axum::response::Response::builder()
                .status(200)
                .body(axum::body::Body::from(buf.freeze()))
                .unwrap()
        }
        Err(e) => {
            tracing::error!("Failed to dequeue task: {}", e);
            let response = PollTaskResponse {
                task: None,
                no_task_reason: Some(format!("error: {}", e)),
            };

            let mut buf = bytes::BytesMut::new();
            response.encode(&mut buf).unwrap();
            axum::response::Response::builder()
                .status(500)
                .body(axum::body::Body::from(buf.freeze()))
                .unwrap()
        }
    }
}

async fn complete_task_handler(
    State(engine): State<Arc<WorkflowEngine>>,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use prost::Message;
    
    let request = match CompleteTaskRequest::decode(body) {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("Failed to decode complete request: {}", e);
            return axum::response::Response::builder()
                .status(400)
                .body(axum::body::Body::from("Invalid request"))
                .unwrap();
        }
    };

    let task_id = match uuid::Uuid::parse_str(&request.task_id) {
        Ok(id) => crate::types::TaskId::from_uuid(id),
        Err(e) => {
            tracing::error!("Invalid task ID: {}", e);
            return axum::response::Response::builder()
                .status(400)
                .body(axum::body::Body::from("Invalid task ID"))
                .unwrap();
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
        let response = CompleteTaskResponse {
            acknowledged: false,
        };

        let mut buf = bytes::BytesMut::new();
        response.encode(&mut buf).unwrap();
        return axum::response::Response::builder()
            .status(500)
            .body(axum::body::Body::from(buf.freeze()))
            .unwrap();
    }

    tracing::info!("Task {} completed", task_id);

    let response = CompleteTaskResponse {
        acknowledged: true,
    };

    let mut buf = bytes::BytesMut::new();
    response.encode(&mut buf).unwrap();
    axum::response::Response::builder()
        .status(200)
        .body(axum::body::Body::from(buf.freeze()))
        .unwrap()
}

async fn heartbeat_handler(
    State(engine): State<Arc<WorkflowEngine>>,
    body: axum::body::Bytes,
) -> axum::response::Response {
    use prost::Message;
    
    let request = match HeartbeatRequest::decode(body) {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("Failed to decode heartbeat request: {}", e);
            return axum::response::Response::builder()
                .status(400)
                .body(axum::body::Body::from("Invalid request"))
                .unwrap();
        }
    };

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

    let response = HeartbeatResponse {
        active: true,
        message: Some("Heartbeat received".to_string()),
    };

    let mut buf = bytes::BytesMut::new();
    response.encode(&mut buf).unwrap();
    axum::response::Response::builder()
        .status(200)
        .body(axum::body::Body::from(buf.freeze()))
        .unwrap()
}

