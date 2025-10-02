# degov-lsp

Language Server Protocol (LSP) server for DeGov YAML DSL.

## Overview

This is a Language Server implementation that provides real-time validation and IntelliSense for DeGov DSL YAML files in editors that support LSP (primarily Visual Studio Code).

## Features

- **Real-time validation**: Validates YAML syntax and DSL structure as you type
- **Diagnostics**: Reports errors and warnings with precise locations
- **Inheritance validation**: Checks model inheritance and detects circular dependencies  
- **Auto-completion**: Suggests DSL keywords and structures
- **Hover information**: Shows documentation on hover

## Building

Build the release version:

```bash
cargo build -p degov-lsp --release
```

The binary will be at `../../target/release/degov-lsp`.

## Running

The LSP server communicates via stdin/stdout following the LSP protocol:

```bash
./target/release/degov-lsp
```

It's designed to be launched by an editor/IDE, not run directly by users.

## Architecture

The server uses:
- **tower-lsp**: LSP framework for Rust
- **tokio**: Async runtime
- **degov-dsl**: Core DSL parsing and validation

## Validation Features

### YAML Parsing

```yaml
apiVersion: v1
kind: DataModel
# ^ Validates YAML syntax
```

### Structure Validation

```yaml
metadata:
  id: de.example/person  # ✓ Valid NSID
  title: Person
  version: 1.0.0
```

### Inheritance Checking

```yaml
spec:
  inherits:
    - de.example/base-model  # Validates parent exists
  schema:
    # ...
```

### Circular Dependency Detection

```yaml
# Error: Circular dependency detected
# A inherits from B, B inherits from A
```

## LSP Capabilities

### Implemented

- ✓ `textDocument/didOpen` - Document opened
- ✓ `textDocument/didChange` - Document changed
- ✓ `textDocument/didSave` - Document saved
- ✓ `textDocument/hover` - Hover information
- ✓ `textDocument/completion` - Auto-completion
- ✓ `textDocument/diagnostic` - Diagnostics/validation

### Future

- ⏳ `textDocument/definition` - Go to definition
- ⏳ `textDocument/references` - Find references
- ⏳ `textDocument/rename` - Rename symbol
- ⏳ `textDocument/formatting` - Format document

## Development

### Watch mode

```bash
cargo watch -x 'build -p degov-lsp'
```

### Testing

```bash
cargo test -p degov-lsp
```

### Debugging

Set environment variables:

```bash
RUST_LOG=debug RUST_BACKTRACE=1 ./target/debug/degov-lsp
```

## Integration

This server is used by the VSCode extension at `../../vscode-extension/`.

## Protocol

The server implements [Language Server Protocol 3.17](https://microsoft.github.io/language-server-protocol/).

## License

See the main DeGov project for license information.

