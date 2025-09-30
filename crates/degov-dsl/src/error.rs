use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DslError {
    #[error("Failed to parse YAML: {0}")]
    YamlParse(#[from] serde_yaml::Error),
    
    #[error("Failed to read file {0}: {1}")]
    FileRead(PathBuf, #[source] std::io::Error),
    
    #[error("Invalid NSID format: {0}")]
    InvalidNsid(String),
    
    #[error("Invalid DID format: {0}")]
    InvalidDid(String),
    
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Validation error: {0}")]
    Validation(String),
    
    #[error("Circular dependency error: {0}")]
    CircularDependency(String),
}

pub type Result<T> = std::result::Result<T, DslError>;

