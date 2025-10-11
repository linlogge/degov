use axum::{extract::State, Router};
use degov_server::{WorkflowService, ServerError};
use degov_rpc::prelude::RpcRouterExt;
use std::sync::Arc;

use crate::Error;

pub mod types {
    include!(concat!(env!("OUT_DIR"), "/engine.rs"));
}

pub fn add_routes(router: Router<Arc<WorkflowService>>) -> Router<Arc<WorkflowService>> {
    router
        .rpc(types::EngineService::start_workflow(start_workflow))
        .rpc(types::EngineService::get_workflow_instance(get_workflow_instance))
        .rpc(types::EngineService::list_workflow_instances(list_workflow_instances))
        .rpc(types::EngineService::trigger_transition(trigger_transition))
        .rpc(types::EngineService::cancel_workflow(cancel_workflow))
        .rpc(types::EngineService::list_workers(list_workers))
        .rpc(types::EngineService::get_worker_status(get_worker_status))
        .rpc(types::EngineService::get_task_status(get_task_status))
}

/// Start a workflow instance
async fn start_workflow(
    State(service): State<Arc<WorkflowService>>,
    request: types::StartWorkflowRequest,
) -> Result<types::StartWorkflowResponse, Error> {
    let instance = service
        .start_workflow(&request.workflow_definition_id, &request.input_json)
        .await
        .map_err(map_server_error)?;

    Ok(types::StartWorkflowResponse {
        instance_id: instance.id.to_string(),
        current_state: instance.current_state,
        status: format!("{:?}", instance.status),
    })
}

/// Get workflow instance details
async fn get_workflow_instance(
    State(service): State<Arc<WorkflowService>>,
    request: types::GetWorkflowInstanceRequest,
) -> Result<types::GetWorkflowInstanceResponse, Error> {
    let instance = service
        .get_workflow_instance(&request.instance_id)
        .await
        .map_err(map_server_error)?;

    Ok(types::GetWorkflowInstanceResponse {
        instance: Some(types::WorkflowInstance {
            id: instance.id.to_string(),
            definition_id: instance.definition_id.to_string(),
            current_state: instance.current_state,
            status: format!("{:?}", instance.status),
            context_json: instance.context.to_string(),
            created_at: instance.created_at.to_rfc3339(),
            updated_at: instance.updated_at.to_rfc3339(),
            completed_at: instance.completed_at.map(|t| t.to_rfc3339()),
        }),
    })
}

/// List workflow instances
async fn list_workflow_instances(
    State(_service): State<Arc<WorkflowService>>,
    _request: types::ListWorkflowInstancesRequest,
) -> Result<types::ListWorkflowInstancesResponse, Error> {
    // TODO: Implement actual listing from database
    // For now, return empty list
    Ok(types::ListWorkflowInstancesResponse {
        instances: vec![],
        total_count: 0,
    })
}

/// Trigger a workflow state transition
async fn trigger_transition(
    State(service): State<Arc<WorkflowService>>,
    request: types::TriggerTransitionRequest,
) -> Result<types::TriggerTransitionResponse, Error> {
    match service.trigger_transition(&request.instance_id, &request.event).await {
        Ok(new_state) => Ok(types::TriggerTransitionResponse {
            success: true,
            new_state,
            error: None,
        }),
        Err(e) => Ok(types::TriggerTransitionResponse {
            success: false,
            new_state: String::new(),
            error: Some(format!("Transition failed: {}", e)),
        }),
    }
}

/// Cancel a workflow
async fn cancel_workflow(
    State(service): State<Arc<WorkflowService>>,
    request: types::CancelWorkflowRequest,
) -> Result<types::CancelWorkflowResponse, Error> {
    match service.cancel_workflow(&request.instance_id).await {
        Ok(_) => Ok(types::CancelWorkflowResponse {
            success: true,
            error: None,
        }),
        Err(e) => Ok(types::CancelWorkflowResponse {
            success: false,
            error: Some(format!("Failed to cancel: {}", e)),
        }),
    }
}

/// List all workers
async fn list_workers(
    State(service): State<Arc<WorkflowService>>,
    _request: types::ListWorkersRequest,
) -> Result<types::ListWorkersResponse, Error> {
    let workers = service.list_workers().await.map_err(map_server_error)?;

    let workers_proto = workers
        .into_iter()
        .map(|w| types::Worker {
            id: w.id.to_string(),
            capabilities: w.capabilities.iter().map(|c| c.as_str().to_string()).collect(),
            hostname: w.hostname,
            status: format!("{:?}", w.status),
            stats: Some(types::WorkerStats {
                active_tasks: w.stats.active_tasks as i32,
                total_tasks_completed: w.stats.total_tasks_completed.to_string(),
                total_tasks_failed: w.stats.total_tasks_failed.to_string(),
            }),
            registered_at: w.registered_at.to_rfc3339(),
            last_heartbeat: w.last_heartbeat.to_rfc3339(),
        })
        .collect();

    Ok(types::ListWorkersResponse {
        workers: workers_proto,
    })
}

/// Get worker status
async fn get_worker_status(
    State(service): State<Arc<WorkflowService>>,
    request: types::GetWorkerStatusRequest,
) -> Result<types::GetWorkerStatusResponse, Error> {
    let worker = service
        .get_worker_status(&request.worker_id)
        .await
        .map_err(map_server_error)?;

    Ok(types::GetWorkerStatusResponse {
        worker: Some(types::Worker {
            id: worker.id.to_string(),
            capabilities: worker.capabilities.iter().map(|c| c.as_str().to_string()).collect(),
            hostname: worker.hostname,
            status: format!("{:?}", worker.status),
            stats: Some(types::WorkerStats {
                active_tasks: worker.stats.active_tasks as i32,
                total_tasks_completed: worker.stats.total_tasks_completed.to_string(),
                total_tasks_failed: worker.stats.total_tasks_failed.to_string(),
            }),
            registered_at: worker.registered_at.to_rfc3339(),
            last_heartbeat: worker.last_heartbeat.to_rfc3339(),
        }),
    })
}

/// Get task status
async fn get_task_status(
    State(service): State<Arc<WorkflowService>>,
    request: types::GetTaskStatusRequest,
) -> Result<types::GetTaskStatusResponse, Error> {
    let task = service
        .get_task_status(&request.task_id)
        .await
        .map_err(map_server_error)?;

    Ok(types::GetTaskStatusResponse {
        task: Some(types::Task {
            id: task.id.to_string(),
            workflow_id: task.workflow_id.to_string(),
            status: format!("{:?}", task.status),
            assigned_worker: task.assigned_worker.map(|w| w.to_string()),
            attempt: task.attempt as i32,
            created_at: task.created_at.to_rfc3339(),
            started_at: task.started_at.map(|t| t.to_rfc3339()),
            completed_at: task.completed_at.map(|t| t.to_rfc3339()),
            result: task.result.map(|r| types::TaskResult {
                success: r.success,
                output: r.output,
                error: r.error,
                execution_time_ms: r.execution_time_ms.to_string(),
            }),
        }),
    })
}

/// Map server errors to API errors
fn map_server_error(err: ServerError) -> Error {
    match err {
        ServerError::InvalidInput(msg) => Error::BadRequest(msg),
        ServerError::PermissionDenied => Error::Forbidden,
        ServerError::NotFound(msg) => Error::NotFound(msg),
        ServerError::EngineError(msg) => Error::Internal(msg),
        ServerError::Internal(msg) => Error::Internal(msg),
    }
}
