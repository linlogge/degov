//! Semantic Analysis for IDE Support
//!
//! Provides hover information, go-to-definition, find references, etc.

use crate::schema::{CompletionItem, CompletionKind, NodeDef, Schema};
use miette::SourceSpan;
use std::collections::HashMap;

/// Semantic information about a document
#[derive(Debug, Clone)]
pub struct SemanticInfo {
    /// Symbols defined in the document
    pub symbols: HashMap<String, Symbol>,
    
    /// References to symbols
    pub references: Vec<Reference>,
    
    /// Document symbols for outline view
    pub document_symbols: Vec<DocumentSymbol>,
    
    /// Hover information at various positions
    pub hover_info: Vec<HoverInfo>,
}

impl SemanticInfo {
    /// Analyze a KDL document and extract semantic information
    pub fn analyze(doc: &kdl::KdlDocument, schema: &Schema, source: &str) -> Self {
        let mut analyzer = SemanticAnalyzer::new(schema, source);
        analyzer.analyze_document(doc);
        analyzer.into_info()
    }
    
    /// Get hover information at a specific offset
    /// Returns the most specific (smallest span) hover info that contains the offset
    pub fn get_hover_at(&self, offset: usize) -> Option<&HoverInfo> {
        self.hover_info
            .iter()
            .filter(|info| {
                let start = info.span.offset();
                let end = start + info.span.len();
                offset >= start && offset < end
            })
            // Sort by span length (smallest first) to get most specific match
            .min_by_key(|info| info.span.len())
    }
    
    /// Find a symbol at a specific offset
    pub fn find_symbol_at(&self, offset: usize) -> Option<&Symbol> {
        self.symbols.values().find(|sym| {
            let start = sym.definition_span.offset();
            let end = start + sym.definition_span.len();
            offset >= start && offset < end
        })
    }
    
    /// Find a reference at a specific offset
    pub fn find_reference_at(&self, offset: usize) -> Option<(&Reference, &str)> {
        self.references.iter().find_map(|ref_| {
            let start = ref_.span.offset();
            let end = start + ref_.span.len();
            if offset >= start && offset < end {
                Some((ref_, ref_.target.as_str()))
            } else {
                None
            }
        })
    }
    
    /// Get all references to a symbol
    pub fn get_references_to(&self, symbol_name: &str) -> Vec<SourceSpan> {
        self.references
            .iter()
            .filter(|r| r.target == symbol_name)
            .map(|r| r.span)
            .collect()
    }
}

/// A symbol (e.g., a defined node with an ID)
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Name of the symbol
    pub name: String,
    
    /// Kind of symbol
    pub kind: SymbolKind,
    
    /// Where it's defined
    pub definition_span: SourceSpan,
    
    /// Documentation
    pub documentation: Option<String>,
    
    /// Type information
    pub type_info: Option<String>,
}

/// Kind of symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Definition,
    Property,
    Value,
    Reference,
}

impl SymbolKind {
    pub fn to_lsp_kind(&self) -> u32 {
        match self {
            SymbolKind::Definition => 5,  // Class
            SymbolKind::Property => 7,    // Property
            SymbolKind::Value => 13,      // Variable
            SymbolKind::Reference => 18,  // TypeParameter
        }
    }
}

/// A reference to a symbol
#[derive(Debug, Clone)]
pub struct Reference {
    /// Target symbol name
    pub target: String,
    
    /// Location of the reference
    pub span: SourceSpan,
}

/// A document symbol for outline view
#[derive(Debug, Clone)]
pub struct DocumentSymbol {
    /// Name of the symbol
    pub name: String,
    
    /// Kind of symbol
    pub kind: SymbolKind,
    
    /// Full span including children
    pub span: SourceSpan,
    
    /// Selection span (just the name)
    pub selection_span: SourceSpan,
    
    /// Child symbols
    pub children: Vec<DocumentSymbol>,
}

/// Hover information
#[derive(Debug, Clone)]
pub struct HoverInfo {
    /// Span this info applies to
    pub span: SourceSpan,
    
    /// Content to show
    pub content: HoverContent,
}

impl HoverInfo {
    pub fn to_markdown(&self) -> String {
        match &self.content {
            HoverContent::Text(text) => text.clone(),
            HoverContent::Documentation { title, description, type_info } => {
                let mut md = String::new();
                md.push_str("### ");
                md.push_str(title);
                md.push_str("\n\n");
                
                if let Some(ty) = type_info {
                    md.push_str("**Type:** `");
                    md.push_str(ty);
                    md.push_str("`\n\n");
                }
                
                if let Some(desc) = description {
                    md.push_str(desc);
                }
                
                md
            }
        }
    }
}

/// Content to show on hover
#[derive(Debug, Clone)]
pub enum HoverContent {
    Text(String),
    Documentation {
        title: String,
        description: Option<String>,
        type_info: Option<String>,
    },
}

/// Semantic analyzer
struct SemanticAnalyzer<'a> {
    schema: &'a Schema,
    _source: &'a str,
    symbols: HashMap<String, Symbol>,
    references: Vec<Reference>,
    document_symbols: Vec<DocumentSymbol>,
    hover_info: Vec<HoverInfo>,
}

impl<'a> SemanticAnalyzer<'a> {
    fn new(schema: &'a Schema, source: &'a str) -> Self {
        Self {
            schema,
            _source: source,
            symbols: HashMap::new(),
            references: Vec::new(),
            document_symbols: Vec::new(),
            hover_info: Vec::new(),
        }
    }
    
    fn analyze_document(&mut self, doc: &kdl::KdlDocument) {
        // Check if this is a root-property-container schema
        let root_is_property_container = self.schema.root.name.as_ref().map_or(true, |n| n.is_empty())
            && !self.schema.root.properties.is_empty();

        for node in doc.nodes() {
            if root_is_property_container {
                // Handle root-level properties
                self.analyze_root_property_or_node(node);
            } else {
                // Standard node analysis
                self.analyze_node(node, &self.schema.root, 0);
            }
        }
    }
    
    fn analyze_root_property_or_node(&mut self, node: &kdl::KdlNode) {
        let node_name = node.name().value();
        
        // Check if it's a property
        if let Some(prop_def) = self.schema.root.properties.get(node_name) {
            if let Some(entry) = node.entries().first() {
                if entry.name().is_none() && node.entries().len() == 1 {
                    // This is a root-level property
                    self.add_property_hover(node, prop_def);
                    self.add_document_symbol_for_property(node, prop_def);
                    return;
                }
            }
        }
        
        // Check if it's a child node
        let matching_child_def = self.schema.root.children.iter().find(|def| {
            if let Some(name) = &def.name {
                name == node_name
            } else {
                false
            }
        });
        
        if let Some(child_def) = matching_child_def {
            self.analyze_node(node, child_def, 0);
        }
    }
    
    fn analyze_node(&mut self, node: &kdl::KdlNode, node_def: &NodeDef, depth: usize) {
        let node_name = node.name().value();
        let node_span = node.span();
        
        // Apply schema modifier if present
        let effective_node_def = node_def.apply_modifier(node);
        
        // Add hover info for the node
        if let Some(description) = &effective_node_def.description {
            self.hover_info.push(HoverInfo {
                span: node_span,
                content: HoverContent::Documentation {
                    title: node_name.to_string(),
                    description: Some(description.clone()),
                    type_info: effective_node_def.name.clone(),
                },
            });
        }
        
        // Add document symbol
        let doc_symbol = DocumentSymbol {
            name: node_name.to_string(),
            kind: SymbolKind::Definition,
            span: node_span,
            selection_span: node_span, // TODO: Get precise name span
            children: Vec::new(),
        };
        
        // Analyze properties
        for entry in node.entries() {
            if let Some(prop_name) = entry.name() {
                let prop_name_str = prop_name.value();
                if let Some(prop_def) = effective_node_def.properties.get(prop_name_str) {
                    self.add_property_hover_from_entry(entry, prop_def);
                }
            }
        }
        
        // Analyze children
        if let Some(children) = node.children() {
            for child in children.nodes() {
                let child_name = child.name().value();
                
                // Check if child is a property in child-node format
                if let Some(prop_def) = effective_node_def.properties.get(child_name) {
                    if let Some(entry) = child.entries().first() {
                        if entry.name().is_none() && child.entries().len() == 1 {
                            self.add_property_hover(child, prop_def);
                            continue;
                        }
                    }
                }
                
                // Find matching child definition
                let matching_child_def = effective_node_def.children.iter().find(|def| {
                    if let Some(name) = &def.name {
                        name == child_name
                    } else {
                        false
                    }
                });
                
                if let Some(child_def) = matching_child_def {
                    self.analyze_node(child, child_def, depth + 1);
                }
            }
        }
        
        // Add the document symbol at the appropriate level
        if depth == 0 {
            self.document_symbols.push(doc_symbol);
        }
    }
    
    fn add_property_hover(&mut self, node: &kdl::KdlNode, prop_def: &crate::PropertyDef) {
        self.hover_info.push(HoverInfo {
            span: node.span(),
            content: HoverContent::Documentation {
                title: node.name().value().to_string(),
                description: prop_def.description.clone(),
                type_info: Some(prop_def.ty.name()),
            },
        });
    }
    
    fn add_property_hover_from_entry(&mut self, entry: &kdl::KdlEntry, prop_def: &crate::PropertyDef) {
        self.hover_info.push(HoverInfo {
            span: entry.span(),
            content: HoverContent::Documentation {
                title: entry.name().map(|n| n.value()).unwrap_or("").to_string(),
                description: prop_def.description.clone(),
                type_info: Some(prop_def.ty.name()),
            },
        });
    }
    
    fn add_document_symbol_for_property(&mut self, node: &kdl::KdlNode, _prop_def: &crate::PropertyDef) {
        self.document_symbols.push(DocumentSymbol {
            name: node.name().value().to_string(),
            kind: SymbolKind::Property,
            span: node.span(),
            selection_span: node.span(),
            children: Vec::new(),
        });
    }
    
    
    fn into_info(self) -> SemanticInfo {
        SemanticInfo {
            symbols: self.symbols,
            references: self.references,
            document_symbols: self.document_symbols,
            hover_info: self.hover_info,
        }
    }
}

/// Completion engine for providing IntelliSense
pub struct CompletionEngine {
    schema: Schema,
}

impl CompletionEngine {
    pub fn new(schema: Schema) -> Self {
        Self { schema }
    }
    
    /// Get completions at a specific position
    pub fn complete(
        &self,
        doc: &kdl::KdlDocument,
        offset: usize,
        source: &str,
    ) -> Vec<CompletionItem> {
        let context = self.determine_context(doc, offset, source);
        
        match context {
            CompletionContext::Root => {
                // Offer root-level completions
                self.get_root_completions()
            }
            CompletionContext::InNode(node_name) => {
                // Offer property and child completions for this node
                self.get_node_completions(&node_name)
            }
            CompletionContext::PropertyValue(node_name, prop_name) => {
                // Offer value completions for this property
                self.get_property_value_completions(&node_name, &prop_name)
            }
            CompletionContext::Unknown => {
                Vec::new()
            }
        }
    }
    
    fn get_root_completions(&self) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        // Check if root is property container
        let root_is_property_container = self.schema.root.name.as_ref().map_or(true, |n| n.is_empty())
            && !self.schema.root.properties.is_empty();
        
        if root_is_property_container {
            // Add root-level properties as completions
            for (prop_name, prop_def) in &self.schema.root.properties {
                completions.push(CompletionItem {
                    label: prop_name.clone(),
                    kind: CompletionKind::Property,
                    detail: Some(prop_def.ty.name()),
                    documentation: prop_def.description.clone(),
                    insert_text: Some(format!("{} \"${{1:value}}\"", prop_name)),
                    is_snippet: true,
                    sort_priority: if prop_def.required { 10 } else { 20 },
                });
            }
            
            // Add child nodes
            for child_def in &self.schema.root.children {
                if let Some(name) = &child_def.name {
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind: CompletionKind::Node,
                        detail: child_def.description.clone(),
                        documentation: child_def.description.clone(),
                        insert_text: Some(format!("{} {{\n    ${{1}}\n}}", name)),
                        is_snippet: true,
                        sort_priority: 15,
                    });
                }
            }
        } else {
            // Standard root node completion
            if let Some(name) = &self.schema.root.name {
                if !name.is_empty() {
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind: CompletionKind::Node,
                        detail: self.schema.root.description.clone(),
                        documentation: self.schema.root.description.clone(),
                        insert_text: Some(format!("{} {{\n    ${{1}}\n}}", name)),
                        is_snippet: true,
                        sort_priority: 10,
                    });
                }
            }
        }
        
        // Add any custom completions from the schema
        completions.extend(self.schema.root.completions.clone());
        
        completions
    }
    
    fn get_node_completions(&self, node_name: &str) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        // Find the node definition
        let node_def = if let Some(name) = &self.schema.root.name {
            if name == node_name {
                Some(&self.schema.root)
            } else {
                self.find_node_def_by_name(&self.schema.root, node_name)
            }
        } else {
            // Check root children
            self.find_node_def_by_name(&self.schema.root, node_name)
        };
        
        if let Some(def) = node_def {
            // Add property completions
            for (prop_name, prop_def) in &def.properties {
                completions.push(CompletionItem {
                    label: prop_name.clone(),
                    kind: CompletionKind::Property,
                    detail: Some(prop_def.ty.name()),
                    documentation: prop_def.description.clone(),
                    insert_text: Some(format!("{} \"${{1:value}}\"", prop_name)),
                    is_snippet: true,
                    sort_priority: if prop_def.required { 10 } else { 20 },
                });
            }
            
            // Add child node completions
            for child_def in &def.children {
                if let Some(name) = &child_def.name {
                    completions.push(CompletionItem {
                        label: name.clone(),
                        kind: CompletionKind::Node,
                        detail: child_def.description.clone(),
                        documentation: child_def.description.clone(),
                        insert_text: Some(format!("{} {{\n    ${{1}}\n}}", name)),
                        is_snippet: true,
                        sort_priority: 15,
                    });
                }
            }
            
            // Add custom completions
            completions.extend(def.completions.clone());
        }
        
        completions
    }
    
    fn get_property_value_completions(&self, node_name: &str, prop_name: &str) -> Vec<CompletionItem> {
        let mut completions = Vec::new();
        
        // Find the node and property definition
        let node_def = if let Some(name) = &self.schema.root.name {
            if name == node_name {
                Some(&self.schema.root)
            } else {
                self.find_node_def_by_name(&self.schema.root, node_name)
            }
        } else {
            self.find_node_def_by_name(&self.schema.root, node_name)
        };
        
        if let Some(def) = node_def {
            if let Some(prop_def) = def.properties.get(prop_name) {
                // If it's an enum, offer enum values
                if let crate::ValueType::Enum(enum_name) = &prop_def.ty {
                    if let Some(enum_def) = self.schema.enums.get(enum_name) {
                        for value in &enum_def.values {
                            let desc = enum_def.value_descriptions.get(value).cloned();
                            completions.push(CompletionItem {
                                label: value.clone(),
                                kind: CompletionKind::Enum,
                                detail: desc.clone(),
                                documentation: desc,
                                insert_text: Some(value.clone()),
                                is_snippet: false,
                                sort_priority: 10,
                            });
                        }
                    }
                }
                
                // Add suggestions from property definition
                for suggestion in &prop_def.suggestions {
                    completions.push(CompletionItem {
                        label: suggestion.clone(),
                        kind: CompletionKind::Value,
                        detail: None,
                        documentation: None,
                        insert_text: Some(suggestion.clone()),
                        is_snippet: false,
                        sort_priority: 20,
                    });
                }
            }
        }
        
        completions
    }
    
    fn find_node_def_by_name<'b>(&'b self, parent: &'b NodeDef, name: &str) -> Option<&'b NodeDef> {
        // Check direct children
        for child in &parent.children {
            if let Some(child_name) = &child.name {
                if child_name == name {
                    return Some(child);
                }
            }
            
            // Recursively search in children
            if let Some(found) = self.find_node_def_by_name(child, name) {
                return Some(found);
            }
        }
        
        None
    }
    
    #[allow(unused_variables)]
    fn determine_context(
        &self,
        doc: &kdl::KdlDocument,
        offset: usize,
        source: &str,
    ) -> CompletionContext {
        // TODO: Implement proper context detection using find_node_at_offset
        // For now, default to root context
        CompletionContext::Root
    }
    
    /// Find the node name at a specific offset
    #[allow(dead_code)]
    fn find_node_at_offset(
        &self,
        doc: &kdl::KdlDocument,
        offset: usize,
        source: &str,
    ) -> Option<String> {
        // Simple heuristic: count braces to determine nesting
        let text_before = &source[..offset.min(source.len())];
        
        // Find all node names before this position
        let mut open_braces = 0;
        let mut current_nodes: Vec<String> = Vec::new();
        
        for node in doc.nodes() {
            let node_span = node.span();
            let node_start = node_span.offset();
            
            if node_start < offset {
                current_nodes.push(node.name().value().to_string());
            }
        }
        
        // Parse the text to find what node we're in
        let mut in_node: Option<String> = None;
        let mut depth = 0;
        
        for line in text_before.lines() {
            // Count opening braces
            open_braces += line.matches('{').count();
            open_braces = open_braces.saturating_sub(line.matches('}').count());
            
            // If we find a node name followed by whitespace or {
            if let Some(pos) = line.find('{') {
                let before_brace = line[..pos].trim();
                if !before_brace.is_empty() {
                    let parts: Vec<&str> = before_brace.split_whitespace().collect();
                    if let Some(&node_name) = parts.first() {
                        if depth == 0 || (depth > 0 && open_braces > depth) {
                            in_node = Some(node_name.to_string());
                            depth = open_braces;
                        }
                    }
                }
            }
        }
        
        // If we're inside braces, return the node name
        if open_braces > 0 && in_node.is_some() {
            return in_node;
        }
        
        None
    }
}

/// Context for completions
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum CompletionContext {
    /// At the root level
    Root,
    
    /// Inside a specific node
    InNode(String),
    
    /// Filling in a property value
    PropertyValue(String, String),
    
    /// Unknown context
    Unknown,
}

