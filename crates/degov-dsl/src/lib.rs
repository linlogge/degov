//! DeGov DSL Parser
//!
//! A parser for the DeGov KDL DSL that defines government services,
//! data models, workflows, permissions, and credentials.
//!
//! Uses the KDL parser with rich diagnostic error reporting powered by miette.

mod error;
mod span;
mod parser;

pub use span::Spanned;
pub use parser::{Parser, Definition};
pub use error::{DslError, DslDiagnostic, DiagnosticKind, Result};
