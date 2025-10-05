//! Node types and operations

use blake3::Hasher;
use serde::{Deserialize, Serialize};
use serde_bytes::ByteBuf;

use crate::error::MstError;

/// BLAKE3 hash of a node's content
pub type NodeHash = [u8; 32];

/// Maximum fanout for inner nodes before splitting
pub const B: u32 = 16;

/// Internal node representation
///
/// Nodes are content-addressed and stored in FoundationDB by (layer, hash).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Node {
	/// Leaf node storing a key-value pair
	///
	/// The node hash is computed from the canonical DAG-CBOR serialization.
	/// Value is pre-encoded application data stored as raw bytes.
	Leaf {
		key: String,
		value: ByteBuf
	},

	/// Inner node storing separators and child hashes
	///
	/// Invariant: children.len() == separators.len() + 1
	///
	/// Children are ordered: child[i] contains keys < separator[i],
	/// child[i+1] contains keys >= separator[i]
	Inner {
		separators: Vec<String>,
		children: Vec<NodeHash>
	},
}

impl Node {
	/// Encode a node to DAG-CBOR bytes
	pub fn encode(&self) -> Result<Vec<u8>, MstError> {
		serde_ipld_dagcbor::to_vec(self).map_err(|e| MstError::DagCbor(e.to_string()))
	}

	/// Decode a node from DAG-CBOR bytes
	pub fn decode(bytes: &[u8]) -> Result<Self, MstError> {
		serde_ipld_dagcbor::from_slice(bytes).map_err(|e| MstError::DagCbor(e.to_string()))
	}

	/// Compute the hash of a node
	pub fn compute_hash(&self) -> Result<NodeHash, MstError> {
		let enc = self.encode()?;
		Ok(hash_data(&enc))
	}
}

/// Hash arbitrary data using BLAKE3
pub fn hash_data(data: &[u8]) -> [u8; 32] {
    let mut hasher = Hasher::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Helper to convert Vec<u8> to ByteBuf for leaf values
pub fn to_bytebuf(v: Vec<u8>) -> ByteBuf {
	ByteBuf::from(v)
}

/// Helper to convert ByteBuf to Vec<u8> for returning values
pub fn from_bytebuf(b: ByteBuf) -> Vec<u8> {
	b.into_vec()
}
