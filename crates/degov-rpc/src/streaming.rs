use futures::{Stream, StreamExt};
use prost::Message;
use serde::Deserialize;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::encoding::{encode_envelope, encode_end_of_stream, encode_streaming_error, Encoding};
use crate::error::{RpcError, RpcErrorCode, RpcResult};

/// Stream of RPC messages
pub struct RpcStream<TRes> {
    inner: Pin<Box<dyn Stream<Item = RpcResult<TRes>> + Send>>,
}

impl<TRes> RpcStream<TRes> {
    pub fn new(stream: impl Stream<Item = RpcResult<TRes>> + Send + 'static) -> Self {
        Self {
            inner: Box::pin(stream),
        }
    }
}

impl<TRes> Stream for RpcStream<TRes> {
    type Item = RpcResult<TRes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.inner.as_mut().poll_next(cx)
    }
}

#[derive(Deserialize)]
struct EndOfStream {
    error: Option<RpcError>,
}

/// Parse a streaming response from the server
pub async fn parse_streaming_response<TRes>(
    response: reqwest::Response,
    encoding: Encoding,
) -> Result<RpcStream<TRes>, RpcError>
where
    TRes: Message + for<'de> serde::Deserialize<'de> + Default + Send + 'static,
{
    let status = response.status();

    // Streaming responses should always be 200 OK
    if !status.is_success() {
        return Err(RpcError::new(
            RpcErrorCode::Unknown,
            format!("Streaming request failed with status: {}", status),
        ));
    }

    let byte_stream = response.bytes_stream();

    // Use unfold to maintain state and parse messages
    let message_stream = futures::stream::unfold(
        (byte_stream, Vec::<u8>::new(), false),
        move |(mut stream, mut buffer, ended)| async move {
            if ended {
                return None;
            }

            loop {
                // Try to parse from existing buffer
                match try_parse_envelope::<TRes>(&mut buffer, encoding) {
                    Ok(ParseResult::Message(msg)) => {
                        return Some((Ok(msg), (stream, buffer, false)));
                    }
                    Ok(ParseResult::EndOfStream(eos)) => {
                        if let Some(error) = eos.error {
                            return Some((Err(error), (stream, buffer, true)));
                        } else {
                            return None; // Clean end of stream
                        }
                    }
                    Ok(ParseResult::Incomplete) => {
                        // Need more data, read from stream
                        match stream.next().await {
                            Some(Ok(chunk)) => {
                                buffer.extend_from_slice(&chunk);
                                // Continue loop to try parsing again
                            }
                            Some(Err(e)) => {
                                let error = RpcError::new(
                                    RpcErrorCode::Internal,
                                    format!("Failed to read chunk: {}", e),
                                );
                                return Some((Err(error), (stream, buffer, true)));
                            }
                            None => {
                                // Stream ended without proper closing
                                if !buffer.is_empty() {
                                    let error = RpcError::new(
                                        RpcErrorCode::Internal,
                                        "Stream ended with incomplete message".to_string(),
                                    );
                                    return Some((Err(error), (stream, buffer, true)));
                                }
                                return None;
                            }
                        }
                    }
                    Err(e) => {
                        return Some((Err(e), (stream, buffer, true)));
                    }
                }
            }
        },
    );

    Ok(RpcStream::new(message_stream))
}

enum ParseResult<T> {
    Message(T),
    EndOfStream(EndOfStream),
    Incomplete,
}

/// Try to parse an enveloped message from the buffer
/// Envelope format: [flags: u8][length: u32 BE][payload: bytes]
fn try_parse_envelope<TRes>(
    buffer: &mut Vec<u8>,
    encoding: Encoding,
) -> Result<ParseResult<TRes>, RpcError>
where
    TRes: Message + for<'de> serde::Deserialize<'de> + Default,
{
    // Need at least 5 bytes for envelope header
    if buffer.len() < 5 {
        return Ok(ParseResult::Incomplete);
    }

    let flags = buffer[0];
    let length = u32::from_be_bytes([buffer[1], buffer[2], buffer[3], buffer[4]]) as usize;

    // Check if we have the full message
    if buffer.len() < 5 + length {
        return Ok(ParseResult::Incomplete);
    }

    // Extract the message payload (clone to avoid borrow issues)
    let payload = buffer[5..5 + length].to_vec();

    // Remove the parsed envelope from the buffer
    *buffer = buffer[5 + length..].to_vec();

    // Check flags to determine message type
    // 0x00 = message, 0x02 = end of stream
    if flags == 0x02 {
        // End of stream
        let eos: EndOfStream = serde_json::from_slice(&payload).map_err(|e| {
            RpcError::new(
                RpcErrorCode::Internal,
                format!("Failed to parse end-of-stream: {}", e),
            )
        })?;
        return Ok(ParseResult::EndOfStream(eos));
    }

    // Parse the message directly from the payload (it's not an envelope)
    let message = match encoding {
        Encoding::Json => {
            serde_json::from_slice(&payload).map_err(|e| {
                RpcError::new(
                    RpcErrorCode::Internal,
                    format!("Failed to deserialize from JSON: {}", e),
                )
            })?
        }
        Encoding::Proto => {
            TRes::decode(&payload[..]).map_err(|e| {
                RpcError::new(
                    RpcErrorCode::Internal,
                    format!("Failed to decode protobuf: {}", e),
                )
            })?
        }
    };

    Ok(ParseResult::Message(message))
}

/// Encode a stream of messages for server responses
pub fn encode_stream<M>(
    stream: Pin<Box<dyn Stream<Item = RpcResult<M>> + Send>>,
    encoding: Encoding,
) -> impl Stream<Item = Result<Vec<u8>, std::convert::Infallible>>
where
    M: Message + serde::Serialize + 'static,
{
    futures::stream::unfold(Some(stream), move |stream| async move {
        match stream {
            None => {
                // We are past the last message, returning None
                // ends the stream without any more messages.
                None
            }
            Some(mut stream) => match stream.next().await {
                Some(Ok(message)) => {
                    // This is a normal message, we need to envelope-encode it.
                    // If an error occurs, we encode it instead and terminate
                    // the stream.
                    match encode_envelope(&message, encoding) {
                        Ok(message) => Some((Ok(message), Some(stream))),
                        Err(error) => Some((Ok(encode_streaming_error(error)), None)),
                    }
                }
                Some(Err(error)) => {
                    // An error in the stream. Send it as the last
                    // message and terminate the stream.
                    Some((Ok(encode_streaming_error(error)), None))
                }
                None => {
                    // Stream was read all the way through without errors,
                    // send the last message.
                    //
                    // Final streaming message ALWAYS has to contain at least
                    // an empty object and is ALWAYS encoded as JSON.
                    // https://connectrpc.com/docs/protocol/#error-end-stream
                    Some((Ok(encode_end_of_stream()), None))
                }
            },
        }
    })
}
