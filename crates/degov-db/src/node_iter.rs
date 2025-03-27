use std::fmt::Debug;

use crate::{Node, Page};

/// A [`Page`] and iteration metadata in the [`NodeIter`] stack.
#[derive(Debug)]
struct PageVisit<'a, const N: usize, K> {
    page: &'a Page<N, K>,

    /// The 0-based index of the node to visit next in this page.
    idx: usize,

    /// The outcome of the last visit to this page.
    state: VisitState,
}

/// Describes what action was previously taken (if any) for the indexed node.
#[derive(Debug)]
enum VisitState {
    /// The indexed node has not yet been visited.
    ///
    /// If the node has a lt_pointer, the state shall move to
    /// [`Self::Descended`] and the tree will be traversed downwards to the
    /// first leaf.
    ///
    /// If the node contains no lt_pointer, it will be yielded to the iterator.
    Unvisited,

    /// The node was previously visited, but not yielded due to the presence of
    /// a lt_pointer to descend down. It will be yielded next.
    Descended,
}

/// An iterator over [`Node`], yielded in ascending key order.
#[derive(Debug)]
pub(crate) struct NodeIter<'a, const N: usize, K> {
    /// A stack of visited pages as the iterator descends the tree.
    ///
    /// Approx log_{16}N max entries.
    stack: Vec<PageVisit<'a, N, K>>,
}

impl<'a, const N: usize, K> NodeIter<'a, N, K> {
    pub(crate) fn new(p: &'a Page<N, K>) -> Self {
        Self {
            stack: vec![PageVisit {
                page: p,
                idx: 0,
                state: VisitState::Unvisited,
            }],
        }
    }
}

impl<'a, const N: usize, K> Iterator for NodeIter<'a, N, K> {
    type Item = &'a Node<N, K>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let p = self.stack.pop()?;

            // Try and load the indexed node in this page.
            let n = match p.page.nodes().get(p.idx) {
                Some(v) => v,
                None => {
                    // No more nodes, instead visit the high page next, if any.
                    if let Some(h) = p.page.high_page() {
                        self.stack.push(PageVisit {
                            page: h,
                            idx: 0,
                            state: VisitState::Unvisited,
                        });
                    }

                    // Loop again, potentially popping the just-added high page,
                    // or popping a higher-level page (moving up the tree) if
                    // none.
                    continue;
                }
            };

            match p.state {
                VisitState::Unvisited => {
                    // This node has not been yielded, nor descended.
                    //
                    // If it has a lt_pointer, descend down it and visit this
                    // node later.
                    if let Some(lt) = n.lt_pointer() {
                        // Push this page back onto the stack to be revisited.
                        self.stack.push(PageVisit {
                            state: VisitState::Descended,
                            ..p
                        });
                        // And push the newly discovered page onto the stack.
                        self.stack.push(PageVisit {
                            state: VisitState::Unvisited,
                            idx: 0,
                            page: lt,
                        });
                        // Pop it off the next loop iteration and visit the
                        // first node.
                        continue;
                    }

                    // Otherwise there's no lt_pointer to follow in this node,
                    // so this node should be yielded and the page's node index
                    // incremented for the next iteration so the next node is
                    // visited.
                }
                VisitState::Descended => {
                    // The current index was previously descended down.
                    assert!(n.lt_pointer().is_some());
                    // But was never yielded.
                    //
                    // Advance the page's node index for the next iteration, and
                    // yield it now.
                }
            }

            self.stack.push(PageVisit {
                state: VisitState::Unvisited,
                idx: p.idx + 1,
                page: p.page,
            });

            return Some(n);
        }
    }
}
