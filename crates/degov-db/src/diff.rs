//! Tree difference calculation algorithm & associated types.

mod diff_builder;
mod page_range;
mod page_range_snapshot;
mod range_list;

use std::{fmt::Debug, iter::Peekable};

pub use page_range::*;
pub use page_range_snapshot::*;
use tracing::{debug, trace};

use crate::diff::diff_builder::DiffListBuilder;

/// An inclusive range of keys that differ between two serialised ordered sets
/// of [`PageRange`].
#[derive(Debug, PartialEq)]
pub struct DiffRange<'a, K> {
    /// The inclusive start & end key bounds of this range to sync.
    start: &'a K,
    end: &'a K,
}

impl<'a, K> Clone for DiffRange<'a, K> {
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            end: self.end,
        }
    }
}

impl<'a, K> DiffRange<'a, K> {
    /// Returns true if the range within `self` overlaps any portion of the
    /// range within `p`.
    pub(crate) fn overlaps(&self, p: &Self) -> bool
    where
        K: PartialOrd,
    {
        p.end() >= self.start() && p.start() <= self.end()
    }

    /// Returns the inclusive start of this [`DiffRange`], identifying the start
    /// of an inconsistency between trees.
    pub fn start(&self) -> &K {
        self.start
    }

    /// Returns the inclusive end of this [`DiffRange`], identifying the end of
    /// an inconsistency between trees.
    pub fn end(&self) -> &K {
        self.end
    }
}

/// Compute the difference between `local` and `peer`, returning the set of
/// [`DiffRange`] covering the inconsistent key ranges found in `peer`.
///
/// ```rust
/// use merkle_search_tree::{MerkleSearchTree, diff::diff};
///
/// // Initialise a "peer" tree.
/// let mut node_a = MerkleSearchTree::default();
/// node_a.upsert("bananas", &42);
/// node_a.upsert("plátanos", &42);
///
/// // Initialise the "local" tree with differing keys
/// let mut node_b = MerkleSearchTree::default();
/// node_b.upsert("donkey", &42);
///
/// // Generate the tree hashes before serialising the page ranges
/// node_a.root_hash();
/// node_b.root_hash();
///
/// // Generate the tree page bounds & hashes, and feed into the diff function
/// let diff_range = diff(
///     node_b.serialise_page_ranges().unwrap().into_iter(),
///     node_a.serialise_page_ranges().unwrap().into_iter(),
/// );
///
/// // The diff_range contains all the inclusive key intervals the "local" tree
/// // should fetch from the "peer" tree to converge.
/// assert_matches::assert_matches!(diff_range.as_slice(), [range] => {
///     assert_eq!(range.start(), &"bananas");
///     assert_eq!(range.end(), &"plátanos");
/// });
/// ```
///
/// # State Convergence
///
/// To converge the state of the two trees, the key ranges in the returned
/// [`DiffRange`] instances should be requested from `peer` and used to update
/// the state of `local`.
///
/// If `local` is a superset of `peer` (contains all the keys in `peer` and the
/// values are consistent), or the two trees are identical, no [`DiffRange`]
/// intervals are returned.
///
/// # Termination
///
/// A single invocation to [`diff()`] always terminates, and completes in `O(n)`
/// time and space. Inconsistent page ranges (if any) are minimised in
/// `O(n_consistent * n_inconsistent)` time and `O(n)` space.
///
/// In the absence of further external updates to either tree, this algorithm
/// terminates (leaving `local` and `peer` fully converged) and no diff is
/// returned within a finite number of sync rounds between the two trees.
///
/// If a one-way sync is performed (pulling inconsistent keys from `peer` and
/// updating `local`, but never syncing the other way around) this algorithm MAY
/// NOT terminate.
pub fn diff<'p, 'a: 'p, T, U, K>(local: T, peer: U) -> Vec<DiffRange<'p, K>>
where
    T: IntoIterator<Item = PageRange<'a, K>>,
    U: IntoIterator<Item = PageRange<'p, K>>,
    K: PartialOrd + Ord + Debug + 'p + 'a,
{
    let local = local.into_iter();
    let peer = peer.into_iter();

    // Any two merkle trees can be expressed as a series of overlapping page
    // ranges, either consistent in content (hashes match), or inconsistent
    // (hashes differ).
    //
    // This algorithm builds two sets of intervals - one for key ranges that are
    // fully consistent between the two trees, and one for inconsistent ranges.
    //
    // This DiffListBuilder helps construct these lists, and merges them into a
    // final, non-overlapping, deduplicated, and minimised set of ranges that
    // are inconsistent between trees as described above.
    let mut diff_builder = DiffListBuilder::default();

    let mut local = local.peekable();
    let mut peer = peer.peekable();

    debug!("calculating diff");

    let root = match peer.peek() {
        Some(v) => v.clone(),
        None => return vec![],
    };

    recurse_diff(&root, &mut peer, &mut local, &mut diff_builder);

    diff_builder.into_diff_vec()
}

#[tracing::instrument(skip(peer, local))]
fn recurse_subtree<'p, 'a: 'p, T, U, K>(
    subtree_root: &PageRange<'p, K>,
    peer: &mut Peekable<U>,
    local: &mut Peekable<T>,
    diff_builder: &mut DiffListBuilder<'p, K>,
) -> bool
where
    T: Iterator<Item = PageRange<'a, K>>,
    U: Iterator<Item = PageRange<'p, K>>,
    K: PartialOrd + Ord + Debug + 'p + 'a,
{
    // Recurse into the subtree, which will exit immediately if the next value
    // in peer is not rooted at subtree_root (without consuming the iter value).
    recurse_diff(subtree_root, peer, local, diff_builder);

    // Invariant - when returning from this call, the entire subtree rooted at
    // the peer_subtree_root should have been evaluated and the next peer page
    // (if any) escapes the subtree.

    while let Some(p) = peer.next_if(|v| subtree_root.is_superset_of(v)) {
        debug!(
            peer_page=?p,
            "requesting unevaluated subtree page"
        );
        // Add all the un-evaluated peer sub-tree pages to the sync list.
        diff_builder.inconsistent(p.start(), p.end());
    }

    debug_assert!(
        peer.peek()
            .map(|v| !subtree_root.is_superset_of(v))
            .unwrap_or(true)
    );

    true
}

#[tracing::instrument(skip(peer, local))]
fn recurse_diff<'p, 'a: 'p, T, U, K>(
    subtree_root: &PageRange<'p, K>,
    peer: &mut Peekable<U>,
    local: &mut Peekable<T>,
    diff_builder: &mut DiffListBuilder<'p, K>,
) where
    T: Iterator<Item = PageRange<'a, K>>,
    U: Iterator<Item = PageRange<'p, K>>,
    K: PartialOrd + Ord + Debug + 'p + 'a,
{
    // The last visited peer page, if any.
    let mut last_p = None;

    // Process this subtree, descending down inconsistent paths recursively, and
    // iterating through the tree.
    loop {
        let p = match maybe_advance_within(subtree_root, peer) {
            Some(v) => v,
            None => {
                trace!("no more peer pages in subtree");
                return;
            }
        };

        let mut l = match maybe_advance_within(&p, local) {
            Some(v) => v,
            None => {
                // If the local subtree range is a superset of the peer subtree
                // range, the two are guaranteed to be inconsistent due to the
                // local node containing more keys (potentially the sole cause
                // of that inconsistency).
                //
                // Fetching any pages from the less-up-to-date peer may be
                // spurious, causing no useful advancement of state.
                if let Some(local) = local.peek() {
                    if local.is_superset_of(&p) {
                        trace!(
                            peer_page=?p,
                            local_page=?local,
                            "local page is a superset of peer"
                        );
                        return;
                    }
                }

                // If there's no matching local page that overlaps with p, then
                // there must be be one or more keys to be synced from the peer
                // to populate the missing local pages.
                //
                // Request the range starting from the end of the last checked p
                // (last_p), or the start of the subtree_root if none.
                let start = last_p
                    .as_ref()
                    .map(|v: &PageRange<'_, K>| v.end())
                    .unwrap_or(subtree_root.start());
                // And end at the next local page key, or the page end.
                //
                // Any pages missing between p.end and the end of this subtree
                // will be added by the caller (recurse_subtree).
                let end = local
                    .peek()
                    .map(|v| v.start().min(p.end()))
                    .unwrap_or(p.end());
                if end >= start {
                    debug!(
                        peer_page=?p,
                        "no more local pages in subtree - requesting missing page ranges"
                    );
                    diff_builder.inconsistent(start, end);
                } else {
                    trace!(
                        peer_page=?p,
                        "no more local pages in subtree"
                    );
                }

                return;
            }
        };

        last_p = Some(p.clone());

        debug_assert!(subtree_root.is_superset_of(&p));

        trace!(
            peer_page=?p,
            local_page=?l,
            "visit page"
        );

        // Advance the local cursor to minimise the comparable range, in turn
        // minimising the sync range.
        while let Some(v) = local.next_if(|v| v.is_superset_of(&p)) {
            trace!(
                peer_page=?p,
                skip_local_page=?l,
                local_page=?v,
                "shrink local diff range"
            );
            l = v;
        }

        if l.hash() == p.hash() {
            debug!(
                peer_page=?p,
                local_page=?l,
                "hash match - consistent page"
            );

            // Record this page as fully consistent.
            diff_builder.consistent(p.start(), p.end());

            // Skip visiting the pages in the subtree rooted at the current
            // page: they're guaranteed to be consistent due to the consistent
            // parent hash.
            skip_subtree(&p, peer);
        } else {
            debug!(
                peer_page=?p,
                local_page=?l,
                "hash mismatch"
            );

            diff_builder.inconsistent(p.start(), p.end());
        }

        // Evaluate the sub-tree, causing all the (consistent) child ranges to
        // be added to the consistent list to, shrink this inconsistent range
        // (or simply advancing through the subtree if this page is consistent).
        recurse_subtree(&p, peer, local, diff_builder);
    }
}

/// Return the next [`PageRange`] if it is part of the sub-tree rooted at
/// `parent`.
fn maybe_advance_within<'a, 'p, K, T>(
    parent: &PageRange<'p, K>,
    cursor: &mut Peekable<T>,
) -> Option<PageRange<'a, K>>
where
    T: Iterator<Item = PageRange<'a, K>>,
    K: PartialOrd + 'a,
{
    if cursor
        .peek()
        .map(|p| !parent.is_superset_of(p))
        .unwrap_or_default()
    {
        return None;
    }

    cursor.next()
}

/// Advance `iter` to the next page that does not form part of the subtree
/// rooted at the given `subtree_root`.
#[inline(always)]
fn skip_subtree<'p, T, K>(subtree_root: &PageRange<'p, K>, iter: &mut Peekable<T>)
where
    T: Iterator<Item = PageRange<'p, K>>,
    K: PartialOrd + Ord + Debug + 'p,
{
    while iter.next_if(|v| subtree_root.is_superset_of(v)).is_some() {}
}
