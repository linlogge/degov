//! Schema Definition Framework
//!
//! This module provides a framework for defining KDL-based language schemas.
//! It's completely generic and can be used to build any KDL-based DSL.

use std::collections::HashMap;
use std::sync::Arc;
use miette::SourceSpan;

/// Schema modifier function type
/// Takes the current NodeDef and the actual KdlNode being validated,
/// and returns a modified NodeDef with conditionally added/removed children or properties
pub type SchemaModifier = Arc<dyn Fn(&NodeDef, &kdl::KdlNode) -> NodeDef + Send + Sync>;

/// A schema defines the structure and validation rules for a KDL-based language
#[derive(Debug, Clone)]
pub struct Schema {
    /// Human-readable name of the schema
    pub name: String,
    
    /// Root node definitions
    pub root: NodeDef,
    
    /// Enum definitions for use in properties
    pub enums: HashMap<String, EnumDef>,
    
    /// Custom validation functions for nodes
    pub validators: HashMap<String, ValidatorDef>,
    
    /// Type validators for custom value types
    pub type_validators: HashMap<String, TypeValidatorDef>,
}

impl Schema {
    /// Create a new empty schema
    pub fn new(name: impl Into<String>, root: NodeDef) -> Self {
        Self {
            name: name.into(),
            root,
            enums: HashMap::new(),
            validators: HashMap::new(),
            type_validators: HashMap::new(),
        }
    }
    
    /// Define an enum type
    pub fn define_enum(&mut self, name: impl Into<String>, def: EnumDef) -> &mut Self {
        self.enums.insert(name.into(), def);
        self
    }
    
    /// Register a node validator function
    pub fn register_validator(&mut self, name: impl Into<String>, def: ValidatorDef) -> &mut Self {
        self.validators.insert(name.into(), def);
        self
    }
    
    /// Register a type validator for custom value types
    pub fn register_type_validator(&mut self, name: impl Into<String>, def: TypeValidatorDef) -> &mut Self {
        self.type_validators.insert(name.into(), def);
        self
    }
    
    /// Get an enum definition
    pub fn get_enum(&self, name: &str) -> Option<&EnumDef> {
        self.enums.get(name)
    }
    
    /// Get a type validator
    pub fn get_type_validator(&self, name: &str) -> Option<&TypeValidatorDef> {
        self.type_validators.get(name)
    }
}

/// Definition of a node type in the schema
#[derive(Clone)]
pub struct NodeDef {
    /// Human-readable description
    pub name: Option<String>,
    pub description: Option<String>,
    
    /// Expected arguments (positional)
    pub arguments: Vec<ArgumentDef>,
    
    /// Expected properties (key-value)
    pub properties: HashMap<String, PropertyDef>,
    
    /// Expected child nodes
    pub children: Vec<NodeDef>,
    
    /// Whether this node can have any properties (open schema)
    pub allow_unknown_properties: bool,
    
    /// Whether this node can have any children (open schema)
    pub allow_unknown_children: bool,
    
    /// Custom validation function
    pub validator: Option<String>,
    
    /// Completion items for this context
    pub completions: Vec<CompletionItem>,
    
    /// Dynamic schema modifier
    /// This function is called during validation and can modify the schema
    /// based on the actual node content (e.g., add/remove properties or children)
    pub schema_modifier: Option<SchemaModifier>,
}

impl std::fmt::Debug for NodeDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NodeDef")
            .field("name", &self.name)
            .field("description", &self.description)
            .field("arguments", &self.arguments)
            .field("properties", &self.properties)
            .field("children", &self.children)
            .field("allow_unknown_properties", &self.allow_unknown_properties)
            .field("allow_unknown_children", &self.allow_unknown_children)
            .field("validator", &self.validator)
            .field("completions", &self.completions)
            .field("schema_modifier", &if self.schema_modifier.is_some() { &"<function>" } else { &"None" })
            .finish()
    }
}

impl NodeDef {
    /// Create a new node definition
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            description: None,
            arguments: Vec::new(),
            properties: HashMap::new(),
            children: Vec::new(),
            allow_unknown_properties: false,
            allow_unknown_children: false,
            validator: None,
            completions: Vec::new(),
            schema_modifier: None,
        }
    }
    
    /// Set the description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
    
    /// Add an argument
    pub fn with_argument(mut self, arg: ArgumentDef) -> Self {
        self.arguments.push(arg);
        self
    }
    
    /// Add a property
    pub fn with_property(mut self, name: impl Into<String>, prop: PropertyDef) -> Self {
        self.properties.insert(name.into(), prop);
        self
    }
    
    /// Add a child node definition
    pub fn with_child(mut self, child: NodeDef) -> Self {
        self.children.push(child);
        self
    }
    
    /// Allow unknown properties
    pub fn allow_unknown_props(mut self) -> Self {
        self.allow_unknown_properties = true;
        self
    }
    
    /// Allow unknown children
    pub fn allow_unknown_children(mut self) -> Self {
        self.allow_unknown_children = true;
        self
    }
    
    /// Set validator
    pub fn with_validator(mut self, validator: impl Into<String>) -> Self {
        self.validator = Some(validator.into());
        self
    }
    
    /// Add a completion item
    pub fn with_completion(mut self, item: CompletionItem) -> Self {
        self.completions.push(item);
        self
    }
    
    /// Add a dynamic schema modifier
    /// This function will be called during validation and can modify the schema
    /// based on the actual node content.
    ///
    /// # Example
    /// ```ignore
    /// NodeDef::new("definition")
    ///     .with_property("kind", PropertyDef::new(ValueType::String))
    ///     .with_schema_modifier(|def, node| {
    ///         // Add different properties based on the "kind" property
    ///         let kind = node.entries().iter()
    ///             .find(|e| e.name().map(|n| n.value()) == Some("kind"))
    ///             .and_then(|e| e.value().as_string());
    ///         
    ///         let mut modified_def = def.clone();
    ///         match kind {
    ///             Some("DataModel") => {
    ///                 modified_def = modified_def
    ///                     .with_property("table", PropertyDef::new(ValueType::String));
    ///             }
    ///             Some("Service") => {
    ///                 modified_def = modified_def
    ///                     .with_property("endpoint", PropertyDef::new(ValueType::String));
    ///             }
    ///             _ => {}
    ///         }
    ///         modified_def
    ///     })
    /// ```
    pub fn with_schema_modifier(
        mut self,
        modifier: impl Fn(&NodeDef, &kdl::KdlNode) -> NodeDef + Send + Sync + 'static,
    ) -> Self {
        self.schema_modifier = Some(Arc::new(modifier));
        self
    }
    
    /// Apply the schema modifier if present
    /// Returns a modified NodeDef based on the actual node content
    pub fn apply_modifier(&self, node: &kdl::KdlNode) -> Self {
        if let Some(modifier) = &self.schema_modifier {
            modifier(self, node)
        } else {
            self.clone()
        }
    }
    
    /// Conditionally add a child node based on a predicate function
    /// The predicate receives the actual KDL node and decides whether to add the child
    ///
    /// # Example
    /// ```ignore
    /// NodeDef::new("definition")
    ///     .with_property("kind", PropertyDef::new(ValueType::String))
    ///     .with_child_conditional(
    ///         |node| {
    ///             // Check if kind="DataModel"
    ///             node.entries().iter()
    ///                 .find(|e| e.name().map(|n| n.value()) == Some("kind"))
    ///                 .and_then(|e| e.value().as_string())
    ///                 .map(|v| v == "DataModel")
    ///                 .unwrap_or(false)
    ///         },
    ///         NodeDef::new("field")
    ///             .with_property("name", PropertyDef::new(ValueType::String))
    ///     )
    /// ```
    pub fn with_child_conditional(
        self,
        predicate: impl Fn(&NodeDef, &kdl::KdlNode) -> bool + Send + Sync + 'static,
        child: NodeDef,
    ) -> Self {
        let existing_modifier = self.schema_modifier.clone();
        
        self.with_schema_modifier(move |def, node| {
            // Apply existing modifier first
            let mut modified_def = if let Some(modifier) = &existing_modifier {
                modifier(def, node)
            } else {
                def.clone()
            };
            
            // Add child if predicate is true
            if predicate(def, node) {
                modified_def = modified_def.with_child(child.clone());
            }
            
            modified_def
        })
    }
    
    /// Conditionally add a property based on a predicate function
    ///
    /// # Example
    /// ```ignore
    /// NodeDef::new("config")
    ///     .with_property("mode", PropertyDef::new(ValueType::String))
    ///     .with_property_conditional(
    ///         |node| {
    ///             // Add debug property only in development mode
    ///             node.entries().iter()
    ///                 .find(|e| e.name().map(|n| n.value()) == Some("mode"))
    ///                 .and_then(|e| e.value().as_string())
    ///                 .map(|v| v == "development")
    ///                 .unwrap_or(false)
    ///         },
    ///         "debug",
    ///         PropertyDef::new(ValueType::Boolean)
    ///     )
    /// ```
    pub fn with_property_conditional(
        self,
        predicate: impl Fn(&kdl::KdlNode) -> bool + Send + Sync + 'static,
        name: impl Into<String>,
        property: PropertyDef,
    ) -> Self {
        let name = name.into();
        let existing_modifier = self.schema_modifier.clone();
        
        self.with_schema_modifier(move |def, node| {
            // Apply existing modifier first
            let mut modified_def = if let Some(modifier) = &existing_modifier {
                modifier(def, node)
            } else {
                def.clone()
            };
            
            // Add property if predicate is true
            if predicate(node) {
                modified_def = modified_def.with_property(name.clone(), property.clone());
            }
            
            modified_def
        })
    }
    
    /// Conditionally add an argument based on a predicate function
    ///
    /// # Example
    /// ```ignore
    /// NodeDef::new("command")
    ///     .with_property("type", PropertyDef::new(ValueType::String))
    ///     .with_argument_conditional(
    ///         |node| {
    ///             // Add path argument only for "file" type commands
    ///             node.entries().iter()
    ///                 .find(|e| e.name().map(|n| n.value()) == Some("type"))
    ///                 .and_then(|e| e.value().as_string())
    ///                 .map(|v| v == "file")
    ///                 .unwrap_or(false)
    ///         },
    ///         ArgumentDef::new("path", ValueType::String)
    ///     )
    /// ```
    pub fn with_argument_conditional(
        self,
        predicate: impl Fn(&kdl::KdlNode) -> bool + Send + Sync + 'static,
        argument: ArgumentDef,
    ) -> Self {
        let existing_modifier = self.schema_modifier.clone();
        
        self.with_schema_modifier(move |def, node| {
            // Apply existing modifier first
            let mut modified_def = if let Some(modifier) = &existing_modifier {
                modifier(def, node)
            } else {
                def.clone()
            };
            
            // Add argument if predicate is true
            if predicate(node) {
                modified_def = modified_def.with_argument(argument.clone());
            }
            
            modified_def
        })
    }
    
    /// Helper function to get a property value from a node (direct or child format)
    /// Useful when building predicates for conditional functions
    pub fn get_node_property_value(node: &kdl::KdlNode, prop_name: &str) -> Option<String> {
        // Check direct properties first (e.g., node prop="value")
        let direct = node
            .entries()
            .iter()
            .find(|e| e.name().map(|n| n.value()) == Some(prop_name))
            .and_then(|e| e.value().as_string())
            .map(|s| s.to_string());
        
        if direct.is_some() {
            return direct;
        }
        
        // Check child node format (e.g., node { prop "value" })
        node.children()
            .and_then(|children| {
                children.nodes().iter().find(|child| {
                    child.name().value() == prop_name
                        && child.entries().len() == 1
                        && child.entries().first().unwrap().name().is_none()
                })
            })
            .and_then(|child| child.entries().first())
            .and_then(|entry| entry.value().as_string())
            .map(|s| s.to_string())
    }
}

impl Default for NodeDef {
    fn default() -> Self {
        Self::new("")
    }
}

/// Definition of an expected argument
#[derive(Debug, Clone)]
pub struct ArgumentDef {
    /// Human-readable name/description
    pub name: String,
    
    /// Expected type
    pub ty: ValueType,
    
    /// Whether this argument is required
    pub required: bool,
    
    /// Default value if not provided
    pub default: Option<KdlValue>,
    
    /// Description for documentation
    pub description: Option<String>,
}

impl ArgumentDef {
    pub fn new(name: impl Into<String>, ty: ValueType) -> Self {
        Self {
            name: name.into(),
            ty,
            required: true,
            default: None,
            description: None,
        }
    }
    
    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }
    
    pub fn with_default(mut self, value: KdlValue) -> Self {
        self.default = Some(value);
        self.required = false;
        self
    }
    
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Definition of a property (key-value pair)
#[derive(Debug, Clone)]
pub struct PropertyDef {
    /// Expected type
    pub ty: ValueType,
    
    /// Whether this property is required
    pub required: bool,
    
    /// Default value if not provided
    pub default: Option<KdlValue>,
    
    /// Description for documentation
    pub description: Option<String>,
    
    /// Possible values (for completion)
    pub suggestions: Vec<String>,
}

impl PropertyDef {
    pub fn new(ty: ValueType) -> Self {
        Self {
            ty,
            required: false,
            default: None,
            description: None,
            suggestions: Vec::new(),
        }
    }
    
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
    
    pub fn with_default(mut self, value: KdlValue) -> Self {
        self.default = Some(value);
        self
    }
    
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
    
    pub fn with_suggestions(mut self, suggestions: Vec<String>) -> Self {
        self.suggestions = suggestions;
        self
    }
}

/// Type definition for values
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType {
    /// String value
    String,
    
    /// Integer value
    Integer,
    
    /// Float value
    Float,
    
    /// Boolean value
    Boolean,
    
    /// Null value
    Null,
    
    /// Enum type (reference to enum definition)
    Enum(String),
    
    /// Reference to another node type
    NodeRef(String),
    
    /// Custom type with user-defined validation
    /// The string references a type validator registered in the schema
    Custom {
        /// Name of the custom type
        name: String,
        /// Optional reference to a type validator function
        validator: Option<String>,
    },
    
    /// Any value type
    Any,
}

impl ValueType {
    /// Create a custom type with a validator
    pub fn custom(name: impl Into<String>, validator: impl Into<String>) -> Self {
        ValueType::Custom {
            name: name.into(),
            validator: Some(validator.into()),
        }
    }
    
    /// Create a custom type without a validator
    pub fn custom_unvalidated(name: impl Into<String>) -> Self {
        ValueType::Custom {
            name: name.into(),
            validator: None,
        }
    }
    
    /// Check if a KDL value matches this type
    /// Note: For Custom types, this only does basic validation. 
    /// Full validation happens via the registered validator.
    pub fn matches(&self, value: &kdl::KdlValue) -> bool {
        match (self, value) {
            (ValueType::String, kdl::KdlValue::String(_)) => true,
            (ValueType::Integer, _) => {
                // Try to parse as integer
                value.as_string().and_then(|s| s.parse::<i64>().ok()).is_some()
                    || matches!(value, kdl::KdlValue::String(s) if s.parse::<i64>().is_ok())
            }
            (ValueType::Float, _) => {
                // Try to parse as float
                value.as_string().and_then(|s| s.parse::<f64>().ok()).is_some()
                    || matches!(value, kdl::KdlValue::String(s) if s.parse::<f64>().is_ok())
            }
            (ValueType::Boolean, kdl::KdlValue::Bool(_)) => true,
            (ValueType::Null, kdl::KdlValue::Null) => true,
            // Enums are represented as strings in KDL, validation happens separately
            (ValueType::Enum(_), kdl::KdlValue::String(_)) => true,
            (ValueType::NodeRef(_), kdl::KdlValue::String(_)) => true,
            // Custom types accept any value; validation is delegated to the validator
            (ValueType::Custom { .. }, _) => true,
            (ValueType::Any, _) => true,
            _ => false,
        }
    }
    
    /// Get a human-readable name for this type
    pub fn name(&self) -> String {
        match self {
            ValueType::String => "string".to_string(),
            ValueType::Integer => "integer".to_string(),
            ValueType::Float => "float".to_string(),
            ValueType::Boolean => "boolean".to_string(),
            ValueType::Null => "null".to_string(),
            ValueType::Enum(name) => format!("enum<{}>", name),
            ValueType::NodeRef(name) => format!("ref<{}>", name),
            ValueType::Custom { name, .. } => name.clone(),
            ValueType::Any => "any".to_string(),
        }
    }
}

/// A generic KDL value wrapper
#[derive(Debug, Clone, PartialEq)]
pub enum KdlValue {
    String(String),
    Integer(i128),
    Float(f64),
    Boolean(bool),
    Null,
}

impl TryFrom<kdl::KdlValue> for KdlValue {
    type Error = String;
    fn try_from(value: kdl::KdlValue) -> Result<Self, Self::Error> {
        match value {
            kdl::KdlValue::String(s) => Ok(KdlValue::String(s)),
            kdl::KdlValue::Integer(i) => Ok(KdlValue::Integer(i)),
            kdl::KdlValue::Float(f) => Ok(KdlValue::Float(f)),
            kdl::KdlValue::Bool(b) => Ok(KdlValue::Boolean(b)),
            kdl::KdlValue::Null => Ok(KdlValue::Null),
        }
    }
}

/// Definition of an enum type
#[derive(Debug, Clone)]
pub struct EnumDef {
    /// Possible values
    pub values: Vec<String>,
    
    /// Description for each value
    pub value_descriptions: HashMap<String, String>,
    
    /// Description of the enum itself
    pub description: Option<String>,
}

impl EnumDef {
    pub fn new(values: Vec<String>) -> Self {
        Self {
            values,
            value_descriptions: HashMap::new(),
            description: None,
        }
    }
    
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
    
    pub fn with_value_desc(mut self, value: impl Into<String>, desc: impl Into<String>) -> Self {
        self.value_descriptions.insert(value.into(), desc.into());
        self
    }
    
    /// Check if a value is valid for this enum
    pub fn is_valid(&self, value: &str) -> bool {
        self.values.iter().any(|v| v == value)
    }
}

/// Definition of a validator function
#[derive(Clone)]
pub struct ValidatorDef {
    /// Description of what this validator does
    pub description: String,
    
    /// The actual validation function
    pub function: Arc<dyn Fn(&ValidationContext) -> ValidationResult + Send + Sync>,
}

impl std::fmt::Debug for ValidatorDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatorDef")
            .field("description", &self.description)
            .field("function", &"<function>")
            .finish()
    }
}

impl ValidatorDef {
    pub fn new(
        description: impl Into<String>,
        function: impl Fn(&ValidationContext) -> ValidationResult + Send + Sync + 'static,
    ) -> Self {
        Self {
            description: description.into(),
            function: Arc::new(function),
        }
    }
}

/// Definition of a type validator function for custom value types
#[derive(Clone)]
pub struct TypeValidatorDef {
    /// Description of what this type represents
    pub description: String,
    
    /// The actual validation function
    /// Takes a KdlValue and returns true if it's valid for this type
    pub function: Arc<dyn Fn(&kdl::KdlValue) -> Result<(), String> + Send + Sync>,
}

impl std::fmt::Debug for TypeValidatorDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TypeValidatorDef")
            .field("description", &self.description)
            .field("function", &"<function>")
            .finish()
    }
}

impl TypeValidatorDef {
    pub fn new(
        description: impl Into<String>,
        function: impl Fn(&kdl::KdlValue) -> Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        Self {
            description: description.into(),
            function: Arc::new(function),
        }
    }
    
    /// Validate a value against this type
    pub fn validate(&self, value: &kdl::KdlValue) -> Result<(), String> {
        (self.function)(value)
    }
}

/// Context passed to validation functions
#[derive(Debug)]
pub struct ValidationContext<'a> {
    /// The node being validated
    pub node: &'a kdl::KdlNode,
    
    /// The full document
    pub document: &'a kdl::KdlDocument,
    
    /// The schema
    pub schema: &'a Schema,
    
    /// Span of the node in the source
    pub span: SourceSpan,
    
    /// Source text
    pub source: &'a str,
}

/// Result of validation
pub type ValidationResult = Result<(), ValidationError>;

/// A validation error
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
    pub span: SourceSpan,
    pub help: Option<String>,
}

impl ValidationError {
    pub fn new(message: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            message: message.into(),
            span,
            help: None,
        }
    }
    
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Completion item for IDE support
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// The text to insert
    pub label: String,
    
    /// Kind of completion
    pub kind: CompletionKind,
    
    /// Detailed description
    pub detail: Option<String>,
    
    /// Documentation
    pub documentation: Option<String>,
    
    /// The actual text to insert (if different from label)
    pub insert_text: Option<String>,
    
    /// Whether this is a snippet with placeholders
    pub is_snippet: bool,
    
    /// Sort priority (lower = higher priority)
    pub sort_priority: u32,
}

impl CompletionItem {
    pub fn new(label: impl Into<String>, kind: CompletionKind) -> Self {
        Self {
            label: label.into(),
            kind,
            detail: None,
            documentation: None,
            insert_text: None,
            is_snippet: false,
            sort_priority: 100,
        }
    }
    
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
    
    pub fn with_docs(mut self, docs: impl Into<String>) -> Self {
        self.documentation = Some(docs.into());
        self
    }
    
    pub fn with_insert_text(mut self, text: impl Into<String>) -> Self {
        self.insert_text = Some(text.into());
        self
    }
    
    pub fn as_snippet(mut self) -> Self {
        self.is_snippet = true;
        self
    }
    
    pub fn with_priority(mut self, priority: u32) -> Self {
        self.sort_priority = priority;
        self
    }
}

/// Kind of completion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Node,
    Property,
    Value,
    Enum,
    Keyword,
    Snippet,
}

impl CompletionKind {
    pub fn to_lsp_kind(&self) -> u32 {
        match self {
            CompletionKind::Node => 7,      // Class
            CompletionKind::Property => 10,  // Property
            CompletionKind::Value => 12,     // Value
            CompletionKind::Enum => 13,      // Enum
            CompletionKind::Keyword => 14,   // Keyword
            CompletionKind::Snippet => 15,   // Snippet
        }
    }
}

