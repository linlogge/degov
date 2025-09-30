use degov_core::Nsid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API version for DeGov DSL
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum ApiVersion {
    #[serde(rename = "degov.gov/v1")]
    V1,
    #[serde(rename = "v1")]
    V1Short,
}

/// Kind of DSL definition
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Kind {
    Service,
    DataModel,
    Workflow,
    Permission,
    Credential,
    Plugin,
    Test,
    Migration,
    Deployment,
}

/// Common metadata for all DSL definitions
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Metadata {
    /// Namespaced ID (NSID) in AT Protocol Lexicon format
    /// Example: "de.berlin/business-registration" or "de.berlin/business-registration#workflow"
    pub id: Nsid,
    
    /// Human-readable title
    pub title: String,
    
    /// Detailed description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    
    /// Version number (semantic versioning)
    pub version: String,
    
    /// Authority information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority: Option<Authority>,
    
    /// Tags for categorization
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    
    /// Additional metadata
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml::Value>,
}

/// Authority that owns and manages a definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Authority {
    /// Decentralized Identifier (DID) of the authority
    pub did: String,
    
    /// Human-readable name of the authority
    pub name: String,
    
    /// Logo URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    
    /// Contact email
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

impl Metadata {
    /// Extract the authority from the NSID (e.g., "de.berlin" from "de.berlin/business")
    pub fn nsid_authority(&self) -> &str {
        self.id.authority()
    }
    
    /// Extract the entity name from the NSID (e.g., "business" from "de.berlin/business")
    pub fn nsid_entity(&self) -> &str {
        self.id.entity()
    }
    
    /// Extract the hash fragment type if present (e.g., "workflow" from "de.berlin/business#workflow")
    pub fn nsid_fragment(&self) -> Option<&str> {
        self.id.fragment()
    }
    
    /// Check if this is a federal (de.bund) definition
    pub fn is_federal(&self) -> bool {
        self.id.is_federal()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_nsid_parsing() {
        let meta = Metadata {
            id: "de.berlin/business-registration#workflow".parse().unwrap(),
            title: "Test".to_string(),
            description: None,
            version: "1.0.0".to_string(),
            authority: None,
            tags: vec![],
            extra: HashMap::new(),
        };
        
        assert_eq!(meta.nsid_authority(), "de.berlin");
        assert_eq!(meta.nsid_entity(), "business-registration");
        assert_eq!(meta.nsid_fragment(), Some("workflow"));
        assert_eq!(meta.is_federal(), false);
    }
    
    #[test]
    fn test_federal_detection() {
        let meta = Metadata {
            id: "de.bund/person".parse().unwrap(),
            title: "Person".to_string(),
            description: None,
            version: "1.0.0".to_string(),
            authority: None,
            tags: vec![],
            extra: HashMap::new(),
        };
        
        assert_eq!(meta.is_federal(), true);
    }
}

