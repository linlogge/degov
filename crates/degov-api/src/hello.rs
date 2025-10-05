use axum::Router;
use degov_rpc::{prelude::*};
use proto::hello::*;
use crate::error::Error;

pub mod proto {
    pub mod hello {
        include!(concat!(env!("OUT_DIR"), "/hello.rs"));
    }
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
