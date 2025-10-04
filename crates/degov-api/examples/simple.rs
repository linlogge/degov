use degov_api::proto::hello::{HelloRequest, HelloResponse};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = degov_api::client::Client::new();
    let response: HelloResponse = client
        .unary_call(
            HelloRequest {
                name: "World".to_string(),
            },
            "/hello.HelloWorldService/SayHello",
        )
        .await?;

    println!("Response: {:?}", response);

    Ok(())
}
