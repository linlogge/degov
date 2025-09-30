pub mod error;
pub mod metadata;
pub mod model;
pub mod service;
pub mod workflow;
pub mod permission;
pub mod credential;
pub mod parser;
pub mod graph;

// Re-export Nsid from degov-core for convenience
pub use degov_core::{Nsid, NsidError};

pub use error::{DslError, Result};
pub use metadata::{Metadata, ApiVersion, Kind};
pub use model::{DataModel, DataModelSpec};
pub use service::{Service, ServiceSpec};
pub use workflow::{Workflow, WorkflowSpec};
pub use permission::{Permission, PermissionSpec};
pub use credential::{Credential, CredentialSpec};
pub use parser::Parser;
pub use graph::DependencyGraph;

/// Common definition structure that wraps all DSL types
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind")]
pub enum Definition {
    Service(Service),
    DataModel(DataModel),
    Workflow(Workflow),
    Permission(Permission),
    Credential(Credential),
}

impl Definition {
    /// Parse a YAML string into a Definition
    pub fn from_yaml(yaml: &str) -> Result<Self> {
        serde_yaml::from_str(yaml).map_err(DslError::YamlParse)
    }
    
    /// Parse a YAML file into a Definition
    pub fn from_file(path: impl AsRef<std::path::Path>) -> Result<Self> {
        let content = std::fs::read_to_string(path.as_ref())
            .map_err(|e| DslError::FileRead(path.as_ref().to_path_buf(), e))?;
        Self::from_yaml(&content)
    }
    
    /// Get the metadata for this definition
    pub fn metadata(&self) -> &Metadata {
        match self {
            Definition::Service(s) => &s.metadata,
            Definition::DataModel(m) => &m.metadata,
            Definition::Workflow(w) => &w.metadata,
            Definition::Permission(p) => &p.metadata,
            Definition::Credential(c) => &c.metadata,
        }
    }
}

