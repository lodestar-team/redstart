//! Language server for Redstart.
//!
//! A `tower-lsp` server exposed via `redstart lsp` (stdio). Because subgraph
//! files are small, it reanalyses whole files on every change rather than
//! reaching for incremental machinery. Provides:
//!
//! - **diagnostics** — lex/parse errors per file plus project-aware semantic
//!   errors from the checker, reflecting unsaved edits via the loader overlay;
//! - **formatting** — the canonical `fmt`;
//! - **document symbols** — an outline of declarations;
//! - **hover**, **go-to-definition**, **completion** — for entities, sources,
//!   ABIs, types, and keywords.

#![forbid(unsafe_code)]

use redstart_parser::ast::Program;
use redstart_parser::{lex, parse};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

/// Start the language server on stdio, blocking until the client disconnects.
pub fn run() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    rt.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(Backend::new);
        Server::new(stdin, stdout, socket).serve(service).await;
    });
}

struct Backend {
    client: Client,
    /// Open documents: uri -> current text.
    docs: Mutex<HashMap<Url, String>>,
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            docs: Mutex::new(HashMap::new()),
        }
    }

    fn snapshot(&self) -> HashMap<Url, String> {
        self.docs.lock().unwrap().clone()
    }

    /// Recompute and publish diagnostics for every open document.
    async fn refresh_all(&self) {
        let docs = self.snapshot();
        // Start each open doc with an empty list so fixed errors are cleared.
        let mut out: HashMap<Url, Vec<Diagnostic>> =
            docs.keys().map(|u| (u.clone(), Vec::new())).collect();

        // 1. Per-file lex/parse diagnostics (immediate, even on broken input).
        for (uri, text) in &docs {
            for (off, len, msg) in parse_lex_diags(text) {
                out.entry(uri.clone())
                    .or_default()
                    .push(diagnostic(text, off, len, msg));
            }
        }

        // 2. Project-aware semantic diagnostics from the checker.
        if let Some(root) = find_root(&docs) {
            let overlay = overlay_map(&docs);
            if let Ok(tree) = redstart_loader::load_with_overlay(&root, overlay) {
                for d in redstart_checker::check_diags(&tree) {
                    if let Ok(uri) = Url::from_file_path(&d.file) {
                        if let Some(text) = text_for(&uri, &docs) {
                            let mut msg = d.message.clone();
                            if let Some(h) = d.help_str() {
                                msg.push_str(&format!(" — {h}"));
                            }
                            out.entry(uri)
                                .or_default()
                                .push(diagnostic(&text, d.offset, d.len, msg));
                        }
                    }
                }
            }
        }

        for (uri, diags) in out {
            self.client.publish_diagnostics(uri, diags, None).await;
        }
    }

    fn set_doc(&self, uri: Url, text: String) {
        self.docs.lock().unwrap().insert(uri, text);
    }

    fn get_doc(&self, uri: &Url) -> Option<String> {
        self.docs.lock().unwrap().get(uri).cloned()
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "redstart-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                completion_provider: Some(CompletionOptions::default()),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "redstart-lsp ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.set_doc(params.text_document.uri, params.text_document.text);
        self.refresh_all().await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.set_doc(params.text_document.uri, change.text);
        }
        self.refresh_all().await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.docs.lock().unwrap().remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, Vec::new(), None)
            .await;
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(text) = self.get_doc(&uri) else {
            return Ok(None);
        };
        let formatted = redstart_parser::fmt::format(&text);
        if formatted == text {
            return Ok(Some(Vec::new()));
        }
        Ok(Some(vec![TextEdit {
            range: Range::new(Position::new(0, 0), to_position(&text, text.len())),
            new_text: formatted,
        }]))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let Some(text) = self.get_doc(&params.text_document.uri) else {
            return Ok(None);
        };
        let program = parse_doc(&text);
        Ok(Some(DocumentSymbolResponse::Nested(symbols(
            &program, &text,
        ))))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(text) = self.get_doc(&uri) else {
            return Ok(None);
        };
        let offset = to_offset(&text, pos);
        let Some(word) = word_at(&text, offset) else {
            return Ok(None);
        };
        let program = parse_doc(&text);
        let Some(desc) = describe(&word, &program) else {
            return Ok(None);
        };
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: desc,
            }),
            range: None,
        }))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let Some(text) = self.get_doc(&uri) else {
            return Ok(None);
        };
        let offset = to_offset(&text, pos);
        let Some(word) = word_at(&text, offset) else {
            return Ok(None);
        };

        let docs = self.snapshot();
        if let Some(root) = find_root(&docs) {
            if let Ok(tree) = redstart_loader::load_with_overlay(&root, overlay_map(&docs)) {
                for module in tree.ordered() {
                    if let Some(span) = find_decl(&module.program, &word) {
                        if let Ok(def_uri) = Url::from_file_path(&module.file_path) {
                            let mtext = text_for(&def_uri, &docs)
                                .unwrap_or_else(|| module.source.to_string());
                            let range = to_range(&mtext, span.0, span.1);
                            return Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
                                def_uri, range,
                            ))));
                        }
                    }
                }
            }
        }
        // Fall back to the current document.
        let program = parse_doc(&text);
        if let Some((off, len)) = find_decl(&program, &word) {
            let range = to_range(&text, off, len);
            return Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
                uri, range,
            ))));
        }
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let items = match self.get_doc(&uri) {
            Some(text) => completions(&parse_doc(&text)),
            None => completions(&Program::default()),
        };
        Ok(Some(CompletionResponse::Array(items)))
    }
}

// ---- analysis helpers ----

fn parse_doc(text: &str) -> Program {
    match lex(text) {
        Ok(lexed) => {
            let (program, _errs) = parse(lexed.tokens(), Arc::from(text));
            program
        }
        Err(_) => Program::default(),
    }
}

/// Lex/parse diagnostics as `(offset, len, message)`.
fn parse_lex_diags(text: &str) -> Vec<(usize, usize, String)> {
    match lex(text) {
        Err(e) => e
            .errors
            .iter()
            .map(|l| {
                (
                    l.start,
                    l.end - l.start,
                    format!("invalid token `{}`", l.text),
                )
            })
            .collect(),
        Ok(lexed) => {
            let (_program, errs) = parse(lexed.tokens(), Arc::from(text));
            errs.iter()
                .map(|e| {
                    let mut msg = e.message.clone();
                    if let Some(h) = &e.help {
                        msg.push_str(&format!(" — {h}"));
                    }
                    (e.span.start, e.span.len(), msg)
                })
                .collect()
        }
    }
}

fn diagnostic(text: &str, offset: usize, len: usize, message: String) -> Diagnostic {
    Diagnostic {
        range: to_range(text, offset, len),
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("redstart".into()),
        message,
        ..Default::default()
    }
}

/// Find the name span of a top-level declaration named `name`, if present.
fn find_decl(program: &Program, name: &str) -> Option<(usize, usize)> {
    let s = |id: &redstart_parser::Ident| (id.span.start, id.span.len());
    program
        .entities
        .iter()
        .find(|e| e.name.name == name)
        .map(|e| s(&e.name))
        .or_else(|| {
            program
                .sources
                .iter()
                .find(|x| x.name.name == name)
                .map(|x| s(&x.name))
        })
        .or_else(|| {
            program
                .templates
                .iter()
                .find(|x| x.name.name == name)
                .map(|x| s(&x.name))
        })
        .or_else(|| {
            program
                .abis
                .iter()
                .find(|x| x.name.name == name)
                .map(|x| s(&x.name))
        })
        .or_else(|| {
            program
                .functions
                .iter()
                .find(|x| x.name.name == name)
                .map(|x| s(&x.name))
        })
}

fn describe(word: &str, program: &Program) -> Option<String> {
    if KEYWORDS.contains(&word) {
        return Some(format!("```redstart\n{word}\n```\nkeyword"));
    }
    if SCALARS.contains(&word) {
        return Some(format!("```redstart\n{word}\n```\nbuilt-in scalar"));
    }
    if program.entities.iter().any(|e| e.name.name == word) {
        return Some(format!("```redstart\nentity {word}\n```"));
    }
    if program.abis.iter().any(|a| a.name.name == word) {
        return Some(format!("```redstart\nabi {word}\n```"));
    }
    if program.sources.iter().any(|x| x.name.name == word)
        || program.templates.iter().any(|x| x.name.name == word)
    {
        return Some(format!("```redstart\nsource {word}\n```"));
    }
    None
}

fn completions(program: &Program) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    for kw in KEYWORDS {
        items.push(CompletionItem {
            label: (*kw).to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
    }
    for ty in SCALARS.iter().chain(GENERICS.iter()) {
        items.push(CompletionItem {
            label: (*ty).to_string(),
            kind: Some(CompletionItemKind::CLASS),
            ..Default::default()
        });
    }
    for e in &program.entities {
        items.push(CompletionItem {
            label: e.name.name.clone(),
            kind: Some(CompletionItemKind::CLASS),
            detail: Some("entity".into()),
            ..Default::default()
        });
    }
    items
}

fn symbols(program: &Program, text: &str) -> Vec<DocumentSymbol> {
    let mut out = Vec::new();
    let mut push = |name: String,
                    detail: &str,
                    kind: SymbolKind,
                    span: &redstart_parser::Span,
                    nspan: &redstart_parser::Span| {
        #[allow(deprecated)]
        out.push(DocumentSymbol {
            name,
            detail: Some(detail.to_string()),
            kind,
            tags: None,
            deprecated: None,
            range: to_range(text, span.start, span.len()),
            selection_range: to_range(text, nspan.start, nspan.len()),
            children: None,
        });
    };
    for a in &program.abis {
        push(
            a.name.name.clone(),
            "abi",
            SymbolKind::NAMESPACE,
            &a.span,
            &a.name.span,
        );
    }
    for e in &program.entities {
        push(
            e.name.name.clone(),
            "entity",
            SymbolKind::CLASS,
            &e.span,
            &e.name.span,
        );
    }
    for s in &program.sources {
        push(
            s.name.name.clone(),
            "source",
            SymbolKind::STRUCT,
            &s.span,
            &s.name.span,
        );
    }
    for t in &program.templates {
        push(
            t.name.name.clone(),
            "template",
            SymbolKind::STRUCT,
            &t.span,
            &t.name.span,
        );
    }
    for h in &program.handlers {
        push(
            format!("handle{}", h.event.name),
            "handler",
            SymbolKind::FUNCTION,
            &h.span,
            &h.event.span,
        );
    }
    for f in &program.functions {
        push(
            f.name.name.clone(),
            "fn",
            SymbolKind::FUNCTION,
            &f.span,
            &f.name.span,
        );
    }
    out
}

// ---- project / overlay ----

fn find_root(docs: &HashMap<Url, String>) -> Option<PathBuf> {
    for uri in docs.keys() {
        if let Ok(path) = uri.to_file_path() {
            let mut dir = path.parent().map(PathBuf::from);
            while let Some(d) = dir {
                if d.join("redstart.toml").exists() {
                    return Some(d);
                }
                dir = d.parent().map(PathBuf::from);
            }
        }
    }
    None
}

fn overlay_map(docs: &HashMap<Url, String>) -> HashMap<PathBuf, String> {
    let mut m = HashMap::new();
    for (uri, text) in docs {
        if let Ok(path) = uri.to_file_path() {
            let key = path.canonicalize().unwrap_or(path);
            m.insert(key, text.clone());
        }
    }
    m
}

fn text_for(uri: &Url, docs: &HashMap<Url, String>) -> Option<String> {
    if let Some(t) = docs.get(uri) {
        return Some(t.clone());
    }
    uri.to_file_path()
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
}

// ---- position conversion (byte offset <-> UTF-16 LSP position) ----

fn to_position(text: &str, offset: usize) -> Position {
    let offset = offset.min(text.len());
    let mut line = 0u32;
    let mut line_start = 0usize;
    for (i, c) in text.char_indices() {
        if i >= offset {
            break;
        }
        if c == '\n' {
            line += 1;
            line_start = i + 1;
        }
    }
    let character = text[line_start..offset].encode_utf16().count() as u32;
    Position::new(line, character)
}

fn to_range(text: &str, offset: usize, len: usize) -> Range {
    Range::new(to_position(text, offset), to_position(text, offset + len))
}

fn to_offset(text: &str, pos: Position) -> usize {
    let mut line = 0u32;
    for (i, c) in text.char_indices() {
        if line == pos.line {
            // Walk pos.character UTF-16 units into this line.
            let mut utf16 = 0u32;
            for (j, ch) in text[i..].char_indices() {
                if utf16 >= pos.character || ch == '\n' {
                    return i + j;
                }
                utf16 += ch.len_utf16() as u32;
            }
            return text.len();
        }
        if c == '\n' {
            line += 1;
        }
    }
    text.len()
}

/// The identifier-like word covering `offset`.
fn word_at(text: &str, offset: usize) -> Option<String> {
    let bytes = text.as_bytes();
    let is_word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    if offset > text.len() {
        return None;
    }
    let mut start = offset;
    while start > 0 && is_word(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = offset;
    while end < bytes.len() && is_word(bytes[end]) {
        end += 1;
    }
    if start == end {
        None
    } else {
        Some(text[start..end].to_string())
    }
}

const KEYWORDS: &[&str] = &[
    "abi",
    "from",
    "entity",
    "enum",
    "interface",
    "implements",
    "aggregation",
    "over",
    "source",
    "template",
    "handler",
    "on",
    "derived",
    "match",
    "let",
    "return",
    "if",
    "else",
    "while",
    "for",
    "in",
    "fn",
    "mod",
    "use",
    "test",
    "true",
    "false",
];
const SCALARS: &[&str] = &[
    "BigInt",
    "BigDecimal",
    "Bytes",
    "Address",
    "String",
    "Bool",
    "Int",
    "Int8",
    "Timestamp",
];
const GENERICS: &[&str] = &["Option", "Result", "Id", "List"];
