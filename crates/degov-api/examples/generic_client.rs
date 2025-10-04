/// Example demonstrating how to use the generic Connect RPC client
/// with any protobuf message types

use degov_api::client::{Client, service_path};
use degov_api::proto::hello::{HelloRequest, HelloResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = Client::new();
    
    // Example 1: Using the generic unary_call method with explicit types
    println!("=== Example 1: Generic unary_call ===");
    let request = HelloRequest {
        name: "World".to_string(),
    };
    
    let response: HelloResponse = client
        .unary_call(request, "/hello.HelloWorldService/SayHello")
        .await?;
    
    println!("Response message: {}", response.message);
    
    // Example 2: Using the service_path helper
    println!("\n=== Example 2: Using service_path helper ===");
    let request = HelloRequest {
        name: "Rust".to_string(),
    };
    
    let path = service_path("hello", "HelloWorldService", "SayHello");
    let response: HelloResponse = client
        .unary_call(request, &path)
        .await?;
    
    println!("Response message: {}", response.message);
    
    // Example 3: Using the convenience method
    println!("\n=== Example 3: Convenience method ===");
    let message = client.say_hello("DeGov".to_string()).await?;
    println!("Response message: {}", message);
    
    // Example 4: Using JSON encoding (requires serde traits on messages)
    println!("\n=== Example 4: JSON encoding ===");
    let request = HelloRequest {
        name: "JSON".to_string(),
    };
    
    let response: HelloResponse = client
        .unary_call_json(request, "/hello.HelloWorldService/SayHello")
        .await?;
    
    println!("Response message: {}", response.message);
    
    // Example 5: Custom base URL
    println!("\n=== Example 5: Custom base URL ===");
    let custom_client = Client::with_url("http://localhost:8080");
    // This would connect to a different server
    // let response: HelloResponse = custom_client
    //     .unary_call(request, "/hello.HelloWorldService/SayHello")
    //     .await?;
    println!("Client configured for: localhost:8080");
    
    Ok(())
}

