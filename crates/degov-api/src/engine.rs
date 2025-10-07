use axum::Router;
use degov_rpc::prelude::RpcRouterExt;

use crate::Error;

pub mod types {
    include!(concat!(env!("OUT_DIR"), "/engine.rs"));
}

pub fn add_routes(router: Router) -> Router {
    router.rpc(types::EngineService::list_workflows(list_workflows_unary))
}

async fn list_workflows_unary(request: types::ListWorkflowsRequest) -> Result<types::ListWorkflowsResponse, Error> {
    Ok(types::ListWorkflowsResponse { workflows: vec![] })
}
