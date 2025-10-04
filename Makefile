.PHONY: all build build-lsp build-extension install-extension clean test

all: build

# Build everything
build: build-lsp build-extension

# Build the LSP server
build-lsp:
	@echo "Building DeGov LSP server..."
	cargo build -p degov-lsp --release

build-lsp-debug:
	@echo "Building DeGov LSP server (debug)..."
	cargo build -p degov-lsp

# Build the VSCode extension
build-extension:
	@echo "Building VSCode extension..."
	cd tools/vscode-extension && npm install && npm run compile

# Package the extension
package-extension: build-lsp build-exftension
	@echo "Packaging VSCode extension..."
	cd tools/vscode-extension && npm exec vsce package --no-dependencies
	@echo "Extension packaged: tools/vscode-extension/degov-dsl-*.vsix"

# Install extension in development mode
install-extension: build-lsp build-extension
	@echo "Installing extension in VSCode..."
	cd tools/vscode-extension && code --install-extension degov-dsl-*.vsix || echo "Build extension first with: make package-extension"

# Run tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean
	cd tools/vscode-extension && rm -rf node_modules out *.vsix

# Development: Watch for changes
watch-lsp:
	cargo watch -x 'build -p degov-lsp'

watch-extension:
	cd tools/vscode-extension && npm run watch

# Help
help:
	@echo "DeGov DSL Build Commands:"
	@echo "  make build              - Build LSP server and extension"
	@echo "  make build-lsp          - Build LSP server (release)"
	@echo "  make build-lsp-debug    - Build LSP server (debug)"
	@echo "  make build-extension    - Build VSCode extension"
	@echo "  make package-extension  - Package extension as .vsix"
	@echo "  make install-extension  - Install extension in VSCode"
	@echo "  make test               - Run tests"
	@echo "  make clean              - Clean build artifacts"
	@echo "  make watch-lsp          - Watch and rebuild LSP server"
	@echo "  make watch-extension    - Watch and rebuild extension"

