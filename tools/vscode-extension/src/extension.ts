import * as path from 'path';
import * as vscode from 'vscode';
import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
	// Find the LSP server binary
	const serverCommand = getServerPath(context);
	
	if (!serverCommand) {
		vscode.window.showErrorMessage(
			'DeGov LSP server not found. Please run: cargo build -p degov-lsp --release'
		);
		return;
	}

	// Configure server options
	const serverOptions: ServerOptions = {
		command: serverCommand,
		args: [],
		transport: TransportKind.stdio
	};

	// Configure client options
	const clientOptions: LanguageClientOptions = {
		// Register the server for DeGov DGL files
		documentSelector: [
			{ scheme: 'file', language: 'dgl' }
		],
		synchronize: {
			// Notify the server about file changes to '.dgl' files in the workspace
			fileEvents: vscode.workspace.createFileSystemWatcher('**/*.dgl')
		}
	};

	// Create the language client and start it
	client = new LanguageClient(
		'degovDgl',
		'DeGov DGL Language Server',
		serverOptions,
		clientOptions
	);

	// Start the client (which also launches the server)
	client.start();
	
	vscode.window.showInformationMessage('DeGov DGL extension activated');
}

export function deactivate(): Thenable<void> | undefined {
	if (!client) {
		return undefined;
	}
	return client.stop();
}

function getServerPath(context: vscode.ExtensionContext): string | undefined {
	// Try multiple possible locations for the LSP server
	const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
	
	if (!workspaceRoot) {
		return undefined;
	}

	// Try release build first, then debug build
	const possiblePaths = [
		path.join(workspaceRoot, 'target', 'release', 'degov-lsp'),
		path.join(workspaceRoot, 'target', 'debug', 'degov-lsp'),
		// Windows executables
		path.join(workspaceRoot, 'target', 'release', 'degov-lsp.exe'),
		path.join(workspaceRoot, 'target', 'debug', 'degov-lsp.exe'),
	];

	const fs = require('fs');
	for (const serverPath of possiblePaths) {
		if (fs.existsSync(serverPath)) {
			return serverPath;
		}
	}

	return undefined;
}
