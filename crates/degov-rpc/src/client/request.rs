use prost::Message;
use reqwest::{Client, Method, Request, Url};
use serde::Serialize;
use std::time::Duration;

use crate::error::{RpcError, RpcErrorCode};
use crate::encoding::{Encoding, encode_message, encode_for_get};

/// Builder for RPC requests
pub struct RpcRequest<TReq> {
    url: Url,
    message: TReq,
    encoding: Encoding,
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
        encoding: Encoding,
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
            encoding,
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
        let body = encode_message(&self.message, self.encoding)?;
        let content_type = self.encoding.content_type(false);

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
        let encoding_str = match self.encoding {
            Encoding::Json => "json",
            Encoding::Proto => "proto",
        };

        // Encode the message
        let message_b64 = encode_for_get(&self.message, self.encoding)?;

        // Build query parameters
        let mut url = self.url.clone();
        url.query_pairs_mut()
            .append_pair("message", &message_b64)
            .append_pair("encoding", encoding_str)
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
        let body = crate::encoding::encode_envelope(&self.message, self.encoding)?;
        let content_type = self.encoding.content_type(true);

        request = request.header("content-type", content_type).body(body);

        request.build().map_err(|e| {
            RpcError::new(
                RpcErrorCode::Internal,
                format!("Failed to build request: {}", e),
            )
        })
    }
}
