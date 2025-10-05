use prost::Message;
use reqwest::Response as ReqwestResponse;
use serde::de::DeserializeOwned;

use crate::server::error::{RpcError, RpcErrorCode};

/// Wrapper for RPC responses
pub struct RpcResponse;

impl RpcResponse {
    /// Parse a unary response
    pub async fn from_unary<TRes>(
        response: ReqwestResponse,
        use_binary: bool,
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
                Self::status_to_error_code(status),
                format!("Request failed with status: {}", status),
            ));
        }

        // Get content type to determine encoding
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|ct| ct.to_str().ok())
            .unwrap_or("");

        let is_json = content_type.contains("application/json");
        let is_proto = content_type.contains("application/proto");

        // Read response body
        let body = response.bytes().await.map_err(|e| {
            RpcError::new(
                RpcErrorCode::Internal,
                format!("Failed to read response body: {}", e),
            )
        })?;

        // Check if it's an error response (errors are always JSON)
        if is_json && !use_binary {
            // Try to parse as error first
            if let Ok(error) = serde_json::from_slice::<RpcError>(&body) {
                return Err(error);
            }
        }

        // Decode based on content type or use_binary flag
        if is_proto || (use_binary && !is_json) {
            TRes::decode(&body[..]).map_err(|e| {
                RpcError::new(
                    RpcErrorCode::Internal,
                    format!("Failed to decode binary protobuf: {}", e),
                )
            })
        } else {
            serde_json::from_slice(&body).map_err(|e| {
                RpcError::new(
                    RpcErrorCode::Internal,
                    format!("Failed to decode JSON: {}", e),
                )
            })
        }
    }

    /// Convert HTTP status code to RPC error code
    fn status_to_error_code(status: reqwest::StatusCode) -> RpcErrorCode {
        use reqwest::StatusCode;

        // Spec: https://connect.build/docs/protocol/#error-codes
        match status {
            StatusCode::BAD_REQUEST => RpcErrorCode::InvalidArgument,
            StatusCode::UNAUTHORIZED => RpcErrorCode::Unauthenticated,
            StatusCode::FORBIDDEN => RpcErrorCode::PermissionDenied,
            StatusCode::NOT_FOUND => RpcErrorCode::NotFound,
            StatusCode::CONFLICT => RpcErrorCode::AlreadyExists,
            StatusCode::PRECONDITION_FAILED => RpcErrorCode::FailedPrecondition,
            StatusCode::REQUEST_TIMEOUT => RpcErrorCode::DeadlineExceeded,
            StatusCode::TOO_MANY_REQUESTS => RpcErrorCode::ResourceExhausted,
            StatusCode::SERVICE_UNAVAILABLE => RpcErrorCode::Unavailable,
            _ => RpcErrorCode::Unknown,
        }
    }
}

