use base64::{engine::general_purpose, Engine as _};
use prost::Message;
use reqwest::{Client, Method, Request, Url};
use serde::Serialize;
use std::time::Duration;

use crate::server::error::{RpcError, RpcErrorCode};

/// Builder for RPC requests
pub struct RpcRequest<TReq> {
    url: Url,
    message: TReq,
    use_binary: bool,
    timeout_ms: Option<u64>,
}

impl<TReq> RpcRequest<TReq>
where
    TReq: Message + Serialize,
{
    pub fn new(
        base_url: Url,
        service_path: &str,
        message: TReq,
        use_binary: bool,
        timeout_ms: Option<u64>,
    ) -> Result<Self, RpcError> {
        let url = base_url.join(service_path).map_err(|e| {
            RpcError::new(
                RpcErrorCode::InvalidArgument,
                format!("Failed to join URL: {}", e),
            )
        })?;

        Ok(Self {
            url,
            message,
            use_binary,
            timeout_ms,
        })
    }

    /// Build a unary POST request
    pub fn build_unary(self, client: &Client) -> Result<Request, RpcError> {
        let mut request = client.request(Method::POST, self.url);

        // Set protocol version header
        request = request.header("connect-protocol-version", "1");

        // Set timeout if specified
        if let Some(timeout_ms) = self.timeout_ms {
            request = request
                .header("connect-timeout-ms", timeout_ms.to_string())
                .timeout(Duration::from_millis(timeout_ms));
        }

        // Encode the message
        let (content_type, body) = if self.use_binary {
            ("application/proto", self.message.encode_to_vec())
        } else {
            let json = serde_json::to_vec(&self.message).map_err(|e| {
                RpcError::new(
                    RpcErrorCode::Internal,
                    format!("Failed to serialize request: {}", e),
                )
            })?;
            ("application/json", json)
        };

        request = request.header("content-type", content_type).body(body);

        request.build().map_err(|e| {
            RpcError::new(
                RpcErrorCode::Internal,
                format!("Failed to build request: {}", e),
            )
        })
    }

    /// Build a unary GET request
    pub fn build_unary_get(self, client: &Client) -> Result<Request, RpcError> {
        let encoding = if self.use_binary { "proto" } else { "json" };

        // Encode the message
        let message_bytes = if self.use_binary {
            self.message.encode_to_vec()
        } else {
            serde_json::to_vec(&self.message).map_err(|e| {
                RpcError::new(
                    RpcErrorCode::Internal,
                    format!("Failed to serialize request: {}", e),
                )
            })?
        };

        // Base64 encode the message for URL safety
        let message_b64 = general_purpose::URL_SAFE.encode(&message_bytes);

        // Build query parameters
        let mut url = self.url.clone();
        url.query_pairs_mut()
            .append_pair("message", &message_b64)
            .append_pair("encoding", encoding)
            .append_pair("base64", "1");

        if let Some(timeout_ms) = self.timeout_ms {
            url.query_pairs_mut()
                .append_pair("connect", &format!("v1&timeout={}ms", timeout_ms));
        } else {
            url.query_pairs_mut().append_pair("connect", "v1");
        }

        let mut request = client.request(Method::GET, url);

        // Set timeout if specified
        if let Some(timeout_ms) = self.timeout_ms {
            request = request.timeout(Duration::from_millis(timeout_ms));
        }

        request.build().map_err(|e| {
            RpcError::new(
                RpcErrorCode::Internal,
                format!("Failed to build request: {}", e),
            )
        })
    }

    /// Build a server-streaming request
    pub fn build_server_stream(self, client: &Client) -> Result<Request, RpcError> {
        let mut request = client.request(Method::POST, self.url);

        // Set protocol version header
        request = request.header("connect-protocol-version", "1");

        // Set streaming flag
        request = request.header("connect-streaming-request", "1");

        // Set timeout if specified
        if let Some(timeout_ms) = self.timeout_ms {
            request = request
                .header("connect-timeout-ms", timeout_ms.to_string())
                .timeout(Duration::from_millis(timeout_ms));
        }

        // Encode the message with envelope
        let (content_type, body) = if self.use_binary {
            let mut envelope = vec![0x00, 0, 0, 0, 0]; // flags=0, length placeholder
            self.message.encode(&mut envelope).map_err(|e| {
                RpcError::new(
                    RpcErrorCode::Internal,
                    format!("Failed to encode request: {}", e),
                )
            })?;
            let length = (envelope.len() - 5) as u32;
            envelope[1..5].copy_from_slice(&length.to_be_bytes());
            ("application/connect+proto", envelope)
        } else {
            let mut envelope = vec![0x00, 0, 0, 0, 0]; // flags=0, length placeholder
            serde_json::to_writer(&mut envelope, &self.message).map_err(|e| {
                RpcError::new(
                    RpcErrorCode::Internal,
                    format!("Failed to serialize request: {}", e),
                )
            })?;
            let length = (envelope.len() - 5) as u32;
            envelope[1..5].copy_from_slice(&length.to_be_bytes());
            ("application/connect+json", envelope)
        };

        request = request.header("content-type", content_type).body(body);

        request.build().map_err(|e| {
            RpcError::new(
                RpcErrorCode::Internal,
                format!("Failed to build request: {}", e),
            )
        })
    }
}
