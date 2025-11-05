//! Core Merkle Search Tree implementation

use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use foundationdb::{Database, Transaction};
use rand::RngCore;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::MstError;
use super::node::{hash_data, Node, NodeHash};

/// In-memory cache for nodes to reduce FDB reads
type NodeCache = Arc<tokio::sync::RwLock<HashMap<(u32, NodeHash), Node>>>;

/// Merkle Search Tree implementation backed by FoundationDB
///
/// This implements a content-addressed tree structure where:
/// - Keys are sorted lexicographically
/// - Each key is assigned a layer based on leading zeros in its hash
/// - Nodes are stored by (layer, hash) in FDB
/// - Tree structure enables efficient sync via hash comparison
/// - Values are stored as raw bytes (DAG-CBOR encoded)
#[derive(Clone)]
pub struct MerkleSearchTree {
    pub(crate) db: Arc<Database>,
	pub(crate) root: Option<(u32, NodeHash)>,
    pub(crate) cache: NodeCache,
}

impl MerkleSearchTree {
	pub async fn new(db: Database) -> Result<Self, MstError> {
		Self::open(db).await
	}

	pub async fn open(db: Database) -> Result<Self, MstError> {
		let db = Arc::new(db);
		let cache = Arc::new(tokio::sync::RwLock::new(HashMap::new()));
		let tmp = Self { db: db.clone(), root: None, cache: cache.clone() };
		let root = tmp.fdb_get_root().await?;
		Ok(Self { db, root, cache })
	}

	/// Get the root hash of the tree
	pub fn root_hash(&self) -> Option<NodeHash> {
		self.root.map(|(_, hash)| hash)
	}

	/// Clear the cache
	pub async fn clear_cache(&self) {
		let mut cache = self.cache.write().await;
		cache.clear();
	}

	// ========== FDB operations ==========

	pub(crate) fn key_root() -> Vec<u8> {
		b"mstr".to_vec()
	}

	pub(crate) fn key_node(layer: u32, hash: NodeHash) -> Vec<u8> {
		let mut k = Vec::with_capacity(4 + 4 + 32);
		k.extend_from_slice(b"mstn");
		k.extend_from_slice(&layer.to_be_bytes());
		k.extend_from_slice(&hash);
		k
	}

	pub(crate) async fn fdb_get_root(&self) -> Result<Option<(u32, NodeHash)>, MstError> {
		let tx = self.db.create_trx()?;
		let result = self.fdb_get_root_with_tx(&tx).await?;
		// Explicitly cancel read-only transaction to release resources
		tx.cancel();
		Ok(result)
	}

	pub(crate) async fn fdb_get_root_with_tx(&self, tx: &Transaction) -> Result<Option<(u32, NodeHash)>, MstError> {
		if let Some(bytes) = tx.get(&Self::key_root(), false).await? {
			let data = bytes.as_ref();
			if data.len() != 4 + 32 { return Ok(None); }
			let mut layer_bytes = [0u8; 4];
			layer_bytes.copy_from_slice(&data[0..4]);
			let layer = u32::from_be_bytes(layer_bytes);
			let mut hash = [0u8; 32];
			hash.copy_from_slice(&data[4..36]);
			Ok(Some((layer, hash)))
		} else {
			Ok(None)
		}
	}

	pub(crate) async fn fdb_set_root(&self, tx: &Transaction, layer: u32, hash: NodeHash) -> Result<(), MstError> {
		let mut v = Vec::with_capacity(4 + 32);
		v.extend_from_slice(&layer.to_be_bytes());
		v.extend_from_slice(&hash);
		tx.set(&Self::key_root(), &v);
		Ok(())
	}

	pub(crate) async fn fdb_get_node(&self, layer: u32, hash: NodeHash) -> Result<Option<Node>, MstError> {
		// Check cache first
		{
			let cache = self.cache.read().await;
			if let Some(node) = cache.get(&(layer, hash)) {
				return Ok(Some(node.clone()));
			}
		}

		// Fetch from FDB
		let tx = self.db.create_trx()?;
		let key = Self::key_node(layer, hash);
		let result = if let Some(bytes) = tx.get(&key, false).await? {
			let node = Node::decode(bytes.as_ref())?;

			// Update cache
			{
				let mut cache = self.cache.write().await;
				cache.insert((layer, hash), node.clone());
			}

			Some(node)
		} else {
			None
		};
		
		// Explicitly cancel read-only transaction to release resources
		tx.cancel();
		Ok(result)
	}

	pub(crate) async fn fdb_put_node(&self, tx: &Transaction, layer: u32, node: &Node) -> Result<NodeHash, MstError> {
		let hash = node.compute_hash()?;
		let key = Self::key_node(layer, hash);
		let val = node.encode()?;
		tx.set(&key, &val);

		// Update cache
		{
			let mut cache = self.cache.write().await;
			cache.insert((layer, hash), node.clone());
		}

		Ok(hash)
	}

	pub(crate) async fn fdb_put_node_raw(&self, tx: &Transaction, layer: u32, hash: NodeHash, raw: &[u8]) -> Result<(), MstError> {
		let key = Self::key_node(layer, hash);
		tx.set(&key, raw);
		Ok(())
	}

	// ========== Encryption helpers ==========

	pub fn encrypt_required_fields<T: Serialize>(value: &T, key_bytes: &[u8; 32]) -> Result<Vec<u8>, MstError> {
		let plaintext = serde_json::to_vec(value)?;
		let key = Key::<Aes256Gcm>::from_slice(key_bytes);
		let cipher = Aes256Gcm::new(key);
		let mut nonce_bytes = [0u8; 12];
		rand::thread_rng().fill_bytes(&mut nonce_bytes);
		let nonce = Nonce::from_slice(&nonce_bytes);
		let ciphertext = cipher.encrypt(nonce, plaintext.as_ref()).map_err(|e| MstError::DagCbor(format!("encrypt: {e}")))?;
		let mut out = Vec::with_capacity(12 + ciphertext.len());
		out.extend_from_slice(&nonce_bytes);
		out.extend_from_slice(&ciphertext);
		Ok(out)
	}

	pub fn decrypt_required_fields<T: DeserializeOwned>(ciphertext_with_nonce: &[u8], key_bytes: &[u8; 32]) -> Result<T, MstError> {
		if ciphertext_with_nonce.len() < 12 { return Err(MstError::DagCbor("ciphertext too short".into())); }
		let (nonce_bytes, ciphertext) = ciphertext_with_nonce.split_at(12);
		let key = Key::<Aes256Gcm>::from_slice(key_bytes);
		let cipher = Aes256Gcm::new(key);
		let nonce = Nonce::from_slice(nonce_bytes);
		let plaintext = cipher.decrypt(nonce, ciphertext).map_err(|e| MstError::DagCbor(format!("decrypt: {e}")))?;
		Ok(serde_json::from_slice(&plaintext)?)
	}

	// ========== Layer computation ==========

    pub(crate) fn compute_layer(key: &str) -> u32 {
        let hash = hash_data(key.as_bytes());
        let hex = hex::encode(&hash);
        let mut layer = 0;
        for c in hex.chars() {
            if c == '0' {
                layer += 1;
            } else {
                break;
            }
        }
        layer
    }
}
