use super::PageRange;
use crate::digest::PageDigest;

/// An owned point-in-time snapshot of the [`PageRange`] returned from a call to
/// [`MerkleSearchTree::serialise_page_ranges()`].
///
/// Generating a [`PageRangeSnapshot`] from a set of [`PageRange`] instances
/// clones all the bounding keys in each [`PageRange`], and therefore can only
/// be generated if the key type `K` implements [`Clone`].
///
/// ```
/// # use merkle_search_tree::{*, diff::*};
/// #
/// let mut t = MerkleSearchTree::default();
/// t.upsert("bananas", &42);
///
/// // Rehash the tree before generating the page ranges
/// let _ = t.root_hash();
///
/// // Generate the hashes & page ranges, immutably borrowing the tree
/// let ranges = t.serialise_page_ranges().unwrap();
///
/// // Obtain an owned PageRangeSnapshot from the borrowed PageRange, in turn
/// // releasing the immutable reference to the tree.
/// let snap = PageRangeSnapshot::from(ranges);
///
/// // The tree is now mutable again.
/// t.upsert("platanos", &42);
/// ```
///
/// A [`PageRangeSnapshot`] can also be generated from owned key values using
/// the [`OwnedPageRange`] type to eliminate clones where unnecessary.
///
/// [`MerkleSearchTree::serialise_page_ranges()`]:
///     crate::MerkleSearchTree::serialise_page_ranges
#[derive(Debug, Clone, PartialEq)]
pub struct PageRangeSnapshot<K>(Vec<OwnedPageRange<K>>);

impl<K> PageRangeSnapshot<K> {
    /// Return an iterator of [`PageRange`] from the snapshot content.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = PageRange<'_, K>>
    where
        K: PartialOrd,
    {
        self.0
            .iter()
            .map(|v| PageRange::new(&v.start, &v.end, v.hash.clone()))
    }
}

impl<'a, K> From<Vec<PageRange<'a, K>>> for PageRangeSnapshot<K>
where
    K: Clone,
{
    fn from(value: Vec<PageRange<'a, K>>) -> Self {
        value.into_iter().collect()
    }
}

impl<'a, K> FromIterator<PageRange<'a, K>> for PageRangeSnapshot<K>
where
    K: Clone + 'a,
{
    fn from_iter<T: IntoIterator<Item = PageRange<'a, K>>>(iter: T) -> Self {
        Self(iter.into_iter().map(OwnedPageRange::from).collect())
    }
}

impl<K> From<Vec<OwnedPageRange<K>>> for PageRangeSnapshot<K> {
    fn from(value: Vec<OwnedPageRange<K>>) -> Self {
        value.into_iter().collect()
    }
}

impl<K> FromIterator<OwnedPageRange<K>> for PageRangeSnapshot<K> {
    fn from_iter<T: IntoIterator<Item = OwnedPageRange<K>>>(iter: T) -> Self {
        Self(iter.into_iter().map(OwnedPageRange::from).collect())
    }
}

/// An owned representation of a [`PageRange`] containing an owned key interval
/// & page hash.
///
/// This type can be used to construct a [`PageRangeSnapshot`] from owned values
/// (eliminating key/hash clones).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedPageRange<K> {
    start: K,
    end: K,
    hash: PageDigest,
}

impl<K> OwnedPageRange<K>
where
    K: PartialOrd,
{
    /// Initialise a new [`OwnedPageRange`] for the given inclusive key
    /// interval, and page hash covering the key range.
    ///
    /// # Panics
    ///
    /// If `start` is greater than `end`, this method panics.
    pub fn new(start: K, end: K, hash: PageDigest) -> Self {
        assert!(start <= end);
        Self { start, end, hash }
    }
}

impl<'a, K> From<PageRange<'a, K>> for OwnedPageRange<K>
where
    K: Clone,
{
    fn from(v: PageRange<'a, K>) -> Self {
        Self {
            start: v.start().clone(),
            end: v.end().clone(),
            hash: v.into_hash(),
        }
    }
}
