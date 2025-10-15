use axum::Router;
use connectare::prelude::RpcRouterExt;

use super::{hello::types::{HelloRequest, HelloResponse, HelloWorldService}, Error};

pub mod types {
    include!(concat!(env!("OUT_DIR"), "/hello.rs"));
}

pub fn add_routes(router: Router) -> Router {
    router.rpc(HelloWorldService::say_hello(say_hello_unary))
}

async fn say_hello_unary(request: HelloRequest) -> Result<HelloResponse, Error> {
    println!("Request: {:?}", request);
    Ok(HelloResponse {
        message: format!(
            "Hello {}!",
            request.name
        ),
    })
}
