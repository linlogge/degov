use std::fmt::Debug;

use super::DiffRange;

/// Helper to construct an ordered list of non-overlapping [`DiffRange`]
/// intervals.
#[derive(Debug)]
pub(crate) struct RangeList<'a, K> {
    sync_ranges: Vec<DiffRange<'a, K>>,
}

impl<'a, K> Default for RangeList<'a, K> {
    fn default() -> Self {
        Self {
            sync_ranges: Default::default(),
        }
    }
}

impl<'a, K> RangeList<'a, K>
where
    K: Ord,
{
    /// Insert the inclusive interval `[start, end]` into the list.
    pub(crate) fn insert(&mut self, start: &'a K, end: &'a K) {
        assert!(start <= end);
        self.sync_ranges.push(DiffRange { start, end });
    }

    /// Consume self and return the deduplicated/merged list of intervals
    /// ordered by range start.
    pub(crate) fn into_vec(mut self) -> Vec<DiffRange<'a, K>> {
        self.sync_ranges.sort_by_key(|v| v.start);
        merge_overlapping(&mut self.sync_ranges);

        // Check invariants in debug builds.
        #[cfg(debug_assertions)]
        {
            for v in self.sync_ranges.windows(2) {
                // Invariant: non-overlapping ranges
                assert!(!v[0].overlaps(&v[1]));
                // Invariant: end bound is always gte than start bound
                assert!(
                    v[0].start <= v[0].end,
                    "diff range contains inverted bounds"
                );
                assert!(
                    v[1].start <= v[1].end,
                    "diff range contains inverted bounds"
                );
            }
        }

        self.sync_ranges
    }
}

/// Perform an in-place merge and deduplication of overlapping intervals.
///
/// Assumes the intervals within `source` are sorted by the start value.
pub(super) fn merge_overlapping<K>(source: &mut Vec<DiffRange<'_, K>>)
where
    K: PartialOrd,
{
    let n_ranges = source.len();
    let mut range_iter = std::mem::take(source).into_iter();

    // Pre-allocate the ranges vec to hold all the elements, pessimistically
    // expecting them to not contain overlapping regions.
    source.reserve(n_ranges);

    // Place the first range into the merged output array.
    match range_iter.next() {
        Some(v) => source.push(v),
        None => return,
    }

    for range in range_iter {
        let last = source.last_mut().unwrap();

        // Invariant: ranges are sorted by range start.
        debug_assert!(range.start >= last.start);

        // Check if this range falls entirely within the existing range.
        if range.end <= last.end {
            // Skip this range that is a subset of the existing range.
            continue;
        }

        // Check for overlap across the end ranges (inclusive).
        if range.start <= last.end {
            // These two ranges overlap - extend the range in "last" to cover
            // both.
            last.end = range.end;
        } else {
            source.push(range);
        }
    }
}
