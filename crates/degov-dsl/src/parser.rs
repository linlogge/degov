//! Core Parser Module
//!
//! Provides the main parsing interface using the schema framework

use crate::error::{DslDiagnostic, DslError, DiagnosticKind};
use crate::schema::{Schema, NodeDef, ValueType};
use crate::semantic::SemanticInfo;
use miette::NamedSource;
use std::sync::Arc;

/// The main parser for schema-validated KDL documents
pub struct Parser {
    source: String,
    source_name: String,
    schema: Option<Schema>,
}

impl Parser {
    /// Create a new parser
    pub fn new(source: String, source_name: String) -> Self {
        Self {
            source,
            source_name,
            schema: None,
        }
    }

    /// Set the schema to validate against
    pub fn with_schema(mut self, schema: Schema) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Parse the document
    pub fn parse(&self) -> Result<ParsedDocument, DslError> {
        // Parse KDL
        let doc = match self.source.parse::<kdl::KdlDocument>() {
            Ok(doc) => doc,
            Err(err) => {
                return Err(crate::error::from_kdl_error(err, self.source_name.clone()));
            }
        };

        let named_source = Arc::new(NamedSource::new(
            self.source_name.clone(),
            self.source.clone(),
        ));

        // Validate against schema if provided
        let mut diagnostics = Vec::new();

        if let Some(schema) = &self.schema {
            diagnostics.extend(self.validate_document(&doc, schema, &named_source));
        }

        // Check for errors
        if diagnostics
            .iter()
            .any(|d| d.severity == miette::Severity::Error)
        {
            return Err(DslError {
                source: named_source,
                diagnostics,
            });
        }

        // Build semantic info
        let semantic_info = if let Some(schema) = &self.schema {
            Some(SemanticInfo::analyze(&doc, schema, &self.source))
        } else {
            None
        };

        Ok(ParsedDocument {
            document: doc,
            semantic_info,
            diagnostics,
            source: named_source,
        })
    }

    fn validate_document(
        &self,
        doc: &kdl::KdlDocument,
        schema: &Schema,
        source: &Arc<NamedSource<String>>,
    ) -> Vec<DslDiagnostic> {
        let mut diagnostics = Vec::new();
        let mut validated_properties = std::collections::HashSet::new();

        // Check if root nodes should be treated as properties
        // This happens when the root NodeDef has an empty name and defines properties
        let root_is_property_container = schema.root.name.as_ref().map_or(true, |n| n.is_empty())
            && !schema.root.properties.is_empty();

        if root_is_property_container {
            // Treat root-level nodes as properties of an implicit root
            for node in doc.nodes() {
                let node_name = node.name().value();
                
                // Check if this node represents a property (single argument, matches property def)
                if let Some(prop_def) = schema.root.properties.get(node_name) {
                    // This is a property-as-node
                    if let Some(entry) = node.entries().first() {
                        if entry.name().is_none() && node.entries().len() == 1 {
                            validated_properties.insert(node_name.to_string());
                            let value = entry.value();
                            diagnostics.extend(self.validate_property_value(
                                value,
                                prop_def,
                                node_name,
                                schema,
                                source,
                                entry.span(),
                            ));
                            continue;
                        }
                    }
                }
                
                // Check if this is a defined child node
                let matching_child_def = schema.root.children.iter().find(|def| {
                    if let Some(name) = &def.name {
                        name == node_name
                    } else {
                        false
                    }
                });
                
                if let Some(child_def) = matching_child_def {
                    // Validate as a child node with its specific definition
                    diagnostics.extend(self.validate_node_against_def(
                        node,
                        child_def,
                        schema,
                        source,
                    ));
                } else if !schema.root.allow_unknown_children {
                    // Unknown node in strict mode
                    diagnostics.push(DslDiagnostic::error(
                        source.clone(),
                        DiagnosticKind::UnknownNode {
                            node_name: node_name.to_string(),
                            suggestion: None,
                        },
                        node.span(),
                    ));
                }
            }
            
            // Check for missing required properties
            for (prop_name, prop_def) in &schema.root.properties {
                if prop_def.required && !validated_properties.contains(prop_name) {
                    diagnostics.push(DslDiagnostic::error(
                        source.clone(),
                        DiagnosticKind::MissingProperty {
                            property: prop_name.clone(),
                        },
                        miette::SourceSpan::new(0.into(), 0),
                    ));
                }
            }
        } else {
            // Standard validation: each root node must match the root definition
            for node in doc.nodes() {
                diagnostics.extend(self.validate_node_against_def(
                    node,
                    &schema.root,
                    schema,
                    source,
                ));
            }
        }

        diagnostics
    }

    #[allow(dead_code)]
    fn validate_node(
        &self,
        node: &kdl::KdlNode,
        schema: &Schema,
        source: &Arc<NamedSource<String>>,
    ) -> Vec<DslDiagnostic> {
        // This is a wrapper for backward compatibility
        self.validate_node_against_def(node, &schema.root, schema, source)
    }

    /// Validate a node against a specific node definition
    fn validate_node_against_def(
        &self,
        node: &kdl::KdlNode,
        node_def: &NodeDef,
        schema: &Schema,
        source: &Arc<NamedSource<String>>,
    ) -> Vec<DslDiagnostic> {
        let mut diagnostics = Vec::new();

        // Apply schema modifier if present to get the actual schema for this node
        let effective_node_def = node_def.apply_modifier(node);

        // 1. Validate arguments
        diagnostics.extend(self.validate_arguments(node, &effective_node_def, source));

        // 2. Validate properties
        diagnostics.extend(self.validate_properties(node, &effective_node_def, schema, source));

        // 3. Validate children
        if let Some(children) = node.children() {
            diagnostics.extend(self.validate_children(
                node,
                children,
                &effective_node_def,
                schema,
                source,
            ));
        } else if !node_def.children.is_empty() {
            // Check for required children
            for child_def in &node_def.children {
                if let Some(_child_name) = &child_def.name {
                    // For now, we'll assume children are optional unless explicitly required
                    // This can be enhanced with a `required` flag on NodeDef
                }
            }
        }

        // 4. Run custom validator if defined
        if let Some(validator_name) = &node_def.validator {
            if let Some(_validator) = schema.validators.get(validator_name) {
                // Custom validation would go here
                // For now, we skip it as it requires ValidationContext
            }
        }

        diagnostics
    }

    /// Validate node arguments
    fn validate_arguments(
        &self,
        node: &kdl::KdlNode,
        node_def: &NodeDef,
        source: &Arc<NamedSource<String>>,
    ) -> Vec<DslDiagnostic> {
        let mut diagnostics = Vec::new();

        // Get actual arguments (entries without names)
        let actual_args: Vec<_> = node
            .entries()
            .iter()
            .filter(|e| e.name().is_none())
            .collect();

        // Check required arguments
        for (idx, arg_def) in node_def.arguments.iter().enumerate() {
            if arg_def.required {
                if idx >= actual_args.len() {
                    // Missing required argument - use node span
                    diagnostics.push(DslDiagnostic::error(
                        source.clone(),
                        DiagnosticKind::MissingProperty {
                            property: arg_def.name.clone(),
                        },
                        node.span(),
                    ));
                } else {
                    // Validate argument type - use entry span for precise error location
                    let entry = actual_args[idx];
                    let actual_value = entry.value();
                    if !arg_def.ty.matches(actual_value) {
                        diagnostics.push(DslDiagnostic::error(
                            source.clone(),
                            DiagnosticKind::TypeMismatch {
                                expected: arg_def.ty.name(),
                                got: value_type_name(actual_value),
                            },
                            entry.span(),
                        ));
                    }
                }
            }
        }

        diagnostics
    }

    /// Validate node properties
    fn validate_properties(
        &self,
        node: &kdl::KdlNode,
        node_def: &NodeDef,
        schema: &Schema,
        source: &Arc<NamedSource<String>>,
    ) -> Vec<DslDiagnostic> {
        let mut diagnostics = Vec::new();
        let mut validated_props = std::collections::HashSet::new();

        // Check required properties (supports both formats: key="value" and { key "value" })
        for (prop_name, prop_def) in &node_def.properties {
            if prop_def.required {
                let value = self.get_property_value(node, prop_name);
                if value.is_none() {
                    diagnostics.push(DslDiagnostic::error(
                        source.clone(),
                        DiagnosticKind::MissingProperty {
                            property: prop_name.clone(),
                        },
                        node.span(),
                    ));
                }
            }
        }

        // Validate existing properties as key-value pairs (node key="value")
        for entry in node.entries() {
            if let Some(prop_name) = entry.name() {
                let prop_name_str = prop_name.value();
                let entry_span = entry.span();
                validated_props.insert(prop_name_str.to_string());
                
                // Check if property is defined in schema
                if let Some(prop_def) = node_def.properties.get(prop_name_str) {
                    let value = entry.value();
                    
                    diagnostics.extend(self.validate_property_value(
                        value,
                        prop_def,
                        prop_name_str,
                        schema,
                        source,
                        entry_span,
                    ));
                } else if !node_def.allow_unknown_properties {
                    // Unknown property in strict mode
                    diagnostics.push(DslDiagnostic::error(
                        source.clone(),
                        DiagnosticKind::UnknownProperty {
                            property: prop_name_str.to_string(),
                            suggestion: None,
                        },
                        entry_span,
                    ));
                }
            }
        }

        // Validate properties defined as child nodes (node { key "value" })
        if let Some(children) = node.children() {
            for child in children.nodes() {
                let child_name = child.name().value();
                
                // Skip if already validated as a key-value property
                if validated_props.contains(child_name) {
                    continue;
                }
                
                // Check if this child is actually a property (not a child node definition)
                if let Some(prop_def) = node_def.properties.get(child_name) {
                    // This child node is a property in child-node format
                    // Get the first argument as the property value
                    if let Some(entry) = child.entries().first() {
                        // Only treat it as property if it's an argument (not named)
                        if entry.name().is_none() {
                            let value = entry.value();
                            let entry_span = entry.span();
                            
                            diagnostics.extend(self.validate_property_value(
                                value,
                                prop_def,
                                child_name,
                                schema,
                                source,
                                entry_span,
                            ));
                        }
                    }
                }
            }
        }

        diagnostics
    }

    /// Validate a single property value (extracted to avoid duplication)
    fn validate_property_value(
        &self,
        value: &kdl::KdlValue,
        prop_def: &crate::PropertyDef,
        _prop_name: &str,
        schema: &Schema,
        source: &Arc<NamedSource<String>>,
        span: miette::SourceSpan,
    ) -> Vec<DslDiagnostic> {
        let mut diagnostics = Vec::new();

        // Type validation
        if !prop_def.ty.matches(value) {
            diagnostics.push(DslDiagnostic::error(
                source.clone(),
                DiagnosticKind::TypeMismatch {
                    expected: prop_def.ty.name(),
                    got: value_type_name(value),
                },
                span,
            ));
        }
        
        // Enum validation
        if let ValueType::Enum(enum_name) = &prop_def.ty {
            if let Some(enum_def) = schema.get_enum(enum_name) {
                if let Some(string_value) = value.as_string() {
                    if !enum_def.is_valid(string_value) {
                        diagnostics.push(DslDiagnostic::error(
                            source.clone(),
                            DiagnosticKind::InvalidValue {
                                message: format!(
                                    "Invalid enum value '{}' for enum '{}'",
                                    string_value, enum_name
                                ),
                                suggestion: Some(format!(
                                    "Valid values are: {}",
                                    enum_def.values.join(", ")
                                )),
                            },
                            span,
                        ));
                    }
                }
            }
        }
        
        // Custom type validation
        if let ValueType::Custom { validator: Some(validator_name), .. } = &prop_def.ty {
            if let Some(type_validator) = schema.get_type_validator(validator_name) {
                if let Err(err) = type_validator.validate(value) {
                    diagnostics.push(DslDiagnostic::error(
                        source.clone(),
                        DiagnosticKind::ValidationError {
                            message: err,
                            suggestion: None,
                        },
                        span,
                    ));
                }
            }
        }

        diagnostics
    }

    /// Validate child nodes
    fn validate_children(
        &self,
        _parent: &kdl::KdlNode,
        children: &kdl::KdlDocument,
        node_def: &NodeDef,
        schema: &Schema,
        source: &Arc<NamedSource<String>>,
    ) -> Vec<DslDiagnostic> {
        let mut diagnostics = Vec::new();

        // Validate each child node
        for child in children.nodes() {
            let child_name = child.name().value();
            let child_span = child.span();

            // Check if this child is actually a property in child-node format
            // Properties have a single argument and are defined in the properties map
            let is_property = if let Some(_prop_def) = node_def.properties.get(child_name) {
                // It's a property if it has a single argument (not a named property)
                if let Some(entry) = child.entries().first() {
                    entry.name().is_none() && child.entries().len() == 1
                } else {
                    false
                }
            } else {
                false
            };

            // Skip child nodes that are actually properties
            if is_property {
                continue;
            }

            // Find matching child definition
            let matching_def = node_def.children.iter().find(|def| {
                if let Some(name) = &def.name {
                    name == child_name
                } else {
                    false
                }
            });

            if let Some(child_def) = matching_def {
                // Validate child against its definition
                diagnostics.extend(self.validate_node_against_def(
                    child, child_def, schema, source,
                ));
            } else if !node_def.allow_unknown_children {
                // Unknown child in strict mode
                diagnostics.push(DslDiagnostic::error(
                    source.clone(),
                    DiagnosticKind::UnknownNode {
                        node_name: child_name.to_string(),
                        suggestion: None,
                    },
                    child_span,
                ));
            }
        }

        diagnostics
    }

    /// Get property value from either KDL property or child node
    /// Supports both formats:
    /// - Direct property: `node prop="value"`
    /// - Child node: `node { prop "value" }`
    fn get_property_value<'a>(
        &self,
        node: &'a kdl::KdlNode,
        prop_name: &str,
    ) -> Option<&'a kdl::KdlValue> {
        // First try to get as a direct KDL property (node prop="value")
        if let Some(val) = node.get(prop_name) {
            return Some(val);
        }

        // Then try to get as a child node with an argument (node { prop "value" })
        if let Some(children) = node.children() {
            for child in children.nodes() {
                if child.name().value() == prop_name {
                    // Get the first argument of the child node as the value
                    if let Some(entry) = child.entries().first() {
                        // Only use it if it's an argument (not a property)
                        if entry.name().is_none() {
                            return Some(entry.value());
                        }
                    }
                }
            }
        }

        None
    }
}

/// Get a human-readable name for a KDL value type
#[allow(dead_code)]
fn value_type_name(value: &kdl::KdlValue) -> String {
    match value {
        kdl::KdlValue::String(s) => {
            // Try to infer type from string content
            if s.parse::<i64>().is_ok() {
                "integer".to_string()
            } else if s.parse::<f64>().is_ok() {
                "number".to_string()
            } else {
                "string".to_string()
            }
        }
        kdl::KdlValue::Bool(_) => "boolean".to_string(),
        kdl::KdlValue::Null => "null".to_string(),
        _ => "unknown".to_string(),
    }
}

/// A parsed and validated document
pub struct ParsedDocument {
    /// The parsed KDL document
    pub document: kdl::KdlDocument,

    /// Semantic information for IDE support
    pub semantic_info: Option<SemanticInfo>,

    /// Any diagnostics (warnings, etc.)
    pub diagnostics: Vec<DslDiagnostic>,

    /// Source information
    pub source: Arc<NamedSource<String>>,
}
