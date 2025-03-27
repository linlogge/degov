use crate::{digest::PageDigest, Node};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Page<const N: usize, K> {
    level: u8,

    /// The cached hash in this page; the cumulation of the hashes of the
    /// sub-tree rooted at this page.
    tree_hash: Option<PageDigest>,

    /// A vector of nodes in this page, ordered min to max by key.
    nodes: Vec<Node<N, K>>,

    /// The page for keys greater-than all keys in nodes.
    high_page: Option<Box<Page<N, K>>>,
}

impl<const N: usize, K> Page<N, K> {
    pub(super) const fn new(level: u8, nodes: Vec<Node<N, K>>) -> Self {
        Self {
            level,
            tree_hash: None,
            nodes,
            high_page: None,
        }
    }
}
