use prost::Message;
use reqwest::Response as ReqwestResponse;
use serde::de::DeserializeOwned;

use crate::error::{RpcError, RpcErrorCode};
use crate::encoding::{Encoding, decode_message};
use crate::protocol::status_to_error_code;

/// Wrapper for RPC responses
pub struct RpcResponse;

impl RpcResponse {
    /// Parse a unary response
    pub async fn from_unary<TRes>(
        response: ReqwestResponse,
        encoding: Encoding,
    ) -> Result<TRes, RpcError>
    where
        TRes: Message + DeserializeOwned + Default,
    {
        let status = response.status();

        // Check if the response is an error
        if !status.is_success() {
            // Try to parse as RPC error
            if let Ok(body) = response.bytes().await {
                if let Ok(error) = serde_json::from_slice::<RpcError>(&body) {
                    return Err(error);
                }
            }

            // Fallback to generic error
            return Err(RpcError::new(
                status_to_error_code(status),
                format!("Request failed with status: {}", status),
            ));
        }

        // Get content type to determine encoding
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        let response_encoding = Encoding::from_content_type(content_type)
            .unwrap_or(encoding); // Fallback to client encoding

        // Read response body
        let body = response.bytes().await.map_err(|e| {
            RpcError::new(
                RpcErrorCode::Internal,
                format!("Failed to read response body: {}", e),
            )
        })?;

        // Check if it's an error response (errors are always JSON)
        if response_encoding == Encoding::Json {
            // Try to parse as error first
            if let Ok(error) = serde_json::from_slice::<RpcError>(&body) {
                return Err(error);
            }
        }

        // Decode the response
        decode_message(&body, response_encoding)
    }

}

