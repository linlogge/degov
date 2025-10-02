use kdl::KdlNode;
use crate::parser::{Parser, get_child_string_value};
use crate::Spanned;
use crate::error::Result;

/// A definition in the DSL (the root structure)
#[derive(Debug, Clone)]
pub struct Definition {
    pub r#type: Spanned<String>,
}

impl Definition {
    /// Parse a Definition from a KDL node
    pub fn from_kdl_node(node: &KdlNode, parser: &Parser) -> Result<Self> {
        let r#type = get_child_string_value(node, "type", parser)?;

        Ok(Self {
            r#type,
        })
    }
}