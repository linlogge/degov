use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use connectare::server::error::{RpcError, RpcErrorCode, RpcIntoError};

// This is an example Error type, to demo impls needed for `degov-rpc`. It uses `thiserror` to
// wrap various error types as syntactic sugar, but you could just as easily write this out by hand.
#[allow(dead_code)]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Returns `400 Bad Request`
    #[error("{0}")]
    BadRequest(String),

    /// Returns `403 Forbidden`
    #[error("user may not perform that action")]
    Forbidden,

    /// Returns `404 Not Found`
    #[error("{0}")]
    NotFound(String),

    /// Returns `500 Internal Server Error`
    #[error("{0}")]
    Internal(String),

    /// Returns `500 Internal Server Error`
    #[error("an internal server error occurred")]
    Anyhow(#[from] anyhow::Error),
}

/// Allows the error type to be returned from RPC handlers.
///
/// This trait is distinct from `IntoResponse` because RPCs cannot return arbitrary HTML responses.
/// Error codes are well-defined in connect-web (which mirrors gRPC), streaming errors don't effect
/// HTTP status codes, and so on.
impl RpcIntoError for Error {
    fn rpc_into_error(self) -> connectare::prelude::RpcError {
        println!("{:#?}", self);

        // Each response is a tuple of well-defined (per the Connect-Web) codes, along with a
        // message.
        match self {
            Self::BadRequest(msg) => {
                RpcError::new(RpcErrorCode::InvalidArgument, msg)
            }
            Self::Forbidden => {
                RpcError::new(RpcErrorCode::PermissionDenied, "Forbidden".to_string())
            }
            Self::NotFound(msg) => RpcError::new(RpcErrorCode::NotFound, msg),
            Self::Internal(msg) => {
                RpcError::new(RpcErrorCode::Internal, msg)
            }
            Self::Anyhow(_) => {
                RpcError::new(RpcErrorCode::Internal, "Internal Server Error".to_string())
            }
        }
    }
}

// This is a normal `IntoResponse` impl, which is used by Axum to convert errors into HTTP
// responses.
impl IntoResponse for Error {
    fn into_response(self) -> Response {
        println!("{:#?}", self);
        match self {
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg).into_response(),
            Self::Forbidden => (StatusCode::FORBIDDEN, "Forbidden").into_response(),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg).into_response(),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg).into_response(),
            Self::Anyhow(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error").into_response()
            }
        }
    }
}
