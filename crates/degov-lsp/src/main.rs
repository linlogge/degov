use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use degov_dsl::Parser;
use miette::Diagnostic as _;
use ropey::Rope;

struct Backend {
    client: Client,
    document_map: DashMap<String, Rope>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            document_map: DashMap::new(),
        }
    }

    async fn on_change(&self, uri: Url, text: &str) {
        let rope = Rope::from_str(text);
        self.document_map.insert(uri.to_string(), rope);
    }

    async fn validate_document(&self, uri: &Url, text: &str) -> Vec<Diagnostic> {
        let rope = Rope::from_str(text);
        let parser = Parser::new(text.to_string(), uri.to_string());
        
        match parser.parse() {
            Ok(_definition) => {
                // Successfully parsed and validated
                self.client
                    .log_message(MessageType::INFO, "✓ Valid DSL document")
                    .await;
                Vec::new()
            }
            Err(dsl_err) => {
                // Convert all diagnostics to LSP diagnostics
                dsl_err
                    .diagnostics
                    .iter()
                    .map(|diag| {
                        Diagnostic::new(
                            Range::new(
                                char_to_position(diag.span.offset(), &rope),
                                char_to_position(diag.span.offset() + diag.span.len(), &rope),
                            ),
                            diag.severity().map(to_lsp_sev),
                            diag.code().map(|c| NumberOrString::String(c.to_string())),
                            Some("degov-dsl".to_string()),
                            diag.to_string(),
                            None,
                            None,
                        )
                    })
                    .collect()
            }
        }
    }
}

/// Convert a character offset to LSP Position using rope
fn char_to_position(char_idx: usize, rope: &Rope) -> Position {
    let line_idx = rope.char_to_line(char_idx);
    let line_char_idx = rope.line_to_char(line_idx);
    let column_idx = char_idx - line_char_idx;
    Position::new(line_idx as u32, column_idx as u32)
}

/// Convert miette severity to LSP diagnostic severity
fn to_lsp_sev(sev: miette::Severity) -> DiagnosticSeverity {
    match sev {
        miette::Severity::Advice => DiagnosticSeverity::HINT,
        miette::Severity::Warning => DiagnosticSeverity::WARNING,
        miette::Severity::Error => DiagnosticSeverity::ERROR,
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                            include_text: Some(true),
                        })),
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![" ".to_string()]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "DeGov DSL Language Server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "DeGov DSL Language Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        self.client
            .log_message(MessageType::INFO, "DeGov DSL Language Server shutting down")
            .await;
        Ok(())
    }
    
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.document_map.remove(&params.text_document.uri.to_string());
        self.client
            .log_message(MessageType::INFO, format!("Closed: {}", params.text_document.uri))
            .await;
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, format!("Opened: {}", params.text_document.uri))
            .await;

        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();
        
        self.on_change(uri.clone(), &text).await;
        let diagnostics = self.validate_document(&uri, &text).await;

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.first() {
            let uri = params.text_document.uri.clone();
            let text = change.text.clone();
            
            self.on_change(uri.clone(), &text).await;
            let diagnostics = self.validate_document(&uri, &text).await;

            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, format!("Saved: {}", params.text_document.uri))
            .await;

        if let Some(text) = params.text {
            let uri = params.text_document.uri.clone();
            
            self.on_change(uri.clone(), &text).await;
            let diagnostics = self.validate_document(&uri, &text).await;

            self.client
                .publish_diagnostics(uri, diagnostics, None)
                .await;
        }
    }

    async fn hover(&self, _params: HoverParams) -> Result<Option<Hover>> {
        Ok(Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String(
                "DeGov DSL - Hover to see documentation".to_string()
            )),
            range: None,
        }))
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        // Provide basic KDL completions for DeGov DSL
        let completions = vec![
            CompletionItem {
                label: "definition".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Start a definition block".to_string()),
                insert_text: Some("definition {\n    type \"DataModel\"\n    schema {\n        \n    }\n}".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "type".to_string(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some("Definition type".to_string()),
                insert_text: Some("type \"DataModel\"".to_string()),
                ..Default::default()
            },
            CompletionItem {
                label: "schema".to_string(),
                kind: Some(CompletionItemKind::FIELD),
                detail: Some("Schema block".to_string()),
                insert_text: Some("schema {\n    \n}".to_string()),
                ..Default::default()
            },
        ];

        Ok(Some(CompletionResponse::Array(completions)))
    }
}

#[tokio::main]
async fn main() {
    // Initialize tracing to write to stderr (stdout is used for LSP protocol)
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_max_level(tracing::Level::ERROR)
        .init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    
    Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}
