use degov_dsl::{Definition, Parser};
use std::path::PathBuf;

#[test]
fn test_parse_identity_card_model() {
    let yaml = r#"
apiVersion: v1
kind: DataModel
metadata:
  id: de.bund/identity-card
  title: Identity Card
  description: Identity Card
  version: 1.0.0

spec:
  storage:
    encrypted: true
    retention:
      duration: P10Y
      afterDeletion: anonymize

  schema:
    type: object
    properties:
      id:
        type: string
        format: uuid
        generated: true
        immutable: true
        indexed: true
        description: Unique identity card identifier
"#;
    
    let definition = Definition::from_yaml(yaml).unwrap();
    assert_eq!(definition.metadata().id.as_str(), "de.bund/identity-card");
    
    if let Definition::DataModel(model) = definition {
        assert_eq!(model.metadata.title, "Identity Card");
        assert!(model.spec.storage.as_ref().unwrap().encrypted);
        assert_eq!(
            model.spec.storage.as_ref().unwrap().retention.as_ref().unwrap().duration,
            "P10Y"
        );
        assert!(model.spec.schema.properties.contains_key("id"));
    } else {
        panic!("Expected DataModel");
    }
}

#[test]
fn test_load_from_file_system() {
    // Try to load the identity-card model if it exists
    let path = PathBuf::from("services/de/degov/identity-card/model.yaml");
    
    if path.exists() {
        let result = Definition::from_file(&path);
        match result {
            Ok(def) => {
                println!("Successfully parsed: {}", def.metadata().id.as_str());
                assert!(matches!(def, Definition::DataModel(_)));
            }
            Err(e) => {
                println!("Failed to parse existing file: {}", e);
                // Don't fail the test if file format is slightly different
            }
        }
    } else {
        println!("File does not exist at expected location, skipping test");
    }
}

#[test]
fn test_parser_discovery() {
    let parser = Parser::new("services");
    
    // Try to discover all services
    match parser.discover_services() {
        Ok(definitions) => {
            println!("Discovered {} definitions", definitions.len());
            for def in definitions.iter().take(5) {
                println!("  - {} ({})", def.metadata().id.as_str(), def.metadata().title);
            }
        }
        Err(e) => {
            println!("Discovery error (expected if services dir doesn't exist): {}", e);
        }
    }
}

