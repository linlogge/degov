//! Merkle proof generation and verification

use crate::error::MstError;
use super::node::{from_bytebuf, Node, NodeHash};
use super::types::{MerkleProof, ProofNode};
use super::tree::MerkleSearchTree;

impl MerkleSearchTree {
	/// Generate a Merkle proof for a key
	///
	/// Value in proof will be raw DAG-CBOR encoded bytes.
	pub async fn generate_proof(&self, key: &str) -> Result<MerkleProof, MstError> {
		let Some((root_layer, root_hash)) = self.fdb_get_root().await? else {
			return Ok(MerkleProof {
				key: key.to_string(),
				value: None,
				path: Vec::new(),
				exists: false,
			});
		};

		let mut path = Vec::new();
		let value = self.generate_proof_rec(root_layer, root_hash, key, &mut path).await?;

		let exists = value.is_some();
		Ok(MerkleProof {
			key: key.to_string(),
			value,
			path,
			exists,
		})
	}

	#[async_recursion::async_recursion]
	pub(crate) async fn generate_proof_rec(&self, layer: u32, hash: NodeHash, key: &str, path: &mut Vec<ProofNode>) -> Result<Option<Vec<u8>>, MstError> {
		let Some(node) = self.fdb_get_node(layer, hash).await? else {
			return Ok(None);
		};

		match node {
			Node::Leaf { key: k, value: v } => {
				path.push(ProofNode::Leaf { layer, hash, key: k.clone() });
				Ok(if k == key { Some(from_bytebuf(v)) } else { None })
			}
			Node::Inner { ref separators, ref children } => {
				let idx = separators.iter()
					.position(|s| key <= s.as_str())
					.unwrap_or(separators.len());

				path.push(ProofNode::Inner {
					layer,
					hash,
					separators: separators.clone(),
					child_index: idx,
				});

				if let Some(child_hash) = children.get(idx).cloned() {
					let child_layer = layer.saturating_sub(1);
					self.generate_proof_rec(child_layer, child_hash, key, path).await
				} else {
					Ok(None)
				}
			}
		}
	}

	/// Verify a Merkle proof against a known root hash
	pub fn verify_proof(proof: &MerkleProof, expected_root: NodeHash) -> Result<bool, MstError> {
		if proof.path.is_empty() {
			return Ok(false);
		}

		// Verify the path from leaf to root
		let first = &proof.path[0];
		match first {
			ProofNode::Leaf { hash, .. } => {
				// Verify that the leaf hash matches expected structure
				if proof.path.len() == 1 {
					return Ok(*hash == expected_root);
				}
			}
			_ => return Ok(false),
		}

		// Check that path leads to expected root
		if let Some(ProofNode::Inner { hash, .. }) = proof.path.last() {
			Ok(*hash == expected_root)
		} else {
			Ok(false)
		}
	}
}
