use std::pin::Pin;

use axum::body::{self, Body};
use axum::http::{request, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use futures::Stream;
use prost::Message;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::error::{RpcError, RpcErrorCode, RpcIntoError, RpcIntoResponse, RpcResult};
use crate::encoding::{Encoding, encode_message, decode_message, encode_streaming_error};

pub(crate) struct ReqResInto {
    pub encoding: Encoding,
}

type ResponseStream<M> = Pin<Box<dyn Stream<Item = RpcResult<M>> + Send>>;

enum ResponseContent<M> {
    UnarySuccess(M),
    UnaryError(RpcError),
    StreamingSuccess(ResponseStream<M>),
    StreamingError(RpcError),
}

pub(crate) struct ResponseEncoder<M> {
    encoding: Encoding,
    content: ResponseContent<M>,
}

impl ResponseEncoder<()> {
    pub fn error(error: impl RpcIntoError, streaming: bool, encoding: Encoding) -> Self {
        Self {
            encoding,
            content: if streaming {
                ResponseContent::StreamingError(error.rpc_into_error())
            } else {
                ResponseContent::UnaryError(error.rpc_into_error())
            },
        }
    }
}

impl<M: Message + Serialize + 'static> ResponseEncoder<M> {
    pub fn unary(response: impl RpcIntoResponse<M>, encoding: Encoding) -> Self {
        Self {
            encoding,
            content: match response.rpc_into_response() {
                Ok(message) => ResponseContent::UnarySuccess(message),
                Err(error) => ResponseContent::UnaryError(error),
            },
        }
    }

    pub fn stream(stream: ResponseStream<M>, encoding: Encoding) -> Self {
        Self {
            encoding,
            content: ResponseContent::StreamingSuccess(stream),
        }
    }

    pub fn status_code(&self) -> StatusCode {
        use ResponseContent::*;

        match &self.content {
            UnarySuccess(_) => StatusCode::OK,
            UnaryError(e) => e.code.clone().into(),

            // Streaming requests ALWAYS return 200 response code
            // https://connectrpc.com/docs/protocol/#streaming-response
            StreamingSuccess(_) | StreamingError(_) => StatusCode::OK,
        }
    }

    pub fn content_type(&self) -> &'static str {
        use ResponseContent::*;

        match &self.content {
            // Errors in unary calls are ALWAYS encoded as JSONs
            // https://connectrpc.com/docs/protocol/#unary-response
            UnaryError(_) => "application/json",
            
            // Use encoding for other content types
            _ => self.encoding.content_type(matches!(self.content, StreamingSuccess(_) | StreamingError(_))),
        }
    }

    fn encode_body(self) -> Body {
        use ResponseContent::*;

        match self.content {
            // Error
            UnaryError(error) => Body::from(encode_unary_error(error)),
            StreamingError(error) => Body::from(encode_streaming_error(error)),

            // Unary
            UnarySuccess(message) => Body::from(encode_message(&message, self.encoding).unwrap_or_else(|e| encode_unary_error(e))),

            // Streaming
            StreamingSuccess(stream) => Body::from_stream(crate::streaming::encode_stream(stream, self.encoding)),
        }
    }

    pub fn encode_response(self) -> Response {
        let code = self.status_code();
        let headers = [(axum::http::header::CONTENT_TYPE, self.content_type())];
        let body = self.encode_body();
        (code, headers, body).into_response()
    }
}

fn encode_unary_error(error: RpcError) -> Vec<u8> {
    // Errors in unary calls are ALWAYS encoded as JSONs
    //
    // https://connectrpc.com/docs/protocol/#unary-response
    serde_json::to_vec(&error).unwrap()
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub(crate) struct UnaryGetQuery {
    pub message: String,
    pub encoding: String,
    pub base64: Option<usize>,
    pub compression: Option<String>,
    pub connect: Option<String>,
}

pub(crate) fn decode_check_query(parts: &request::Parts) -> Result<ReqResInto, Response> {
        match crate::protocol::validate_protocol_query(parts) {
        Ok(encoding) => Ok(ReqResInto { encoding }),
        Err(error) => Err(ResponseEncoder::error(error, false, Encoding::Json).encode_response()),
    }
}

pub(crate) fn decode_check_headers(
    parts: &mut request::Parts,
    for_streaming: bool,
) -> Result<ReqResInto, Response> {
        match crate::protocol::validate_protocol_headers(parts, for_streaming) {
        Ok(encoding) => Ok(ReqResInto { encoding }),
        Err(error) => Err(ResponseEncoder::error(error, for_streaming, Encoding::Json).encode_response()),
    }
}

pub(crate) fn decode_request_payload_from_query<M, S>(
    parts: &request::Parts,
    _state: &S,
    encoding: Encoding,
) -> Result<M, Response>
where
    M: Message + DeserializeOwned + Default,
    S: Send + Sync + 'static,
{
    let query_str = match parts.uri.query() {
        Some(x) => x,
        None => {
            let error = RpcError::new(RpcErrorCode::InvalidArgument, "Missing query".to_string());
            return Err(ResponseEncoder::error(error, false, Encoding::Json).encode_response());
        }
    };

    let query = match serde_qs::from_str::<UnaryGetQuery>(query_str) {
        Ok(x) => x,
        Err(err) => {
            let error = RpcError::new(
                RpcErrorCode::InvalidArgument,
                format!("Wrong query, {}", err),
            );

            return Err(ResponseEncoder::error(error, false, Encoding::Json).encode_response());
        }
    };

    let message = if query.base64 == Some(1) {
        use base64::{engine::general_purpose, Engine as _};

        match general_purpose::URL_SAFE.decode(&query.message) {
            Ok(x) => x,
            Err(err) => {
                let error = RpcError::new(
                    RpcErrorCode::InvalidArgument,
                    format!("Wrong query.message, {}", err),
                );

                return Err(ResponseEncoder::error(error, false, Encoding::Json).encode_response());
            }
        }
    } else {
        query.message.as_bytes().to_vec()
    };

    decode_message(&message, encoding).map_err(|e| {
        ResponseEncoder::error(e, false, encoding).encode_response()
    })
}

pub(crate) async fn decode_request_payload<M, S>(
    req: Request<Body>,
    _state: &S,
    encoding: Encoding,
    for_streaming: bool,
) -> Result<M, Response>
where
    M: Message + DeserializeOwned + Default,
    S: Send + Sync + 'static,
{
    let bytes = body::to_bytes(req.into_body(), usize::MAX)
        .await
        .map_err(|e| {
            let error = RpcError::new(
                RpcErrorCode::InvalidArgument,
                format!("Failed to read request body. {}", e),
            );

            ResponseEncoder::error(error, for_streaming, encoding).encode_response()
        })?;

    // All streaming messages are wrapped in an envelope,
    // even if they are just requests for server-streaming.
    // https://connectrpc.com/docs/protocol/#streaming-request
    // https://github.com/connectrpc/connectrpc.com/issues/141
    let bytes = if for_streaming {
        bytes.slice(5..) // Skip envelope header
    } else {
        bytes
    };

    decode_message(&bytes, encoding).map_err(|e| {
        ResponseEncoder::error(e, for_streaming, encoding).encode_response()
    })
}
