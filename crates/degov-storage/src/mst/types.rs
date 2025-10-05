//! Supporting types for MST operations

use serde::{Deserialize, Serialize};

use super::node::NodeHash;

/// Statistics about the tree structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TreeStats {
	pub height: u32,
	pub total_nodes: usize,
	pub leaf_count: usize,
	pub inner_count: usize,
}

/// Difference between two trees
///
/// Values are stored as raw DAG-CBOR encoded bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeDiff {
	pub added: Vec<(String, Vec<u8>)>,
	pub removed: Vec<(String, Vec<u8>)>,
	pub modified: Vec<(String, Vec<u8>, Vec<u8>)>,
}

/// A Merkle proof for a key's existence or non-existence
///
/// Value is stored as raw DAG-CBOR encoded bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
	pub key: String,
	pub value: Option<Vec<u8>>,
	pub path: Vec<ProofNode>,
	pub exists: bool,
}

/// A node in a Merkle proof path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProofNode {
	Leaf {
		layer: u32,
		hash: NodeHash,
		key: String,
	},
	Inner {
		layer: u32,
		hash: NodeHash,
		separators: Vec<String>,
		child_index: usize,
	},
}

/// Result of a reconciliation operation
#[derive(Debug, Clone, Default)]
pub struct ReconcileResult {
	pub new_root: Option<(u32, NodeHash)>,
	pub keys_added: usize,
	pub conflicts_resolved: usize,
}
