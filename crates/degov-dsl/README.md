# degov-dsl

A Rust library for parsing and working with DeGov YAML DSL (Domain-Specific Language) definitions.

## Overview

The `degov-dsl` crate provides a type-safe parser for DeGov's YAML-based configuration language, which is used to define government services, data models, workflows, permissions, and credentials.

## Features

- **Type-safe parsing**: Strongly-typed Rust structures for all DSL constructs
- **Multiple definition types**: Support for Services, DataModels, Workflows, Permissions, and Credentials
- **NSID support**: AT Protocol Lexicon-style namespaced identifiers (e.g., `de.berlin/business`)
- **File system discovery**: Automatically discover and load definitions from directory structures
- **Comprehensive validation**: Parse-time validation of YAML structure
- **Graph-based inheritance resolution**: Uses petgraph for dependency tracking and cycle detection
- **Schema merging**: Automatic property inheritance with override support

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
degov-dsl = "0.1"
```

## Usage

### Parsing a YAML string

```rust
use degov_dsl::{Definition, DataModel};

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
"#;

let definition = Definition::from_yaml(yaml)?;
println!("Parsed: {}", definition.metadata().id);

if let Definition::DataModel(model) = definition {
    println!("Properties: {}", model.spec.schema.properties.len());
}
```

### Loading from files

```rust
use degov_dsl::{Definition, Parser};

// Load a single file
let definition = Definition::from_file("services/de/berlin/business/model.yaml")?;

// Or use the Parser for NSID-based loading
let parser = Parser::new("services");
let definitions = parser.load_by_nsid("de.berlin/business")?;

for def in definitions {
    println!("- {} ({})", def.metadata().id, def.metadata().title);
}
```

### Discovering all services

```rust
use degov_dsl::Parser;

let parser = Parser::new("services");
let all_definitions = parser.discover_services()?;

println!("Found {} definitions", all_definitions.len());
```

## DSL Types

The library supports parsing the following definition types:

- **Service**: Top-level service definitions that group models, workflows, and credentials
- **DataModel**: Data structure definitions with schemas, validations, and storage config
- **Workflow**: State machine definitions for multi-step processes
- **Permission**: Access control rules with role-based and attribute-based permissions
- **Credential**: Verifiable credential schemas for issuing certificates to citizens

## NSID Format

NSIDs (Namespaced Identifiers) follow the AT Protocol Lexicon format:

```
{authority}/{entity-name}[#{fragment}]
```

Examples:
- `de.bund/person` - Federal person model
- `de.berlin/business` - Berlin business model
- `de.berlin/business-registration#workflow` - Workflow definition
- `de.berlin/business-license#credential` - Credential definition

## Directory Structure

The parser expects files to be organized following reverse DNS notation:

```
services/
├── de/
│   ├── bund/                  # Federal level
│   │   └── person/
│   │       └── model.yaml
│   ├── berlin/                # Municipal level
│   │   └── business/
│   │       ├── model.yaml
│   │       ├── workflow.yaml
│   │       └── permissions.yaml
│   └── bayern/
│       └── ...
```

## Inheritance Resolution

The library supports model inheritance with automatic dependency resolution:

```rust
use degov_dsl::Parser;

let parser = Parser::new("services");

// Load a model and resolve all its parent dependencies
let resolved = parser.load_and_resolve_model("de.degov/natural-person")?;
println!("Properties: {}", resolved.spec.schema.properties.len());

// Discover and resolve all models in the workspace
let all_models = parser.discover_and_resolve_all()?;
```

For more details, see [GRAPH.md](./GRAPH.md).

## Examples

See the `examples/` directory for more usage examples:

```bash
# Basic parsing
cargo run --example parse_model

# Inheritance resolution
cargo run --example resolve_inheritance
```

## Testing

Run the test suite:

```bash
cargo test
```

## Documentation

- [DSL.md](../../DSL.md) - Complete DSL syntax specification
- [GRAPH.md](./GRAPH.md) - Inheritance resolution and graph operations

## License

See the main DeGov project for license information.


