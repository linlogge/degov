use axum::http::{request::Parts, StatusCode};
use serde::{Deserialize, Serialize};

use crate::encoding::Encoding;
use crate::error::{RpcError, RpcErrorCode};

/// Connect-Web protocol version
pub const CONNECT_PROTOCOL_VERSION: &str = "1";

/// Check and validate Connect-Web protocol headers
pub fn validate_protocol_headers(parts: &mut Parts, for_streaming: bool) -> Result<Encoding, RpcError> {
    // Check the version header, if specified
    if let Some(version) = parts.headers.get("connect-protocol-version") {
        let version = version.to_str().unwrap_or_default();
        if version != CONNECT_PROTOCOL_VERSION {
            return Err(RpcError::new(
                RpcErrorCode::InvalidArgument,
                format!("Unsupported protocol version: {}", version),
            ));
        }
    }

    // Decode the content type (binary or JSON)
    let encoding = match parts.headers.get("content-type") {
        Some(content_type) => {
    let content_type_str = content_type.to_str().unwrap_or_default();
    let content_type_lower = content_type_str.to_lowercase();
    let content_type = content_type_lower
        .split(';')
        .next()
        .unwrap_or_default()
        .trim();

            match (content_type, for_streaming) {
                ("application/json", false) => Encoding::Json,
                ("application/proto", false) => Encoding::Proto,
                ("application/connect+json", true) => Encoding::Json,
                ("application/connect+proto", true) => Encoding::Proto,
                _ => {
                    return Err(RpcError::new(
                        RpcErrorCode::InvalidArgument,
                        format!("Unsupported content type: {}", content_type),
                    ));
                }
            }
        }
        None => {
            return Err(RpcError::new(
                RpcErrorCode::InvalidArgument,
                "Missing Content-Type header".to_string(),
            ));
        }
    };

    Ok(encoding)
}

/// Check and validate Connect-Web protocol query parameters for GET requests
pub fn validate_protocol_query(parts: &Parts) -> Result<Encoding, RpcError> {
    let query_str = parts.uri.query().ok_or_else(|| {
        RpcError::new(RpcErrorCode::InvalidArgument, "Missing query parameters".to_string())
    })?;

    let query: UnaryGetQuery = serde_qs::from_str(query_str).map_err(|e| {
        RpcError::new(
            RpcErrorCode::InvalidArgument,
            format!("Invalid query parameters: {}", e),
        )
    })?;

    let encoding = match query.encoding.as_str() {
        "json" => Encoding::Json,
        "proto" => Encoding::Proto,
        _ => {
            return Err(RpcError::new(
                RpcErrorCode::InvalidArgument,
                format!("Unsupported encoding: {}", query.encoding),
            ));
        }
    };

    Ok(encoding)
}

/// Get timeout from headers
pub fn get_timeout_ms(parts: &Parts) -> Option<u64> {
    parts
        .headers
        .get("connect-timeout-ms")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.parse().ok())
}

/// Get timeout from query parameters
pub fn get_timeout_from_query(parts: &Parts) -> Option<u64> {
    let query_str = parts.uri.query()?;
    let query: UnaryGetQuery = serde_qs::from_str(query_str).ok()?;
    
    // Parse connect parameter for timeout
    query.connect.as_ref()
        .and_then(|c| c.split('&').find(|p| p.starts_with("timeout=")))
        .and_then(|p| p.strip_prefix("timeout="))
        .and_then(|t| t.strip_suffix("ms"))
        .and_then(|t| t.parse().ok())
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct UnaryGetQuery {
    pub message: String,
    pub encoding: String,
    pub base64: Option<usize>,
    pub compression: Option<String>,
    pub connect: Option<String>,
}

/// Convert HTTP status code to RPC error code
pub fn status_to_error_code(status: StatusCode) -> RpcErrorCode {
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

/// Convert RPC error code to HTTP status code
pub fn error_code_to_status(code: &RpcErrorCode) -> StatusCode {
    match code {
        RpcErrorCode::Canceled => StatusCode::REQUEST_TIMEOUT,
        RpcErrorCode::Unknown => StatusCode::INTERNAL_SERVER_ERROR,
        RpcErrorCode::InvalidArgument => StatusCode::BAD_REQUEST,
        RpcErrorCode::DeadlineExceeded => StatusCode::REQUEST_TIMEOUT,
        RpcErrorCode::NotFound => StatusCode::NOT_FOUND,
        RpcErrorCode::AlreadyExists => StatusCode::CONFLICT,
        RpcErrorCode::PermissionDenied => StatusCode::FORBIDDEN,
        RpcErrorCode::ResourceExhausted => StatusCode::TOO_MANY_REQUESTS,
        RpcErrorCode::FailedPrecondition => StatusCode::PRECONDITION_FAILED,
        RpcErrorCode::Aborted => StatusCode::CONFLICT,
        RpcErrorCode::OutOfRange => StatusCode::BAD_REQUEST,
        RpcErrorCode::Unimplemented => StatusCode::NOT_FOUND,
        RpcErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        RpcErrorCode::Unavailable => StatusCode::SERVICE_UNAVAILABLE,
        RpcErrorCode::DataLoss => StatusCode::INTERNAL_SERVER_ERROR,
        RpcErrorCode::Unauthenticated => StatusCode::UNAUTHORIZED,
    }
}
