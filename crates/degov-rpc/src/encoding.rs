use base64::{engine::general_purpose, Engine as _};
use prost::Message;
use serde::Serialize;

use crate::error::{RpcError, RpcErrorCode};

/// Encoding format for RPC messages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Json,
    Proto,
}

impl Encoding {
    pub fn from_content_type(content_type: &str) -> Result<Self, RpcError> {
    let content_type_lower = content_type.to_lowercase();
    let content_type = content_type_lower
        .split(';')
        .next()
        .unwrap_or_default()
        .trim();

        match content_type {
            "application/json" | "application/connect+json" => Ok(Encoding::Json),
            "application/proto" | "application/connect+proto" => Ok(Encoding::Proto),
            _ => Err(RpcError::new(
                RpcErrorCode::InvalidArgument,
                format!("Unsupported content type: {}", content_type),
            )),
        }
    }

    pub fn content_type(&self, streaming: bool) -> &'static str {
        match (self, streaming) {
            (Encoding::Json, false) => "application/json",
            (Encoding::Json, true) => "application/connect+json",
            (Encoding::Proto, false) => "application/proto",
            (Encoding::Proto, true) => "application/connect+proto",
        }
    }
}

/// Encode a message to bytes using the specified encoding
pub fn encode_message<T>(message: &T, encoding: Encoding) -> Result<Vec<u8>, RpcError>
where
    T: Message + Serialize,
{
    match encoding {
        Encoding::Json => serde_json::to_vec(message).map_err(|e| {
            RpcError::new(
                RpcErrorCode::Internal,
                format!("Failed to serialize to JSON: {}", e),
            )
        }),
        Encoding::Proto => Ok(message.encode_to_vec()),
    }
}

/// Decode bytes to a message using the specified encoding
pub fn decode_message<T>(data: &[u8], encoding: Encoding) -> Result<T, RpcError>
where
    T: Message + for<'de> serde::Deserialize<'de> + Default,
{
    match encoding {
        Encoding::Json => serde_json::from_slice(data).map_err(|e| {
            RpcError::new(
                RpcErrorCode::Internal,
                format!("Failed to deserialize from JSON: {}", e),
            )
        }),
        Encoding::Proto => T::decode(data).map_err(|e| {
            RpcError::new(
                RpcErrorCode::Internal,
                format!("Failed to decode protobuf: {}", e),
            )
        }),
    }
}

/// Encode a message with envelope for streaming
pub fn encode_envelope<T>(message: &T, encoding: Encoding) -> Result<Vec<u8>, RpcError>
where
    T: Message + Serialize,
{
    // First encode the message
    let message_bytes = match encoding {
        Encoding::Json => {
            serde_json::to_vec(message).map_err(|e| {
                RpcError::new(
                    RpcErrorCode::Internal,
                    format!("Failed to serialize to JSON: {}", e),
                )
            })?
        }
        Encoding::Proto => {
            message.encode_to_vec()
        }
    };

    // Create envelope: [flags: u8][length: u32 BE][payload: bytes]
    let mut result = vec![0, 0, 0, 0, 0]; // flags=0, length placeholder
    result.extend_from_slice(&message_bytes);
    
    let length = message_bytes.len() as u32;
    result[1..5].copy_from_slice(&length.to_be_bytes());
    Ok(result)
}

/// Decode a message from envelope for streaming
pub fn decode_envelope<T>(data: &[u8], encoding: Encoding) -> Result<T, RpcError>
where
    T: Message + for<'de> serde::Deserialize<'de> + Default,
{
    if data.len() < 5 {
        return Err(RpcError::new(
            RpcErrorCode::InvalidArgument,
            "Incomplete envelope header".to_string(),
        ));
    }

    let _flags = data[0];
    let length = u32::from_be_bytes([data[1], data[2], data[3], data[4]]) as usize;

    if data.len() < 5 + length {
        return Err(RpcError::new(
            RpcErrorCode::InvalidArgument,
            "Incomplete envelope payload".to_string(),
        ));
    }

    let payload = &data[5..5 + length];
    decode_message(payload, encoding)
}

/// Encode a message for GET requests (base64 encoded)
pub fn encode_for_get<T>(message: &T, encoding: Encoding) -> Result<String, RpcError>
where
    T: Message + Serialize,
{
    let bytes = encode_message(message, encoding)?;
    Ok(general_purpose::URL_SAFE.encode(&bytes))
}

/// Decode a message from GET requests (base64 encoded)
pub fn decode_from_get<T>(encoded: &str, encoding: Encoding) -> Result<T, RpcError>
where
    T: Message + for<'de> serde::Deserialize<'de> + Default,
{
    let bytes = general_purpose::URL_SAFE.decode(encoded).map_err(|e| {
        RpcError::new(
            RpcErrorCode::InvalidArgument,
            format!("Failed to decode base64: {}", e),
        )
    })?;
    decode_message(&bytes, encoding)
}

/// Encode an error for streaming responses
pub fn encode_streaming_error(error: RpcError) -> Vec<u8> {
    #[derive(Serialize)]
    struct EndOfStream {
        error: RpcError,
    }

    let message = EndOfStream { error };
    let mut result = vec![0x2, 0, 0, 0, 0]; // flags=0x02 (end of stream), length placeholder
    serde_json::to_writer(&mut result, &message).unwrap();

    let size = ((result.len() - 5) as u32).to_be_bytes();
    result[1..5].copy_from_slice(&size);
    result
}

/// Encode end of stream marker
pub fn encode_end_of_stream() -> Vec<u8> {
    vec![0x2, 0, 0, 0, 2, b'{', b'}'] // flags=0x02, length=2, "{}"
}
