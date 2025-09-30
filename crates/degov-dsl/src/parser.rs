use crate::{Definition, DslError, Nsid, Result, DependencyGraph, DataModel};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Parser for DeGov DSL YAML files
pub struct Parser {
    /// Root directory for service definitions
    root: PathBuf,
}

impl Parser {
    /// Create a new parser with the given root directory
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
        }
    }
    
    /// Parse a single YAML file
    pub fn parse_file(&self, path: impl AsRef<Path>) -> Result<Definition> {
        Definition::from_file(path)
    }
    
    /// Parse a YAML string
    pub fn parse_yaml(&self, yaml: &str) -> Result<Definition> {
        Definition::from_yaml(yaml)
    }
    
    /// Load a definition by its NSID
    /// Example: "de.berlin/business-registration" looks for:
    /// - services/de/berlin/business-registration/model.yaml (DataModel)
    /// - services/de/berlin/business-registration/service.yaml (Service)
    /// - services/de/berlin/business-registration/workflow.yaml (Workflow with #workflow fragment)
    pub fn load_by_nsid(&self, nsid: &Nsid) -> Result<Vec<Definition>> {
        let path = self.nsid_to_path(nsid)?;
        self.load_directory(&path)
    }
    
    /// Load a definition by parsing an NSID string
    pub fn load_by_nsid_str(&self, nsid: &str) -> Result<Vec<Definition>> {
        let nsid = nsid.parse::<Nsid>()
            .map_err(|e| DslError::InvalidNsid(format!("{}: {}", nsid, e)))?;
        self.load_by_nsid(&nsid)
    }
    
    /// Load all definitions from a directory
    pub fn load_directory(&self, path: impl AsRef<Path>) -> Result<Vec<Definition>> {
        let path = path.as_ref();
        let mut definitions = Vec::new();
        
        if !path.exists() {
            return Err(DslError::FileRead(
                path.to_path_buf(),
                std::io::Error::new(std::io::ErrorKind::NotFound, "Directory not found"),
            ));
        }
        
        // Look for common definition files
        let file_names = [
            "service.yaml",
            "model.yaml",
            "workflow.yaml",
            "permissions.yaml",
            "credential.yaml",
            "plugin.yaml",
        ];
        
        for file_name in &file_names {
            let file_path = path.join(file_name);
            if file_path.exists() {
                match self.parse_file(&file_path) {
                    Ok(def) => definitions.push(def),
                    Err(e) => eprintln!("Warning: Failed to parse {}: {}", file_path.display(), e),
                }
            }
        }
        
        Ok(definitions)
    }
    
    /// Convert an NSID to a file system path
    /// Example: "de.berlin/business-registration" -> "services/de/berlin/business-registration"
    fn nsid_to_path(&self, nsid: &Nsid) -> Result<PathBuf> {
        let authority = nsid.authority();
        let entity = nsid.entity();
        
        // Split authority by dots and build path
        // "de.berlin" -> "de/berlin"
        let authority_path = authority.replace('.', "/");
        
        Ok(self.root.join(authority_path).join(entity))
    }
    
    /// Discover all services in the root directory
    pub fn discover_services(&self) -> Result<Vec<Definition>> {
        let mut definitions = Vec::new();
        
        if !self.root.exists() {
            return Ok(definitions);
        }
        
        self.discover_recursive(&self.root, &mut definitions)?;
        
        Ok(definitions)
    }
    
    /// Recursively discover definitions in a directory tree
    fn discover_recursive(&self, dir: &Path, definitions: &mut Vec<Definition>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }
        
        // Try to load definitions from this directory
        match self.load_directory(dir) {
            Ok(defs) => definitions.extend(defs),
            Err(_) => {
                // Not a service directory, continue searching subdirectories
            }
        }
        
        // Search subdirectories
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        self.discover_recursive(&entry.path(), definitions)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Load a model with all its dependencies and resolve inheritance
    pub fn load_and_resolve_model(&self, nsid: &str) -> Result<DataModel> {
        let mut graph = DependencyGraph::new();
        graph.load_model_with_dependencies(self, nsid)?;
        
        let resolved = graph.resolve_all()?;
        resolved.get(nsid)
            .cloned()
            .ok_or_else(|| DslError::MissingField(
                format!("Model not found after resolution: {}", nsid)
            ))
    }
    
    /// Load multiple models and resolve all their inheritance relationships
    pub fn load_and_resolve_models(&self, nsids: &[&str]) -> Result<HashMap<String, DataModel>> {
        let mut graph = DependencyGraph::new();
        
        for nsid in nsids {
            graph.load_model_with_dependencies(self, nsid)?;
        }
        
        graph.resolve_all()
    }
    
    /// Discover all models in the workspace and resolve their inheritance
    pub fn discover_and_resolve_all(&self) -> Result<HashMap<String, DataModel>> {
        let definitions = self.discover_services()?;
        let mut graph = DependencyGraph::new();
        
        // Add all DataModel definitions to the graph
        for def in definitions {
            if let Definition::DataModel(model) = def {
                graph.add_model(model)?;
            }
        }
        
        // Resolve all inheritance relationships
        graph.resolve_all()
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new("services")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_nsid_to_path() {
        let parser = Parser::new("services");
        
        let nsid: Nsid = "de.berlin/business-registration".parse().unwrap();
        let path = parser.nsid_to_path(&nsid).unwrap();
        assert_eq!(
            path,
            PathBuf::from("services/de/berlin/business-registration")
        );
        
        // Test with hash fragment
        let nsid: Nsid = "de.berlin/business-registration#workflow".parse().unwrap();
        let path = parser.nsid_to_path(&nsid).unwrap();
        assert_eq!(
            path,
            PathBuf::from("services/de/berlin/business-registration")
        );
    }
    
    #[test]
    fn test_load_by_nsid_str() {
        let parser = Parser::new("services");
        
        // Test with invalid NSID string
        assert!(parser.load_by_nsid_str("invalid").is_err());
        assert!(parser.load_by_nsid_str("too/many/parts").is_err());
    }
}

