use kdl::{KdlDocument, KdlNode};
use std::sync::Arc;
use miette::NamedSource;
use crate::error::{Result, DslError, DslDiagnostic, DiagnosticKind, from_kdl_error};
use crate::Spanned;

mod definition;

pub use definition::Definition;

/// Parser context that maintains source information for error reporting
pub struct Parser {
    source: Arc<NamedSource<String>>,
    src: String,
}

impl Parser {
    /// Create a new parser with source content and a name for error reporting
    pub fn new(src: String, source_name: String) -> Self {
        let source = Arc::new(NamedSource::new(source_name, src.clone()));
        Self {
            source,
            src,
        }
    }
    
    /// Get the source as Arc<NamedSource<String>> for error reporting
    pub fn source(&self) -> Arc<NamedSource<String>> {
        self.source.clone()
    }
    
    /// Get the source name
    pub fn source_name(&self) -> &str {
        self.source.name()
    }

    /// Parse the source into a Definition
    pub fn parse(&self) -> Result<Definition> {
        let doc: KdlDocument = self.src.parse()
            .map_err(|e| from_kdl_error(e, self.source_name().to_string()))?;

        let definition_node = doc.get("definition").ok_or_else(|| {
            let diagnostic = DslDiagnostic::error(
                self.source.clone(),
                DiagnosticKind::MissingNode {
                    node_name: "definition".to_string(),
                },
                (0, self.src.len().min(1)).into(),
            );
            DslError::single(diagnostic)
        })?;

        Definition::from_kdl_node(definition_node, self)
    }
    
    /// Get the source content as a string slice
    pub(crate) fn src(&self) -> &str {
        &self.src
    }
}

/// Helper to extract a child node by name
pub(crate) fn get_child_node<'a>(
    parent: &'a KdlNode,
    child_name: &str,
    parser: &Parser,
) -> Result<&'a KdlNode> {
    parent
        .children()
        .and_then(|doc| doc.get(child_name))
        .ok_or_else(|| {
            let diagnostic = DslDiagnostic::error(
                parser.source(),
                DiagnosticKind::MissingChild {
                    parent_name: parent.name().value().to_string(),
                    child_name: child_name.to_string(),
                },
                parent.span(),
            );
            DslError::single(diagnostic)
        })
}

/// Helper to extract an optional child node by name
pub(crate) fn get_optional_child_node<'a>(
    parent: &'a KdlNode,
    child_name: &str,
) -> Option<&'a KdlNode> {
    parent.children().and_then(|doc| doc.get(child_name))
}

/// Helper to extract a string value from a node's entry by index
pub(crate) fn get_string_entry(
    node: &KdlNode,
    index: usize,
    parser: &Parser,
) -> Result<Spanned<String>> {
    let entry = node.entries().get(index).ok_or_else(|| {
        let diagnostic = DslDiagnostic::error(
            parser.source(),
            DiagnosticKind::MissingProperty {
                property: format!("entry at index {}", index),
            },
            node.span(),
        );
        DslError::single(diagnostic)
    })?;

    let value = entry.value().as_string().ok_or_else(|| {
        let diagnostic = DslDiagnostic::error(
            parser.source(),
            DiagnosticKind::TypeMismatch {
                expected: "string".to_string(),
                got: format!("{:?}", entry.value()),
            },
            entry.span(),
        );
        DslError::single(diagnostic)
    })?;

    Ok(Spanned::new(value.to_string(), entry.span()))
}

/// Helper to extract a string value from a child node's first entry
pub(crate) fn get_child_string_value(
    parent: &KdlNode,
    child_name: &str,
    parser: &Parser,
) -> Result<Spanned<String>> {
    let child = get_child_node(parent, child_name, parser)?;
    get_string_entry(child, 0, parser)
}

/// Helper to extract an optional string value from a child node's first entry
pub(crate) fn get_optional_child_string_value(
    parent: &KdlNode,
    child_name: &str,
    parser: &Parser,
) -> Result<Option<Spanned<String>>> {
    match get_optional_child_node(parent, child_name) {
        Some(child) => Ok(Some(get_string_entry(child, 0, parser)?)),
        None => Ok(None),
    }
}

/// Helper to extract a boolean value from a child node's first entry
pub(crate) fn get_child_bool_value(
    parent: &KdlNode,
    child_name: &str,
    parser: &Parser,
) -> Result<Spanned<bool>> {
    let child = get_child_node(parent, child_name, parser)?;
    let entry = child.entries().get(0).ok_or_else(|| {
        let diagnostic = DslDiagnostic::error(
            parser.source(),
            DiagnosticKind::MissingProperty {
                property: "value".to_string(),
            },
            child.span(),
        );
        DslError::single(diagnostic)
    })?;

    let value = entry.value().as_bool().ok_or_else(|| {
        let diagnostic = DslDiagnostic::error(
            parser.source(),
            DiagnosticKind::TypeMismatch {
                expected: "boolean".to_string(),
                got: format!("{:?}", entry.value()),
            },
            entry.span(),
        );
        DslError::single(diagnostic)
    })?;

    Ok(Spanned::new(value, entry.span()))
}

/// Helper to get all children of a node
pub(crate) fn get_children(node: &KdlNode) -> Vec<&KdlNode> {
    node.children()
        .map(|doc| doc.nodes().iter().collect())
        .unwrap_or_else(Vec::new)
}
