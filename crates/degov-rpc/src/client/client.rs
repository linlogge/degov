use prost::Message;
use reqwest::{Client as ReqwestClient, Url};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::sync::Arc;

use super::request::RpcRequest;
use super::response::RpcResponse;
use super::stream::{parse_streaming_response, RpcStream};
use crate::error::{RpcError, RpcErrorCode};
use crate::encoding::Encoding;

/// Configuration for the RPC client
#[derive(Clone, Debug)]
pub struct RpcClientConfig {
    /// Base URL for the RPC server
    pub base_url: Url,
    /// Encoding format for the RPC client
    pub encoding: Encoding,
    /// Optional timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

impl RpcClientConfig {
    pub fn new(base_url: impl Into<String>) -> Result<Self, RpcError> {
        let url = Url::parse(&base_url.into()).map_err(|e| {
            RpcError::new(
                RpcErrorCode::InvalidArgument,
                format!("Invalid base URL: {}", e),
            )
        })?;

        Ok(Self {
            base_url: url,
            encoding: Encoding::Json,
            timeout_ms: None,
        })
    }

    pub fn with_encoding(mut self, encoding: Encoding) -> Self {
        self.encoding = encoding;
        self
    }

    pub fn with_binary(mut self, use_binary: bool) -> Self {
        self.encoding = if use_binary { Encoding::Proto } else { Encoding::Json };
        self
    }

    pub fn with_timeout_ms(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = Some(timeout_ms);
        self
    }
}

/// RPC client for making Connect-Web requests
#[derive(Clone)]
pub struct RpcClient {
    client: ReqwestClient,
    config: Arc<RpcClientConfig>,
}

impl RpcClient {
    pub fn new(config: RpcClientConfig) -> Self {
        Self {
            client: ReqwestClient::new(),
            config: Arc::new(config),
        }
    }

    pub fn with_client(config: RpcClientConfig, client: ReqwestClient) -> Self {
        Self {
            client,
            config: Arc::new(config),
        }
    }

    /// Make a unary RPC call
    pub async fn unary<TReq, TRes>(
        &self,
        service_path: impl AsRef<str>,
        request: TReq,
    ) -> Result<TRes, RpcError>
    where
        TReq: Message + Serialize,
        TRes: Message + DeserializeOwned + Default,
    {
        let rpc_request = RpcRequest::new(
            self.config.base_url.clone(),
            service_path.as_ref(),
            request,
            self.config.encoding,
            self.config.timeout_ms,
        )?;

        let http_request = rpc_request.build_unary(&self.client)?;

        let http_response = self.client.execute(http_request).await.map_err(|e| {
            RpcError::new(
                RpcErrorCode::Unavailable,
                format!("Failed to execute request: {}", e),
            )
        })?;

        RpcResponse::from_unary(http_response, self.config.encoding).await
    }

    /// Make a unary RPC call using GET method (for idempotent operations)
    pub async fn unary_get<TReq, TRes>(
        &self,
        service_path: impl AsRef<str>,
        request: TReq,
    ) -> Result<TRes, RpcError>
    where
        TReq: Message + Serialize,
        TRes: Message + DeserializeOwned + Default,
    {
        let rpc_request = RpcRequest::new(
            self.config.base_url.clone(),
            service_path.as_ref(),
            request,
            self.config.encoding,
            self.config.timeout_ms,
        )?;

        let http_request = rpc_request.build_unary_get(&self.client)?;

        let http_response = self.client.execute(http_request).await.map_err(|e| {
            RpcError::new(
                RpcErrorCode::Unavailable,
                format!("Failed to execute request: {}", e),
            )
        })?;

        RpcResponse::from_unary(http_response, self.config.encoding).await
    }

    /// Get the underlying reqwest client
    pub fn reqwest_client(&self) -> &ReqwestClient {
        &self.client
    }

    /// Get the client configuration
    pub fn config(&self) -> &RpcClientConfig {
        &self.config
    }

    /// Make a server-streaming RPC call
    pub async fn server_stream<TReq, TRes>(
        &self,
        service_path: impl AsRef<str>,
        request: TReq,
    ) -> Result<RpcStream<TRes>, RpcError>
    where
        TReq: Message + Serialize,
        TRes: Message + DeserializeOwned + Default + Send + 'static,
    {
        let rpc_request = RpcRequest::new(
            self.config.base_url.clone(),
            service_path.as_ref(),
            request,
            self.config.encoding,
            self.config.timeout_ms,
        )?;

        let http_request = rpc_request.build_server_stream(&self.client)?;

        let http_response = self.client.execute(http_request).await.map_err(|e| {
            RpcError::new(
                RpcErrorCode::Unavailable,
                format!("Failed to execute request: {}", e),
            )
        })?;

        parse_streaming_response(http_response, self.config.encoding).await
    }
}
