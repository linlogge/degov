use crate::{digest::ValueDigest, page::Page};

/// Storage of a single key/value pair.
///
/// Keys are stored immutably in the [`Node`], alongside the hash of a value
/// (and not the value itself).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Node<const N: usize, K> {
    key: K,
    value_hash: ValueDigest<N>,

    /// A pointer to a page with a strictly lower tree level, and containing
    /// strictly smaller/less-than keys when compared to "key".
    lt_pointer: Option<Box<Page<N, K>>>,
}

impl<const N: usize, K: AsRef<[u8]>> Node<N, K> {
    pub(crate) const fn new(
        key: K,
        value: ValueDigest<N>,
        lt_pointer: Option<Box<Page<N, K>>>,
    ) -> Self {
        Self {
            key,
            value_hash: value,
            lt_pointer,
        }
    }

    /// Return the key of this node.
    pub fn key(&self) -> &K {
        &self.key
    }

    /// Return the hash of the value for this node.
    pub fn value_hash(&self) -> &ValueDigest<N> {
        &self.value_hash
    }

    pub(crate) fn update_value_hash(&mut self, hash: ValueDigest<N>) {
        self.value_hash = hash;
    }

    pub(crate) fn lt_pointer(&self) -> Option<&Page<N, K>> {
        self.lt_pointer.as_deref()
    }

    pub(crate) fn lt_pointer_mut(&mut self) -> &mut Option<Box<Page<N, K>>> {
        &mut self.lt_pointer
    }
}
