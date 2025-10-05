# Connect-RPC Client

This package now includes both server and client implementations for Connect-Web RPC using Rust.

## Client Features

- **Unary RPC calls** (POST and GET methods)
- **Server-streaming RPC calls**
- **Binary and JSON encoding** support
- **Built on reqwest** for robust HTTP client functionality
- **Type-safe** with Rust's type system
- **Async/await** support with Tokio
- **Error handling** with comprehensive RPC error codes

## Quick Start

Add the dependency to your `Cargo.toml`:

```toml
[dependencies]
connect-rpc = "0.5.3"
reqwest = "0.12"
prost = "0.13"
tokio = { version = "1", features = ["full"] }
```

## Usage Examples

### Basic Unary RPC Call

```rust
use connect_rpc::prelude::*;
use prost::Message;
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
pub struct HelloRequest {
    #[prost(string, tag = "1")]
    pub name: String,
}

#[derive(Clone, PartialEq, Message, Serialize, Deserialize, Default)]
pub struct HelloResponse {
    #[prost(string, tag = "1")]
    pub message: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client configuration
    let config = RpcClientConfig::new("http://localhost:3000")?
        .with_binary(false)  // Use JSON encoding
        .with_timeout_ms(5000);  // 5 second timeout

    // Create the RPC client
    let client = RpcClient::new(config);

    // Make a unary RPC call
    let request = HelloRequest {
        name: "World".to_string(),
    };

    let response = client
        .unary::<HelloRequest, HelloResponse>(
            "/greet.v1.GreetService/Greet",
            request,
        )
        .await?;

    println!("Response: {}", response.message);

    Ok(())
}
```

### GET Request for Idempotent Operations

```rust
// Use GET for idempotent operations
let response = client
    .unary_get::<HelloRequest, HelloResponse>(
        "/greet.v1.GreetService/Greet",
        request,
    )
    .await?;
```

### Server-Streaming RPC

```rust
use futures::StreamExt;

#[derive(Clone, PartialEq, Message, Serialize, Deserialize)]
pub struct StreamRequest {
    #[prost(int32, tag = "1")]
    pub count: i32,
}

#[derive(Clone, PartialEq, Message, Serialize, Deserialize, Default)]
pub struct StreamResponse {
    #[prost(int32, tag = "1")]
    pub index: i32,
    #[prost(string, tag = "2")]
    pub message: String,
}

let stream_request = StreamRequest { count: 10 };

let mut stream = client
    .server_stream::<StreamRequest, StreamResponse>(
        "/greet.v1.GreetService/StreamGreet",
        stream_request,
    )
    .await?;

// Process the stream
while let Some(result) = stream.next().await {
    match result {
        Ok(response) => {
            println!("[{}] {}", response.index, response.message);
        }
        Err(e) => {
            eprintln!("Stream error: {:?}", e);
            break;
        }
    }
}
```

### Binary Encoding

```rust
// Use binary protobuf encoding for better performance
let binary_config = RpcClientConfig::new("http://localhost:3000")?
    .with_binary(true);

let binary_client = RpcClient::new(binary_config);

// All requests will now use binary encoding
let response = binary_client
    .unary::<HelloRequest, HelloResponse>(
        "/greet.v1.GreetService/Greet",
        request,
    )
    .await?;
```

### Custom Reqwest Client

```rust
// Use a custom reqwest client with specific configuration
let reqwest_client = reqwest::Client::builder()
    .user_agent("my-app/1.0")
    .build()?;

let config = RpcClientConfig::new("http://localhost:3000")?;
let client = RpcClient::with_client(config, reqwest_client);
```

## Error Handling

The client uses the same error types as the server, implementing the Connect-Web error protocol:

```rust
use connect_rpc::prelude::*;

match client.unary::<HelloRequest, HelloResponse>("/service/method", request).await {
    Ok(response) => {
        println!("Success: {:?}", response);
    }
    Err(error) => {
        match error.code {
            RpcErrorCode::NotFound => println!("Service not found"),
            RpcErrorCode::Unauthenticated => println!("Authentication required"),
            RpcErrorCode::PermissionDenied => println!("Permission denied"),
            _ => println!("Error: {}", error.message),
        }
    }
}
```

## Configuration Options

### RpcClientConfig

- `base_url`: Base URL for the RPC server (required)
- `use_binary`: Use binary protobuf encoding (default: false, uses JSON)
- `timeout_ms`: Request timeout in milliseconds (optional)

### Methods

- `unary<TReq, TRes>()`: Make a unary RPC call using POST
- `unary_get<TReq, TRes>()`: Make a unary RPC call using GET (for idempotent operations)
- `server_stream<TReq, TRes>()`: Make a server-streaming RPC call

## Protocol Compliance

The client implements the [Connect-Web protocol](https://connect.build/docs/protocol/):

- ✅ Unary requests (POST and GET)
- ✅ Server-streaming
- ✅ JSON and binary protobuf encoding
- ✅ Error handling with proper HTTP status codes
- ✅ Timeout support
- ✅ Protocol version negotiation

## Example

See `examples/client_example.rs` for a complete working example:

```bash
cargo run --example client_example
```

## Architecture

The client follows the same design principles as the server:

- **client.rs**: Main client implementation using reqwest
- **request.rs**: Request building and encoding
- **response.rs**: Response parsing and decoding
- **stream.rs**: Streaming response handling
- **error.rs**: Shared error types with server (in server module)

All components are designed to be type-safe, async, and follow Rust best practices.

