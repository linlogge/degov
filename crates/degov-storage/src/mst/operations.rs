//! Tree operations (insert, get, delete, batch, diff, stats)

use foundationdb::Transaction;

use crate::error::MstError;
use super::iterator::{MstIterator, MstIteratorTyped};
use super::node::{from_bytebuf, to_bytebuf, Node, NodeHash, B};
use super::tree::MerkleSearchTree;
use super::types::{TreeDiff, TreeStats};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::marker::PhantomData;

impl MerkleSearchTree {
	/// Insert or update a key-value pair in the MST
	///
	/// This properly uses layer computation based on key hashing to determine
	/// where leaves should be placed in the tree structure.
	/// Value must be DAG-CBOR encoded bytes.
	pub async fn put(&mut self, key: String, value: Vec<u8>) -> Result<(), MstError> {
		let tx = self.db.create_trx()?;
		let current_root = self.fdb_get_root_with_tx(&tx).await?;
		let key_layer = Self::compute_layer(&key);
		let (new_layer, new_root) = self.insert_rec(&tx, current_root, key, value, key_layer).await?;
		self.fdb_set_root(&tx, new_layer, new_root).await?;
		tx.commit().await?;
		self.root = Some((new_layer, new_root));
		Ok(())
	}

	#[async_recursion::async_recursion]
	pub(crate) async fn insert_rec(&self, tx: &Transaction, node: Option<(u32, NodeHash)>, key: String, value: Vec<u8>, key_layer: u32) -> Result<(u32, NodeHash), MstError> {
		match node {
			None => {
				// Empty tree: create leaf at its computed layer
				let leaf = Node::Leaf { key, value: to_bytebuf(value) };
				let h = self.fdb_put_node(tx, key_layer, &leaf).await?;
				Ok((key_layer, h))
			}
			Some((node_layer, hash)) => {
				let existing = self.fdb_get_node(node_layer, hash).await?
					.ok_or(MstError::NodeNotFound)?;

				match existing {
					Node::Leaf { key: existing_key, value: existing_value } => {
						if existing_key == key {
							// Update existing leaf
							let leaf = Node::Leaf { key, value: to_bytebuf(value) };
							let h = self.fdb_put_node(tx, key_layer, &leaf).await?;
							Ok((key_layer, h))
						} else {
							// Split: create inner node to hold both leaves
							let existing_layer = Self::compute_layer(&existing_key);

							// Determine which layer this inner node should be at
							// It should be at the max of the two leaf layers
							let inner_layer = std::cmp::max(key_layer, existing_layer);

							// Store both leaves at their respective layers
							let new_leaf = Node::Leaf { key: key.clone(), value: to_bytebuf(value) };
							let new_leaf_hash = self.fdb_put_node(tx, key_layer, &new_leaf).await?;

							let existing_leaf = Node::Leaf { key: existing_key.clone(), value: existing_value };
							let existing_leaf_hash = self.fdb_put_node(tx, existing_layer, &existing_leaf).await?;

							// Create inner node with both leaves
							let (separators, children) = if key < existing_key {
								(vec![existing_key], vec![new_leaf_hash, existing_leaf_hash])
							} else {
								(vec![key], vec![existing_leaf_hash, new_leaf_hash])
							};

							let inner = Node::Inner { separators, children };
							let h = self.fdb_put_node(tx, inner_layer, &inner).await?;
							Ok((inner_layer, h))
						}
					}
					Node::Inner { separators, children } => {
						// Find the position where the key should go
						let idx = separators.iter()
							.position(|s| key.as_str() <= s.as_str())
							.unwrap_or(separators.len());

						let child_hash = children.get(idx).cloned();
						let child_layer = node_layer.saturating_sub(1);

						// Recursively insert into the appropriate child
						let (_new_child_layer, new_child_hash) = self.insert_rec(
							tx,
							child_hash.map(|h| (child_layer, h)),
							key.clone(),
							value,
							key_layer
						).await?;

						// Update the inner node with the new child
						let mut new_children = children.clone();
						if idx < new_children.len() {
							new_children[idx] = new_child_hash;
						} else {
							new_children.push(new_child_hash);
						}

						// Check if we need to rebalance (split large nodes)
						if new_children.len() > (B as usize) * 2 {
							self.split_node(tx, node_layer, separators, new_children).await
						} else {
							let new_inner = Node::Inner { separators, children: new_children };
							let h = self.fdb_put_node(tx, node_layer, &new_inner).await?;
							Ok((node_layer, h))
						}
					}
				}
			}
		}
	}

	/// Split a node that has grown too large
	pub(crate) async fn split_node(&self, tx: &Transaction, layer: u32, separators: Vec<String>, children: Vec<NodeHash>) -> Result<(u32, NodeHash), MstError> {
		let mid = children.len() / 2;

		let left_children = children[..mid].to_vec();
		let right_children = children[mid..].to_vec();

		let left_seps = separators[..mid.saturating_sub(1)].to_vec();
		let right_seps = if mid < separators.len() {
			separators[mid..].to_vec()
		} else {
			Vec::new()
		};

		let split_key = separators.get(mid.saturating_sub(1))
			.cloned()
			.unwrap_or_else(|| String::new());

		let left_node = Node::Inner { separators: left_seps, children: left_children };
		let right_node = Node::Inner { separators: right_seps, children: right_children };

		let left_hash = self.fdb_put_node(tx, layer, &left_node).await?;
		let right_hash = self.fdb_put_node(tx, layer, &right_node).await?;

		// Create new parent
		let parent = Node::Inner {
			separators: vec![split_key],
			children: vec![left_hash, right_hash],
		};
		let parent_hash = self.fdb_put_node(tx, layer + 1, &parent).await?;

		Ok((layer + 1, parent_hash))
	}

	/// Get a value by key
	///
	/// Returns the raw DAG-CBOR encoded bytes, or None if key doesn't exist.
	pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, MstError> {
		// Naive traversal: if root is a leaf and matches, return. If inner, linear scan of separators.
		let Some((mut layer, root_hash)) = self.fdb_get_root().await? else { return Ok(None) };
		let mut node = match self.fdb_get_node(layer, root_hash).await? { Some(n) => n, None => return Ok(None) };
		loop {
			match node {
				Node::Leaf { key: k, value: v } => {
					return Ok(if k == key { Some(from_bytebuf(v)) } else { None });
				}
				Node::Inner { separators, children } => {
					// Find child index: first separator >= key
					let mut idx = 0usize;
					while idx < separators.len() && key > separators[idx].as_str() { idx += 1; }
					let child_hash = children.get(idx).cloned();
					match child_hash {
						Some(h) => {
							let child_layer = layer.saturating_sub(1);
							node = match self.fdb_get_node(child_layer, h).await? { Some(n) => n, None => return Ok(None) };
							layer = child_layer;
						}
						None => return Ok(None),
					}
				}
			}
		}
	}

	/// Get all key-value pairs in a range [start, end)
	///
	/// Returns raw DAG-CBOR encoded bytes for values.
	pub async fn get_range(&self, start: &str, end: &str) -> Result<Vec<(String, Vec<u8>)>, MstError> {
		let Some((root_layer, root_hash)) = self.fdb_get_root().await? else { return Ok(Vec::new()) };
		let mut results = Vec::new();
		let mut stack: Vec<(u32, Node)> = Vec::new();
		if let Some(root) = self.fdb_get_node(root_layer, root_hash).await? { stack.push((root_layer, root)); }
		while let Some((layer, node)) = stack.pop() {
			match node {
				Node::Leaf { key, value } => {
					if key.as_str() >= start && key.as_str() < end { results.push((key, from_bytebuf(value))); }
				}
				Node::Inner { separators, children } => {
					// Determine child index range that could intersect [start, end)
					let mut i_start = 0usize;
					while i_start < separators.len() && start > separators[i_start].as_str() { i_start += 1; }
					let mut i_end = i_start;
					while i_end < separators.len() && end > separators[i_end].as_str() { i_end += 1; }
					let child_layer = layer.saturating_sub(1);
					let idx_range = i_start..=std::cmp::min(i_end, children.len().saturating_sub(1));
					for idx in idx_range.rev() { // push in reverse to traverse left-to-right when popping
						if let Some(h) = children.get(idx).cloned() {
							if let Some(child) = self.fdb_get_node(child_layer, h).await? {
								stack.push((child_layer, child));
							}
						}
					}
				}
			}
		}
		results.sort_by(|a, b| a.0.cmp(&b.0));
		Ok(results)
	}

	pub async fn delete(&mut self, key: &str) -> Result<(), MstError> {
		let Some((root_layer, root_hash)) = self.fdb_get_root().await? else { return Ok(()) };
		let tx = self.db.create_trx()?;
		let (_new_layer, new_hash, removed) = self.delete_rec(&tx, root_layer, Some(root_hash), key).await?;
		if removed {
			if let Some(h) = new_hash {
				self.fdb_set_root(&tx, _new_layer, h).await?;
				self.root = Some((_new_layer, h));
			} else {
				// Tree became empty
				// Clear root key
				tx.clear(&Self::key_root());
				self.root = None;
			}
			tx.commit().await?;
		}
		Ok(())
	}

	#[async_recursion::async_recursion]
	pub(crate) async fn delete_rec(&self, tx: &Transaction, layer: u32, node_hash: Option<NodeHash>, key: &str) -> Result<(u32, Option<NodeHash>, bool), MstError> {
		let Some(h) = node_hash else { return Ok((layer, None, false)) };
		let Some(node) = self.fdb_get_node(layer, h).await? else { return Ok((layer, Some(h), false)) };
		match node {
			Node::Leaf { key: k, value: _ } => {
				if k == key { Ok((layer, None, true)) } else { Ok((layer, Some(h), false)) }
			}
			Node::Inner { mut separators, mut children } => {
				let mut idx = 0usize;
				while idx < separators.len() && key > separators[idx].as_str() { idx += 1; }
				let child_layer = layer.saturating_sub(1);
				let target_hash = children.get(idx).cloned();
				let (child_layer, new_child_hash, removed) = self.delete_rec(tx, child_layer, target_hash, key).await?;
				if !removed { return Ok((layer, Some(h), false)); }
				match new_child_hash {
					Some(new_h) => {
						children[idx] = new_h;
						// Recompute node
						let new_node = Node::Inner { separators, children };
						let new_hash = self.fdb_put_node(tx, layer, &new_node).await?;
						Ok((layer, Some(new_hash), true))
					}
					None => {
						// Child disappeared; remove it and possibly adjust separator
						children.remove(idx);
						if idx < separators.len() { separators.remove(idx); } else if !separators.is_empty() { separators.pop(); }
						if children.len() == 0 {
							Ok((layer, None, true))
						} else if children.len() == 1 {
							// Collapse
							Ok((child_layer, Some(children[0]), true))
						} else {
							let new_node = Node::Inner { separators, children };
							let new_hash = self.fdb_put_node(tx, layer, &new_node).await?;
							Ok((layer, Some(new_hash), true))
						}
					}
				}
			}
		}
	}

	/// Get tree statistics
	pub async fn stats(&self) -> Result<TreeStats, MstError> {
		let Some((root_layer, root_hash)) = self.fdb_get_root().await? else {
			return Ok(TreeStats::default());
		};

		let mut stats = TreeStats {
			height: root_layer + 1,
			..Default::default()
		};

		self.collect_stats(root_layer, root_hash, &mut stats).await?;
		Ok(stats)
	}

	#[async_recursion::async_recursion]
	pub(crate) async fn collect_stats(&self, layer: u32, hash: NodeHash, stats: &mut TreeStats) -> Result<(), MstError> {
		let Some(node) = self.fdb_get_node(layer, hash).await? else {
			return Ok(());
		};

		stats.total_nodes += 1;

		match node {
			Node::Leaf { .. } => {
				stats.leaf_count += 1;
			}
			Node::Inner { children, .. } => {
				stats.inner_count += 1;
				let child_layer = layer.saturating_sub(1);
				for child_hash in children {
					self.collect_stats(child_layer, child_hash, stats).await?;
				}
			}
		}

		Ok(())
	}

	/// Compute difference between this tree and another tree root
	///
	/// Values in diff will be raw DAG-CBOR encoded bytes.
	pub async fn diff(&self, other_root: Option<(u32, NodeHash)>) -> Result<TreeDiff, MstError> {
		let self_root = self.fdb_get_root().await?;
		let mut diff = TreeDiff {
			added: Vec::new(),
			removed: Vec::new(),
			modified: Vec::new(),
		};

		self.diff_rec(self_root, other_root, &mut diff).await?;
		Ok(diff)
	}

	#[async_recursion::async_recursion]
	pub(crate) async fn diff_rec(&self, a: Option<(u32, NodeHash)>, b: Option<(u32, NodeHash)>, diff: &mut TreeDiff) -> Result<(), MstError> {
		match (a, b) {
			(None, None) => Ok(()),
			(Some((layer, hash)), None) => {
				// All keys in 'a' are removed
				self.collect_all_keys(layer, hash, &mut diff.removed).await
			}
			(None, Some((layer, hash))) => {
				// All keys in 'b' are added
				self.collect_all_keys(layer, hash, &mut diff.added).await
			}
			(Some((layer_a, hash_a)), Some((layer_b, hash_b))) => {
				if hash_a == hash_b {
					return Ok(());
				}

				let node_a = self.fdb_get_node(layer_a, hash_a).await?;
				let node_b = self.fdb_get_node(layer_b, hash_b).await?;

				match (&node_a, &node_b) {
					(Some(Node::Leaf { key: ka, value: va }), Some(Node::Leaf { key: kb, value: vb })) => {
						if ka == kb {
							diff.modified.push((ka.clone(), from_bytebuf(va.clone()), from_bytebuf(vb.clone())));
						} else {
							diff.removed.push((ka.clone(), from_bytebuf(va.clone())));
							diff.added.push((kb.clone(), from_bytebuf(vb.clone())));
						}
					}
					(Some(Node::Inner { children: ca, .. }), Some(Node::Inner { children: cb, .. })) => {
						let child_layer = std::cmp::min(layer_a, layer_b).saturating_sub(1);
						let max_len = std::cmp::max(ca.len(), cb.len());
						for i in 0..max_len {
							let child_a = ca.get(i).map(|&h| (child_layer, h));
							let child_b = cb.get(i).map(|&h| (child_layer, h));
							self.diff_rec(child_a, child_b, diff).await?;
						}
					}
					_ => {
						if let Some(node) = node_a {
							self.collect_node_keys(layer_a, hash_a, node, &mut diff.removed).await?;
						}
						if let Some(node) = node_b {
							self.collect_node_keys(layer_b, hash_b, node, &mut diff.added).await?;
						}
					}
				}

				Ok(())
			}
		}
	}

	#[async_recursion::async_recursion]
	pub(crate) async fn collect_all_keys(&self, layer: u32, hash: NodeHash, keys: &mut Vec<(String, Vec<u8>)>) -> Result<(), MstError> {
		let Some(node) = self.fdb_get_node(layer, hash).await? else {
			return Ok(());
		};

		self.collect_node_keys(layer, hash, node, keys).await
	}

	#[async_recursion::async_recursion]
	pub(crate) async fn collect_node_keys(&self, layer: u32, _hash: NodeHash, node: Node, keys: &mut Vec<(String, Vec<u8>)>) -> Result<(), MstError> {
		match node {
			Node::Leaf { key, value } => {
				keys.push((key, from_bytebuf(value)));
			}
			Node::Inner { children, .. } => {
				let child_layer = layer.saturating_sub(1);
				for child_hash in children {
					self.collect_all_keys(child_layer, child_hash, keys).await?;
				}
			}
		}
		Ok(())
	}

	/// Batch insert multiple key-value pairs
	///
	/// Values must be DAG-CBOR encoded bytes.
	/// 
	/// Note: Large batches may need to be split into smaller chunks to avoid
	/// transaction timeouts (FDB default is 5 seconds).
	pub async fn put_batch(&mut self, entries: Vec<(String, Vec<u8>)>) -> Result<(), MstError> {
		// Process in smaller chunks to avoid transaction timeouts
		const BATCH_SIZE: usize = 100;
		
		for chunk in entries.chunks(BATCH_SIZE) {
			let tx = self.db.create_trx()?;
			// Set a longer timeout for batch operations (default is 5000ms)
			tx.set_option(foundationdb::options::TransactionOption::Timeout(10000))?;
			
			let mut current_root = self.fdb_get_root().await?;

			for (key, value) in chunk {
				let key_layer = Self::compute_layer(&key);
				let (new_layer, new_root) = self.insert_rec(&tx, current_root, key.clone(), value.clone(), key_layer).await?;
				current_root = Some((new_layer, new_root));
			}

			if let Some((layer, hash)) = current_root {
				self.fdb_set_root(&tx, layer, hash).await?;
				tx.commit().await?;
				self.root = Some((layer, hash));
			}
		}

		Ok(())
	}

	/// Batch delete multiple keys
	/// 
	/// Note: Large batches may need to be split into smaller chunks to avoid
	/// transaction timeouts (FDB default is 5 seconds).
	pub async fn delete_batch(&mut self, keys: Vec<&str>) -> Result<(), MstError> {
		// Process in smaller chunks to avoid transaction timeouts
		const BATCH_SIZE: usize = 100;
		
		for chunk in keys.chunks(BATCH_SIZE) {
			let tx = self.db.create_trx()?;
			// Set a longer timeout for batch operations
			tx.set_option(foundationdb::options::TransactionOption::Timeout(10000))?;
			
			let mut current_root = self.fdb_get_root().await?;

			for key in chunk {
				if let Some((root_layer, root_hash)) = current_root {
					let (_new_layer, new_hash, _removed) = self.delete_rec(&tx, root_layer, Some(root_hash), key).await?;
					current_root = new_hash.map(|h| (_new_layer, h));
				}
			}

			if let Some((layer, hash)) = current_root {
				self.fdb_set_root(&tx, layer, hash).await?;
				self.root = Some((layer, hash));
			} else {
				tx.clear(&Self::key_root());
				self.root = None;
			}

			tx.commit().await?;
		}
		
		Ok(())
	}

	/// Iterate over all key-value pairs in order
	///
	/// Returns raw DAG-CBOR encoded bytes for values.
	pub async fn iter(&self) -> Result<MstIterator, MstError> {
		let root = self.fdb_get_root().await?;
		let mut entries = Vec::new();

		if let Some((layer, hash)) = root {
			self.collect_all_keys(layer, hash, &mut entries).await?;
		}

		entries.sort_by(|a, b| a.0.cmp(&b.0));

		Ok(MstIterator {
			entries,
			position: 0,
		})
	}

	// ========== Typed helper methods ==========

	/// Encode a value to bytes using the application's chosen format
	///
	/// For this implementation, values are serialized to JSON format.
	/// The bytes are stored opaquely in the leaf node's value field, which is
	/// marked with serde_bytes to be serialized as a byte array when the Node
	/// itself is DAG-CBOR encoded.
	pub fn encode_value<T: Serialize>(value: &T) -> Result<Vec<u8>, MstError> {
		serde_json::to_vec(value).map_err(|e| MstError::SerdeError(e))
	}

	/// Decode bytes to a typed value using the application's chosen format
	pub fn decode_value<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, MstError> {
		serde_json::from_slice(bytes).map_err(|e| MstError::SerdeError(e))
	}

	/// Insert a typed value (automatically encodes to bytes)
	pub async fn put_typed<T: Serialize>(&mut self, key: String, value: &T) -> Result<(), MstError> {
		let bytes = Self::encode_value(value)?;
		self.put(key, bytes).await
	}

	/// Get a typed value (automatically decodes from bytes)
	pub async fn get_typed<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, MstError> {
		match self.get(key).await? {
			Some(bytes) => Ok(Some(Self::decode_value(&bytes)?)),
			None => Ok(None),
		}
	}

	/// Get a range of typed values
	pub async fn get_range_typed<T: DeserializeOwned>(&self, start: &str, end: &str) -> Result<Vec<(String, T)>, MstError> {
		let raw_results = self.get_range(start, end).await?;
		raw_results
			.into_iter()
			.map(|(k, v)| Ok((k, Self::decode_value(&v)?)))
			.collect()
	}

	/// Batch insert typed values
	pub async fn put_batch_typed<T: Serialize>(&mut self, entries: Vec<(String, T)>) -> Result<(), MstError> {
		let encoded: Result<Vec<_>, MstError> = entries
			.into_iter()
			.map(|(k, v)| Ok::<_, MstError>((k, Self::encode_value(&v)?)))
			.collect();
		self.put_batch(encoded?).await
	}

	/// Iterate over typed values
	pub async fn iter_typed<T: DeserializeOwned>(&self) -> Result<MstIteratorTyped<T>, MstError> {
		let root = self.fdb_get_root().await?;
		let mut entries = Vec::new();

		if let Some((layer, hash)) = root {
			self.collect_all_keys(layer, hash, &mut entries).await?;
		}

		entries.sort_by(|a, b| a.0.cmp(&b.0));

		Ok(MstIteratorTyped {
			entries,
			position: 0,
			_phantom: PhantomData,
		})
	}
}
