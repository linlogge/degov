use crate::error::Error;
use axum::http::{self, HeaderValue, Method};
use prost::Message;
use reqwest::Client as HttpClient;

/// Helper to build Connect RPC service paths
/// 
/// # Example
/// ```
/// let path = service_path("hello", "HelloWorldService", "SayHello");
/// // Returns: "/hello.HelloWorldService/SayHello"
/// ```
pub fn service_path(package: &str, service: &str, method: &str) -> String {
    format!("/{}.{}/{}", package, service, method)
}

pub struct Client {
    base_url: String,
    http_client: HttpClient,
}

impl Client {
    /// Create a new Connect RPC client with the default base URL
    pub fn new() -> Self {
        Self {
            base_url: "http://localhost:3030".to_string(),
            http_client: HttpClient::new(),
        }
    }

    /// Create a new Connect RPC client with a custom base URL
    pub fn with_url(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            http_client: HttpClient::new(),
        }
    }

    /// Make a unary RPC call with any protobuf message
    /// 
    /// # Arguments
    /// * `request` - The protobuf request message
    /// * `service_path` - The full service path (e.g., "/package.ServiceName/MethodName")
    /// 
    /// # Example
    /// ```no_run
    /// let client = Client::new();
    /// let request = HelloRequest { name: "World".to_string() };
    /// let response: HelloResponse = client
    ///     .unary_call(request, "/hello.HelloWorldService/SayHello")
    ///     .await?;
    /// ```
    pub async fn unary_call<Req, Res>(
        &self,
        request: Req,
        service_path: &str,
    ) -> Result<Res, Error>
    where
        Req: Message,
        Res: Message + Default,
    {
        // Convert the prost message to an http::Request for Connect RPC
        let http_request = self.prost_to_connect_http(request, service_path)?;

        println!("Sending request to {}: {:?}", service_path, http_request);
        
        // Execute the HTTP request
        let response = self.execute_request::<Res>(http_request).await?;
        
        println!("Received response: {:?}", response);
        
        Ok(response)
    }

    /// Make a unary RPC call using JSON encoding instead of binary protobuf
    /// 
    /// # Arguments
    /// * `request` - The protobuf request message (must also implement serde::Serialize)
    /// * `service_path` - The full service path (e.g., "/package.ServiceName/MethodName")
    pub async fn unary_call_json<Req, Res>(
        &self,
        request: Req,
        service_path: &str,
    ) -> Result<Res, Error>
    where
        Req: Message + serde::Serialize,
        Res: Message + Default + for<'de> serde::Deserialize<'de>,
    {
        // Convert the prost message to an http::Request with JSON encoding
        let http_request = self.prost_to_connect_http_json(request, service_path)?;

        println!("Sending JSON request to {}: {:?}", service_path, http_request);
        
        // Execute the HTTP request and decode JSON response
        let response = self.execute_request_json::<Res>(http_request).await?;
        
        println!("Received JSON response: {:?}", response);
        
        Ok(response)
    }

    /// Convenience method for the HelloWorldService.SayHello RPC
    pub async fn say_hello(&self, name: String) -> Result<String, Error> {
        use crate::hello::proto::hello::{HelloRequest, HelloResponse};
        
        let request = HelloRequest { name };
        let response: HelloResponse = self
            .unary_call(request, "/hello.HelloWorldService/SayHello")
            .await?;
        Ok(response.message)
    }

    /// Execute an HTTP request and parse the response
    async fn execute_request<R>(
        &self,
        http_request: http::Request<Vec<u8>>,
    ) -> Result<R, Error>
    where
        R: Message + Default,
    {
        // Convert http::Request to reqwest::Request
        let uri = http_request.uri().to_string();
        let method = http_request.method().clone();
        let headers = http_request.headers().clone();
        let body = http_request.into_body();

        // Build reqwest request
        let mut reqwest_request = self.http_client
            .request(method, &uri)
            .body(body);

        // Add headers
        for (key, value) in headers.iter() {
            reqwest_request = reqwest_request.header(key, value);
        }

        // Send the request
        let response = reqwest_request
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send HTTP request: {}", e))?;

        // Check status code
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Server returned error status {}: {}",
                status,
                body
            ).into());
        }

        // Read response body
        let response_bytes = response
            .bytes()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?;

        // Decode the protobuf response
        let decoded = R::decode(&response_bytes[..])
            .map_err(|e| anyhow::anyhow!("Failed to decode response: {}", e))?;

        Ok(decoded)
    }

    /// Execute an HTTP request with JSON encoding and parse the response
    async fn execute_request_json<R>(
        &self,
        http_request: http::Request<Vec<u8>>,
    ) -> Result<R, Error>
    where
        R: for<'de> serde::Deserialize<'de>,
    {
        // Convert http::Request to reqwest::Request
        let uri = http_request.uri().to_string();
        let method = http_request.method().clone();
        let headers = http_request.headers().clone();
        let body = http_request.into_body();

        // Build reqwest request
        let mut reqwest_request = self.http_client
            .request(method, &uri)
            .body(body);

        // Add headers
        for (key, value) in headers.iter() {
            reqwest_request = reqwest_request.header(key, value);
        }

        // Send the request
        let response = reqwest_request
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to send HTTP request: {}", e))?;

        // Check status code
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Server returned error status {}: {}",
                status,
                body
            ).into());
        }

        // Read and parse JSON response
        let response_json = response
            .json::<R>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to decode JSON response: {}", e))?;

        Ok(response_json)
    }

    /// Convert a prost message to an http::Request following the Connect RPC protocol
    /// 
    /// This implements the Connect protocol specification:
    /// - Uses POST method
    /// - Sets Content-Type to application/proto for binary protobuf encoding
    /// - Includes Connect-Protocol-Version header
    /// - Encodes the message using protobuf binary format
    /// 
    /// See: https://connectrpc.com/docs/protocol
    fn prost_to_connect_http<M>(
        &self,
        message: M,
        service_path: &str,
    ) -> Result<http::Request<Vec<u8>>, Error>
    where
        M: Message,
    {
        // Encode the protobuf message to binary format
        let body = message.encode_to_vec();

        // Construct the full URI
        let uri = format!("{}{}", self.base_url, service_path);

        // Build the HTTP request
        let request = http::Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/proto"),
            )
            .header(
                "Connect-Protocol-Version",
                HeaderValue::from_static("1"),
            )
            .body(body)
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP request: {}", e))?;

        Ok(request)
    }

    /// Convert a prost message to an http::Request using JSON encoding
    /// 
    /// This is an alternative encoding supported by Connect RPC that uses JSON
    /// instead of binary protobuf. Useful for debugging or when interoperability
    /// with non-protobuf systems is needed.
    #[allow(dead_code)]
    fn prost_to_connect_http_json<M>(
        &self,
        message: M,
        service_path: &str,
    ) -> Result<http::Request<Vec<u8>>, Error>
    where
        M: Message + serde::Serialize,
    {
        // Serialize to JSON
        let body = serde_json::to_vec(&message)
            .map_err(|e| anyhow::anyhow!("Failed to serialize to JSON: {}", e))?;

        // Construct the full URI
        let uri = format!("{}{}", self.base_url, service_path);

        // Build the HTTP request
        let request = http::Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header(
                http::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            )
            .header(
                "Connect-Protocol-Version",
                HeaderValue::from_static("1"),
            )
            .body(body)
            .map_err(|e| anyhow::anyhow!("Failed to build HTTP request: {}", e))?;

        Ok(request)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
