use crate::{DataModel, Definition, DslError, Nsid, Parser, Result};
use indexmap::IndexMap;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::{toposort, is_cyclic_directed};
use petgraph::Direction;
use std::collections::{HashMap, HashSet, VecDeque};

/// A dependency graph for resolving model inheritance using petgraph
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// The underlying directed graph
    graph: DiGraph<String, ()>,
    
    /// Map of model NSID to its node index in the graph
    node_indices: HashMap<String, NodeIndex>,
    
    /// Map of model NSID to its definition
    models: HashMap<String, DataModel>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
            models: HashMap::new(),
        }
    }
    
    /// Add a model to the graph
    pub fn add_model(&mut self, model: DataModel) -> Result<()> {
        let nsid = model.metadata.id.to_string();
        let inherits = model.spec.inherits.clone();
        
        // Get or create node for this model
        let node_idx = *self.node_indices.entry(nsid.clone())
            .or_insert_with(|| self.graph.add_node(nsid.clone()));
        
        // Store the model
        self.models.insert(nsid.clone(), model);
        
        // Add edges from this model to its parents (child -> parent)
        for parent_nsid in &inherits {
            // Get or create node for parent
            let parent_idx = *self.node_indices.entry(parent_nsid.clone())
                .or_insert_with(|| self.graph.add_node(parent_nsid.clone()));
            
            // Add edge: child -> parent (dependency direction)
            self.graph.add_edge(node_idx, parent_idx, ());
        }
        
        Ok(())
    }
    
    /// Add multiple models to the graph
    pub fn add_models(&mut self, models: Vec<DataModel>) -> Result<()> {
        for model in models {
            self.add_model(model)?;
        }
        Ok(())
    }
    
    /// Load a model and all its dependencies from the parser
    pub fn load_model_with_dependencies(
        &mut self,
        parser: &Parser,
        nsid: &str,
    ) -> Result<()> {
        let mut to_load = VecDeque::new();
        let mut loaded = HashSet::new();
        
        to_load.push_back(nsid.to_string());
        
        while let Some(current_nsid) = to_load.pop_front() {
            if loaded.contains(&current_nsid) {
                continue;
            }
            
            // Parse the NSID
            let parsed_nsid = current_nsid.parse::<Nsid>()
                .map_err(|e| DslError::InvalidNsid(format!("{}: {}", current_nsid, e)))?;
            
            // Load the definitions
            let definitions = parser.load_by_nsid(&parsed_nsid)?;
            
            // Find the DataModel definition
            let model = definitions.iter()
                .find_map(|def| match def {
                    Definition::DataModel(m) => Some(m.clone()),
                    _ => None,
                })
                .ok_or_else(|| DslError::MissingField(
                    format!("No DataModel found for {}", current_nsid)
                ))?;
            
            // Add parent dependencies to the queue
            for parent in &model.spec.inherits {
                if !loaded.contains(parent) {
                    to_load.push_back(parent.clone());
                }
            }
            
            // Add the model to the graph
            self.add_model(model)?;
            loaded.insert(current_nsid);
        }
        
        Ok(())
    }
    
    /// Check for circular dependencies in the graph
    pub fn detect_cycles(&self) -> Result<()> {
        if is_cyclic_directed(&self.graph) {
            Err(DslError::CircularDependency(
                "Circular inheritance detected in the dependency graph".to_string()
            ))
        } else {
            Ok(())
        }
    }
    
    /// Perform topological sort to get the resolution order
    /// Returns models in order from derived to base (child to parent)
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        match toposort(&self.graph, None) {
            Ok(sorted_indices) => {
                Ok(sorted_indices.iter()
                    .map(|&idx| self.graph[idx].clone())
                    .collect())
            }
            Err(_) => Err(DslError::CircularDependency(
                "Circular dependency detected during topological sort".to_string()
            ))
        }
    }
    
    /// Resolve all models in the graph by merging parent schemas
    /// Returns a map of resolved models
    pub fn resolve_all(&self) -> Result<HashMap<String, DataModel>> {
        // Get topological order (children first, then parents)
        let order = self.topological_sort()?;
        let mut resolved: HashMap<String, DataModel> = HashMap::new();
        
        // Process in reverse order (parents first, then children)
        for nsid in order.iter().rev() {
            let model = self.models.get(nsid)
                .ok_or_else(|| DslError::MissingField(
                    format!("Model not found: {}", nsid)
                ))?;
            
            let resolved_model = self.resolve_model(model, &resolved)?;
            resolved.insert(nsid.clone(), resolved_model);
        }
        
        Ok(resolved)
    }
    
    /// Resolve a single model by merging its parent schemas
    fn resolve_model(
        &self,
        model: &DataModel,
        resolved_cache: &HashMap<String, DataModel>,
    ) -> Result<DataModel> {
        // If no inheritance, return as-is
        if model.spec.inherits.is_empty() {
            return Ok(model.clone());
        }
        
        // Start with an empty base model
        let mut merged = model.clone();
        let mut merged_properties = IndexMap::new();
        let mut merged_required = Vec::new();
        let mut merged_indexes = Vec::new();
        let mut merged_computed = IndexMap::new();
        
        // Merge each parent in order
        for parent_nsid in &model.spec.inherits {
            let parent = resolved_cache.get(parent_nsid)
                .ok_or_else(|| DslError::MissingField(
                    format!("Parent model not found or not yet resolved: {}", parent_nsid)
                ))?;
            
            // Merge properties (child overrides parent)
            for (name, prop) in &parent.spec.schema.properties {
                if !merged_properties.contains_key(name) {
                    merged_properties.insert(name.clone(), prop.clone());
                }
            }
            
            // Merge required fields
            for req in &parent.spec.schema.required {
                if !merged_required.contains(req) {
                    merged_required.push(req.clone());
                }
            }
            
            // Merge indexes
            for index in &parent.spec.indexes {
                if !merged_indexes.iter().any(|i: &crate::model::Index| i.name == index.name) {
                    merged_indexes.push(index.clone());
                }
            }
            
            // Merge computed fields
            for (name, computed) in &parent.spec.computed {
                if !merged_computed.contains_key(name) {
                    merged_computed.insert(name.clone(), computed.clone());
                }
            }
        }
        
        // Now merge the child's own properties (these override parent)
        for (name, prop) in &model.spec.schema.properties {
            merged_properties.insert(name.clone(), prop.clone());
        }
        
        // Merge child's required fields
        for req in &model.spec.schema.required {
            if !merged_required.contains(req) {
                merged_required.push(req.clone());
            }
        }
        
        // Merge child's indexes
        for index in &model.spec.indexes {
            // Remove any parent index with the same name
            merged_indexes.retain(|i| i.name != index.name);
            merged_indexes.push(index.clone());
        }
        
        // Merge child's computed fields
        for (name, computed) in &model.spec.computed {
            merged_computed.insert(name.clone(), computed.clone());
        }
        
        // Update the merged model
        merged.spec.schema.properties = merged_properties;
        merged.spec.schema.required = merged_required;
        merged.spec.indexes = merged_indexes;
        merged.spec.computed = merged_computed;
        
        // Clear inherits since we've resolved them
        merged.spec.inherits = Vec::new();
        
        Ok(merged)
    }
    
    /// Get a model from the graph
    pub fn get_model(&self, nsid: &str) -> Option<&DataModel> {
        self.models.get(nsid)
    }
    
    /// Get all models in the graph
    pub fn models(&self) -> &HashMap<String, DataModel> {
        &self.models
    }
    
    /// Get the dependencies of a model (models it inherits from)
    pub fn get_dependencies(&self, nsid: &str) -> Option<Vec<String>> {
        let node_idx = self.node_indices.get(nsid)?;
        let deps: Vec<String> = self.graph
            .neighbors_directed(*node_idx, Direction::Outgoing)
            .map(|idx| self.graph[idx].clone())
            .collect();
        Some(deps)
    }
    
    /// Get the models that depend on a given model (models that inherit from it)
    pub fn get_dependents(&self, nsid: &str) -> Option<Vec<String>> {
        let node_idx = self.node_indices.get(nsid)?;
        let dependents: Vec<String> = self.graph
            .neighbors_directed(*node_idx, Direction::Incoming)
            .map(|idx| self.graph[idx].clone())
            .collect();
        Some(dependents)
    }
    
    /// Get the number of models in the graph
    pub fn len(&self) -> usize {
        self.models.len()
    }
    
    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.models.is_empty()
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{ApiVersion, Authority, Metadata};
    use crate::model::{DataModelSpec, Property, PropertyType, Schema, SchemaType};
    
    fn create_test_model(id: &str, inherits: Vec<&str>) -> DataModel {
        use std::collections::HashMap;
        
        DataModel {
            api_version: ApiVersion::V1,
            kind: "DataModel".to_string(),
            metadata: Metadata {
                id: id.parse().unwrap(),
                title: format!("Test Model {}", id),
                description: Some(format!("Test description for {}", id)),
                version: "1.0.0".to_string(),
                authority: Some(Authority {
                    name: "Test Authority".to_string(),
                    did: "did:test:authority".to_string(),
                    logo: None,
                    email: None,
                }),
                tags: Vec::new(),
                extra: HashMap::new(),
            },
            spec: DataModelSpec {
                inherits: inherits.iter().map(|s| s.to_string()).collect(),
                storage: None,
                schema: Schema {
                    schema_type: SchemaType::Object,
                    properties: IndexMap::new(),
                    items: None,
                    required: Vec::new(),
                },
                indexes: Vec::new(),
                computed: IndexMap::new(),
            },
        }
    }
    
    #[test]
    fn test_add_model() {
        let mut graph = DependencyGraph::new();
        let model = create_test_model("test.org/model1", vec![]);
        
        graph.add_model(model).unwrap();
        assert_eq!(graph.models.len(), 1);
    }
    
    #[test]
    fn test_simple_inheritance() {
        let mut graph = DependencyGraph::new();
        let base = create_test_model("test.org/base", vec![]);
        let derived = create_test_model("test.org/derived", vec!["test.org/base"]);
        
        graph.add_model(base).unwrap();
        graph.add_model(derived).unwrap();
        
        let order = graph.topological_sort().unwrap();
        assert_eq!(order.len(), 2);
        // In petgraph's toposort, nodes with no dependencies come first
        assert!(order.iter().position(|s| s == "test.org/base").unwrap() 
            > order.iter().position(|s| s == "test.org/derived").unwrap());
    }
    
    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        
        // Create circular dependency: A -> B -> C -> A
        let model_a = create_test_model("test.org/a", vec!["test.org/b"]);
        let model_b = create_test_model("test.org/b", vec!["test.org/c"]);
        let model_c = create_test_model("test.org/c", vec!["test.org/a"]);
        
        graph.add_model(model_a).unwrap();
        graph.add_model(model_b).unwrap();
        graph.add_model(model_c).unwrap();
        
        assert!(graph.detect_cycles().is_err());
    }
    
    #[test]
    fn test_multiple_inheritance() {
        let mut graph = DependencyGraph::new();
        let base1 = create_test_model("test.org/base1", vec![]);
        let base2 = create_test_model("test.org/base2", vec![]);
        let derived = create_test_model("test.org/derived", vec!["test.org/base1", "test.org/base2"]);
        
        graph.add_model(base1).unwrap();
        graph.add_model(base2).unwrap();
        graph.add_model(derived).unwrap();
        
        graph.detect_cycles().unwrap();
        let order = graph.topological_sort().unwrap();
        assert_eq!(order.len(), 3);
    }
    
    #[test]
    fn test_diamond_inheritance() {
        let mut graph = DependencyGraph::new();
        // Diamond: Base -> [Left, Right] -> Derived
        let base = create_test_model("test.org/base", vec![]);
        let left = create_test_model("test.org/left", vec!["test.org/base"]);
        let right = create_test_model("test.org/right", vec!["test.org/base"]);
        let derived = create_test_model("test.org/derived", vec!["test.org/left", "test.org/right"]);
        
        graph.add_model(base).unwrap();
        graph.add_model(left).unwrap();
        graph.add_model(right).unwrap();
        graph.add_model(derived).unwrap();
        
        graph.detect_cycles().unwrap();
        let order = graph.topological_sort().unwrap();
        assert_eq!(order.len(), 4);
    }
    
    #[test]
    fn test_resolve_with_properties() {
        let mut graph = DependencyGraph::new();
        
        // Create base model with a property
        let mut base = create_test_model("test.org/base", vec![]);
        base.spec.schema.properties.insert(
            "baseField".to_string(),
            Property {
                property_type: PropertyType::String,
                r#ref: None,
                format: None,
                description: Some("Base field".to_string()),
                required: false,
                nullable: false,
                immutable: false,
                indexed: false,
                encrypted: false,
                pii: false,
                generated: false,
                default: None,
                values: None,
                pattern: None,
                min_length: None,
                max_length: None,
                min: None,
                max: None,
                min_items: None,
                max_items: None,
                items: None,
                properties: IndexMap::new(),
                validations: Vec::new(),
            }
        );
        base.spec.schema.required.push("baseField".to_string());
        
        // Create derived model with its own property
        let mut derived = create_test_model("test.org/derived", vec!["test.org/base"]);
        derived.spec.schema.properties.insert(
            "derivedField".to_string(),
            Property {
                property_type: PropertyType::Integer,
                r#ref: None,
                format: None,
                description: Some("Derived field".to_string()),
                required: false,
                nullable: false,
                immutable: false,
                indexed: false,
                encrypted: false,
                pii: false,
                generated: false,
                default: None,
                values: None,
                pattern: None,
                min_length: None,
                max_length: None,
                min: None,
                max: None,
                min_items: None,
                max_items: None,
                items: None,
                properties: IndexMap::new(),
                validations: Vec::new(),
            }
        );
        
        graph.add_model(base).unwrap();
        graph.add_model(derived).unwrap();
        
        let resolved = graph.resolve_all().unwrap();
        let resolved_derived = resolved.get("test.org/derived").unwrap();
        
        // Should have both properties
        assert!(resolved_derived.spec.schema.properties.contains_key("baseField"));
        assert!(resolved_derived.spec.schema.properties.contains_key("derivedField"));
        assert!(resolved_derived.spec.schema.required.contains(&"baseField".to_string()));
    }
    
    #[test]
    fn test_get_dependencies() {
        let mut graph = DependencyGraph::new();
        let base = create_test_model("test.org/base", vec![]);
        let derived = create_test_model("test.org/derived", vec!["test.org/base"]);
        
        graph.add_model(base).unwrap();
        graph.add_model(derived).unwrap();
        
        let deps = graph.get_dependencies("test.org/derived").unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], "test.org/base");
    }
    
    #[test]
    fn test_get_dependents() {
        let mut graph = DependencyGraph::new();
        let base = create_test_model("test.org/base", vec![]);
        let derived = create_test_model("test.org/derived", vec!["test.org/base"]);
        
        graph.add_model(base).unwrap();
        graph.add_model(derived).unwrap();
        
        let dependents = graph.get_dependents("test.org/base").unwrap();
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0], "test.org/derived");
    }
}
