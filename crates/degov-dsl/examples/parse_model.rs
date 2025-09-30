use degov_dsl::{Definition, Parser};

fn main() {
    // Example 1: Parse a YAML string directly
    println!("Example 1: Parsing YAML string");
    let yaml = r#"
apiVersion: degov.gov/v1
kind: DataModel
metadata:
  id: de.example/person
  title: Person Model
  version: 1.0.0
spec:
  schema:
    type: object
    properties:
      name:
        type: string
        required: true
      age:
        type: integer
        min: 0
"#;
    
    match Definition::from_yaml(yaml) {
        Ok(def) => {
            println!("✓ Successfully parsed: {} - {}", 
                def.metadata().id.as_str(), 
                def.metadata().title
            );
            
            if let Definition::DataModel(model) = def {
                println!("  Properties: {}", model.spec.schema.properties.len());
            }
        }
        Err(e) => eprintln!("✗ Parse error: {}", e),
    }
    
    // Example 2: Using the Parser to load from file system
    println!("\nExample 2: Loading by NSID");
    let parser = Parser::new("services");
    
    match parser.load_by_nsid_str("de.degov/identity-card") {
        Ok(definitions) => {
            println!("✓ Found {} definitions", definitions.len());
            for def in definitions {
                println!("  - {} ({})", def.metadata().id.as_str(), def.metadata().title);
            }
        }
        Err(e) => eprintln!("✗ Load error: {}", e),
    }
    
    // Example 3: Discover all services
    println!("\nExample 3: Discovering all services");
    match parser.discover_services() {
        Ok(definitions) => {
            println!("✓ Discovered {} definitions", definitions.len());
            
            // Group by kind
            let mut by_kind = std::collections::HashMap::new();
            for def in definitions {
                let kind = match def {
                    Definition::Service(_) => "Service",
                    Definition::DataModel(_) => "DataModel",
                    Definition::Workflow(_) => "Workflow",
                    Definition::Permission(_) => "Permission",
                    Definition::Credential(_) => "Credential",
                };
                *by_kind.entry(kind).or_insert(0) += 1;
            }
            
            for (kind, count) in by_kind {
                println!("  - {}: {}", kind, count);
            }
        }
        Err(e) => eprintln!("✗ Discovery error: {}", e),
    }
}

