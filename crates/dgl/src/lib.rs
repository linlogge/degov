//! DeGov DGL Framework
//!
//! A comprehensive framework for building KDL-based domain-specific languages.
//! This library provides:
//!
//! - **Schema Definition**: Define language structure with Rust types
//! - **Validation**: Both sync and async validation with custom functions
//! - **IDE Support**: Semantic analysis, hover, completion, go-to-definition
//! - **Graph Conversion**: Convert DGL to petgraph for analysis
//! - **Error Reporting**: Rich diagnostics with miette integration
//!
//! # Example
//!
//! ```rust,ignore
//! use degov_dgl::prelude::*;
//!
//! // Define your schema
//! let mut schema = Schema::new("my-dgl");
//! schema.add_root("definition");
//! schema.define_node("definition", NodeDef::new()
//!     .with_description("Root definition node")
//!     .with_child(ChildDef::new("metadata").required())
//! );
//!
//! // Parse a document
//! let parser = Parser::new(source, "file.kdl".to_string())
//!     .with_schema(schema);
//!
//! let parsed = parser.parse()?;
//!
//! // Access the graph
//! let graph = &parsed.graph;
//! let stats = graph.stats();
//! ```

mod error;
mod span;
mod parser;
mod schema;
mod validation;
pub mod semantic;
pub mod syntax;

// v1 schema implementation
pub mod v1;

// Re-export main types
pub use error::{DglError, DglDiagnostic, DiagnosticKind, Result};
pub use span::Spanned;
pub use schema::{
    Schema, NodeDef, ArgumentDef, PropertyDef, ValueType, KdlValue,
    EnumDef, ValidatorDef, TypeValidatorDef, ValidationContext, ValidationError, ValidationResult,
    CompletionItem, CompletionKind, SchemaModifier,
};
pub use validation::{
    Validator, AsyncValidator, ValidatorRegistry, ValidationPipeline,
    FnValidator, AsyncFnValidator, builtin,
};
pub use semantic::{
    SemanticInfo, Symbol, SymbolKind, Reference, DocumentSymbol, 
    HoverInfo, HoverContent, CompletionEngine,
};
pub use parser::{Parser, ParsedDocument};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Schema, NodeDef, ArgumentDef, SchemaModifier, PropertyDef, ValueType,
        EnumDef, ValidatorDef, TypeValidatorDef, CompletionItem, CompletionKind,
        Parser, ParsedDocument,
        Validator, AsyncValidator, ValidatorRegistry,
        Result, DglError,
    };
}
