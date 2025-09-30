//! Reconciliation and sync operations

use crate::error::MstError;
use crate::node::{from_bytebuf, to_bytebuf, Node, NodeHash};
use crate::types::ReconcileResult;
use crate::tree::MerkleSearchTree;

/// Trait for fetching nodes from a remote peer during reconciliation
#[async_trait::async_trait]
pub trait NodeFetcher: Send + Sync {
	async fn fetch_node(&self, layer: u32, hash: NodeHash) -> Result<Option<Vec<u8>>, MstError>;
}

/// Trait for resolving conflicts during tree reconciliation
///
/// When both local and remote trees have modified the same key,
/// the resolver decides which value to keep or how to merge them.
/// Values are raw DAG-CBOR encoded bytes.
pub trait ConflictResolver {
	/// Resolve a conflict between local and remote values
	///
	/// # Arguments
	/// * `key` - The key that has conflicting values
	/// * `local` - The local value (DAG-CBOR encoded bytes)
	/// * `remote` - The remote value (DAG-CBOR encoded bytes)
	///
	/// # Returns
	/// The resolved value to use (DAG-CBOR encoded bytes)
	fn resolve(&self, key: &str, local: &[u8], remote: &[u8]) -> Result<Vec<u8>, MstError>;
}

/// Simple resolver that always prefers the remote value
pub struct PreferRemoteResolver;

impl ConflictResolver for PreferRemoteResolver {
	fn resolve(&self, _key: &str, _local: &[u8], remote: &[u8]) -> Result<Vec<u8>, MstError> {
		Ok(remote.to_vec())
	}
}

/// Simple resolver that always prefers the local value
pub struct PreferLocalResolver;

impl ConflictResolver for PreferLocalResolver {
	fn resolve(&self, _key: &str, local: &[u8], _remote: &[u8]) -> Result<Vec<u8>, MstError> {
		Ok(local.to_vec())
	}
}

impl MerkleSearchTree {
	/// Reconcile this tree with another tree, using a custom conflict resolver
	///
	/// This performs a three-way merge when possible, using the resolver
	/// to handle conflicts when both sides have modified the same key.
	pub async fn reconcile_with<R>(&mut self, other: Option<(u32, NodeHash)>, fetcher: &dyn NodeFetcher, resolver: &R) -> Result<ReconcileResult, MstError>
	where
		R: ConflictResolver + Send + Sync,
	{
		let self_root = self.fdb_get_root().await?;
		let mut result = ReconcileResult::default();

		self.sync_subtree(self_root, other, fetcher, resolver, &mut result).await?;

		// Update our root if reconciliation succeeded
		if let Some(new_root) = result.new_root {
			let tx = self.db.create_trx()?;
			self.fdb_set_root(&tx, new_root.0, new_root.1).await?;
			tx.commit().await?;
			self.root = Some(new_root);
		}

		Ok(result)
	}

	/// Simple reconciliation that prefers remote values on conflict
	pub async fn reconcile_with_simple(&mut self, other: Option<(u32, NodeHash)>, fetcher: &dyn NodeFetcher) -> Result<ReconcileResult, MstError> {
		self.reconcile_with(other, fetcher, &PreferRemoteResolver).await
	}

	#[async_recursion::async_recursion]
	pub(crate) async fn sync_subtree<R>(&self, a: Option<(u32, NodeHash)>, b: Option<(u32, NodeHash)>, fetcher: &dyn NodeFetcher, resolver: &R, result: &mut ReconcileResult) -> Result<Option<(u32, NodeHash)>, MstError>
	where
		R: ConflictResolver + Send + Sync,
	{
		match (a, b) {
			(None, None) => Ok(None),
			(None, Some((layer_b, hash_b))) => {
				// Pull entire subtree from peer
				self.fetch_and_store_recursive(layer_b, hash_b, fetcher).await?;
				result.keys_added += 1;
				result.new_root = Some((layer_b, hash_b));
				Ok(Some((layer_b, hash_b)))
			}
			(Some(x), None) => {
				result.new_root = Some(x);
				Ok(Some(x))
			}
			(Some((layer_a, hash_a)), Some((layer_b, hash_b))) => {
				if hash_a == hash_b {
					result.new_root = Some((layer_a, hash_a));
					return Ok(Some((layer_a, hash_a)));
				}
				let node_a = self.fdb_get_node(layer_a, hash_a).await?;
				let node_b = if let Some(n) = self.fdb_get_node(layer_b, hash_b).await? { Some(n) } else {
					// fetch missing b
					if let Some(raw) = fetcher.fetch_node(layer_b, hash_b).await? {
						let tx = self.db.create_trx()?;
						self.fdb_put_node_raw(&tx, layer_b, hash_b, &raw).await?;
						tx.commit().await?;
					}
					self.fdb_get_node(layer_b, hash_b).await?
				};

				match (node_a, node_b) {
					(None, None) => {
						result.new_root = Some((layer_a, hash_a));
						Ok(Some((layer_a, hash_a)))
					}
					(Some(Node::Leaf { key: ka, value: va }), Some(Node::Leaf { key: kb, value: vb })) => {
						if ka == kb {
							// Conflict: same key modified on both sides
							let va_vec = from_bytebuf(va);
							let vb_vec = from_bytebuf(vb);
							let resolved = resolver.resolve(&ka, &va_vec, &vb_vec)?;
							let tx = self.db.create_trx()?;
							let leaf = Node::Leaf { key: kb.clone(), value: to_bytebuf(resolved) };
							let h = self.fdb_put_node(&tx, layer_a, &leaf).await?;
							tx.commit().await?;
							result.conflicts_resolved += 1;
							result.new_root = Some((layer_a, h));
							Ok(Some((layer_a, h)))
						} else {
							// keys differ: pull remote subtree
							self.fetch_and_store_recursive(layer_b, hash_b, fetcher).await?;
							result.keys_added += 1;
							result.new_root = Some((layer_b, hash_b));
							Ok(Some((layer_b, hash_b)))
						}
					}
					(Some(Node::Inner { separators: sa, children: ca }), Some(Node::Inner { separators: sb, children: cb })) => {
						// If structures align, descend pairwise; else fallback to full fetch of remote
						if sa == sb && ca.len() == cb.len() {
							let child_layer_a = layer_a.saturating_sub(1);
							let child_layer_b = layer_b.saturating_sub(1);
							let mut new_children = Vec::with_capacity(ca.len());
							for (ha, hb) in ca.into_iter().zip(cb.into_iter()) {
								let res = self.sync_subtree(Some((child_layer_a, ha)), Some((child_layer_b, hb)), fetcher, resolver, result).await?;
								new_children.push(res.map(|(_, h)| h).unwrap_or(ha));
							}
							let tx = self.db.create_trx()?;
							let new_inner = Node::Inner { separators: sa, children: new_children };
							let new_hash = self.fdb_put_node(&tx, layer_a, &new_inner).await?;
							tx.commit().await?;
							result.new_root = Some((layer_a, new_hash));
							Ok(Some((layer_a, new_hash)))
						} else {
							self.fetch_and_store_recursive(layer_b, hash_b, fetcher).await?;
							result.new_root = Some((layer_b, hash_b));
							Ok(Some((layer_b, hash_b)))
						}
					}
					(_, Some(_)) => {
						self.fetch_and_store_recursive(layer_b, hash_b, fetcher).await?;
						result.new_root = Some((layer_b, hash_b));
						Ok(Some((layer_b, hash_b)))
					}
					(Some(_), None) => {
						result.new_root = Some((layer_a, hash_a));
						Ok(Some((layer_a, hash_a)))
					}
				}
			}
		}
	}

	#[async_recursion::async_recursion]
	pub(crate) async fn fetch_and_store_recursive(&self, layer: u32, hash: NodeHash, fetcher: &dyn NodeFetcher) -> Result<(), MstError> {
		if self.fdb_get_node(layer, hash).await?.is_some() { return Ok(()); }
		let Some(raw) = fetcher.fetch_node(layer, hash).await? else { return Ok(()) };
		// Store node
		let tx = self.db.create_trx()?;
		self.fdb_put_node_raw(&tx, layer, hash, &raw).await?;
		tx.commit().await?;
		// Decode to traverse children
		let node: Node = Node::decode(&raw)?;
		if let Node::Inner { separators: _, children } = node {
			let child_layer = layer.saturating_sub(1);
			for ch in children {
				self.fetch_and_store_recursive(child_layer, ch, fetcher).await?;
			}
		}
		Ok(())
	}
}
