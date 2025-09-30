# Inheritance Resolution with Graph Representation

This document describes the graph-based inheritance resolution system for DeGov DSL models.

## Overview

The `DependencyGraph` structure uses the `petgraph` library to represent and resolve model inheritance relationships. It provides:

- **Dependency tracking**: Models and their inheritance relationships
- **Topological sorting**: Correct resolution order from base to derived models
- **Cycle detection**: Prevents circular inheritance
- **Schema merging**: Combines properties, indexes, and computed fields from parent models

## Usage

### Basic Example

```rust
use degov_dsl::{DependencyGraph, Parser};

// Create a parser
let parser = Parser::new("services");

// Load and resolve a single model with all dependencies
let resolved_model = parser.load_and_resolve_model("de.degov/natural-person")?;
println!("Properties: {}", resolved_model.spec.schema.properties.len());
```

### Manual Graph Construction

```rust
use degov_dsl::DependencyGraph;

let mut graph = DependencyGraph::new();

// Load models
graph.load_model_with_dependencies(&parser, "de.degov/citizen")?;

// Check for circular dependencies
graph.detect_cycles()?;

// Get resolution order
let order = graph.topological_sort()?;

// Resolve all models
let resolved = graph.resolve_all()?;
```

### Batch Resolution

```rust
// Resolve multiple models at once
let resolved = parser.load_and_resolve_models(&[
    "de.degov/natural-person",
    "de.bund/citizen",
])?;

// Or discover and resolve all models in workspace
let all_resolved = parser.discover_and_resolve_all()?;
```

## Architecture

### Graph Structure

The dependency graph uses a directed graph where:
- **Nodes**: Model NSIDs (e.g., `"de.degov/natural-person"`)
- **Edges**: Inheritance relationships (child → parent)

### Resolution Process

1. **Load models**: Add models to the graph with their inheritance declarations
2. **Detect cycles**: Ensure no circular inheritance exists
3. **Topological sort**: Determine resolution order (parents before children)
4. **Merge schemas**: For each model in order:
   - Start with parent properties
   - Apply child overrides
   - Merge required fields, indexes, and computed fields

### Inheritance Rules

1. **Property inheritance**: Child models inherit all parent properties
2. **Property override**: Child properties with the same name override parent properties
3. **Required fields**: Combined from all parents and the child
4. **Indexes**: Child indexes with the same name replace parent indexes
5. **Computed fields**: Child computed fields override parent computed fields
6. **Multiple inheritance**: Properties from all parents are merged (first parent wins on conflicts)

## API Reference

### DependencyGraph

```rust
// Creation
let mut graph = DependencyGraph::new();

// Adding models
graph.add_model(model)?;
graph.add_models(vec![model1, model2])?;
graph.load_model_with_dependencies(&parser, "nsid")?;

// Analysis
graph.detect_cycles()?;
let order = graph.topological_sort()?;
let deps = graph.get_dependencies("nsid");
let dependents = graph.get_dependents("nsid");

// Resolution
let resolved = graph.resolve_all()?;
```

### Parser Extensions

```rust
// Single model resolution
let model = parser.load_and_resolve_model("nsid")?;

// Multiple models
let models = parser.load_and_resolve_models(&["nsid1", "nsid2"])?;

// All workspace models
let all = parser.discover_and_resolve_all()?;
```

## Examples

### Simple Inheritance

```yaml
# Base model
apiVersion: v1
kind: DataModel
metadata:
  id: de.bund/person
spec:
  schema:
    type: object
    properties:
      name:
        type: string
      age:
        type: integer
```

```yaml
# Derived model
apiVersion: v1
kind: DataModel
metadata:
  id: de.bund/citizen
spec:
  inherits:
    - de.bund/person
  schema:
    type: object
    properties:
      citizenId:
        type: string
```

After resolution, `de.bund/citizen` will have: `name`, `age`, and `citizenId` properties.

### Multiple Inheritance

```yaml
apiVersion: v1
kind: DataModel
metadata:
  id: de.berlin/business-owner
spec:
  inherits:
    - de.bund/person
    - de.berlin/taxpayer
  schema:
    type: object
    properties:
      businessId:
        type: string
```

The resolved model inherits from both `person` and `taxpayer`.

### Diamond Inheritance

```
       Base
       /  \
    Left  Right
       \  /
     Derived
```

The graph correctly handles diamond inheritance patterns, ensuring each property is included only once.

## Error Handling

### Circular Dependencies

```rust
// Will return DslError::CircularDependency
// A inherits from B, B inherits from C, C inherits from A
graph.detect_cycles()?;
```

### Missing Parents

```rust
// Will return DslError::MissingField if parent not loaded
graph.resolve_all()?;
```

## Testing

Run the graph tests:

```bash
cargo test -p degov-dsl graph
```

Run the integration tests:

```bash
cargo test -p degov-dsl test_graph
```

Run the example:

```bash
cargo run -p degov-dsl --example resolve_inheritance
```

## Implementation Details

### Using petgraph

The implementation uses `petgraph::graph::DiGraph` for efficient graph operations:

- **Topological sort**: `petgraph::algo::toposort`
- **Cycle detection**: `petgraph::algo::is_cyclic_directed`
- **Graph traversal**: `graph.neighbors_directed`

### Performance

- **Time complexity**: O(V + E) for topological sort and cycle detection
- **Space complexity**: O(V) for node storage, O(E) for edge storage
- **Memory**: Models are cloned during resolution; consider using `Rc` or `Arc` for large graphs

## Future Enhancements

- [ ] Support for trait/interface-style inheritance
- [ ] Lazy resolution (resolve only when needed)
- [ ] Caching of resolved models
- [ ] Visualization of dependency graphs
- [ ] Support for conditional inheritance based on configuration

