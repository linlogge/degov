use axum::Router;
use std::sync::Arc;
use crate::WorkflowService;

mod error;
mod engine;
pub mod hello;

pub use error::Error;

pub async fn add_api_routes(
    mut router: Router,
    workflow_service: Arc<WorkflowService>,
) -> Router {
    // Create RPC router with engine routes (stateful)
    let engine_router = engine::add_routes(Router::<Arc<WorkflowService>>::new())
        .with_state(workflow_service);
    
    // Create hello routes (stateless)
    let hello_router = hello::add_routes(Router::new());
    
    // Nest both under /rpc
    router = router
        .nest("/rpc", engine_router)
        .merge(Router::new().nest("/rpc", hello_router));

    router
}
