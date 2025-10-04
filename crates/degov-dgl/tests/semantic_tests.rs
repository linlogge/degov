//! Tests for semantic analysis and IDE features

use degov_dgl::prelude::*;
use degov_dgl::semantic::{CompletionEngine, SemanticInfo};

#[test]
fn test_semantic_info_basic() {
    // Create a simple schema with properties
    let root = NodeDef::new("")
        .with_property(
            "id",
            PropertyDef::new(ValueType::String)
                .required()
                .with_description("Unique identifier"),
        )
        .with_property(
            "version",
            PropertyDef::new(ValueType::String)
                .with_description("Version number"),
        );
    
    let schema = Schema::new("test", root);
    
    // Parse a document
    let source = r#"
id "my-id"
version "1.0.0"
    "#;
    
    let parser = Parser::new(source.to_string(), "test.dgl".to_string())
        .with_schema(schema.clone());
    
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse successfully");
    
    let doc = result.unwrap();
    
    // Analyze the document
    let semantic_info = SemanticInfo::analyze(&doc.document, &schema, source);
    
    // Check document symbols
    assert_eq!(semantic_info.document_symbols.len(), 2);
    assert_eq!(semantic_info.document_symbols[0].name, "id");
    assert_eq!(semantic_info.document_symbols[1].name, "version");
    
    // Check hover info
    assert!(!semantic_info.hover_info.is_empty());
}

#[test]
fn test_hover_info_at_position() {
    let root = NodeDef::new("")
        .with_property(
            "name",
            PropertyDef::new(ValueType::String)
                .with_description("The name property"),
        );
    
    let schema = Schema::new("test", root);
    
    let source = r#"name "test-value""#;
    
    let parser = Parser::new(source.to_string(), "test.dgl".to_string())
        .with_schema(schema.clone());
    
    let result = parser.parse();
    assert!(result.is_ok());
    
    let doc = result.unwrap();
    let semantic_info = SemanticInfo::analyze(&doc.document, &schema, source);
    
    // Try to get hover info at the start of "name"
    let hover = semantic_info.get_hover_at(0);
    assert!(hover.is_some(), "Should have hover info at position 0");
    
    if let Some(hover) = hover {
        let markdown = hover.to_markdown();
        assert!(markdown.contains("name"));
    }
}

#[test]
fn test_completion_engine_root() {
    let root = NodeDef::new("")
        .with_property("id", PropertyDef::new(ValueType::String).required())
        .with_property("name", PropertyDef::new(ValueType::String))
        .with_property("version", PropertyDef::new(ValueType::String));
    
    let schema = Schema::new("test", root);
    let engine = CompletionEngine::new(schema);
    
    // Get completions at root level
    let source = "";
    let doc = kdl::KdlDocument::new();
    let completions = engine.complete(&doc, 0, source);
    
    // Should have at least 3 property completions
    assert!(completions.len() >= 3, "Should have property completions");
    
    // Check that required property has higher priority
    let id_completion = completions.iter().find(|c| c.label == "id");
    let name_completion = completions.iter().find(|c| c.label == "name");
    
    assert!(id_completion.is_some());
    assert!(name_completion.is_some());
    
    if let (Some(id), Some(name)) = (id_completion, name_completion) {
        assert!(id.sort_priority < name.sort_priority, "Required property should have higher priority");
    }
}

#[test]
fn test_completion_with_enum() {
    let root = NodeDef::new("")
        .with_property(
            "environment",
            PropertyDef::new(ValueType::Enum("Environment".to_string())),
        );
    
    let mut schema = Schema::new("test", root);
    
    // Add enum definition
    schema.define_enum(
        "Environment",
        EnumDef::new(vec![
            "development".to_string(),
            "staging".to_string(),
            "production".to_string(),
        ])
        .with_value_desc("development", "Development environment")
        .with_value_desc("staging", "Staging environment")
        .with_value_desc("production", "Production environment"),
    );
    
    let engine = CompletionEngine::new(schema);
    
    // For now, we just test that it creates successfully
    // Full enum completion would require implementing context detection
    let source = "";
    let doc = kdl::KdlDocument::new();
    let completions = engine.complete(&doc, 0, source);
    
    // Should have the environment property
    assert!(completions.iter().any(|c| c.label == "environment"));
}

#[test]
fn test_nested_node_analysis() {
    let child_def = NodeDef::new("child")
        .with_description("A child node")
        .with_property("child_prop", PropertyDef::new(ValueType::String));
    
    let root = NodeDef::new("root")
        .with_description("Root node")
        .with_property("root_prop", PropertyDef::new(ValueType::String))
        .with_child(child_def);
    
    let schema = Schema::new("test", root);
    
    let source = r#"
root root_prop="value" {
    child child_prop="child_value"
}
    "#;
    
    let parser = Parser::new(source.to_string(), "test.dgl".to_string())
        .with_schema(schema.clone());
    
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse successfully");
    
    let doc = result.unwrap();
    let semantic_info = SemanticInfo::analyze(&doc.document, &schema, source);
    
    // Should have document symbols for the structure
    assert!(!semantic_info.document_symbols.is_empty());
    assert!(!semantic_info.hover_info.is_empty());
}

