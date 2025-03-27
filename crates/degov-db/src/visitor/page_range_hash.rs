use super::Visitor;
use crate::{diff::PageRange, Node, Page};

/// Record the page range & hashes for the visited pages.
#[derive(Debug)]
pub(crate) struct PageRangeHashVisitor<'a, K> {
    out: Vec<PageRange<'a, K>>,
}

impl<'a, K> Default for PageRangeHashVisitor<'a, K> {
    fn default() -> Self {
        Self {
            out: Default::default(),
        }
    }
}

impl<'a, const N: usize, K> Visitor<'a, N, K> for PageRangeHashVisitor<'a, K>
where
    K: PartialOrd,
{
    fn visit_node(&mut self, _node: &'a Node<N, K>) -> bool {
        true
    }

    fn visit_page(&mut self, page: &'a Page<N, K>, _high_page: bool) -> bool {
        self.out.push(PageRange::from(page));
        true
    }
}

impl<'a, K> PageRangeHashVisitor<'a, K> {
    pub(crate) fn finalise(self) -> Vec<PageRange<'a, K>> {
        self.out
    }
}
