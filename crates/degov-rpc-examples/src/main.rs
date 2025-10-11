//!
//! $ cargo run -p axum-connect-example
//! Head over to Buf Studio to test out RPCs, with auto-completion!
//! https://buf.build/studio/athilenius/axum-connect/main/hello.HelloWorldService/SayHello?target=http%3A%2F%2Flocalhost%3A3030
//!

use async_stream::stream;
use axum::Router;
use degov_rpc::prelude::*;
use axum_extra::extract::Host;
use error::Error;
use proto::hello::*;
use tower_http::cors::CorsLayer;

// Take a peak at error.rs to see how errors work in axum-connect.
mod error;

mod proto {
    // Include the generated code in a `proto` module.
    //
    // Note: I'm not super happy with this pattern. I hope to add support to `protoc-gen-prost` in
    // the near-ish future instead see:
    // https://github.com/neoeinstein/protoc-gen-prost/issues/82#issuecomment-1877107220 That will
    // better align with Buf.build's philosophy. This is how it works for now though.
    pub mod hello {
        include!(concat!(env!("OUT_DIR"), "/hello.rs"));
    }
}

#[tokio::main]
async fn main() {
    // Build our application with a route. Note the `rpc` method which was added by `axum-connect`.
    // It expect a service method handler, wrapped in it's respective type. The handler (below) is
    // just a normal Rust function. Just like Axum, it also supports extractors!
    let app = Router::new()
        // A standard unary (POST based) Connect-Web request handler.
        .rpc(HelloWorldService::say_hello(say_hello_unary))
        // A GET version of the same thing, which has well-defined semantics for caching.
        .rpc(HelloWorldService::say_hello_unary_get(say_hello_unary));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3030")
        .await
        .unwrap();
    println!("listening on http://{:?}", listener.local_addr().unwrap());

    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        let client = RpcClient::new(RpcClientConfig::new("http://127.0.0.1:3030").unwrap());
        let client = HelloWorldServiceClient::new(client);

        let request = HelloRequest {
            name: Some("World".to_string()),
        };
        let response = client.say_hello(request).await.unwrap();
        println!("response: {:?}", response);
    });
    
    axum::serve(listener, app.layer(CorsLayer::very_permissive()))
        .await
        .unwrap();
}

/// The bread-and-butter of Connect-Web, a Unary request handler.
///
/// Just to demo error handling, I've chose to return a `Result` here. If your method is
/// infallible, you could just as easily return a `HellResponse` directly. The error type I'm using
/// is defined in `error.rs` and is worth taking a quick peak at.
///
/// Like Axum, both the request AND response types just need to implement RpcFromRequestParts` and
/// `RpcIntoResponse` respectively. This allows for a ton of flexibility in what your handlers
/// actually accept/return. This is a concept very core to Axum, so I won't go too deep into the
/// ideology here.
async fn say_hello_unary(Host(host): Host, request: HelloRequest) -> Result<HelloResponse, Error> {
    Ok(HelloResponse {
        message: format!(
            "Hello {}! You're addressing the hostname: {}.",
            request.name.unwrap_or_else(|| "unnamed".to_string()),
            host
        ),
    })
}
