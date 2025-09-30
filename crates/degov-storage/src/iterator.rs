//! Iterator implementations for MST

use serde::de::DeserializeOwned;
use std::marker::PhantomData;

use crate::error::MstError;
use crate::tree::MerkleSearchTree;

/// Iterator over tree entries
///
/// Values are raw DAG-CBOR encoded bytes.
pub struct MstIterator {
	pub(crate) entries: Vec<(String, Vec<u8>)>,
	pub(crate) position: usize,
}

impl Iterator for MstIterator {
	type Item = (String, Vec<u8>);

	fn next(&mut self) -> Option<Self::Item> {
		if self.position < self.entries.len() {
			let item = self.entries[self.position].clone();
			self.position += 1;
			Some(item)
		} else {
			None
		}
	}
}

impl MstIterator {
	pub fn len(&self) -> usize {
		self.entries.len()
	}

	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}
}

/// Typed iterator over tree entries
///
/// Automatically decodes values from DAG-CBOR to the specified type.
pub struct MstIteratorTyped<T> {
	pub(crate) entries: Vec<(String, Vec<u8>)>,
	pub(crate) position: usize,
	pub(crate) _phantom: PhantomData<T>,
}

impl<T: DeserializeOwned> Iterator for MstIteratorTyped<T> {
	type Item = Result<(String, T), MstError>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.position < self.entries.len() {
			let (key, bytes) = &self.entries[self.position];
			self.position += 1;
			Some(MerkleSearchTree::decode_value(bytes).map(|v| (key.clone(), v)))
		} else {
			None
		}
	}
}

impl<T> MstIteratorTyped<T> {
	pub fn len(&self) -> usize {
		self.entries.len()
	}

	pub fn is_empty(&self) -> bool {
		self.entries.is_empty()
	}
}
