use axum::Router;

mod error;
pub mod hello;

pub use error::Error;

pub async fn add_api_routes(mut router: Router) -> Router {
    let mut rpc_router = Router::new();
    rpc_router = hello::add_routes(rpc_router);

    router = router.nest("/rpc", rpc_router);

    router
}
