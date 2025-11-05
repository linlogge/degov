//! Tests for dynamic schema modifiers

use dgv_dgl::prelude::*;

#[test]
fn test_basic_schema_modifier() {
    // Create a node definition with a schema modifier that adds properties
    // based on a "type" property value
    let node_def = NodeDef::new("item")
        .with_property("type", PropertyDef::new(ValueType::String).required())
        .with_schema_modifier(|def, node| {
            // Check the value of the "type" property
            let type_value = node
                .entries()
                .iter()
                .find(|e| e.name().map(|n| n.value()) == Some("type"))
                .and_then(|e| e.value().as_string());

            let mut modified_def = def.clone();

            match type_value {
                Some("book") => {
                    // Add book-specific properties
                    modified_def = modified_def
                        .with_property("author", PropertyDef::new(ValueType::String).required())
                        .with_property("isbn", PropertyDef::new(ValueType::String));
                }
                Some("movie") => {
                    // Add movie-specific properties
                    modified_def = modified_def
                        .with_property("director", PropertyDef::new(ValueType::String).required())
                        .with_property("runtime", PropertyDef::new(ValueType::Integer));
                }
                _ => {}
            }

            modified_def
        });

    let root = NodeDef::new("catalog").with_child(node_def);
    let schema = Schema::new("test", root);

    // Test with a book
    let source_book = r#"
catalog {
    item type="book" author="J.R.R. Tolkien" isbn="978-0547928227"
}
    "#;

    let parser = Parser::new(source_book.to_string(), "test.dgl".to_string()).with_schema(schema.clone());
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse book successfully");

    // Test with a movie
    let source_movie = r#"
catalog {
    item type="movie" director="Peter Jackson" runtime="178"
}
    "#;

    let parser = Parser::new(source_movie.to_string(), "test.dgl".to_string()).with_schema(schema.clone());
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse movie successfully");

    // Test missing required property for book
    let source_book_missing = r#"
catalog {
    item type="book" isbn="978-0547928227"
}
    "#;

    let parser =
        Parser::new(source_book_missing.to_string(), "test.dgl".to_string()).with_schema(schema.clone());
    let result = parser.parse();
    assert!(result.is_err(), "Should fail when book is missing required author");

    // Test missing required property for movie
    let source_movie_missing = r#"
catalog {
    item type="movie" runtime="178"
}
    "#;

    let parser = Parser::new(source_movie_missing.to_string(), "test.dgl".to_string())
        .with_schema(schema.clone());
    let result = parser.parse();
    assert!(result.is_err(), "Should fail when movie is missing required director");
}

#[test]
fn test_schema_modifier_with_child_nodes() {
    // Create a node definition that adds different child nodes based on a property
    let node_def = NodeDef::new("definition")
        .with_property("kind", PropertyDef::new(ValueType::String).required())
        .with_schema_modifier(|def, node| {
            // Check kind property in children block
            let kind_value = node
                .children()
                .and_then(|children| {
                    children.nodes().iter().find(|child| {
                        child.name().value() == "kind"
                            && child.entries().first().is_some()
                    })
                })
                .and_then(|kind_node| kind_node.entries().first())
                .and_then(|entry| entry.value().as_string())
                .or_else(|| {
                    // Also check direct property
                    node.entries()
                        .iter()
                        .find(|e| e.name().map(|n| n.value()) == Some("kind"))
                        .and_then(|e| e.value().as_string())
                });

            let mut modified_def = def.clone();

            match kind_value {
                Some("DataModel") => {
                    // Add DataModel-specific child nodes
                    modified_def = modified_def.with_child(
                        NodeDef::new("field")
                            .with_property("name", PropertyDef::new(ValueType::String).required())
                            .with_property("type", PropertyDef::new(ValueType::String).required()),
                    );
                }
                Some("Service") => {
                    // Add Service-specific child nodes
                    modified_def = modified_def.with_child(
                        NodeDef::new("endpoint")
                            .with_property("path", PropertyDef::new(ValueType::String).required())
                            .with_property("method", PropertyDef::new(ValueType::String).required()),
                    );
                }
                _ => {}
            }

            modified_def
        });

    let root = NodeDef::new("model").with_child(node_def);
    let schema = Schema::new("test", root);

    // Test with DataModel kind
    let source_datamodel = r#"
model {
    definition kind="DataModel" {
        field name="id" type="string"
        field name="name" type="string"
    }
}
    "#;

    let parser = Parser::new(source_datamodel.to_string(), "test.dgl".to_string())
        .with_schema(schema.clone());
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse DataModel successfully");

    // Test with Service kind
    let source_service = r#"
model {
    definition kind="Service" {
        endpoint path="/api/users" method="GET"
        endpoint path="/api/users" method="POST"
    }
}
    "#;

    let parser =
        Parser::new(source_service.to_string(), "test.dgl".to_string()).with_schema(schema.clone());
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse Service successfully");
}

#[test]
fn test_schema_modifier_removing_properties() {
    // Create a node definition that removes properties based on a condition
    let node_def = NodeDef::new("config")
        .with_property("mode", PropertyDef::new(ValueType::String).required())
        .with_property("debug", PropertyDef::new(ValueType::Boolean))
        .with_property("log_level", PropertyDef::new(ValueType::String))
        .with_schema_modifier(|def, node| {
            let mode_value = node
                .entries()
                .iter()
                .find(|e| e.name().map(|n| n.value()) == Some("mode"))
                .and_then(|e| e.value().as_string());

            let mut modified_def = def.clone();

            // In production mode, remove debug-related properties
            if mode_value == Some("production") {
                modified_def.properties.remove("debug");
                modified_def.properties.remove("log_level");
            }

            modified_def
        });

    let root = NodeDef::new("app").with_child(node_def);
    let schema = Schema::new("test", root);

    // Test development mode (debug properties allowed)
    let source_dev = r#"
app {
    config mode="development" debug=#true log_level="debug"
}
    "#;

    let parser = Parser::new(source_dev.to_string(), "test.dgl".to_string()).with_schema(schema.clone());
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse development config successfully");

    // Test production mode (no debug properties)
    let source_prod = r#"
app {
    config mode="production"
}
    "#;

    let parser = Parser::new(source_prod.to_string(), "test.dgl".to_string()).with_schema(schema.clone());
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse production config successfully");
}

#[test]
fn test_nested_schema_modifiers() {
    // Test that schema modifiers work in nested contexts
    let field_def = NodeDef::new("field")
        .with_property("type", PropertyDef::new(ValueType::String).required())
        .with_property("name", PropertyDef::new(ValueType::String).required())
        .with_schema_modifier(|def, node| {
            let type_value = node
                .entries()
                .iter()
                .find(|e| e.name().map(|n| n.value()) == Some("type"))
                .and_then(|e| e.value().as_string());

            let mut modified_def = def.clone();

            // Add type-specific validation properties
            match type_value {
                Some("string") => {
                    modified_def = modified_def
                        .with_property("max_length", PropertyDef::new(ValueType::Integer))
                        .with_property("pattern", PropertyDef::new(ValueType::String));
                }
                Some("integer") => {
                    modified_def = modified_def
                        .with_property("min", PropertyDef::new(ValueType::Integer))
                        .with_property("max", PropertyDef::new(ValueType::Integer));
                }
                _ => {}
            }

            modified_def
        });

    let model_def = NodeDef::new("model").with_child(field_def);

    let root = NodeDef::new("schema").with_child(model_def);
    let schema = Schema::new("test", root);

    let source = r#"
schema {
    model {
        field type="string" name="username" max_length="50" pattern="[a-z]+"
        field type="integer" name="age" min="0" max="150"
    }
}
    "#;

    let parser = Parser::new(source.to_string(), "test.dgl".to_string()).with_schema(schema);
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse nested modified schema successfully");
}

#[test]
fn test_schema_modifier_with_semantic_analysis() {
    // Test that semantic analysis works with dynamically modified schemas
    let node_def = NodeDef::new("item")
        .with_property("type", PropertyDef::new(ValueType::String).required())
        .with_schema_modifier(|def, node| {
            let type_value = node
                .entries()
                .iter()
                .find(|e| e.name().map(|n| n.value()) == Some("type"))
                .and_then(|e| e.value().as_string());

            let mut modified_def = def.clone();

            if type_value == Some("book") {
                modified_def = modified_def
                    .with_property("title", PropertyDef::new(ValueType::String).required().with_description("Book title"));
            }

            modified_def
        });

    let root = NodeDef::new("catalog").with_child(node_def);
    let schema = Schema::new("test", root);

    let source = r#"
catalog {
    item type="book" title="The Hobbit"
}
    "#;

    let parser = Parser::new(source.to_string(), "test.dgl".to_string()).with_schema(schema.clone());
    let result = parser.parse();
    assert!(result.is_ok(), "Should parse successfully");

    let doc = result.unwrap();
    let semantic_info = dgv_dgl::semantic::SemanticInfo::analyze(&doc.document, &schema, source);

    // Check that semantic analysis found symbols
    assert!(!semantic_info.document_symbols.is_empty(), "Should have document symbols");
    assert!(!semantic_info.hover_info.is_empty(), "Should have hover info");
}

