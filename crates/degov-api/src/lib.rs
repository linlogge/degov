use axum::Router;

mod error;
pub mod hello;

pub use error::Error;

pub async fn add_api_routes(mut router: Router) -> Router {
    router = hello::add_routes(router);

    router
}
