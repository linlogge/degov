use connect_rpc::prelude::*;
use degov_api::proto::hello::*;
use std::net::SocketAddr;
use tokio::time::{sleep, Duration};

// Server handler
async fn say_hello_handler(request: HelloRequest) -> HelloResponse {
    HelloResponse {
        message: format!("Hello, {}! (from generated service)", request.name),
    }
}

async fn run_server() {
    let app = axum::Router::new()
        .rpc(HelloWorldService::say_hello(say_hello_handler))
        .rpc(HelloWorldService::say_hello_unary_get(say_hello_handler));

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    
    println!("🚀 Server started on http://localhost:3001\n");
    
    axum::serve(listener, app).await.unwrap();
}

async fn run_client() {
    // Wait for server to start
    sleep(Duration::from_millis(500)).await;
    
    println!("📡 Client connecting to server...\n");
    
    // Create client using generated code
    let config = RpcClientConfig::new("http://localhost:3001")
        .unwrap()
        .with_binary(true);
    
    let client = HelloWorldServiceClient::from_config(config);
    
    // Example 1: Call using generated method
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📤 Using Generated Client");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let request = HelloRequest {
        name: "Generated Client User".to_string(),
    };
    
    match client.say_hello(request.clone()).await {
        Ok(response) => {
            println!("✅ Response: {}\n", response.message);
        }
        Err(e) => {
            eprintln!("❌ Error: {:?}\n", e);
        }
    }
    
    // Example 2: Call using GET method
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("📤 Using Generated Client (GET)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    match client.say_hello_get(request).await {
        Ok(response) => {
            println!("✅ Response: {}\n", response.message);
        }
        Err(e) => {
            eprintln!("❌ Error: {:?}\n", e);
        }
    }
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ Generated client example completed!");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 Connect-RPC Generated Client Example\n");
    
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
