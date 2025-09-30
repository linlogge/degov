use degov_dsl::{DependencyGraph, DataModel};
use degov_dsl::metadata::{ApiVersion, Authority, Metadata};
use degov_dsl::model::{DataModelSpec, Property, PropertyType, Schema, SchemaType};
use indexmap::IndexMap;

fn create_test_model(id: &str, inherits: Vec<&str>, add_properties: bool) -> DataModel {
    use std::collections::HashMap;
    
    let mut properties = IndexMap::new();
    
    if add_properties {
        // Add a property specific to this model
        let field_name = id.split('/').last().unwrap().replace("-", "_");
        properties.insert(
            format!("{}_field", field_name),
            Property {
                property_type: PropertyType::String,
                r#ref: None,
                format: None,
                description: Some(format!("Field from {}", id)),
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
    }
    
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
                properties,
                items: None,
                required: Vec::new(),
            },
            indexes: Vec::new(),
            computed: IndexMap::new(),
        },
    }
}

#[test]
fn test_simple_inheritance_resolution() {
    let mut graph = DependencyGraph::new();
    
    // Create base model with one property
    let base = create_test_model("test.org/person", vec![], true);
    
    // Create derived model that inherits from base
    let citizen = create_test_model("test.org/citizen", vec!["test.org/person"], true);
    
    graph.add_model(base).unwrap();
    graph.add_model(citizen).unwrap();
    
    // Resolve all models
    let resolved = graph.resolve_all().unwrap();
    
    // Check that citizen has both its own property and the inherited one
    let resolved_citizen = resolved.get("test.org/citizen").unwrap();
    assert!(resolved_citizen.spec.schema.properties.contains_key("person_field"));
    assert!(resolved_citizen.spec.schema.properties.contains_key("citizen_field"));
    assert_eq!(resolved_citizen.spec.schema.properties.len(), 2);
    
    // Check that inherits field is cleared after resolution
    assert_eq!(resolved_citizen.spec.inherits.len(), 0);
}

#[test]
fn test_multiple_inheritance() {
    let mut graph = DependencyGraph::new();
    
    // Create two base models
    let person = create_test_model("test.org/person", vec![], true);
    let taxpayer = create_test_model("test.org/taxpayer", vec![], true);
    
    // Create a model that inherits from both
    let citizen = create_test_model("test.org/citizen", vec!["test.org/person", "test.org/taxpayer"], true);
    
    graph.add_model(person).unwrap();
    graph.add_model(taxpayer).unwrap();
    graph.add_model(citizen).unwrap();
    
    // Resolve all models
    let resolved = graph.resolve_all().unwrap();
    
    // Check that citizen has all three properties
    let resolved_citizen = resolved.get("test.org/citizen").unwrap();
    assert!(resolved_citizen.spec.schema.properties.contains_key("person_field"));
    assert!(resolved_citizen.spec.schema.properties.contains_key("taxpayer_field"));
    assert!(resolved_citizen.spec.schema.properties.contains_key("citizen_field"));
    assert_eq!(resolved_citizen.spec.schema.properties.len(), 3);
}

#[test]
fn test_deep_inheritance_chain() {
    let mut graph = DependencyGraph::new();
    
    // Create a chain: base -> middle -> derived
    let base = create_test_model("test.org/base", vec![], true);
    let middle = create_test_model("test.org/middle", vec!["test.org/base"], true);
    let derived = create_test_model("test.org/derived", vec!["test.org/middle"], true);
    
    graph.add_model(base).unwrap();
    graph.add_model(middle).unwrap();
    graph.add_model(derived).unwrap();
    
    // Resolve all models
    let resolved = graph.resolve_all().unwrap();
    
    // Check that derived has all three properties
    let resolved_derived = resolved.get("test.org/derived").unwrap();
    assert!(resolved_derived.spec.schema.properties.contains_key("base_field"));
    assert!(resolved_derived.spec.schema.properties.contains_key("middle_field"));
    assert!(resolved_derived.spec.schema.properties.contains_key("derived_field"));
    assert_eq!(resolved_derived.spec.schema.properties.len(), 3);
}

#[test]
fn test_diamond_inheritance() {
    let mut graph = DependencyGraph::new();
    
    // Create diamond: base -> [left, right] -> derived
    let base = create_test_model("test.org/base", vec![], true);
    let left = create_test_model("test.org/left", vec!["test.org/base"], true);
    let right = create_test_model("test.org/right", vec!["test.org/base"], true);
    let derived = create_test_model("test.org/derived", vec!["test.org/left", "test.org/right"], true);
    
    graph.add_model(base).unwrap();
    graph.add_model(left).unwrap();
    graph.add_model(right).unwrap();
    graph.add_model(derived).unwrap();
    
    // Resolve all models
    let resolved = graph.resolve_all().unwrap();
    
    // Check that derived has all four properties (base property appears once)
    let resolved_derived = resolved.get("test.org/derived").unwrap();
    assert!(resolved_derived.spec.schema.properties.contains_key("base_field"));
    assert!(resolved_derived.spec.schema.properties.contains_key("left_field"));
    assert!(resolved_derived.spec.schema.properties.contains_key("right_field"));
    assert!(resolved_derived.spec.schema.properties.contains_key("derived_field"));
    assert_eq!(resolved_derived.spec.schema.properties.len(), 4);
}

#[test]
fn test_property_override() {
    let mut graph = DependencyGraph::new();
    
    // Create base model with a property
    let mut base = create_test_model("test.org/base", vec![], false);
    base.spec.schema.properties.insert(
        "name".to_string(),
        Property {
            property_type: PropertyType::String,
            description: Some("Base name".to_string()),
            required: false,
            nullable: false,
            immutable: false,
            indexed: false,
            encrypted: false,
            pii: false,
            generated: false,
            r#ref: None,
            format: None,
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
    
    // Create derived model that overrides the property
    let mut derived = create_test_model("test.org/derived", vec!["test.org/base"], false);
    derived.spec.schema.properties.insert(
        "name".to_string(),
        Property {
            property_type: PropertyType::String,
            description: Some("Derived name (overridden)".to_string()),
            required: true, // Different from base
            nullable: false,
            immutable: false,
            indexed: false,
            encrypted: false,
            pii: false,
            generated: false,
            r#ref: None,
            format: None,
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
    
    // Resolve all models
    let resolved = graph.resolve_all().unwrap();
    
    // Check that the derived version is used
    let resolved_derived = resolved.get("test.org/derived").unwrap();
    let name_prop = resolved_derived.spec.schema.properties.get("name").unwrap();
    assert_eq!(name_prop.description, Some("Derived name (overridden)".to_string()));
    assert_eq!(name_prop.required, true);
}

#[test]
fn test_circular_dependency_detection() {
    let mut graph = DependencyGraph::new();
    
    // Create circular dependency: A -> B -> C -> A
    let model_a = create_test_model("test.org/a", vec!["test.org/b"], false);
    let model_b = create_test_model("test.org/b", vec!["test.org/c"], false);
    let model_c = create_test_model("test.org/c", vec!["test.org/a"], false);
    
    graph.add_model(model_a).unwrap();
    graph.add_model(model_b).unwrap();
    graph.add_model(model_c).unwrap();
    
    // Should detect cycle
    assert!(graph.detect_cycles().is_err());
    assert!(graph.topological_sort().is_err());
}

#[test]
fn test_get_dependencies_and_dependents() {
    let mut graph = DependencyGraph::new();
    
    let base = create_test_model("test.org/base", vec![], false);
    let middle = create_test_model("test.org/middle", vec!["test.org/base"], false);
    let derived = create_test_model("test.org/derived", vec!["test.org/middle"], false);
    
    graph.add_model(base).unwrap();
    graph.add_model(middle).unwrap();
    graph.add_model(derived).unwrap();
    
    // Check dependencies (what a model inherits from)
    let middle_deps = graph.get_dependencies("test.org/middle").unwrap();
    assert_eq!(middle_deps.len(), 1);
    assert!(middle_deps.contains(&"test.org/base".to_string()));
    
    // Check dependents (what inherits from a model)
    let base_dependents = graph.get_dependents("test.org/base").unwrap();
    assert_eq!(base_dependents.len(), 1);
    assert!(base_dependents.contains(&"test.org/middle".to_string()));
}

#[test]
fn test_empty_graph() {
    let graph = DependencyGraph::new();
    assert!(graph.is_empty());
    assert_eq!(graph.len(), 0);
    
    let resolved = graph.resolve_all().unwrap();
    assert!(resolved.is_empty());
}

#[test]
fn test_no_inheritance() {
    let mut graph = DependencyGraph::new();
    
    // Create a model with no inheritance
    let model = create_test_model("test.org/standalone", vec![], true);
    graph.add_model(model).unwrap();
    
    // Resolve should work fine
    let resolved = graph.resolve_all().unwrap();
    let resolved_model = resolved.get("test.org/standalone").unwrap();
    
    // Should have its own property
    assert!(resolved_model.spec.schema.properties.contains_key("standalone_field"));
    assert_eq!(resolved_model.spec.schema.properties.len(), 1);
}

