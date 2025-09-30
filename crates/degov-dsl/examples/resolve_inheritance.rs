use degov_dsl::{DependencyGraph, Parser, Result};

fn main() -> Result<()> {
    println!("=== DeGov DSL Inheritance Resolution Example ===\n");
    
    // Create a parser
    let parser = Parser::new("services");
    
    // Example 1: Load and resolve a single model with its dependencies
    println!("Example 1: Load and resolve a single model");
    println!("-------------------------------------------");
    match parser.load_and_resolve_model("de.degov/natural-person") {
        Ok(model) => {
            println!("✓ Loaded model: {}", model.metadata.id);
            println!("  Title: {}", model.metadata.title);
            println!("  Properties: {}", model.spec.schema.properties.len());
            println!("  Indexes: {}", model.spec.indexes.len());
            println!("  Computed fields: {}", model.spec.computed.len());
        }
        Err(e) => {
            println!("✗ Failed to load model: {}", e);
        }
    }
    
    println!("\n");
    
    // Example 2: Load multiple models and resolve all inheritance
    println!("Example 2: Load multiple models");
    println!("--------------------------------");
    let models_to_load = vec!["de.degov/natural-person"];
    match parser.load_and_resolve_models(&models_to_load) {
        Ok(resolved) => {
            println!("✓ Resolved {} models:", resolved.len());
            for (nsid, model) in &resolved {
                println!("  - {} ({} properties)", 
                    nsid, model.spec.schema.properties.len());
            }
        }
        Err(e) => {
            println!("✗ Failed to load models: {}", e);
        }
    }
    
    println!("\n");
    
    // Example 3: Manually build a dependency graph
    println!("Example 3: Manual dependency graph");
    println!("-----------------------------------");
    let mut graph = DependencyGraph::new();
    
    // Load a model manually
    match parser.load_by_nsid_str("de.degov/natural-person") {
        Ok(definitions) => {
            for def in definitions {
                if let degov_dsl::Definition::DataModel(model) = def {
                    println!("✓ Adding model to graph: {}", model.metadata.id);
                    if !model.spec.inherits.is_empty() {
                        println!("  Inherits from: {:?}", model.spec.inherits);
                    }
                    graph.add_model(model)?;
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to load: {}", e);
        }
    }
    
    // Check for cycles
    println!("\n✓ Checking for circular dependencies...");
    match graph.detect_cycles() {
        Ok(_) => println!("  No circular dependencies found"),
        Err(e) => println!("  ✗ Circular dependency detected: {}", e),
    }
    
    // Get topological order
    println!("\n✓ Computing topological order...");
    match graph.topological_sort() {
        Ok(order) => {
            println!("  Resolution order (base to derived):");
            for (i, nsid) in order.iter().rev().enumerate() {
                println!("    {}. {}", i + 1, nsid);
            }
        }
        Err(e) => println!("  ✗ Failed to sort: {}", e),
    }
    
    // Resolve all models
    println!("\n✓ Resolving inheritance...");
    match graph.resolve_all() {
        Ok(resolved) => {
            println!("  Successfully resolved {} models", resolved.len());
            for (nsid, model) in &resolved {
                println!("  - {}", nsid);
                println!("    Properties: {}", model.spec.schema.properties.len());
                println!("    Required: {}", model.spec.schema.required.len());
                println!("    Indexes: {}", model.spec.indexes.len());
                println!("    Computed: {}", model.spec.computed.len());
                println!("    Inherits (after resolution): {:?}", model.spec.inherits);
            }
        }
        Err(e) => println!("  ✗ Failed to resolve: {}", e),
    }
    
    println!("\n");
    
    // Example 4: Discover and resolve all models in the workspace
    println!("Example 4: Discover all models");
    println!("-------------------------------");
    match parser.discover_and_resolve_all() {
        Ok(resolved) => {
            println!("✓ Discovered and resolved {} models:", resolved.len());
            for (nsid, model) in &resolved {
                println!("  - {} (v{})", nsid, model.metadata.version);
            }
        }
        Err(e) => {
            println!("✗ Failed to discover: {}", e);
        }
    }
    
    Ok(())
}

