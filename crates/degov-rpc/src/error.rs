use axum::http::StatusCode;
use prost::Message;
use serde::{Deserialize, Serialize};

// Forward declarations to avoid circular dependencies

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcError {
    pub code: RpcErrorCode,
    pub message: String,
    pub details: Vec<RpcErrorDetail>,
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for RpcError {}

pub trait RpcIntoError {
    fn rpc_into_error(self) -> RpcError;
}

impl RpcIntoError for RpcError {
    fn rpc_into_error(self) -> RpcError {
        self
    }
}

impl RpcError {
    pub fn new(code: RpcErrorCode, message: String) -> Self {
        Self {
            code,
            message,
            details: vec![],
        }
    }
}

impl<C, M> RpcIntoError for (C, M)
where
    C: Into<RpcErrorCode>,
    M: Into<String>,
{
    fn rpc_into_error(self) -> RpcError {
        RpcError {
            code: self.0.into(),
            message: self.1.into(),
            details: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RpcErrorDetail {
    #[serde(rename = "type")]
    pub proto_type: String,
    #[serde(rename = "value")]
    pub proto_b62_value: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RpcErrorCode {
    Canceled,
    Unknown,
    InvalidArgument,
    DeadlineExceeded,
    NotFound,
    AlreadyExists,
    PermissionDenied,
    ResourceExhausted,
    FailedPrecondition,
    Aborted,
    OutOfRange,
    Unimplemented,
    Internal,
    Unavailable,
    DataLoss,
    Unauthenticated,
}

impl From<RpcErrorCode> for StatusCode {
    fn from(val: RpcErrorCode) -> Self {
        match val {
            // Spec: https://connect.build/docs/protocol/#error-codes
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
}

impl<T> RpcIntoResponse<T> for RpcErrorCode
where
    T: Message,
{
    fn rpc_into_response(self) -> RpcResult<T> {
        Err(RpcError::new(self, "".to_string()))
    }
}

impl<T> RpcIntoResponse<T> for RpcError
where
    T: Message,
{
    fn rpc_into_response(self) -> RpcResult<T> {
        Err(self)
    }
}

// Forward declarations to avoid circular dependencies
pub type RpcResult<M> = Result<M, RpcError>;

pub trait RpcIntoResponse<T>: Send + Sync + 'static
where
    T: Message,
{
    fn rpc_into_response(self) -> RpcResult<T>;
}

impl<T> RpcIntoResponse<T> for T
where
    T: Message + 'static,
{
    fn rpc_into_response(self) -> RpcResult<T> {
        Ok(self)
    }
}

impl<T, E> RpcIntoResponse<T> for Result<T, E>
where
    T: Message + 'static,
    E: RpcIntoError + Send + Sync + 'static,
{
    fn rpc_into_response(self) -> RpcResult<T> {
        self.map_err(|e| e.rpc_into_error())
    }
}
