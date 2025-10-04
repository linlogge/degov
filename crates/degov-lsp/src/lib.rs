use dashmap::DashMap;
use degov_dgl::v1::create_schema;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use degov_dgl::{Parser, Schema, SemanticInfo, CompletionEngine};
use miette::Diagnostic as _;
use ropey::Rope;

struct Backend {
    client: Client,
    document_map: DashMap<String, DocumentData>,
    schema: Schema,
    completion_engine: CompletionEngine,
}

/// Data associated with a document
struct DocumentData {
    /// The rope for efficient text manipulation
    rope: Rope,
    /// Semantic information
    semantic_info: Option<SemanticInfo>,
}

impl Backend {
    fn new(client: Client) -> Self {
        let schema = create_schema();
        let completion_engine = CompletionEngine::new(schema.clone());
        
        Self {
            client,
            document_map: DashMap::new(),
            schema,
            completion_engine,
        }
    }

    async fn on_change(&self, uri: Url, text: &str) {
        let rope = Rope::from_str(text);
        
        // Parse and analyze the document
        let semantic_info = if let Ok(doc) = text.parse::<kdl::KdlDocument>() {
            Some(SemanticInfo::analyze(&doc, &self.schema, text))
        } else {
            None
        };
        
        self.document_map.insert(
            uri.to_string(),
            DocumentData {
                rope,
                semantic_info,
            },
        );
    }

    async fn validate_document(&self, uri: &Url, text: &str) -> Vec<Diagnostic> {
        let rope = Rope::from_str(text);
        let parser = Parser::new(text.to_string(), uri.to_string())
            .with_schema(self.schema.clone());
        
        match parser.parse() {
            Ok(parsed) => {
                // Successfully parsed and validated
                self.client
                    .log_message(MessageType::INFO, "✓ Valid DGL document")
                    .await;
                
                // Convert any warnings to diagnostics
                parsed
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
                            Some("degov-dgl".to_string()),
                            diag.to_string(),
                            None,
                            None,
                        )
                    })
                    .collect()
            }
            Err(dgl_err) => {
                // Convert all diagnostics to LSP diagnostics
                dgl_err
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
                            Some("degov-dgl".to_string()),
                            diag.to_string(),
                            None,
                            None,
                        )
                    })
                    .collect()
            }
        }
    }

    /// Convert LSP position to character offset
    fn position_to_offset(&self, uri: &Url, position: Position) -> Option<usize> {
        let doc_data = self.document_map.get(&uri.to_string())?;
        let rope = &doc_data.rope;
        
        let line_idx = position.line as usize;
        let column_idx = position.character as usize;
        
        if line_idx >= rope.len_lines() {
            return None;
        }
        
        let line_start = rope.line_to_char(line_idx);
        Some(line_start + column_idx)
    }
}

// Helper trait to convert u32 to CompletionItemKind
trait FromU32 {
    fn from(value: u32) -> Self;
}

impl FromU32 for CompletionItemKind {
    fn from(value: u32) -> Self {
        match value {
            1 => CompletionItemKind::TEXT,
            7 => CompletionItemKind::CLASS,
            10 => CompletionItemKind::PROPERTY,
            12 => CompletionItemKind::VALUE,
            14 => CompletionItemKind::KEYWORD,
            15 => CompletionItemKind::SNIPPET,
            18 => CompletionItemKind::REFERENCE,
            _ => CompletionItemKind::TEXT,
        }
    }
}

impl FromU32 for InsertTextFormat {
    fn from(value: u32) -> Self {
        match value {
            1 => InsertTextFormat::PLAIN_TEXT,
            2 => InsertTextFormat::SNIPPET,
            _ => InsertTextFormat::PLAIN_TEXT,
        }
    }
}

impl FromU32 for SymbolKind {
    fn from(value: u32) -> Self {
        match value {
            3 => SymbolKind::NAMESPACE,
            5 => SymbolKind::CLASS,
            7 => SymbolKind::PROPERTY,
            8 => SymbolKind::FIELD,
            13 => SymbolKind::VARIABLE,
            15 => SymbolKind::STRING,
            _ => SymbolKind::STRING,
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
                    trigger_characters: Some(vec![" ".to_string(), "{".to_string(), "\n".to_string()]),
                    resolve_provider: Some(false),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "DeGov DGL Language Server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "DeGov DGL Language Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        self.client
            .log_message(MessageType::INFO, "DeGov DGL Language Server shutting down")
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

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        
        // Get the document data
        let doc_data = match self.document_map.get(&uri.to_string()) {
            Some(data) => data,
            None => return Ok(None),
        };
        
        // Get semantic info
        let semantic_info = match &doc_data.semantic_info {
            Some(info) => info,
            None => return Ok(None),
        };
        
        // Convert position to offset
        let offset = match self.position_to_offset(&uri, position) {
            Some(offset) => offset,
            None => return Ok(None),
        };
        
        // Find hover info at this position
        if let Some(hover_info) = semantic_info.get_hover_at(offset) {
            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: hover_info.to_markdown(),
                }),
                range: None,
            }));
        }
        
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        
        // Get the document
        let doc_data = match self.document_map.get(&uri.to_string()) {
            Some(data) => data,
            None => return Ok(None),
        };
        
        // Get the text
        let text = doc_data.rope.to_string();
        
        // Parse the document
        let doc = match text.parse::<kdl::KdlDocument>() {
            Ok(doc) => doc,
            Err(_) => return Ok(None),
        };
        
        // Convert position to offset
        let offset = match self.position_to_offset(&uri, position) {
            Some(offset) => offset,
            None => return Ok(None),
        };
        
        // Get completions
        let completions = self.completion_engine.complete(&doc, offset, &text);
        let lsp_completions: Vec<_> = completions.iter().map(|c| {
            CompletionItem {
                label: c.label.clone(),
                kind: Some(FromU32::from(c.kind.to_lsp_kind())),
                detail: c.detail.clone(),
                documentation: c.documentation.as_ref().map(|d| {
                    Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: d.clone(),
                    })
                }),
                deprecated: None,
                sort_text: Some(c.sort_priority.to_string()),
                insert_text: c.insert_text.clone(),
                insert_text_format: Some(if c.is_snippet {
                    InsertTextFormat::SNIPPET
                } else {
                    InsertTextFormat::PLAIN_TEXT
                }),
                ..Default::default()
            }
        }).collect();
        
        Ok(Some(CompletionResponse::Array(lsp_completions)))
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        
        // Get the document data
        let doc_data = match self.document_map.get(&uri.to_string()) {
            Some(data) => data,
            None => return Ok(None),
        };
        
        let semantic_info = match &doc_data.semantic_info {
            Some(info) => info,
            None => return Ok(None),
        };
        
        // Convert position to offset
        let offset = match self.position_to_offset(&uri, position) {
            Some(offset) => offset,
            None => return Ok(None),
        };
        
        // Find reference at this position
        if let Some((_, symbol_name)) = semantic_info.find_reference_at(offset) {
            // Find the symbol definition
            if let Some(symbol) = semantic_info.symbols.get(symbol_name) {
                let range = Range::new(
                    char_to_position(symbol.definition_span.offset(), &doc_data.rope),
                    char_to_position(
                        symbol.definition_span.offset() + symbol.definition_span.len(),
                        &doc_data.rope,
                    ),
                );
                
                return Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
                    uri.clone(),
                    range,
                ))));
            }
        }
        
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        
        // Get the document data
        let doc_data = match self.document_map.get(&uri.to_string()) {
            Some(data) => data,
            None => return Ok(None),
        };
        
        let semantic_info = match &doc_data.semantic_info {
            Some(info) => info,
            None => return Ok(None),
        };
        
        // Convert position to offset
        let offset = match self.position_to_offset(&uri, position) {
            Some(offset) => offset,
            None => return Ok(None),
        };
        
        // Find symbol at this position
        if let Some(symbol) = semantic_info.find_symbol_at(offset) {
            // Get all references to this symbol
            let references = semantic_info.get_references_to(&symbol.name);
            
            let locations: Vec<_> = references
                .iter()
                .map(|span| {
                    Location::new(
                        uri.clone(),
                        Range::new(
                            char_to_position(span.offset(), &doc_data.rope),
                            char_to_position(span.offset() + span.len(), &doc_data.rope),
                        ),
                    )
                })
                .collect();
            
            return Ok(Some(locations));
        }
        
        Ok(None)
    }

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        
        // Get the document data
        let doc_data = match self.document_map.get(&uri.to_string()) {
            Some(data) => data,
            None => return Ok(None),
        };
        
        let semantic_info = match &doc_data.semantic_info {
            Some(info) => info,
            None => return Ok(None),
        };
        
        // Convert document symbols to LSP format
        let symbols: Vec<_> = semantic_info
            .document_symbols
            .iter()
            .map(|sym| {
                #[allow(deprecated)]
                DocumentSymbol {
                    name: sym.name.clone(),
                    detail: None,
                    kind: <SymbolKind as FromU32>::from(sym.kind.to_lsp_kind() as u32),
                    tags: None,
                    deprecated: None,
                    range: Range::new(
                        char_to_position(sym.span.offset(), &doc_data.rope),
                        char_to_position(sym.span.offset() + sym.span.len(), &doc_data.rope),
                    ),
                    selection_range: Range::new(
                        char_to_position(sym.selection_span.offset(), &doc_data.rope),
                        char_to_position(
                            sym.selection_span.offset() + sym.selection_span.len(),
                            &doc_data.rope,
                        ),
                    ),
                    children: None,
                }
            })
            .collect();
        
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }
}

pub async fn start_server() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend::new(client));
    
    Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}