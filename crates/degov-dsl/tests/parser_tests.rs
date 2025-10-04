//! Parser Tests
//!
//! Comprehensive test suite for the DSL parser covering:
//! - Basic parsing (empty docs, nodes, properties, children)
//! - Schema validation (required fields, type checking, enums)
//! - Error handling (invalid syntax, type mismatches)
//! - Complex documents (nested structures, mixed syntax)
//! - Real-world scenarios (data models, service definitions)
//! - Edge cases (unicode, special characters, long values)

use degov_dsl::prelude::*;
use degov_dsl::{ArgumentDef, EnumDef, KdlValue};

// ============================================================================
// BASIC PARSING TESTS
// ============================================================================

#[test]
fn test_empty_document() {
    let source = "";
    let parser = Parser::new(source.to_string(), "empty.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
    let doc = result.unwrap();
    assert_eq!(doc.document.nodes().len(), 0);
}

#[test]
fn test_single_node_no_schema() {
    let source = r#"node-name"#;
    let parser = Parser::new(source.to_string(), "single.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
    let doc = result.unwrap();
    assert_eq!(doc.document.nodes().len(), 1);
}

#[test]
fn test_node_with_string_argument() {
    let source = r#"person "John Doe""#;
    let parser = Parser::new(source.to_string(), "arg.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
    let doc = result.unwrap();
    let node = doc.document.nodes().first().unwrap();
    assert_eq!(node.name().value(), "person");
    assert_eq!(node.entries().len(), 1);
}

#[test]
fn test_node_with_properties() {
    let source = r#"person name="John" age="30""#;
    let parser = Parser::new(source.to_string(), "props.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
    let doc = result.unwrap();
    let node = doc.document.nodes().first().unwrap();
    assert!(node.get("name").is_some());
    assert!(node.get("age").is_some());
}

#[test]
fn test_node_with_children() {
    let source = r#"
parent {
    child1
    child2 "value"
}
    "#;
    let parser = Parser::new(source.to_string(), "children.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
    let doc = result.unwrap();
    let parent = doc.document.nodes().first().unwrap();
    let children = parent.children().unwrap();
    assert_eq!(children.nodes().len(), 2);
}

#[test]
fn test_multiple_root_nodes() {
    let source = r#"
node1
node2 "value"
node3 key="value"
    "#;
    let parser = Parser::new(source.to_string(), "multiple.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
    let doc = result.unwrap();
    assert_eq!(doc.document.nodes().len(), 3);
}

// ============================================================================
// SCHEMA VALIDATION TESTS - REQUIRED PROPERTIES
// ============================================================================

#[test]
fn test_required_property_present() {
    let source = r#"person name="John""#;
    
    let root = NodeDef::new("person")
        .with_property("name", PropertyDef::new(ValueType::String).required());
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "required.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_required_property_missing() {
    let source = r#"person age="30""#;
    
    let root = NodeDef::new("person")
        .with_property("name", PropertyDef::new(ValueType::String).required());
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "missing-required.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    // Should fail due to missing required property
    assert!(result.is_err());
}

#[test]
fn test_optional_property_missing() {
    let source = r#"person name="John""#;
    
    let root = NodeDef::new("person")
        .with_property("name", PropertyDef::new(ValueType::String).required())
        .with_property("age", PropertyDef::new(ValueType::Integer)); // optional
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "optional.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

// ============================================================================
// SCHEMA VALIDATION TESTS - TYPE CHECKING
// ============================================================================

#[test]
fn test_string_type_validation() {
    let source = r#"person name="John""#;
    
    let root = NodeDef::new("person")
        .with_property("name", PropertyDef::new(ValueType::String));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "string-type.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_integer_type_validation() {
    let source = r#"person age="30""#;
    
    let root = NodeDef::new("person")
        .with_property("age", PropertyDef::new(ValueType::Integer));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "int-type.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_type_mismatch_error() {
    let source = r#"person age="not-a-number""#;
    
    let root = NodeDef::new("person")
        .with_property("age", PropertyDef::new(ValueType::Integer));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "type-mismatch.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    // Should fail due to type mismatch
    assert!(result.is_err());
}

#[test]
fn test_boolean_type_validation() {
    // KDL boolean syntax uses # prefix: #true or #false
    let source = r#"config enabled=#true"#;
    
    let root = NodeDef::new("config")
        .with_property("enabled", PropertyDef::new(ValueType::Boolean));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "bool-type.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

// ============================================================================
// SCHEMA VALIDATION TESTS - ENUM TYPES
// ============================================================================

#[test]
fn test_enum_valid_value() {
    let source = r#"person status="active""#;
    
    let mut schema = Schema::new("test-schema", NodeDef::new("person"));
    schema.define_enum("Status", EnumDef::new(vec![
        "active".to_string(),
        "inactive".to_string(),
        "pending".to_string(),
    ]));
    
    schema.root = schema.root.clone()
        .with_property("status", PropertyDef::new(ValueType::Enum("Status".to_string())));
    
    let parser = Parser::new(source.to_string(), "enum-valid.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_enum_invalid_value() {
    let source = r#"person status="unknown-status""#;
    
    let mut schema = Schema::new("test-schema", NodeDef::new("person"));
    schema.define_enum("Status", EnumDef::new(vec![
        "active".to_string(),
        "inactive".to_string(),
        "pending".to_string(),
    ]));
    
    schema.root = schema.root.clone()
        .with_property("status", PropertyDef::new(ValueType::Enum("Status".to_string())));
    
    let parser = Parser::new(source.to_string(), "enum-invalid.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    // Should fail due to invalid enum value
    assert!(result.is_err());
}

// ============================================================================
// SCHEMA VALIDATION TESTS - CHILD NODES
// ============================================================================

#[test]
fn test_expected_child_present() {
    let source = r#"
parent {
    required-child "value"
}
    "#;
    
    let root = NodeDef::new("parent")
        .with_child(NodeDef::new("required-child")
            .with_argument(ArgumentDef::new("value", ValueType::String)));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "child-present.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_unexpected_child_node_strict() {
    let source = r#"
parent {
    unexpected-child "value"
}
    "#;
    
    let root = NodeDef::new("parent"); // No children defined, strict mode
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "unexpected-child.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    // Should fail due to unexpected child
    assert!(result.is_err());
}

#[test]
fn test_unexpected_child_node_open_schema() {
    let source = r#"
parent {
    any-child "value"
}
    "#;
    
    let root = NodeDef::new("parent")
        .allow_unknown_children(); // Open schema
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "open-children.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

// ============================================================================
// SCHEMA VALIDATION TESTS - ARGUMENTS
// ============================================================================

#[test]
fn test_required_argument_present() {
    let source = r#"person "John Doe""#;
    
    let root = NodeDef::new("person")
        .with_argument(ArgumentDef::new("name", ValueType::String));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "arg-present.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_required_argument_missing() {
    let source = r#"person"#;
    
    let root = NodeDef::new("person")
        .with_argument(ArgumentDef::new("name", ValueType::String)); // required by default
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "arg-missing.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    // Should fail due to missing argument
    assert!(result.is_err());
}

#[test]
fn test_optional_argument_missing() {
    let source = r#"person"#;
    
    let root = NodeDef::new("person")
        .with_argument(ArgumentDef::new("name", ValueType::String).optional());
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "opt-arg-missing.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_multiple_arguments() {
    let source = r#"person "John" "Doe" "30""#;
    
    let root = NodeDef::new("person")
        .with_argument(ArgumentDef::new("firstName", ValueType::String))
        .with_argument(ArgumentDef::new("lastName", ValueType::String))
        .with_argument(ArgumentDef::new("age", ValueType::Integer));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "multi-args.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

// ============================================================================
// SCHEMA VALIDATION TESTS - DEFAULT VALUES
// ============================================================================

#[test]
fn test_property_with_default_value() {
    let source = r#"person name="John""#;
    
    let root = NodeDef::new("person")
        .with_property("name", PropertyDef::new(ValueType::String).required())
        .with_property("status", PropertyDef::new(ValueType::String)
            .with_default(KdlValue::String("active".to_string())));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "default.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

// ============================================================================
// SCHEMA VALIDATION TESTS - CHILD NODE PROPERTIES
// ============================================================================

#[test]
fn test_property_as_child_node() {
    // Test that properties can be defined as child nodes: node { prop "value" }
    let source = r#"
person {
    name "John"
    age "30"
}
    "#;
    
    let root = NodeDef::new("person")
        .with_property("name", PropertyDef::new(ValueType::String))
        .with_property("age", PropertyDef::new(ValueType::Integer));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "child-props.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_property_mixed_formats() {
    // Test that both formats can be used together
    let source = r#"
person name="John" {
    age "30"
    country "USA"
}
    "#;
    
    let root = NodeDef::new("person")
        .with_property("name", PropertyDef::new(ValueType::String))
        .with_property("age", PropertyDef::new(ValueType::Integer))
        .with_property("country", PropertyDef::new(ValueType::String));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "mixed-props.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_required_property_as_child_node() {
    // Test that required properties work with child node format
    let source = r#"
person {
    name "John"
}
    "#;
    
    let root = NodeDef::new("person")
        .with_property("name", PropertyDef::new(ValueType::String).required());
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "required-child-prop.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_required_property_missing_child_node_format() {
    // Test that missing required property is detected even with mixed formats
    let source = r#"
person {
    age "30"
}
    "#;
    
    let root = NodeDef::new("person")
        .with_property("name", PropertyDef::new(ValueType::String).required())
        .with_property("age", PropertyDef::new(ValueType::Integer));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "missing-child-prop.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_err());
}

#[test]
fn test_type_validation_child_node_property() {
    // Test that type validation works for child node properties
    let source = r#"
person {
    age "not-a-number"
}
    "#;
    
    let root = NodeDef::new("person")
        .with_property("age", PropertyDef::new(ValueType::Integer));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "child-type-mismatch.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_err());
}

// ============================================================================
// ROOT LEVEL PROPERTIES TESTS
// ============================================================================

#[test]
fn test_root_properties_direct_format() {
    // Test properties directly on root node: root prop="value"
    let source = r#"config name="MyApp" version="1.0.0""#;
    
    let root = NodeDef::new("config")
        .with_property("name", PropertyDef::new(ValueType::String))
        .with_property("version", PropertyDef::new(ValueType::String));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "root-direct.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_root_properties_child_format() {
    // Test properties as children on root node: root { prop "value" }
    let source = r#"
config {
    name "MyApp"
    version "1.0.0"
}
    "#;
    
    let root = NodeDef::new("config")
        .with_property("name", PropertyDef::new(ValueType::String))
        .with_property("version", PropertyDef::new(ValueType::String));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "root-child.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_root_properties_mixed_format() {
    // Test mixed property formats on root node
    let source = r#"
config name="MyApp" {
    version "1.0.0"
    debug "true"
}
    "#;
    
    let root = NodeDef::new("config")
        .with_property("name", PropertyDef::new(ValueType::String))
        .with_property("version", PropertyDef::new(ValueType::String))
        .with_property("debug", PropertyDef::new(ValueType::String));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "root-mixed.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_multiple_root_nodes_with_properties() {
    // Test multiple root nodes each with properties
    let source = r#"
person name="John" age="30"
person name="Jane" age="25"
person {
    name "Bob"
    age "35"
}
    "#;
    
    let root = NodeDef::new("person")
        .with_property("name", PropertyDef::new(ValueType::String))
        .with_property("age", PropertyDef::new(ValueType::Integer));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "multi-root-props.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
    let doc = result.unwrap();
    assert_eq!(doc.document.nodes().len(), 3);
}

#[test]
fn test_root_required_property_validation() {
    // Test that required properties are validated at root level
    let source = r#"config version="1.0.0""#;
    
    let root = NodeDef::new("config")
        .with_property("name", PropertyDef::new(ValueType::String).required())
        .with_property("version", PropertyDef::new(ValueType::String));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "root-missing-required.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    // Should fail - missing required "name" property
    assert!(result.is_err());
}

#[test]
fn test_root_properties_with_nested_children() {
    // Test root node with both properties and nested children
    let source = r#"
app name="MyApp" version="1.0.0" {
    database {
        host "localhost"
        port "5432"
    }
    server {
        port "8080"
    }
}
    "#;
    
    let db_node = NodeDef::new("database")
        .with_property("host", PropertyDef::new(ValueType::String))
        .with_property("port", PropertyDef::new(ValueType::Integer));
    
    let server_node = NodeDef::new("server")
        .with_property("port", PropertyDef::new(ValueType::Integer));
    
    let root = NodeDef::new("app")
        .with_property("name", PropertyDef::new(ValueType::String))
        .with_property("version", PropertyDef::new(ValueType::String))
        .with_child(db_node)
        .with_child(server_node);
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "root-props-nested.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_root_type_validation_child_format() {
    // Test type validation for root properties in child format
    let source = r#"
config {
    port "not-a-number"
}
    "#;
    
    let root = NodeDef::new("config")
        .with_property("port", PropertyDef::new(ValueType::Integer));
    let schema = Schema::new("test-schema", root);
    
    let parser = Parser::new(source.to_string(), "root-type-error.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    // Should fail - type mismatch
    assert!(result.is_err());
}

#[test]
fn test_root_enum_validation() {
    // Test enum validation at root level
    let source = r#"
config {
    environment "production"
}
    "#;
    
    let mut schema = Schema::new("test-schema", NodeDef::new("config"));
    schema.define_enum("Environment", EnumDef::new(vec![
        "development".to_string(),
        "staging".to_string(),
        "production".to_string(),
    ]));
    
    schema.root = schema.root.clone()
        .with_property("environment", PropertyDef::new(ValueType::Enum("Environment".to_string())));
    
    let parser = Parser::new(source.to_string(), "root-enum.dgv".to_string())
        .with_schema(schema);
    let result = parser.parse();
    
    assert!(result.is_ok());
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[test]
fn test_invalid_kdl_syntax() {
    let source = r#"
person {
    name "John"
    age 30
    unclosed {
"#;
    let parser = Parser::new(source.to_string(), "invalid-syntax.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_err());
}

#[test]
fn test_malformed_property() {
    let source = r#"person name="#; // Missing value
    let parser = Parser::new(source.to_string(), "malformed.dgv".to_string());
    let result = parser.parse();
    
    // This is actually invalid KDL syntax - the test expectation was wrong
    // KDL requires a value after =
    assert!(result.is_err());
}

// ============================================================================
// COMPLEX DOCUMENT TESTS
// ============================================================================

#[test]
fn test_nested_children() {
    let source = r#"
root {
    level1 {
        level2 {
            level3 "value"
        }
    }
}
    "#;
    let parser = Parser::new(source.to_string(), "nested.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_mixed_arguments_and_properties() {
    let source = r#"person "John" "Doe" age="30" country="USA""#;
    let parser = Parser::new(source.to_string(), "mixed.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_comments_ignored() {
    let source = r#"
// This is a comment
person "John" // inline comment
/* block
   comment */
    "#;
    let parser = Parser::new(source.to_string(), "comments.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
}

// ============================================================================
// REAL-WORLD SCENARIO TESTS
// ============================================================================

#[test]
fn test_data_model_structure() {
    let source = r#"
definition {
    metadata {
        id "de.degov/natural-person"
        version "1.0.0"
        title "Natural Person"
    }
    
    fields {
        field "givenName" type="string" required="true"
        field "familyName" type="string" required="true"
        field "birthDate" type="date" required="true"
    }
}
    "#;
    let parser = Parser::new(source.to_string(), "data-model.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_service_definition_structure() {
    let source = r#"
service {
    metadata {
        id "de.berlin/identity-card"
        name "Identity Card Service"
    }
    
    endpoints {
        endpoint "create" method="POST" path="/identity-card"
        endpoint "verify" method="GET" path="/identity-card/verify"
    }
}
    "#;
    let parser = Parser::new(source.to_string(), "service.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn test_empty_string_values() {
    let source = r#"node value="""#;
    let parser = Parser::new(source.to_string(), "empty-string.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_unicode_content() {
    let source = r#"person name="日本" country="Deutschland" emoji="🎉""#;
    let parser = Parser::new(source.to_string(), "unicode.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_special_characters_in_strings() {
    let source = r#"text value="Hello\nWorld\t\"quoted\"""#;
    let parser = Parser::new(source.to_string(), "special-chars.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
}

#[test]
fn test_very_long_property_value() {
    let long_string = "a".repeat(10000);
    let source = format!(r#"node value="{}""#, long_string);
    let parser = Parser::new(source, "long-value.dgv".to_string());
    let result = parser.parse();
    
    assert!(result.is_ok());
}

