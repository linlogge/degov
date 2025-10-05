use axum::Router;
use connect_rpc::prelude::*;
use connect_rpc::server::handler::{RpcHandlerStream, RpcHandlerUnary};
use futures::{stream, StreamExt};
use prost::Message;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tokio::time::{sleep, Duration};

// Shared message types
#[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
pub struct GreetRequest {
    #[prost(string, tag = "1")]
    pub name: String,
}

#[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
pub struct GreetResponse {
    #[prost(string, tag = "1")]
    pub message: String,
}

#[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
pub struct CountRequest {
    #[prost(int32, tag = "1")]
    pub count: i32,
}

#[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
pub struct CountResponse {
    #[prost(int32, tag = "1")]
    pub number: i32,
}

// Server handlers
async fn greet(request: GreetRequest) -> GreetResponse {
    GreetResponse {
        message: format!("Hello, {}!", request.name),
    }
}

async fn count_stream(
    request: CountRequest,
) -> impl futures::Stream<Item = CountResponse> + Send {
    let count = request.count.max(1).min(10);
    
    stream::iter(1..=count).then(move |i| async move {
        sleep(Duration::from_millis(200)).await;
        CountResponse { number: i }
    })
}

async fn run_server() {
    let app = Router::new()
        .route(
            "/greet.v1.GreetService/Greet",
            axum::routing::post(|req: axum::http::Request<axum::body::Body>| async move {
                greet.call(req, ()).await
            })
            .get(|req: axum::http::Request<axum::body::Body>| async move {
                greet.call(req, ()).await
            }),
        )
        .route(
            "/greet.v1.GreetService/Count",
            axum::routing::post(|req: axum::http::Request<axum::body::Body>| async move {
                count_stream.call(req, ()).await
            }),
        );

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    println!("🚀 Server started on http://localhost:3000\n");
    
    axum::serve(listener, app).await.unwrap();
}

async fn run_client() {
    // Wait for server to start
    sleep(Duration::from_millis(500)).await;
    
    println!("📡 Client connecting to server...\n");
    
    // Create client
    let config = RpcClientConfig::new("http://localhost:3000")
        .unwrap()
        .with_binary(false);
    let client = RpcClient::new(config);
    
    // Example 1: Unary POST request
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📤 Example 1: Unary POST Request");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let request = GreetRequest {
        name: "Alice".to_string(),
    };
    
    match client
        .unary::<GreetRequest, GreetResponse>("/greet.v1.GreetService/Greet", request)
        .await
    {
        Ok(response) => {
            println!("✅ Response: {}\n", response.message);
        }
        Err(e) => {
            eprintln!("❌ Error: {:?}\n", e);
        }
    }
    
    // Example 2: Unary GET request
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📤 Example 2: Unary GET Request");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let request = GreetRequest {
        name: "Bob".to_string(),
    };
    
    match client
        .unary_get::<GreetRequest, GreetResponse>("/greet.v1.GreetService/Greet", request)
        .await
    {
        Ok(response) => {
            println!("✅ Response: {}\n", response.message);
        }
        Err(e) => {
            eprintln!("❌ Error: {:?}\n", e);
        }
    }
    
    // Example 3: Server streaming
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📤 Example 3: Server Streaming");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let request = CountRequest { count: 5 };
    
    match client
        .server_stream::<CountRequest, CountResponse>("/greet.v1.GreetService/Count", request)
        .await
    {
        Ok(mut stream) => {
            println!("📥 Receiving stream:");
            while let Some(result) = stream.next().await {
                match result {
                    Ok(response) => {
                        println!("  └─ Count: {}", response.number);
                    }
                    Err(e) => {
                        eprintln!("  └─ ❌ Stream error: {:?}", e);
                        break;
                    }
                }
            }
            println!("✅ Stream completed\n");
        }
        Err(e) => {
            eprintln!("❌ Failed to start stream: {:?}\n", e);
        }
    }
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ All examples completed!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 Connect-RPC Simple Client-Server Example\n");
    
    // Spawn server in background
    tokio::spawn(async {
        run_server().await;
    });
    
    // Run client
    run_client().await;
    
    // Give a moment before exiting
    sleep(Duration::from_secs(1)).await;
    
    Ok(())
}
