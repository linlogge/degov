use std::error::Error;
use std::fmt::{self, Display};
use std::sync::Arc;
use miette::{Diagnostic, LabeledSpan, Severity, SourceSpan, NamedSource};

/// The top-level error type for DSL parsing failures.
/// Contains multiple diagnostics that can be displayed together.
#[derive(Debug, Clone)]
pub struct DslError {
    /// Original input with source name for better error messages
    pub source: Arc<NamedSource<String>>,
    
    /// All diagnostics collected during parsing
    pub diagnostics: Vec<DslDiagnostic>,
}

impl DslError {
    /// Create a new DslError with the given source
    pub fn new(input: String, source_name: String) -> Self {
        Self {
            source: Arc::new(NamedSource::new(source_name, input)),
            diagnostics: Vec::new(),
        }
    }
    
    /// Create a DslError with a single diagnostic
    pub fn single(diagnostic: DslDiagnostic) -> Self {
        Self {
            source: diagnostic.source.clone(),
            diagnostics: vec![diagnostic],
        }
    }
    
    /// Add a diagnostic to this error
    pub fn add_diagnostic(&mut self, diagnostic: DslDiagnostic) {
        self.diagnostics.push(diagnostic);
    }
    
    /// Check if there are any errors (vs warnings)
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }
    
    /// Get the number of errors
    pub fn error_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Error).count()
    }
    
    /// Get the number of warnings
    pub fn warning_count(&self) -> usize {
        self.diagnostics.iter().filter(|d| d.severity == Severity::Warning).count()
    }
}

impl Display for DslError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let error_count = self.error_count();
        let warning_count = self.warning_count();
        
        match (error_count, warning_count) {
            (0, 0) => write!(f, "No diagnostics"),
            (0, w) => write!(f, "Found {} warning{}", w, if w == 1 { "" } else { "s" }),
            (e, 0) => write!(f, "Found {} error{}", e, if e == 1 { "" } else { "s" }),
            (e, w) => write!(f, "Found {} error{} and {} warning{}", 
                e, if e == 1 { "" } else { "s" },
                w, if w == 1 { "" } else { "s" }),
        }
    }
}

impl Error for DslError {}

impl Diagnostic for DslError {
    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&*self.source)
    }
    
    fn related<'a>(&'a self) -> Option<Box<dyn Iterator<Item = &'a dyn Diagnostic> + 'a>> {
        Some(Box::new(
            self.diagnostics.iter().map(|d| d as &dyn Diagnostic)
        ))
    }
}

/// An individual diagnostic message for a DSL parsing issue
#[derive(Debug, Clone)]
pub struct DslDiagnostic {
    /// Shared source for the diagnostic (includes filename)
    pub source: Arc<NamedSource<String>>,
    
    /// The kind of diagnostic
    pub kind: DiagnosticKind,
    
    /// Primary span for the diagnostic
    pub span: SourceSpan,
    
    /// Optional secondary spans with labels
    pub related_spans: Vec<(SourceSpan, String)>,
    
    /// Severity level
    pub severity: Severity,
}

impl DslDiagnostic {
    /// Create a new error diagnostic
    pub fn error(source: Arc<NamedSource<String>>, kind: DiagnosticKind, span: SourceSpan) -> Self {
        Self {
            source,
            kind,
            span,
            related_spans: Vec::new(),
            severity: Severity::Error,
        }
    }
    
    /// Create a new warning diagnostic
    pub fn warning(source: Arc<NamedSource<String>>, kind: DiagnosticKind, span: SourceSpan) -> Self {
        Self {
            source,
            kind,
            span,
            related_spans: Vec::new(),
            severity: Severity::Warning,
        }
    }
    
    /// Add a related span with a label
    pub fn with_related_span(mut self, span: SourceSpan, label: impl Into<String>) -> Self {
        self.related_spans.push((span, label.into()));
        self
    }
}

impl Display for DslDiagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.kind.message())
    }
}

impl Error for DslDiagnostic {}

impl Diagnostic for DslDiagnostic {
    fn source_code(&self) -> Option<&dyn miette::SourceCode> {
        Some(&*self.source)
    }
    
    fn severity(&self) -> Option<Severity> {
        Some(self.severity)
    }
    
    fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        self.kind.help().map(|s| Box::new(s) as Box<dyn Display>)
    }
    
    fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
        Some(Box::new(self.kind.code()))
    }
    
    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let primary = LabeledSpan::new_with_span(
            Some(self.kind.label()),
            self.span,
        );
        
        let related = self.related_spans.iter().map(|(span, label)| {
            LabeledSpan::new_with_span(Some(label.clone()), *span)
        });
        
        Some(Box::new(std::iter::once(primary).chain(related)))
    }
}

/// Different kinds of diagnostics that can occur during DSL parsing
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    ParseError { message: String },
    MissingNode { node_name: String },
    MissingChild { parent_name: String, child_name: String },
    MissingProperty { property: String },
    TypeMismatch { expected: String, got: String },
    InvalidValue { message: String, suggestion: Option<String> },
    ValidationError { message: String, suggestion: Option<String> },
    Duplicate { item_type: String, name: String },
    UnknownNode { node_name: String, suggestion: Option<String> },
    UnknownProperty { property: String, suggestion: Option<String> },
}

impl DiagnosticKind {
    pub fn code(&self) -> &'static str {
        match self {
            Self::ParseError { .. } => "dsl::parse_error",
            Self::MissingNode { .. } => "dsl::missing_node",
            Self::MissingChild { .. } => "dsl::missing_child",
            Self::MissingProperty { .. } => "dsl::missing_property",
            Self::TypeMismatch { .. } => "dsl::type_mismatch",
            Self::InvalidValue { .. } => "dsl::invalid_value",
            Self::ValidationError { .. } => "dsl::validation",
            Self::Duplicate { .. } => "dsl::duplicate",
            Self::UnknownNode { .. } => "dsl::unknown_node",
            Self::UnknownProperty { .. } => "dsl::unknown_property",
        }
    }
    
    pub fn message(&self) -> String {
        match self {
            Self::ParseError { message } => format!("Parse error: {}", message),
            Self::MissingNode { node_name } => format!("Missing required node: '{}'", node_name),
            Self::MissingChild { parent_name, child_name } => {
                format!("Missing required child node: '{}' in '{}'", child_name, parent_name)
            }
            Self::MissingProperty { property } => format!("Missing required property: '{}'", property),
            Self::TypeMismatch { expected, got } => {
                format!("Invalid type: expected {}, got {}", expected, got)
            }
            Self::InvalidValue { message, .. } => format!("Invalid value: {}", message),
            Self::ValidationError { message, .. } => format!("Validation error: {}", message),
            Self::Duplicate { item_type, name } => {
                format!("Duplicate {}: '{}'", item_type, name)
            }
            Self::UnknownNode { node_name, .. } => format!("Unknown node: '{}'", node_name),
            Self::UnknownProperty { property, .. } => format!("Unknown property: '{}'", property),
        }
    }
    
    pub fn label(&self) -> String {
        match self {
            Self::ParseError { .. } => "parse error here".to_string(),
            Self::MissingNode { node_name } => format!("'{}' node is required here", node_name),
            Self::MissingChild { child_name, .. } => {
                format!("'{}' child node is required", child_name)
            }
            Self::MissingProperty { property } => format!("missing '{}'", property),
            Self::TypeMismatch { expected, got } => {
                format!("expected {}, found {}", expected, got)
            }
            Self::InvalidValue { message, .. } => message.clone(),
            Self::ValidationError { message, .. } => message.clone(),
            Self::Duplicate { item_type, name } => format!("duplicate {} '{}'", item_type, name),
            Self::UnknownNode { node_name, .. } => format!("unknown node '{}'", node_name),
            Self::UnknownProperty { property, .. } => format!("unknown property '{}'", property),
        }
    }
    
    pub fn help(&self) -> Option<String> {
        match self {
            Self::ParseError { .. } => Some("Check the syntax of your DSL file".to_string()),
            Self::MissingNode { node_name } => {
                Some(format!("Add a '{}' node to define this field", node_name))
            }
            Self::MissingChild { parent_name, child_name } => {
                Some(format!("Add a '{}' node inside the '{}' block", child_name, parent_name))
            }
            Self::MissingProperty { property } => {
                Some(format!("Add the '{}' property to this node", property))
            }
            Self::TypeMismatch { expected, .. } => {
                Some(format!("Change this value to be of type '{}'", expected))
            }
            Self::InvalidValue { suggestion, .. } | Self::ValidationError { suggestion, .. } => {
                suggestion.clone()
            }
            Self::Duplicate { item_type, .. } => {
                Some(format!("Each {} must have a unique name", item_type))
            }
            Self::UnknownNode { suggestion, .. } | Self::UnknownProperty { suggestion, .. } => {
                suggestion.clone().or_else(|| Some("Check the documentation for valid options".to_string()))
            }
        }
    }
}

pub type Result<T> = std::result::Result<T, DslError>;

pub fn from_kdl_error(err: kdl::KdlError, source_name: String) -> DslError {
    let source = Arc::new(NamedSource::new(source_name, err.input.to_string()));
    let diagnostics = err.diagnostics.into_iter().map(|kdl_diag| {
        DslDiagnostic {
            source: source.clone(),
            kind: DiagnosticKind::ParseError {
                message: kdl_diag.message.unwrap_or_else(|| "Unknown parse error".to_string()),
            },
            span: kdl_diag.span,
            related_spans: Vec::new(),
            severity: kdl_diag.severity,
        }
    }).collect();
    
    DslError {
        source,
        diagnostics,
    }
}
