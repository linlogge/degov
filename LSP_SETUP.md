# DeGov DSL Language Server Setup Guide

This guide explains how to set up and use the DeGov DSL Language Server with Visual Studio Code.

## Overview

The DeGov DSL Language Server provides real-time validation and IntelliSense for DeGov YAML files:

- **Real-time validation** as you type
- **Error diagnostics** with precise locations
- **Inheritance validation** and circular dependency detection
- **Auto-completion** for DSL keywords
- **Hover information** for documentation

## Architecture

```
┌─────────────────────┐
│  VSCode Extension   │  (TypeScript)
│  - Language Client  │
│  - UI Integration   │
└──────────┬──────────┘
           │ LSP Protocol (JSON-RPC over stdio)
           │
┌──────────▼──────────┐
│   degov-lsp Server  │  (Rust)
│  - YAML Parsing     │
│  - DSL Validation   │
│  - Graph Resolution │
└──────────┬──────────┘
           │
┌──────────▼──────────┐
│    degov-dsl Crate  │  (Rust)
│  - Parser           │
│  - DependencyGraph  │
│  - Validation       │
└─────────────────────┘
```

## Quick Start

### 1. Build the Language Server

```bash
# From project root
cd /Users/noelsigmunczyk/Projects/degov

# Build release version (recommended)
cargo build -p degov-lsp --release

# Or build debug version for development
cargo build -p degov-lsp
```

The binary will be at:
- Release: `target/release/degov-lsp`
- Debug: `target/debug/degov-lsp`

### 2. Set Up the VSCode Extension

```bash
cd vscode-extension
npm install
npm run compile
```

### 3. Launch the Extension

**Option A: Development Mode (F5)**

1. Open `vscode-extension/` in VSCode
2. Press **F5** to launch Extension Development Host
3. A new VSCode window opens with the extension loaded

**Option B: Package and Install**

```bash
# From project root
make package-extension

# Install the .vsix file
code --install-extension vscode-extension/degov-dsl-*.vsix
```

## Usage

### Activating the Extension

The extension activates automatically when you:
- Open a YAML file in the `services/` directory
- Open files matching: `**/model.yaml`, `**/service.yaml`, etc.

### Features

#### 1. YAML Syntax Validation

```yaml
apiVersion: v1
kind: DataModel
metadata
  id: de.example/person  # ✗ Missing colon - shows error
```

#### 2. DSL Structure Validation

```yaml
apiVersion: v1
kind: DataModel
metadata:
  # ✗ Missing required 'id' field - shows error
  title: Person
  version: 1.0.0
```

#### 3. Inheritance Validation

```yaml
spec:
  inherits:
    - de.example/nonexistent  # ✗ Parent not found - shows error
```

#### 4. Circular Dependency Detection

```yaml
# model-a.yaml
spec:
  inherits:
    - de.example/model-b

# model-b.yaml
spec:
  inherits:
    - de.example/model-a  # ✗ Circular dependency - shows error
```

#### 5. Auto-completion

Type in a YAML file and get suggestions:
- `apiVersion` → Suggests `v1` or `degov.gov/v1`
- `kind` → Suggests `DataModel`, `Service`, etc.
- `inherits` → Suggests parent model NSIDs

#### 6. Hover Information

Hover over DSL keywords to see documentation.

### Commands

Access via Command Palette (Ctrl/Cmd+Shift+P):

- **DeGov DSL: Restart Language Server** - Restart if something goes wrong
- **DeGov DSL: Show Output Channel** - View extension logs

### Configuration

Settings available in VSCode settings (File → Preferences → Settings):

```json
{
  "degovDsl.trace.server": "off",  // or "messages", "verbose"
  "degovDsl.servicesPath": "services"
}
```

## Development Workflow

### Using Make Commands

```bash
# Build everything
make build

# Build LSP server only
make build-lsp

# Build extension only  
make build-extension

# Package extension
make package-extension

# Clean all build artifacts
make clean
```

### Manual Development

**Watch LSP Server Changes:**
```bash
cargo watch -x 'build -p degov-lsp'
```

**Watch Extension Changes:**
```bash
cd vscode-extension
npm run watch
```

### Testing

```bash
# Test the DSL crate
cargo test -p degov-dsl

# Test the LSP server
cargo test -p degov-lsp

# Test the extension
cd vscode-extension
npm test
```

## Troubleshooting

### Extension Not Loading

**Symptom:** Extension doesn't activate

**Solutions:**
1. Check that you're in a workspace with a `services/` directory
2. Open a YAML file that matches the activation patterns
3. Check VSCode's Output channel: View → Output → "DeGov DSL"

### Server Not Found Error

**Symptom:** "DeGov DSL Language Server not found"

**Solutions:**
1. Build the server: `cargo build -p degov-lsp --release`
2. Check the binary exists: `ls target/release/degov-lsp`
3. Restart VSCode

### No Validation Errors Shown

**Symptom:** Errors exist but no red squiggles appear

**Solutions:**
1. Check file is valid YAML (VSCode should show YAML errors)
2. Check file path matches activation pattern
3. Restart language server: Command Palette → "DeGov DSL: Restart Language Server"
4. Check Output channel for server errors

### Server Crashes

**Symptom:** Extension stops working after a while

**Solutions:**
1. Build debug version: `cargo build -p degov-lsp`
2. Enable verbose logging: Set `RUST_LOG=debug` in extension.ts
3. Check Output channel for panic messages
4. Report the issue with logs

### Performance Issues

**Symptom:** Editor is slow, high CPU usage

**Solutions:**
1. Check if validation is running on large files
2. Use release build (it's much faster)
3. Profile the server: `cargo flamegraph -p degov-lsp`

## File Locations

- **LSP Server Source:** `crates/degov-lsp/src/main.rs`
- **LSP Server Binary:** `target/release/degov-lsp`
- **Extension Source:** `vscode-extension/src/extension.ts`
- **Extension Build:** `vscode-extension/out/extension.js`
- **Extension Package:** `vscode-extension/degov-dsl-*.vsix`

## Debugging

### Debug Extension Client

1. Open `vscode-extension/` in VSCode
2. Set breakpoints in `src/extension.ts`
3. Press **F5**
4. Breakpoints will hit in the main window

### Debug LSP Server

1. Build debug version:
   ```bash
   cargo build -p degov-lsp
   ```

2. Add logging to server code:
   ```rust
   tracing::info!("Validating document: {}", uri);
   ```

3. View logs in Output channel

4. For detailed LSP communication:
   ```json
   {
     "degovDsl.trace.server": "verbose"
   }
   ```

## Next Steps

- [ ] Add more validation rules
- [ ] Implement code actions (quick fixes)
- [ ] Add semantic tokens for syntax highlighting
- [ ] Implement go-to-definition
- [ ] Add find-references
- [ ] Support workspace-wide diagnostics
- [ ] Add code formatting
- [ ] Implement rename refactoring

## Resources

- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/)
- [tower-lsp Documentation](https://docs.rs/tower-lsp/)
- [VSCode Extension API](https://code.visualstudio.com/api)
- [DeGov DSL Documentation](./crates/degov-dsl/README.md)

## Support

For issues or questions:
1. Check the Output channel for error messages
2. Review this troubleshooting guide
3. Check existing issues in the repository
4. Open a new issue with logs and reproduction steps

## License

See the main DeGov project for license information.

