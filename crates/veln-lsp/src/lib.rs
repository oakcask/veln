//! LSP-facing semantic token helpers for Veln editors.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use veln_analysis::{DoctestMode, checked_project_diagnostics, parse_diagnostic_to_envelope};
use veln_ast::{SurfaceModule, lower_surface_ast};
use veln_diagnostics::{Diagnostic, Severity};
use veln_editor::{encode_lsp_semantic_tokens, semantic_token_legend};
use veln_project::{Project, classify_companion_source, discover_source_paths};
use veln_source::{SourceFile, SourceSpan};
use veln_syntax::{Token, TokenKind, format_tree, lex, parse};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticTokensLegend {
    pub token_types: Vec<&'static str>,
    pub token_modifiers: Vec<&'static str>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticTokensFull {
    pub data: Vec<u32>,
}

pub fn legend() -> SemanticTokensLegend {
    let (token_types, token_modifiers) = semantic_token_legend();
    SemanticTokensLegend {
        token_types,
        token_modifiers,
    }
}

pub fn semantic_tokens_full(source: &SourceFile) -> SemanticTokensFull {
    let tokens = veln_editor::collect_semantic_tokens(source);
    let data = encode_lsp_semantic_tokens(&tokens)
        .into_iter()
        .flat_map(|token| {
            [
                token.delta_line,
                token.delta_start,
                token.length,
                token.token_type,
                token.token_modifiers,
            ]
        })
        .collect();
    SemanticTokensFull { data }
}

pub fn diagnostics(source: &SourceFile) -> Vec<Diagnostic> {
    let parsed = parse(source);
    let parse_diagnostics = parsed
        .diagnostics
        .iter()
        .map(parse_diagnostic_to_envelope)
        .collect::<Vec<_>>();
    if !parse_diagnostics.is_empty() {
        return parse_diagnostics;
    }

    let lowered = lower_surface_ast(&parsed.tree);
    let module = SurfaceModule {
        module: lowered.module,
        uses: lowered.uses,
        aliases: lowered.aliases,
        effects: lowered.effects,
        handlers: lowered.handlers,
        types: lowered.types,
        schemas: lowered.schemas,
        codecs: lowered.codecs,
        functions: lowered.functions,
    };
    veln_sema::lower_checked_surface_module(&module).diagnostics
}

pub fn run_stdio() -> io::Result<()> {
    Server::default().run(io::stdin().lock(), io::stdout().lock())
}

#[derive(Default)]
struct Server {
    documents: BTreeMap<String, String>,
    workspace_roots: Vec<PathBuf>,
    published_diagnostic_uris: BTreeSet<String>,
    should_exit: bool,
}

impl Server {
    fn run(&mut self, input: impl Read, mut output: impl Write) -> io::Result<()> {
        let mut input = BufReader::new(input);
        while !self.should_exit {
            let Some(message) = read_message(&mut input)? else {
                break;
            };
            for response in self.handle_message(&message) {
                write_message(&mut output, &response)?;
            }
        }
        Ok(())
    }

    fn handle_message(&mut self, message: &str) -> Vec<String> {
        let method = extract_string_field(message, "method");
        let id = extract_id(message);
        let Some(method) = method.as_deref() else {
            return Vec::new();
        };

        match method {
            "initialize" => self.handle_initialize(message, id),
            "initialized" => Vec::new(),
            "shutdown" => self.handle_shutdown(id),
            "exit" => self.handle_exit(),
            "textDocument/didOpen" | "textDocument/didChange" => {
                self.handle_document_update(message)
            }
            "textDocument/didClose" => self.handle_document_close(message),
            "textDocument/semanticTokens/full" => self.handle_semantic_tokens(message, id),
            "textDocument/definition" => self.handle_definition(message, id),
            "textDocument/references" => self.handle_references(message, id),
            "textDocument/formatting" => self.handle_formatting(message, id),
            "textDocument/prepareRename" => self.handle_prepare_rename(message, id),
            "textDocument/rename" => self.handle_rename(message, id),
            _ => self.handle_unknown_method(id),
        }
    }

    fn handle_initialize(&mut self, message: &str, id: Option<String>) -> Vec<String> {
        self.workspace_roots = resolve_workspace_roots(message);
        let mut responses = id
            .map(|id| response(&id, &initialize_result()))
            .into_iter()
            .collect::<Vec<_>>();
        responses.extend(self.publish_workspace_diagnostics());
        responses
    }

    fn handle_shutdown(&self, id: Option<String>) -> Vec<String> {
        id.map(|id| response(&id, "null")).into_iter().collect()
    }

    fn handle_exit(&mut self) -> Vec<String> {
        self.should_exit = true;
        Vec::new()
    }

    fn handle_document_update(&mut self, message: &str) -> Vec<String> {
        let Some((uri, text)) = document_uri_and_text(message) else {
            return Vec::new();
        };
        self.documents.insert(uri.clone(), text);
        if self.workspace_source_path(&uri).is_some() {
            self.publish_workspace_diagnostics()
        } else {
            vec![publish_diagnostics(&uri, self.document_text(&uri))]
        }
    }

    fn handle_document_close(&mut self, message: &str) -> Vec<String> {
        let Some(uri) = extract_string_field(message, "uri") else {
            return Vec::new();
        };
        self.documents.remove(&uri);
        if self.workspace_source_path(&uri).is_some() {
            self.publish_workspace_diagnostics()
        } else {
            vec![empty_publish_diagnostics(&uri)]
        }
    }

    fn handle_semantic_tokens(&self, message: &str, id: Option<String>) -> Vec<String> {
        id.map(|id| {
            let uri = extract_string_field(message, "uri").unwrap_or_default();
            response(&id, &semantic_tokens_result(&uri, self.document_text(&uri)))
        })
        .into_iter()
        .collect()
    }

    fn handle_definition(&self, message: &str, id: Option<String>) -> Vec<String> {
        id.map(|id| {
            let result = self
                .symbol_at_request(message)
                .and_then(|request| request.index.symbol_location_json(&request.symbol))
                .unwrap_or_else(|| "null".to_string());
            response(&id, &result)
        })
        .into_iter()
        .collect()
    }

    fn handle_references(&self, message: &str, id: Option<String>) -> Vec<String> {
        id.map(|id| {
            let result = self
                .symbol_at_request(message)
                .map(|request| request.index.symbol_references_json(&request.symbol, true))
                .unwrap_or_else(|| "[]".to_string());
            response(&id, &result)
        })
        .into_iter()
        .collect()
    }

    fn handle_formatting(&self, message: &str, id: Option<String>) -> Vec<String> {
        id.map(|id| {
            let uri = extract_string_field(message, "uri").unwrap_or_default();
            response(&id, &formatting_result(&uri, self.document_text(&uri)))
        })
        .into_iter()
        .collect()
    }

    fn handle_prepare_rename(&self, message: &str, id: Option<String>) -> Vec<String> {
        id.map(|id| {
            let result = self
                .symbol_at_request(message)
                .map(|request| range_json(Some(&request.selection)))
                .unwrap_or_else(|| "null".to_string());
            response(&id, &result)
        })
        .into_iter()
        .collect()
    }

    fn handle_rename(&self, message: &str, id: Option<String>) -> Vec<String> {
        id.map(|id| {
            let result = extract_string_field(message, "newName")
                .filter(|new_name| is_identifier(new_name))
                .and_then(|new_name| {
                    self.symbol_at_request(message).map(|request| {
                        request
                            .index
                            .symbol_workspace_edit_json(&request.symbol, &new_name)
                    })
                })
                .unwrap_or_else(|| "{\"changes\":{}}".to_string());
            response(&id, &result)
        })
        .into_iter()
        .collect()
    }

    fn handle_unknown_method(&self, id: Option<String>) -> Vec<String> {
        id.map(|id| error_response(&id, -32601, "method not found"))
            .into_iter()
            .collect()
    }

    fn document_text(&self, uri: &str) -> String {
        if let Some(text) = self.documents.get(uri) {
            return text.clone();
        }
        uri_to_path(uri)
            .and_then(|path| fs::read_to_string(path).ok())
            .unwrap_or_default()
    }

    fn publish_workspace_diagnostics(&mut self) -> Vec<String> {
        let mut next_uris = BTreeSet::new();
        let mut responses = Vec::new();
        for root in self.workspace_roots.clone() {
            let Ok(mut project) = Project::discover(root.clone(), &[]) else {
                continue;
            };
            self.overlay_open_workspace_documents(&root, &mut project);
            let diagnostics = checked_project_diagnostics(project.clone(), DoctestMode::Exclude);
            let mut diagnostics_by_path = diagnostics_by_path(diagnostics);
            for source in &project.files {
                diagnostics_by_path
                    .entry(source.path().as_str().to_string())
                    .or_default();
            }

            for (source_path, diagnostics) in diagnostics_by_path {
                let uri = path_to_uri(&root.join(&source_path));
                next_uris.insert(uri.clone());
                responses.push(publish_diagnostics_for_uri(&uri, &diagnostics));
            }
        }
        for uri in self
            .published_diagnostic_uris
            .difference(&next_uris)
            .cloned()
            .collect::<Vec<_>>()
        {
            responses.push(empty_publish_diagnostics(&uri));
        }
        self.published_diagnostic_uris = next_uris;
        responses
    }

    fn overlay_open_workspace_documents(&self, root: &Path, project: &mut Project) {
        for (uri, text) in &self.documents {
            let Some(source) = owned_workspace_source_file(root, uri, text) else {
                continue;
            };
            let source_path = source.path().as_str().to_string();
            if let Some(existing) = project
                .files
                .iter_mut()
                .find(|file| file.path().as_str() == source_path)
            {
                *existing = source;
            } else {
                project.files.push(source);
            }
        }
        project
            .files
            .sort_by(|left, right| left.path().as_str().cmp(right.path().as_str()));
    }

    fn workspace_source_path(&self, uri: &str) -> Option<String> {
        let root = workspace_root_for_uri(&self.workspace_roots, uri)?;
        owned_workspace_relative_source_path(root, uri)
    }

    fn symbol_at_request(&self, message: &str) -> Option<SymbolRequest> {
        let uri = extract_string_field(message, "uri")?;
        let position = extract_position(message)?;
        let root = workspace_root_for_uri(&self.workspace_roots, &uri)?;
        let source_path = workspace_relative_source_path(root, &uri)?;
        let mut project = Project::discover(root.to_path_buf(), &[]).ok()?;
        self.overlay_open_workspace_documents(root, &mut project);
        let index = LspSymbolIndex::new(root, project.files);
        index.symbol_at_position(&source_path, position)
    }
}

fn document_uri_and_text(message: &str) -> Option<(String, String)> {
    Some((
        extract_string_field(message, "uri")?,
        extract_string_field(message, "text")?,
    ))
}

fn initialize_result() -> String {
    let legend = legend();
    format!(
        "{{\"capabilities\":{{\"textDocumentSync\":1,\"definitionProvider\":true,\"referencesProvider\":true,\"documentFormattingProvider\":true,\"renameProvider\":{{\"prepareProvider\":true}},\"semanticTokensProvider\":{{\"legend\":{{\"tokenTypes\":[{}],\"tokenModifiers\":[{}]}},\"full\":true,\"range\":false}}}}}}",
        json_string_list(&legend.token_types),
        json_string_list(&legend.token_modifiers),
    )
}

fn semantic_tokens_result(uri: &str, text: String) -> String {
    let source = SourceFile::new(display_path(uri), text);
    let data = semantic_tokens_full(&source)
        .data
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"data\":[{data}]}}")
}

fn formatting_result(uri: &str, text: String) -> String {
    let source = SourceFile::new(display_path(uri), text.clone());
    let parsed = parse(&source);
    if !parsed.diagnostics.is_empty() {
        return "[]".to_string();
    }
    let formatted = format_tree(&parsed.tree);
    if formatted == text {
        return "[]".to_string();
    }
    format!(
        "[{{\"range\":{},\"newText\":\"{}\"}}]",
        full_document_range_json(&text),
        escape_json(&formatted)
    )
}

fn publish_diagnostics(uri: &str, text: String) -> String {
    let source = SourceFile::new(display_path(uri), text);
    let diagnostics = diagnostics(&source)
        .into_iter()
        .filter(|diagnostic| diagnostic_applies_to_uri(diagnostic, uri))
        .collect::<Vec<_>>();
    publish_diagnostics_for_uri(uri, &diagnostics)
}

fn publish_diagnostics_for_uri(uri: &str, diagnostics: &[Diagnostic]) -> String {
    let diagnostics = diagnostics
        .iter()
        .map(lsp_diagnostic_json)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{{\"uri\":\"{}\",\"diagnostics\":[{diagnostics}]}}}}",
        escape_json(uri)
    )
}

fn empty_publish_diagnostics(uri: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{{\"uri\":\"{}\",\"diagnostics\":[]}}}}",
        escape_json(uri)
    )
}

fn diagnostic_applies_to_uri(diagnostic: &Diagnostic, uri: &str) -> bool {
    diagnostic
        .span
        .as_ref()
        .is_none_or(|span| span.file.as_str() == display_path(uri))
}

fn lsp_diagnostic_json(diagnostic: &Diagnostic) -> String {
    format!(
        "{{\"range\":{},\"severity\":{},\"code\":\"{}\",\"source\":\"veln\",\"message\":\"{}\"}}",
        range_json(diagnostic.span.as_ref()),
        severity_code(diagnostic.severity),
        escape_json(&diagnostic.id),
        escape_json(&diagnostic.message),
    )
}

fn range_json(span: Option<&SourceSpan>) -> String {
    let Some(span) = span else {
        return "{\"start\":{\"line\":0,\"character\":0},\"end\":{\"line\":0,\"character\":0}}"
            .to_string();
    };
    format!(
        "{{\"start\":{},\"end\":{}}}",
        position_json(span.start.line, span.start.column),
        position_json(span.end.line, span.end.column),
    )
}

fn position_json(line: usize, column: usize) -> String {
    format!(
        "{{\"line\":{},\"character\":{}}}",
        line.saturating_sub(1),
        column.saturating_sub(1),
    )
}

fn full_document_range_json(text: &str) -> String {
    let mut line = 0usize;
    let mut character = 0usize;
    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }
    format!(
        "{{\"start\":{{\"line\":0,\"character\":0}},\"end\":{{\"line\":{line},\"character\":{character}}}}}"
    )
}

fn severity_code(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 1,
        Severity::Warning => 2,
        Severity::Info => 3,
        Severity::Hint => 4,
    }
}

fn diagnostics_by_path(diagnostics: Vec<Diagnostic>) -> BTreeMap<String, Vec<Diagnostic>> {
    let mut by_path = BTreeMap::<String, Vec<Diagnostic>>::new();
    for diagnostic in diagnostics {
        if let Some(span) = &diagnostic.span {
            by_path
                .entry(span.file.as_str().to_string())
                .or_default()
                .push(diagnostic);
        }
    }
    by_path
}

fn resolve_workspace_roots(message: &str) -> Vec<PathBuf> {
    let client_roots = extract_workspace_folder_uris(message)
        .into_iter()
        .filter_map(|uri| uri_to_path(&uri))
        .filter_map(filesystem_workspace_root)
        .collect::<Vec<_>>();
    let mut roots = Vec::new();
    for root in client_roots {
        roots.extend(resolve_workspace_project_roots(&root));
    }
    if roots.is_empty()
        && let Some(root) =
            extract_string_field(message, "rootUri").and_then(|uri| uri_to_path(&uri))
    {
        let Some(root) = filesystem_workspace_root(root) else {
            return roots;
        };
        roots.extend(resolve_workspace_project_roots(&root));
    }
    roots.sort();
    roots.dedup();
    roots
}

fn filesystem_workspace_root(root: PathBuf) -> Option<PathBuf> {
    fs::canonicalize(root).ok()
}

fn resolve_workspace_project_roots(root: &Path) -> Vec<PathBuf> {
    if has_regular_manifest(root) {
        return vec![root.to_path_buf()];
    }
    let mut roots = Vec::new();
    collect_manifest_project_roots(root, &mut roots);
    if roots.is_empty() {
        roots.push(root.to_path_buf());
    }
    roots
}

fn collect_manifest_project_roots(dir: &Path, roots: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name = file_name.to_string_lossy();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_name == ".git" || !file_type.is_dir() {
            continue;
        }
        if has_regular_manifest(&path) {
            roots.push(path);
        } else {
            collect_manifest_project_roots(&path, roots);
        }
    }
}

fn has_regular_manifest(root: &Path) -> bool {
    fs::symlink_metadata(root.join("veln.toml"))
        .is_ok_and(|metadata| metadata.file_type().is_file())
}

fn extract_workspace_folder_uris(message: &str) -> Vec<String> {
    let Some(index) = message.find("\"workspaceFolders\"") else {
        return Vec::new();
    };
    let mut rest = &message[index..];
    let mut uris = Vec::new();
    while let Some(uri_index) = rest.find("\"uri\"") {
        rest = &rest[uri_index..];
        if let Some(uri) = extract_string_field(rest, "uri") {
            uris.push(uri);
        }
        rest = &rest["\"uri\"".len()..];
    }
    uris
}

fn workspace_root_for_uri<'a>(roots: &'a [PathBuf], uri: &str) -> Option<&'a Path> {
    let uri_path = uri_to_path(uri)?;
    let absolute = if uri_path.is_absolute() {
        uri_path
    } else {
        env::current_dir().ok()?.join(uri_path)
    };
    roots
        .iter()
        .filter(|root| absolute.starts_with(root))
        .max_by_key(|root| root.components().count())
        .map(PathBuf::as_path)
}

fn owned_workspace_source_file(root: &Path, uri: &str, text: &str) -> Option<SourceFile> {
    owned_workspace_relative_source_path(root, uri)
        .map(|path| SourceFile::new(path, text.to_string()))
}

fn owned_workspace_relative_source_path(root: &Path, uri: &str) -> Option<String> {
    let relative = workspace_relative_source_path(root, uri)?;
    let input = PathBuf::from(&relative);
    discover_source_paths(root, std::slice::from_ref(&input)).ok()?;
    Some(relative)
}

fn workspace_relative_source_path(root: &Path, uri: &str) -> Option<String> {
    let uri_path = uri_to_path(uri)?;
    let absolute = if uri_path.is_absolute() {
        uri_path
    } else {
        root.join(uri_path)
    };
    if absolute
        .extension()
        .is_none_or(|extension| extension != "veln")
    {
        return None;
    }
    let relative = absolute.strip_prefix(root).ok()?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}

#[derive(Clone, Debug)]
struct Position {
    line: usize,
    character: usize,
}

#[derive(Clone, Debug)]
struct FunctionSymbol {
    module: String,
    name: String,
    declaration: SourceSpan,
}

#[derive(Debug)]
struct SymbolRequest {
    index: LspSymbolIndex,
    symbol: LspSymbol,
    selection: SourceSpan,
}

#[derive(Clone, Debug)]
enum LspSymbol {
    Function(FunctionSymbol),
    Local(LocalSymbol),
}

#[derive(Clone, Debug)]
struct LocalSymbol {
    name: String,
    declaration: SourceSpan,
    scope_file: String,
    scope_start: usize,
    scope_end: usize,
    kind: LocalSymbolKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum LocalSymbolKind {
    HandlerContextParameter,
    HandlerOperationClauseParameter,
}

#[derive(Debug)]
struct LspFile {
    uri: String,
    source: SourceFile,
    module: String,
    companion_target_module: Option<String>,
    uses: BTreeSet<String>,
}

#[derive(Debug)]
struct LspSymbolIndex {
    files: Vec<LspFile>,
    functions: Vec<FunctionSymbol>,
}

#[derive(Debug)]
struct FunctionScope {
    body_start: usize,
    end: usize,
    params: BTreeSet<String>,
    result_binding: Option<String>,
    local_bindings: Vec<LocalBinding>,
}

#[derive(Debug)]
struct LocalBinding {
    name: String,
    start: usize,
    end: usize,
}

#[derive(Debug)]
struct ClauseBinding {
    name: String,
    declaration: SourceSpan,
    start: usize,
    end: usize,
    kind: LocalSymbolKind,
}

impl LspSymbolIndex {
    fn new(root: &Path, sources: Vec<SourceFile>) -> Self {
        let files = sources
            .into_iter()
            .map(|source| {
                let path = source.path().as_str().to_string();
                let companion_target_module = classify_companion_source(&path)
                    .and_then(|companion| module_name_from_path(&companion.target_path));
                let module = explicit_module_name(source.text())
                    .or_else(|| module_name_from_path(&path))
                    .unwrap_or_default();
                let uses = use_modules(source.text());
                let uri = path_to_uri(&root.join(&path));
                LspFile {
                    uri,
                    source,
                    module,
                    companion_target_module,
                    uses,
                }
            })
            .collect::<Vec<_>>();
        let functions = files.iter().flat_map(function_declarations).collect();
        Self { files, functions }
    }

    fn symbol_at_position(self, source_path: &str, position: Position) -> Option<SymbolRequest> {
        let file = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == source_path)?;
        let offset = offset_for_position(file.source.text(), position)?;
        let tokens = lex(&file.source).tokens;
        let (token_index, token) = identifier_token_at(&tokens, offset)?;
        let selection = file.source.span(token.range);
        let name = file
            .source
            .text()
            .get(selection.start.offset..selection.end.offset)?
            .to_string();
        let symbol = self.symbol_for_selection(file, &tokens, token_index, &name, &selection)?;
        Some(SymbolRequest {
            index: self,
            symbol,
            selection,
        })
    }

    fn symbol_for_selection(
        &self,
        file: &LspFile,
        tokens: &[Token],
        token_index: usize,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<LspSymbol> {
        if let Some(symbol) =
            handler_operation_clause_symbol(file, tokens, token_index, name, selection)
        {
            return Some(LspSymbol::Local(symbol));
        }

        if is_handler_operation_clause_operation_name(tokens, token_index) {
            return None;
        }

        if let Some(symbol) = self.functions.iter().find(|symbol| {
            symbol.name == name
                && symbol.declaration.file == selection.file
                && symbol.declaration.start.offset == selection.start.offset
                && symbol.declaration.end.offset == selection.end.offset
        }) {
            return Some(LspSymbol::Function(symbol.clone()));
        }

        if !is_call_target_token(tokens, token_index) {
            return None;
        }
        let Some(qualifier) = qualifier_for_token(tokens, token_index) else {
            return self
                .functions
                .iter()
                .find(|symbol| symbol.name == name && symbol.module == file.module)
                .cloned()
                .map(LspSymbol::Function);
        };
        self.functions
            .iter()
            .find(|symbol| {
                symbol.name == name
                    && symbol.module == qualifier
                    && file.uses.contains(&symbol.module)
                    && file
                        .companion_target_module
                        .as_ref()
                        .is_some_and(|target| target == &symbol.module)
            })
            .cloned()
            .map(LspSymbol::Function)
    }

    fn location_json(&self, span: &SourceSpan) -> Option<String> {
        let uri = self
            .files
            .iter()
            .find(|file| file.source.path() == &span.file)?
            .uri
            .as_str();
        Some(format!(
            "{{\"uri\":\"{}\",\"range\":{}}}",
            escape_json(uri),
            range_json(Some(span))
        ))
    }

    fn workspace_edit_json(&self, symbol: &FunctionSymbol, new_name: &str) -> String {
        let mut changes = BTreeMap::<String, Vec<SourceSpan>>::new();
        changes
            .entry(uri_for_span(&self.files, &symbol.declaration))
            .or_default()
            .push(symbol.declaration.clone());
        for file in &self.files {
            for span in self.references_in_file(file, symbol) {
                changes.entry(file.uri.clone()).or_default().push(span);
            }
        }

        let changes = changes
            .into_iter()
            .map(|(uri, mut spans)| {
                spans.sort_by_key(|span| span.start.offset);
                spans.dedup_by_key(|span| (span.start.offset, span.end.offset));
                let edits = spans
                    .iter()
                    .map(|span| {
                        format!(
                            "{{\"range\":{},\"newText\":\"{}\"}}",
                            range_json(Some(span)),
                            escape_json(new_name)
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(",");
                format!("\"{}\":[{}]", escape_json(&uri), edits)
            })
            .collect::<Vec<_>>()
            .join(",");
        format!("{{\"changes\":{{{changes}}}}}")
    }

    fn local_workspace_edit_json(&self, symbol: &LocalSymbol, new_name: &str) -> String {
        let spans = self.local_references(symbol, true);
        let edits = spans
            .iter()
            .map(|span| {
                format!(
                    "{{\"range\":{},\"newText\":\"{}\"}}",
                    range_json(Some(span)),
                    escape_json(new_name)
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let uri = uri_for_span(&self.files, &symbol.declaration);
        format!("{{\"changes\":{{\"{}\":[{edits}]}}}}", escape_json(&uri))
    }

    fn references_json(&self, symbol: &FunctionSymbol, include_declaration: bool) -> String {
        let mut locations = Vec::new();
        if include_declaration && let Some(location) = self.location_json(&symbol.declaration) {
            locations.push(location);
        }
        for file in &self.files {
            for span in self.references_in_file(file, symbol) {
                if let Some(location) = self.location_json(&span) {
                    locations.push(location);
                }
            }
        }
        locations.sort();
        locations.dedup();
        format!("[{}]", locations.join(","))
    }

    fn local_references_json(&self, symbol: &LocalSymbol, include_declaration: bool) -> String {
        let locations = self
            .local_references(symbol, include_declaration)
            .into_iter()
            .filter_map(|span| self.location_json(&span))
            .collect::<Vec<_>>();
        format!("[{}]", locations.join(","))
    }

    fn local_references(&self, symbol: &LocalSymbol, include_declaration: bool) -> Vec<SourceSpan> {
        let Some(file) = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == symbol.scope_file)
        else {
            return Vec::new();
        };
        let tokens = lex(&file.source).tokens;
        let mut spans = Vec::new();
        if include_declaration {
            spans.push(symbol.declaration.clone());
        }
        spans.extend(
            tokens
                .iter()
                .enumerate()
                .filter(|(index, token)| {
                    token.text == symbol.name
                        && token.kind == TokenKind::Ident
                        && token.range.start >= symbol.scope_start
                        && token.range.start < symbol.scope_end
                        && !is_field_name(&tokens, *index)
                        && !is_local_binding_name(&tokens, *index)
                        && (symbol.kind != LocalSymbolKind::HandlerContextParameter
                            || inside_handler_operation_clause_body(&tokens, token.range.start))
                        && !local_binding_shadows_name(
                            &tokens,
                            &symbol.name,
                            token.range.start,
                            symbol.scope_start,
                            symbol.scope_end,
                        )
                        && (symbol.kind != LocalSymbolKind::HandlerContextParameter
                            || !handler_operation_clause_parameter_shadows_name(
                                &tokens,
                                &symbol.name,
                                token.range.start,
                                symbol.scope_start,
                                symbol.scope_end,
                            ))
                })
                .map(|(_, token)| file.source.span(token.range)),
        );
        spans.sort_by_key(|span| span.start.offset);
        spans.dedup_by_key(|span| (span.start.offset, span.end.offset));
        spans
    }

    fn symbol_location_json(&self, symbol: &LspSymbol) -> Option<String> {
        match symbol {
            LspSymbol::Function(symbol) => self.location_json(&symbol.declaration),
            LspSymbol::Local(symbol) => self.location_json(&symbol.declaration),
        }
    }

    fn symbol_references_json(&self, symbol: &LspSymbol, include_declaration: bool) -> String {
        match symbol {
            LspSymbol::Function(symbol) => self.references_json(symbol, include_declaration),
            LspSymbol::Local(symbol) => self.local_references_json(symbol, include_declaration),
        }
    }

    fn symbol_workspace_edit_json(&self, symbol: &LspSymbol, new_name: &str) -> String {
        match symbol {
            LspSymbol::Function(symbol) => self.workspace_edit_json(symbol, new_name),
            LspSymbol::Local(symbol) => self.local_workspace_edit_json(symbol, new_name),
        }
    }

    fn references_in_file(&self, file: &LspFile, symbol: &FunctionSymbol) -> Vec<SourceSpan> {
        if file.module == symbol.module {
            return call_references(&file.source, &symbol.name);
        }
        if file.uses.contains(&symbol.module)
            && file
                .companion_target_module
                .as_ref()
                .is_some_and(|target| target == &symbol.module)
        {
            return qualified_references(&file.source, &symbol.module, &symbol.name);
        }
        Vec::new()
    }
}

fn uri_for_span(files: &[LspFile], span: &SourceSpan) -> String {
    files
        .iter()
        .find(|file| file.source.path() == &span.file)
        .map(|file| file.uri.clone())
        .unwrap_or_else(|| span.file.as_str().to_string())
}

fn function_declarations(file: &LspFile) -> Vec<FunctionSymbol> {
    let mut functions = Vec::new();
    let tokens = lex(&file.source).tokens;
    for (index, token) in tokens.iter().enumerate() {
        if token.kind == TokenKind::Fn
            && let Some(name) = next_non_layout_token(&tokens, index)
            && is_identifier(&name.text)
        {
            functions.push(FunctionSymbol {
                module: file.module.clone(),
                name: name.text.clone(),
                declaration: file.source.span(name.range),
            });
        }
    }
    functions
}

fn handler_operation_clause_symbol(
    file: &LspFile,
    tokens: &[Token],
    token_index: usize,
    name: &str,
    selection: &SourceSpan,
) -> Option<LocalSymbol> {
    handler_operation_clause_bindings(file, tokens)
        .into_iter()
        .find(|binding| {
            let token_offset = tokens[token_index].range.start;
            binding.name == name
                && ((selection.start.offset >= binding.declaration.start.offset
                    && selection.start.offset < binding.declaration.end.offset)
                    || (token_offset >= binding.start
                        && token_offset < binding.end
                        && (binding.kind != LocalSymbolKind::HandlerContextParameter
                            || inside_handler_operation_clause_body(tokens, token_offset))
                        && !local_binding_shadows_name(
                            tokens,
                            &binding.name,
                            token_offset,
                            binding.start,
                            binding.end,
                        )))
        })
        .map(|binding| LocalSymbol {
            name: binding.name,
            declaration: binding.declaration,
            scope_file: file.source.path().as_str().to_string(),
            scope_start: binding.start,
            scope_end: binding.end,
            kind: binding.kind,
        })
}

fn handler_operation_clause_bindings(file: &LspFile, tokens: &[Token]) -> Vec<ClauseBinding> {
    let mut clause_bindings = Vec::new();
    for (arrow_index, arrow) in tokens.iter().enumerate() {
        if arrow.kind != TokenKind::FatArrow
            || !inside_top_level_block(tokens, arrow_index, TokenKind::Handler)
        {
            continue;
        }
        let line_start_index = line_start_index(tokens, arrow_index);
        let body_end =
            handler_operation_clause_body_end(tokens, arrow_index, file.source.text().len());
        let Some(lparen_index) = tokens[line_start_index..arrow_index]
            .iter()
            .position(|token| token.kind == TokenKind::LParen)
            .map(|index| line_start_index + index)
        else {
            continue;
        };
        let Some(rparen_index) = tokens[lparen_index + 1..arrow_index]
            .iter()
            .position(|token| token.kind == TokenKind::RParen)
            .map(|index| lparen_index + 1 + index)
        else {
            continue;
        };
        for token in &tokens[lparen_index + 1..rparen_index] {
            if token.kind == TokenKind::Ident && is_identifier(&token.text) {
                clause_bindings.push(ClauseBinding {
                    name: token.text.clone(),
                    declaration: file.source.span(token.range),
                    start: arrow.range.end,
                    end: body_end,
                    kind: LocalSymbolKind::HandlerOperationClauseParameter,
                });
            }
        }
    }
    clause_bindings.extend(handler_context_parameter_bindings(file, tokens));
    clause_bindings
}

fn handler_context_parameter_bindings(file: &LspFile, tokens: &[Token]) -> Vec<ClauseBinding> {
    tokens
        .iter()
        .enumerate()
        .filter(|(_, token)| token.kind == TokenKind::Handler)
        .flat_map(|(handler_index, _)| {
            handler_context_parameter_bindings_for_handler(file, tokens, handler_index)
        })
        .collect()
}

fn handler_context_parameter_bindings_for_handler(
    file: &LspFile,
    tokens: &[Token],
    handler_index: usize,
) -> Vec<ClauseBinding> {
    let Some(body_start) = tokens[handler_index..]
        .iter()
        .find(|token| token.kind == TokenKind::Newline)
        .map(|token| token.range.end)
    else {
        return Vec::new();
    };
    let handler_end = function_scope_end(tokens, handler_index + 1).unwrap_or(body_start);
    let Some(lparen_index) = tokens[handler_index..]
        .iter()
        .position(|token| token.kind == TokenKind::LParen)
        .map(|index| handler_index + index)
    else {
        return Vec::new();
    };
    let Some(rparen_index) = matching_rparen_index(tokens, lparen_index, tokens.len()) else {
        return Vec::new();
    };
    tokens[lparen_index + 1..rparen_index]
        .iter()
        .enumerate()
        .filter(|(_, token)| token.kind == TokenKind::Ident && is_identifier(&token.text))
        .filter(|(relative_index, _)| {
            let index = lparen_index + 1 + relative_index;
            next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::Colon)
        })
        .map(|(_, token)| ClauseBinding {
            name: token.text.clone(),
            declaration: file.source.span(token.range),
            start: body_start,
            end: handler_end,
            kind: LocalSymbolKind::HandlerContextParameter,
        })
        .collect()
}

fn handler_operation_clause_body_end(
    tokens: &[Token],
    arrow_index: usize,
    file_end: usize,
) -> usize {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[arrow_index + 1..].iter().enumerate() {
        let index = arrow_index + 1 + relative_index;
        match token.kind {
            TokenKind::Eof => return file_end,
            TokenKind::If if !is_else_if(tokens, index) => nested_blocks += 1,
            TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::End if nested_blocks == 0 => return token.range.start,
            TokenKind::End => nested_blocks = nested_blocks.saturating_sub(1),
            TokenKind::FatArrow if nested_blocks == 0 && !is_satisfy_arrow(tokens, index) => {
                return match_arm_pattern_start_from_arrow(tokens, token.range.start);
            }
            _ => {}
        }
    }
    file_end
}

fn line_start_index(tokens: &[Token], index: usize) -> usize {
    tokens[..index]
        .iter()
        .rposition(|token| token.kind == TokenKind::Newline)
        .map_or(0, |index| index + 1)
}

fn matching_rparen_index(tokens: &[Token], lparen_index: usize, end_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (relative_index, token) in tokens[lparen_index..end_index].iter().enumerate() {
        let index = lparen_index + relative_index;
        match token.kind {
            TokenKind::LParen => depth += 1,
            TokenKind::RParen => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn call_references(source: &SourceFile, name: &str) -> Vec<SourceSpan> {
    let tokens = lex(source).tokens;
    let scopes = function_scopes(&tokens);
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.text == name
                && is_identifier(&token.text)
                && previous_non_layout_token(&tokens, *index)
                    .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
                && !is_field_name(&tokens, *index)
                && !is_function_declaration_name(&tokens, *index)
                && !is_parameter_name(&tokens, *index)
                && !is_local_binding_name(&tokens, *index)
                && !is_handler_operation_clause_operation_name(&tokens, *index)
                && (token_scope(&scopes, token.range.start)
                    .is_some_and(|scope| !scope.shadows(name, &tokens, *index))
                    || is_handler_operation_clause_call_target(&tokens, *index)
                    || is_function_alias_target_reference(&tokens, *index, name)
                    || is_codec_implementation_function_reference(&tokens, *index, name))
        })
        .map(|(_, token)| source.span(token.range))
        .collect()
}

fn qualified_references(source: &SourceFile, module: &str, name: &str) -> Vec<SourceSpan> {
    let tokens = lex(source).tokens;
    let module_segments = module.split("::").collect::<Vec<_>>();
    tokens
        .iter()
        .enumerate()
        .filter(|(index, token)| {
            token.text == name
                && is_call_target_token(&tokens, *index)
                && qualified_reference_matches(&tokens, *index, &module_segments)
        })
        .map(|(_, token)| source.span(token.range))
        .collect()
}

fn function_scopes(tokens: &[Token]) -> Vec<FunctionScope> {
    let mut scopes = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(token.kind, TokenKind::Fn | TokenKind::Test) {
            continue;
        }
        let Some(body_start) = tokens[index..]
            .iter()
            .find(|token| token.kind == TokenKind::Newline)
            .map(|token| token.range.end)
        else {
            continue;
        };
        let end = function_scope_end(tokens, index + 1).unwrap_or(body_start);
        let params = parameter_names(tokens, index, body_start);
        let result_binding = result_binding_name(tokens, index, body_start);
        let local_bindings = local_bindings(tokens, body_start, end);
        scopes.push(FunctionScope {
            body_start,
            end,
            params,
            result_binding,
            local_bindings,
        });
    }
    scopes
}

impl FunctionScope {
    fn shadows(&self, name: &str, tokens: &[Token], index: usize) -> bool {
        let offset = tokens[index].range.start;
        self.params.contains(name)
            || self
                .result_binding
                .as_deref()
                .is_some_and(|binding| binding == name && is_ensure_reference(tokens, index))
            || self.local_bindings.iter().any(|binding| {
                binding.name == name && binding.start <= offset && offset < binding.end
            })
    }
}

fn function_scope_end(tokens: &[Token], start: usize) -> Option<usize> {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[start..].iter().enumerate() {
        let index = start + relative_index;
        match token.kind {
            TokenKind::If if !is_else_if(tokens, index) => nested_blocks += 1,
            TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::End if nested_blocks == 0 => return Some(token.range.start),
            TokenKind::End => nested_blocks -= 1,
            TokenKind::Eof => return None,
            _ => {}
        }
    }
    None
}

fn parameter_names(tokens: &[Token], start: usize, body_start: usize) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut depth = 0usize;
    let mut expect_parameter_name = false;
    for token in tokens[start..]
        .iter()
        .take_while(|token| token.range.start < body_start)
    {
        match token.kind {
            TokenKind::LParen => {
                depth += 1;
                if depth == 1 {
                    expect_parameter_name = true;
                }
            }
            TokenKind::RParen => {
                depth = depth.saturating_sub(1);
                expect_parameter_name = false;
            }
            TokenKind::Comma if depth == 1 => expect_parameter_name = true,
            TokenKind::Ident if depth == 1 && expect_parameter_name => {
                names.insert(token.text.clone());
                expect_parameter_name = false;
            }
            token_kind if !is_layout_token_kind(token_kind) && depth == 1 => {
                expect_parameter_name = false;
            }
            _ => {}
        }
    }
    names
}

fn result_binding_name(tokens: &[Token], start: usize, body_start: usize) -> Option<String> {
    let arrow_index = tokens[start..]
        .iter()
        .position(|token| token.kind == TokenKind::Arrow)
        .map(|index| start + index)?;
    if tokens[arrow_index].range.start >= body_start {
        return None;
    }
    let candidate_index = next_non_layout_index(tokens, arrow_index)?;
    let candidate = &tokens[candidate_index];
    if candidate.kind != TokenKind::Ident || !is_identifier(&candidate.text) {
        return None;
    }
    next_non_layout_token(tokens, candidate_index)
        .is_some_and(|next| next.kind == TokenKind::Colon)
        .then(|| candidate.text.clone())
}

fn local_bindings(tokens: &[Token], body_start: usize, end: usize) -> Vec<LocalBinding> {
    let mut bindings = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.range.start < body_start
            || token.range.start >= end
            || token.kind != TokenKind::Let
        {
            continue;
        }
        let binding_end = local_binding_scope_end(tokens, index, end);
        bindings.extend(
            let_pattern_binding_names(tokens, index)
                .into_iter()
                .map(|(name, _)| LocalBinding {
                    name,
                    start: let_binding_scope_start(tokens, index),
                    end: binding_end,
                }),
        );
    }
    bindings.extend(match_arm_pattern_binding_names(tokens, body_start, end));
    bindings.extend(satisfy_candidate_binding_names(tokens, body_start, end));
    bindings
}

fn let_binding_scope_start(tokens: &[Token], let_index: usize) -> usize {
    tokens[let_index + 1..]
        .iter()
        .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .last()
        .map(|token| token.range.end)
        .unwrap_or_else(|| tokens[let_index].range.end)
}

fn local_binding_scope_end(tokens: &[Token], let_index: usize, function_end: usize) -> usize {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[let_index + 1..].iter().enumerate() {
        let index = let_index + 1 + relative_index;
        if token.range.start >= function_end {
            break;
        }
        match token.kind {
            TokenKind::If if !is_else_if(tokens, index) => nested_blocks += 1,
            TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::Else if nested_blocks == 0 => return token.range.start,
            TokenKind::End if nested_blocks == 0 => return token.range.start,
            TokenKind::End => nested_blocks -= 1,
            _ => {}
        }
    }
    function_end
}

fn local_binding_shadows_name(
    tokens: &[Token],
    name: &str,
    offset: usize,
    scope_start: usize,
    scope_end: usize,
) -> bool {
    local_bindings(tokens, scope_start, scope_end)
        .iter()
        .any(|binding| binding.name == name && offset >= binding.start && offset < binding.end)
}

fn handler_operation_clause_parameter_shadows_name(
    tokens: &[Token],
    name: &str,
    offset: usize,
    scope_start: usize,
    scope_end: usize,
) -> bool {
    if offset < scope_start || offset >= scope_end {
        return false;
    }
    let file_end = tokens.last().map_or(scope_end, |token| token.range.end);
    tokens.iter().enumerate().any(|(arrow_index, arrow)| {
        if arrow.kind != TokenKind::FatArrow
            || !is_handler_operation_clause_arrow(tokens, arrow_index)
        {
            return false;
        }
        let Some((lparen_index, rparen_index)) =
            handler_operation_clause_parameter_range(tokens, arrow_index)
        else {
            return false;
        };
        let body_end = handler_operation_clause_body_end(tokens, arrow_index, file_end);
        offset >= tokens[lparen_index].range.start
            && offset < body_end
            && handler_operation_clause_parameter_names_in_range(tokens, lparen_index, rparen_index)
                .contains(name)
    })
}

fn handler_operation_clause_parameter_range(
    tokens: &[Token],
    arrow_index: usize,
) -> Option<(usize, usize)> {
    let lparen_index = tokens[..arrow_index]
        .iter()
        .rposition(|token| token.kind == TokenKind::LParen)?;
    let rparen_index = tokens[lparen_index + 1..arrow_index]
        .iter()
        .position(|token| token.kind == TokenKind::RParen)
        .map(|index| lparen_index + 1 + index)?;
    Some((lparen_index, rparen_index))
}

fn handler_operation_clause_parameter_names_in_range(
    tokens: &[Token],
    lparen_index: usize,
    rparen_index: usize,
) -> BTreeSet<String> {
    tokens[lparen_index + 1..rparen_index]
        .iter()
        .filter(|token| token.kind == TokenKind::Ident && is_identifier(&token.text))
        .map(|token| token.text.clone())
        .collect()
}

fn let_pattern_binding_names(tokens: &[Token], let_index: usize) -> Vec<(String, usize)> {
    let mut names = Vec::new();
    let mut depth = 0usize;
    let mut index = let_index + 1;
    while index < tokens.len() {
        let token = &tokens[index];
        if token.kind == TokenKind::Eof || token.kind == TokenKind::Newline {
            break;
        }
        if depth == 0 && matches!(token.kind, TokenKind::Colon | TokenKind::Equal) {
            break;
        }
        match token.kind {
            TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
            TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => {
                depth = depth.saturating_sub(1);
            }
            TokenKind::Ident if is_pattern_binding_token(tokens, index) => {
                names.push((token.text.clone(), token.range.end));
            }
            _ => {}
        }
        index += 1;
    }
    names
}

fn match_arm_pattern_binding_names(
    tokens: &[Token],
    body_start: usize,
    function_end: usize,
) -> Vec<LocalBinding> {
    let mut bindings = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.range.start < body_start
            || token.range.start >= function_end
            || token.kind != TokenKind::FatArrow
            || !inside_match(tokens, index, body_start)
        {
            continue;
        }
        let scope_start = token.range.end;
        let scope_end = match_arm_scope_end(tokens, index + 1, function_end);
        let pattern_start = match_arm_pattern_start(tokens, index, body_start);
        for name in pattern_binding_names_in_range(tokens, pattern_start, index) {
            bindings.push(LocalBinding {
                name,
                start: scope_start,
                end: scope_end,
            });
        }
    }
    bindings
}

fn satisfy_candidate_binding_names(
    tokens: &[Token],
    body_start: usize,
    function_end: usize,
) -> Vec<LocalBinding> {
    let mut bindings = Vec::new();
    for (index, token) in tokens.iter().enumerate() {
        if token.range.start < body_start
            || token.range.start >= function_end
            || token.kind != TokenKind::Ident
            || token.text != "satisfy"
        {
            continue;
        }
        let Some(candidate_index) = next_non_layout_index(tokens, index) else {
            continue;
        };
        let candidate = &tokens[candidate_index];
        if candidate.kind != TokenKind::Ident || !is_identifier(&candidate.text) {
            continue;
        }
        let Some(arrow_index) = next_non_layout_index(tokens, candidate_index) else {
            continue;
        };
        if tokens[arrow_index].kind != TokenKind::FatArrow {
            continue;
        }
        let end = tokens[arrow_index + 1..]
            .iter()
            .find(|token| token.kind == TokenKind::Newline || token.range.start >= function_end)
            .map(|token| token.range.start)
            .unwrap_or(function_end);
        bindings.push(LocalBinding {
            name: candidate.text.clone(),
            start: tokens[arrow_index].range.end,
            end,
        });
    }
    bindings
}

fn inside_match(tokens: &[Token], index: usize, body_start: usize) -> bool {
    let mut nested_blocks = 0usize;
    for token in tokens[..index]
        .iter()
        .rev()
        .take_while(|token| token.range.start >= body_start)
    {
        match token.kind {
            TokenKind::End => nested_blocks += 1,
            TokenKind::Match if nested_blocks == 0 => return true,
            TokenKind::If | TokenKind::Handler | TokenKind::Match => {
                nested_blocks = nested_blocks.saturating_sub(1);
            }
            _ => {}
        }
    }
    false
}

fn match_arm_scope_end(tokens: &[Token], start: usize, function_end: usize) -> usize {
    let mut nested_blocks = 0usize;
    for (relative_index, token) in tokens[start..].iter().enumerate() {
        let index = start + relative_index;
        if token.range.start >= function_end {
            break;
        }
        match token.kind {
            TokenKind::If | TokenKind::Match | TokenKind::Handler => nested_blocks += 1,
            TokenKind::End if nested_blocks == 0 => return token.range.start,
            TokenKind::End => nested_blocks -= 1,
            TokenKind::FatArrow if nested_blocks == 0 && !is_satisfy_arrow(tokens, index) => {
                return match_arm_pattern_start_from_arrow(tokens, token.range.start);
            }
            _ => {}
        }
    }
    function_end
}

fn match_arm_pattern_start(tokens: &[Token], arrow_index: usize, body_start: usize) -> usize {
    tokens[..arrow_index]
        .iter()
        .rev()
        .take_while(|token| token.range.start >= body_start)
        .find(|token| token.kind == TokenKind::Newline || token.kind == TokenKind::Match)
        .map_or(body_start, |token| token.range.end)
}

fn match_arm_pattern_start_from_arrow(tokens: &[Token], arrow_start: usize) -> usize {
    tokens
        .iter()
        .position(|token| token.range.start == arrow_start)
        .map_or(arrow_start, |index| {
            match_arm_pattern_start(tokens, index, 0)
        })
}

fn pattern_binding_names_in_range(tokens: &[Token], start: usize, end_index: usize) -> Vec<String> {
    tokens[..end_index]
        .iter()
        .enumerate()
        .filter(|(_, token)| token.range.start >= start)
        .filter(|(index, token)| {
            token.kind == TokenKind::Ident && is_pattern_binding_token(tokens, *index)
        })
        .map(|(_, token)| token.text.clone())
        .collect()
}

fn is_pattern_binding_token(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    token.kind == TokenKind::Ident
        && is_identifier(&token.text)
        && token.text != "true"
        && token.text != "false"
        && previous_non_layout_token(tokens, index)
            .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
        && next_non_layout_token(tokens, index)
            .is_none_or(|next| !matches!(next.kind, TokenKind::DoubleColon | TokenKind::Colon))
}

fn is_else_if(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index)
        .is_some_and(|previous| previous.kind == TokenKind::Else)
}

fn token_scope(scopes: &[FunctionScope], offset: usize) -> Option<&FunctionScope> {
    scopes
        .iter()
        .find(|scope| offset >= scope.body_start && offset < scope.end)
}

fn is_function_declaration_name(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index)
        .is_some_and(|previous| matches!(previous.kind, TokenKind::Fn | TokenKind::Test))
}

fn is_parameter_name(tokens: &[Token], index: usize) -> bool {
    next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::Colon)
}

fn is_local_binding_name(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index).is_some_and(|previous| previous.kind == TokenKind::Let)
        || is_let_pattern_binding_name(tokens, index)
        || is_match_arm_pattern_binding_name(tokens, index)
        || is_satisfy_candidate_binding_name(tokens, index)
}

fn is_let_pattern_binding_name(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    if token.kind != TokenKind::Ident {
        return false;
    }
    let Some(let_index) = tokens[..index]
        .iter()
        .enumerate()
        .rev()
        .take_while(|(_, token)| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .find_map(|(previous_index, token)| {
            (token.kind == TokenKind::Let).then_some(previous_index)
        })
    else {
        return false;
    };
    let_pattern_binding_names(tokens, let_index)
        .iter()
        .any(|(name, start)| name == &token.text && *start == token.range.end)
}

fn is_match_arm_pattern_binding_name(tokens: &[Token], index: usize) -> bool {
    let token = &tokens[index];
    token.kind == TokenKind::Ident
        && tokens[index + 1..]
            .iter()
            .take_while(|next| next.kind != TokenKind::Newline && next.kind != TokenKind::Eof)
            .any(|next| next.kind == TokenKind::FatArrow)
        && is_pattern_binding_token(tokens, index)
}

fn is_satisfy_candidate_binding_name(tokens: &[Token], index: usize) -> bool {
    tokens[index].kind == TokenKind::Ident
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::Ident && previous.text == "satisfy")
        && next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::FatArrow)
}

fn is_satisfy_arrow(tokens: &[Token], index: usize) -> bool {
    let Some(candidate_index) = previous_non_layout_index(tokens, index) else {
        return false;
    };
    if tokens[candidate_index].kind != TokenKind::Ident {
        return false;
    }
    previous_non_layout_token(tokens, candidate_index)
        .is_some_and(|previous| previous.kind == TokenKind::Ident && previous.text == "satisfy")
}

fn is_field_name(tokens: &[Token], index: usize) -> bool {
    previous_non_layout_token(tokens, index).is_some_and(|previous| previous.kind == TokenKind::Dot)
        || next_non_layout_token(tokens, index).is_some_and(|next| next.kind == TokenKind::Colon)
}

fn is_ensure_reference(tokens: &[Token], index: usize) -> bool {
    tokens[..index]
        .iter()
        .rev()
        .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .any(|token| token.kind == TokenKind::Ensure)
}

fn is_function_alias_target_reference(tokens: &[Token], index: usize, name: &str) -> bool {
    tokens[index].text == name
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::Equal)
        && tokens[..index]
            .iter()
            .rev()
            .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
            .any(|token| token.kind == TokenKind::Fn)
}

fn is_codec_implementation_function_reference(tokens: &[Token], index: usize, name: &str) -> bool {
    tokens[index].text == name
        && previous_non_layout_token(tokens, index)
            .is_some_and(|previous| previous.kind == TokenKind::Ident && previous.text == "with")
        && inside_codec_declaration(tokens, index)
}

fn is_call_target_token(tokens: &[Token], index: usize) -> bool {
    next_non_whitespace_token(tokens, index).is_some_and(|next| next.kind == TokenKind::LParen)
}

fn is_handler_operation_clause_call_target(tokens: &[Token], index: usize) -> bool {
    is_call_target_token(tokens, index)
        && inside_handler_operation_clause_body(tokens, tokens[index].range.start)
}

fn is_handler_operation_clause_operation_name(tokens: &[Token], index: usize) -> bool {
    tokens
        .get(index)
        .is_some_and(|token| token.kind == TokenKind::Ident && is_identifier(&token.text))
        && tokens[index + 1..]
            .iter()
            .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
            .position(|token| token.kind == TokenKind::FatArrow)
            .map(|relative_index| index + 1 + relative_index)
            .is_some_and(|arrow_index| {
                is_handler_operation_clause_arrow(tokens, arrow_index)
                    && line_tokens_before(tokens, arrow_index)
                        .iter()
                        .position(|token| {
                            !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline)
                        })
                        .is_some_and(|first_index| {
                            let line_start = line_start_index(tokens, arrow_index);
                            line_start + first_index == index
                        })
            })
}

fn inside_handler_operation_clause_body(tokens: &[Token], offset: usize) -> bool {
    let file_end = tokens.last().map_or(offset, |token| token.range.end);
    tokens.iter().enumerate().any(|(arrow_index, arrow)| {
        arrow.kind == TokenKind::FatArrow
            && is_handler_operation_clause_arrow(tokens, arrow_index)
            && offset >= arrow.range.end
            && offset < handler_operation_clause_body_end(tokens, arrow_index, file_end)
    })
}

fn is_handler_operation_clause_arrow(tokens: &[Token], arrow_index: usize) -> bool {
    if !inside_top_level_block(tokens, arrow_index, TokenKind::Handler) {
        return false;
    }
    let line_tokens = line_tokens_before(tokens, arrow_index);
    line_tokens
        .iter()
        .find(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Newline))
        .is_some_and(|token| token.kind == TokenKind::Ident && is_identifier(&token.text))
        && line_tokens
            .iter()
            .any(|token| token.kind == TokenKind::LParen)
        && line_tokens
            .iter()
            .any(|token| token.kind == TokenKind::RParen)
}

fn line_tokens_before(tokens: &[Token], index: usize) -> &[Token] {
    &tokens[line_start_index(tokens, index)..index]
}

fn next_non_whitespace_token(tokens: &[Token], index: usize) -> Option<&Token> {
    tokens[index + 1..]
        .iter()
        .take_while(|token| token.kind != TokenKind::Newline && token.kind != TokenKind::Eof)
        .find(|token| token.kind != TokenKind::Whitespace)
}

fn inside_codec_declaration(tokens: &[Token], index: usize) -> bool {
    inside_top_level_block(tokens, index, TokenKind::Codec)
}

fn inside_top_level_block(tokens: &[Token], index: usize, start_kind: TokenKind) -> bool {
    enclosing_top_level_block_index(tokens, index, start_kind).is_some()
}

fn enclosing_top_level_block_index(
    tokens: &[Token],
    index: usize,
    start_kind: TokenKind,
) -> Option<usize> {
    let mut nested_blocks = 0usize;
    for (candidate_index, token) in tokens[..index].iter().enumerate().rev() {
        match token.kind {
            TokenKind::End => nested_blocks += 1,
            kind if kind == start_kind && nested_blocks == 0 => return Some(candidate_index),
            TokenKind::Fn
            | TokenKind::Test
            | TokenKind::If
            | TokenKind::Match
            | TokenKind::Handler
            | TokenKind::Codec => nested_blocks = nested_blocks.saturating_sub(1),
            _ => {}
        }
    }
    None
}

fn identifier_token_at(tokens: &[Token], offset: usize) -> Option<(usize, &Token)> {
    tokens.iter().enumerate().find(|(_, token)| {
        token.kind == TokenKind::Ident
            && offset >= token.range.start
            && offset < token.range.end
            && is_identifier(&token.text)
    })
}

fn qualifier_for_token(tokens: &[Token], name_index: usize) -> Option<String> {
    let separator_index = previous_non_layout_index(tokens, name_index)?;
    if tokens[separator_index].kind != TokenKind::DoubleColon {
        return None;
    }
    let segment_index = previous_non_layout_index(tokens, separator_index)?;
    let mut segments = vec![tokens[segment_index].text.as_str()];
    let mut cursor = segment_index;
    while let Some(previous_separator) = previous_non_layout_index(tokens, cursor) {
        if tokens[previous_separator].kind != TokenKind::DoubleColon {
            break;
        }
        let Some(previous_segment) = previous_non_layout_index(tokens, previous_separator) else {
            break;
        };
        segments.push(tokens[previous_segment].text.as_str());
        cursor = previous_segment;
    }
    segments.reverse();
    Some(segments.join("::"))
}

fn qualified_reference_matches(
    tokens: &[Token],
    name_index: usize,
    module_segments: &[&str],
) -> bool {
    let mut expected_index = name_index;
    for expected_segment in module_segments.iter().rev() {
        let Some(separator_index) = previous_non_layout_index(tokens, expected_index) else {
            return false;
        };
        if tokens[separator_index].kind != TokenKind::DoubleColon {
            return false;
        }
        let Some(segment_index) = previous_non_layout_index(tokens, separator_index) else {
            return false;
        };
        if tokens[segment_index].text != *expected_segment {
            return false;
        }
        expected_index = segment_index;
    }
    previous_non_layout_token(tokens, expected_index)
        .is_none_or(|previous| previous.kind != TokenKind::DoubleColon)
}

fn next_non_layout_token(tokens: &[Token], index: usize) -> Option<&Token> {
    next_non_layout_index(tokens, index).map(|index| &tokens[index])
}

fn next_non_layout_index(tokens: &[Token], index: usize) -> Option<usize> {
    tokens[index + 1..]
        .iter()
        .position(|token| !is_layout_token(token))
        .map(|relative_index| index + 1 + relative_index)
}

fn previous_non_layout_token(tokens: &[Token], index: usize) -> Option<&Token> {
    let previous = previous_non_layout_index(tokens, index)?;
    Some(&tokens[previous])
}

fn previous_non_layout_index(tokens: &[Token], index: usize) -> Option<usize> {
    tokens[..index]
        .iter()
        .enumerate()
        .rev()
        .find(|(_, token)| !is_layout_token(token))
        .map(|(index, _)| index)
}

fn is_layout_token(token: &Token) -> bool {
    is_layout_token_kind(token.kind)
}

fn is_layout_token_kind(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Whitespace | TokenKind::Newline)
}

fn explicit_module_name(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let rest = line.trim_start().strip_prefix("mod ")?;
        leading_module_path(rest).map(str::to_string)
    })
}

fn module_name_from_path(path: &str) -> Option<String> {
    Some(path.strip_suffix(".veln")?.replace('/', "::"))
}

fn use_modules(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter_map(|line| {
            let rest = line.trim_start().strip_prefix("use ")?;
            leading_module_path(rest).map(str::to_string)
        })
        .collect()
}

fn leading_module_path(input: &str) -> Option<&str> {
    let end = input
        .char_indices()
        .take_while(|(_, ch)| is_identifier_char(*ch) || *ch == ':')
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    Some(&input[..end])
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars.next().is_some_and(is_identifier_start) && chars.all(is_identifier_char)
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_identifier_char(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

fn offset_for_position(text: &str, position: Position) -> Option<usize> {
    let line_start = line_start_offset(text, position.line)?;
    let line = text[line_start..]
        .split_once('\n')
        .map_or(&text[line_start..], |(line, _)| line);
    let offset = line
        .char_indices()
        .nth(position.character)
        .map(|(index, _)| line_start + index)
        .unwrap_or(line_start + line.len());
    Some(offset)
}

fn line_start_offset(text: &str, zero_based_line: usize) -> Option<usize> {
    if zero_based_line == 0 {
        return Some(0);
    }
    let mut line = 0;
    for (index, ch) in text.char_indices() {
        if ch == '\n' {
            line += 1;
            if line == zero_based_line {
                return Some(index + 1);
            }
        }
    }
    None
}

fn path_to_uri(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    format!("file://{}", percent_encode_uri_path(&path))
}

fn percent_encode_uri_path(path: &str) -> String {
    let mut encoded = String::new();
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                encoded.push(byte as char)
            }
            byte => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn read_message(input: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut content_length = None;
    loop {
        let mut line = String::new();
        if input.read_line(&mut line)? == 0 {
            return Ok(None);
        }
        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse::<usize>().ok();
        }
    }
    let Some(content_length) = content_length else {
        return Ok(None);
    };
    let mut body = vec![0; content_length];
    input.read_exact(&mut body)?;
    Ok(Some(String::from_utf8_lossy(&body).into_owned()))
}

fn write_message(output: &mut impl Write, body: &str) -> io::Result<()> {
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    output.flush()
}

fn response(id: &str, result: &str) -> String {
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}")
}

fn error_response(id: &str, code: i32, message: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":{code},\"message\":\"{}\"}}}}",
        escape_json(message)
    )
}

fn extract_id(message: &str) -> Option<String> {
    let key = "\"id\"";
    let index = message.find(key)?;
    let after_key = &message[index + key.len()..];
    let after_colon = after_key[after_key.find(':')? + 1..].trim_start();
    if after_colon.starts_with('"') {
        let value = parse_json_string(after_colon)?;
        Some(format!("\"{}\"", escape_json(&value)))
    } else {
        let end = after_colon
            .find(|ch: char| !ch.is_ascii_digit() && ch != '-')
            .unwrap_or(after_colon.len());
        Some(after_colon[..end].to_string())
    }
}

fn extract_position(message: &str) -> Option<Position> {
    let position_index = message.find("\"position\"")?;
    let position = &message[position_index..];
    Some(Position {
        line: extract_usize_field(position, "line")?,
        character: extract_usize_field(position, "character")?,
    })
}

fn extract_usize_field(message: &str, field: &str) -> Option<usize> {
    let key = format!("\"{field}\"");
    let index = message.find(&key)?;
    let after_key = &message[index + key.len()..];
    let after_colon = after_key[after_key.find(':')? + 1..].trim_start();
    let end = after_colon
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(after_colon.len());
    after_colon[..end].parse().ok()
}

fn extract_string_field(message: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let index = message.find(&key)?;
    let after_key = &message[index + key.len()..];
    let after_colon = after_key[after_key.find(':')? + 1..].trim_start();
    parse_json_string(after_colon)
}

fn parse_json_string(input: &str) -> Option<String> {
    let mut chars = input.chars();
    if chars.next()? != '"' {
        return None;
    }
    let mut value = String::new();
    let mut escaped = false;
    while let Some(ch) = chars.next() {
        if escaped {
            match ch {
                '"' => value.push('"'),
                '\\' => value.push('\\'),
                '/' => value.push('/'),
                'b' => value.push('\u{0008}'),
                'f' => value.push('\u{000c}'),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                'u' => {
                    let code = chars.by_ref().take(4).collect::<String>();
                    let Ok(value_code) = u32::from_str_radix(&code, 16) else {
                        return None;
                    };
                    value.push(char::from_u32(value_code)?);
                }
                _ => return None,
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(value);
        } else {
            value.push(ch);
        }
    }
    None
}

fn json_string_list(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", escape_json(value)))
        .collect::<Vec<_>>()
        .join(",")
}

fn escape_json(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            ch => vec![ch],
        })
        .collect()
}

fn display_path(uri: &str) -> String {
    uri_to_path(uri)
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| uri.to_string())
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    Some(PathBuf::from(percent_decode(path)))
}

fn percent_decode(value: &str) -> String {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let code = chars.by_ref().take(2).collect::<String>();
            if let Ok(byte) = u8::from_str_radix(&code, 16) {
                output.push(byte as char);
            }
        } else {
            output.push(ch);
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use veln_diagnostics::{DiagnosticKind, JsonValue};

    #[test]
    fn legend_exposes_standard_types_and_custom_modifiers() {
        let legend = legend();

        assert!(legend.token_types.contains(&"function"));
        assert!(legend.token_types.contains(&"parameter"));
        assert!(legend.token_types.contains(&"namespace"));
        assert!(legend.token_modifiers.contains(&"declaration"));
        assert!(legend.token_modifiers.contains(&"defaultLibrary"));
        assert!(legend.token_modifiers.contains(&"test"));
        assert!(legend.token_modifiers.contains(&"result"));
        assert!(legend.token_modifiers.contains(&"hole"));
    }

    #[test]
    fn full_tokens_are_flat_lsp_integer_data() {
        let source = SourceFile::new("main.veln", "fn main() -> Int\n  main()\nend\n");

        let response = semantic_tokens_full(&source);

        assert_eq!(response.data.len() % 5, 0);
        assert!(response.data.len() >= 10);
    }

    #[test]
    fn server_returns_semantic_tokens_for_handler_clause_satisfy_body() {
        let mut server = Server::default();
        let project = TempProject::new("semantic-handler-satisfy-body");
        project.write(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick(value: Int) -> Int\n",
                "  fallback() -> Int\n",
                "end\n",
                "\n",
                "handler choose() handles Choose\n",
                "  pick(value) => _choice satisfy candidate => candidate == value\n",
                "  fallback() => 0\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&semantic_tokens_request(&main_uri));

        assert_eq!(responses.len(), 1);
        assert!(responses[0].contains(r#""id":2,"result":{"data":["#));
        assert!(!responses[0].contains(r#""data":[]"#), "{}", responses[0]);
    }

    #[test]
    fn server_initializes_with_semantic_token_capability() {
        let mut server = Server::default();
        let project = TempProject::new("initialize-empty-root");
        let root_uri = path_to_uri(&project.root);

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
        ));

        assert!(responses[0].contains(r#""semanticTokensProvider""#));
        assert!(responses[0].contains(r#""definitionProvider":true"#));
        assert!(responses[0].contains(r#""renameProvider":{"prepareProvider":true}"#));
        assert!(responses[0].contains(r#""tokenTypes":["namespace","type""#));
        assert_eq!(
            server.workspace_roots.as_slice(),
            std::slice::from_ref(&project.root)
        );
    }

    #[test]
    fn server_uses_anonymous_workspace_root_when_no_manifest_exists() {
        let mut server = Server::default();
        let project = TempProject::new("initialize-workspace-folder");
        let root_uri = path_to_uri(&project.root);

        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"fixture"}}]}}}}"#
        ));

        assert_eq!(
            server.workspace_roots.as_slice(),
            std::slice::from_ref(&project.root)
        );
    }

    #[test]
    fn server_initializes_all_workspace_roots_from_workspace_folders() {
        let mut server = Server::default();
        let alpha = TempProject::new("initialize-alpha-workspace-folder");
        let beta = TempProject::new("initialize-beta-workspace-folder");
        let alpha_uri = path_to_uri(&alpha.root);
        let beta_uri = path_to_uri(&beta.root);

        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{alpha_uri}","name":"alpha"}},{{"uri":"{beta_uri}","name":"beta"}}]}}}}"#
        ));

        let mut expected = vec![alpha.root.clone(), beta.root.clone()];
        expected.sort();
        assert_eq!(server.workspace_roots, expected);
    }

    #[test]
    fn server_stops_workspace_root_selection_at_manifest_root() {
        let mut server = Server::default();
        let workspace = TempProject::new("manifest-workspace-root");
        workspace.write("veln.toml", "[package]\nname = \"outer\"\n");
        workspace.write("nested/veln.toml", "[package]\nname = \"nested\"\n");
        workspace.write("nested/main.veln", "pub fn nested() -> Int\n  1\nend\n");
        let root_uri = path_to_uri(&workspace.root);

        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"outer"}}]}}}}"#
        ));

        assert_eq!(server.workspace_roots, vec![workspace.root.clone()]);
    }

    #[test]
    fn server_selects_first_manifest_root_on_each_workspace_branch() {
        let mut server = Server::default();
        let workspace = TempProject::new("manifest-roots-on-branches");
        workspace.write("alpha/package/veln.toml", "[package]\nname = \"alpha\"\n");
        workspace.write(
            "alpha/package/nested/veln.toml",
            "[package]\nname = \"alpha-nested\"\n",
        );
        workspace.write(
            "beta/deep/package/veln.toml",
            "[package]\nname = \"beta\"\n",
        );
        let root_uri = path_to_uri(&workspace.root);

        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"repo"}}]}}}}"#
        ));

        assert_eq!(
            server.workspace_roots,
            vec![
                workspace.root.join("alpha/package"),
                workspace.root.join("beta/deep/package"),
            ]
        );
    }

    #[test]
    fn server_keeps_explicit_outer_and_nested_workspace_projects() {
        let mut server = Server::default();
        let workspace = TempProject::new("explicit-outer-and-nested-roots");
        workspace.write("veln.toml", "[package]\nname = \"outer\"\n");
        workspace.write("nested/veln.toml", "[package]\nname = \"nested\"\n");
        let outer_uri = path_to_uri(&workspace.root);
        let nested_uri = path_to_uri(&workspace.root.join("nested"));

        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{outer_uri}","name":"outer"}},{{"uri":"{nested_uri}","name":"nested"}},{{"uri":"{nested_uri}","name":"nested-again"}}]}}}}"#
        ));

        assert_eq!(
            server.workspace_roots,
            vec![workspace.root.clone(), workspace.root.join("nested")]
        );
    }

    #[test]
    fn server_does_not_initialize_loaded_dependency_as_workspace_project() {
        let mut server = Server::default();
        let workspace = TempProject::new("dependency-workspace-isolation");
        workspace.write(
            "veln.toml",
            "[package]\nname = \"app\"\n\n[dependencies.\"example.com/lib\"]\npath = \"vendor/lib\"\n",
        );
        workspace.write(
            "app.veln",
            "use lib from \"example.com/lib\"\n\nfn main() -> Int\n  add_one(1)\nend\n",
        );
        workspace.write(
            "vendor/lib/veln.toml",
            "[package]\nname = \"example.com/lib\"\n\n[lib]\nexports = [\"lib.veln\"]\n",
        );
        workspace.write(
            "vendor/lib/lib.veln",
            "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
        );
        let root_uri = path_to_uri(&workspace.root);
        let app_uri = path_to_uri(&workspace.root.join("app.veln"));
        let dependency_uri = path_to_uri(&workspace.root.join("vendor/lib/lib.veln"));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"app"}}]}}}}"#
        ));

        assert_eq!(server.workspace_roots, vec![workspace.root.clone()]);
        let publish = publish_for_uri(&responses, &app_uri);
        assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
        assert!(!publish.contains("module.missing_identity"), "{publish}");
        assert!(
            responses
                .iter()
                .all(|response| !response.contains(&dependency_uri)),
            "dependency sources must not be published as a workspace project"
        );
    }

    #[cfg(unix)]
    #[test]
    fn server_deduplicates_workspace_folders_by_filesystem_identity() {
        use std::os::unix::fs::symlink;

        let mut server = Server::default();
        let workspace = TempProject::new("workspace-filesystem-identity");
        workspace.write("package/veln.toml", "[package]\nname = \"package\"\n");
        symlink(workspace.root.join("package"), workspace.root.join("alias"))
            .expect("workspace alias should be created");
        let package_uri = path_to_uri(&workspace.root.join("package"));
        let alias_uri = path_to_uri(&workspace.root.join("alias"));

        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{alias_uri}","name":"alias"}},{{"uri":"{package_uri}","name":"package"}}]}}}}"#
        ));

        assert_eq!(server.workspace_roots, vec![workspace.root.join("package")]);
    }

    #[cfg(unix)]
    #[test]
    fn server_does_not_follow_directory_symlinks_during_manifest_discovery() {
        use std::os::unix::fs::symlink;

        let mut server = Server::default();
        let workspace = TempProject::new("workspace-directory-symlink");
        workspace.write("folder/readme.txt", "workspace without a manifest\n");
        workspace.write("linked-package/veln.toml", "[package]\nname = \"linked\"\n");
        symlink(
            workspace.root.join("linked-package"),
            workspace.root.join("folder/package-link"),
        )
        .expect("directory symlink should be created");
        let folder = workspace.root.join("folder");
        let folder_uri = path_to_uri(&folder);

        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{folder_uri}","name":"folder"}}]}}}}"#
        ));

        assert_eq!(server.workspace_roots, vec![folder]);
    }

    #[test]
    fn server_excludes_git_directories_from_manifest_discovery() {
        let mut server = Server::default();
        let workspace = TempProject::new("workspace-git-exclusion");
        workspace.write(
            ".git/generated/veln.toml",
            "[package]\nname = \"generated\"\n",
        );
        let root_uri = path_to_uri(&workspace.root);

        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"repo"}}]}}}}"#
        ));

        assert_eq!(server.workspace_roots, vec![workspace.root.clone()]);
    }

    #[test]
    fn server_uses_nested_manifest_roots_from_workspace_folders() {
        let mut server = Server::default();
        let workspace = TempProject::new("nested-manifest-workspace-folder");
        workspace.write(
            "examples/specification/check/external-package-imports/veln.toml",
            "[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n",
        );
        workspace.write(
            "examples/specification/check/external-package-imports/app.veln",
            "use foo from \"github.com/oakcask/foo\"\n\nfn main() -> Int\n  add_one(1)\nend\n",
        );
        workspace.write(
            "examples/specification/check/external-package-imports/vendor/foo/veln.toml",
            "[package]\nname = \"github.com/oakcask/foo\"\n\n[lib]\nexports = [\"foo.veln\"]\n",
        );
        workspace.write(
            "examples/specification/check/external-package-imports/vendor/foo/foo.veln",
            "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
        );
        let root_uri = path_to_uri(&workspace.root);
        let app_uri = path_to_uri(
            &workspace
                .root
                .join("examples/specification/check/external-package-imports/app.veln"),
        );

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"repo"}}]}}}}"#
        ));

        assert_eq!(
            server.workspace_roots,
            vec![
                workspace
                    .root
                    .join("examples/specification/check/external-package-imports")
            ]
        );
        let publish = publish_for_uri(&responses, &app_uri);
        assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
        assert!(!publish.contains("module.missing_identity"), "{publish}");
    }

    #[test]
    fn server_uses_nested_manifest_roots_below_target_workspace_directories() {
        let mut server = Server::default();
        let workspace = TempProject::new("nested-manifest-target-workspace-folder");
        workspace.write(
            "target/generated-package/veln.toml",
            "[package]\nname = \"generated\"\n",
        );
        workspace.write(
            "target/generated-package/main.veln",
            "pub fn generated() -> Int\n  1\nend\n",
        );
        let root_uri = path_to_uri(&workspace.root);
        let generated_uri = path_to_uri(&workspace.root.join("target/generated-package/main.veln"));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"repo"}}]}}}}"#
        ));

        assert_eq!(
            server.workspace_roots,
            vec![workspace.root.join("target/generated-package")]
        );
        let publish = publish_for_uri(&responses, &generated_uri);
        assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
    }

    #[test]
    fn server_does_not_infer_workspace_root_without_client_identity() {
        let mut server = Server::default();

        let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"capabilities":{}}}"#,
        );

        assert_eq!(server.workspace_roots, Vec::<PathBuf>::new());
        assert_eq!(responses.len(), 1);
        assert!(responses[0].contains(r#""semanticTokensProvider""#));
    }

    #[test]
    fn server_publishes_unopened_workspace_file_diagnostics() {
        let mut server = Server::default();
        let project = TempProject::new("unopened-workspace-diagnostics");
        project.write("broken.veln", "fn broken() -> Int\n  missing\nend\n");
        let root_uri = path_to_uri(&project.root);
        let broken_uri = path_to_uri(&project.root.join("broken.veln"));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
        ));

        let publish = publish_for_uri(&responses, &broken_uri);
        assert!(publish.contains(r#""code":"name.unresolved""#), "{publish}");
    }

    #[test]
    fn server_uses_unsaved_workspace_text_over_disk_text() {
        let mut server = Server::default();
        let project = TempProject::new("unsaved-workspace-overlay");
        project.write("main.veln", "fn main() -> Int\n  missing\nend\n");
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
        ));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{main_uri}","text":"fn main() -> Int\n  1\nend\n"}}}}}}"#
        ));

        let publish = publish_for_uri(&responses, &main_uri);
        assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
        assert!(!publish.contains("name.unresolved"), "{publish}");
    }

    #[test]
    fn server_does_not_overlay_open_documents_owned_by_nested_manifest() {
        let mut server = Server::default();
        let project = TempProject::new("nested-open-document-overlay-boundary");
        project.write(
            "veln.toml",
            "[package]\nname = \"outer\"\n\n[lib]\nexports = [\"app.veln\", \"nested/hidden.veln\"]\n",
        );
        project.write("app.veln", "pub fn app() -> Int\n  1\nend\n");
        project.write("nested/veln.toml", "[package]\nname = \"nested\"\n");
        project.write("nested/hidden.veln", "pub fn hidden() -> Int\n  2\nend\n");
        let root_uri = path_to_uri(&project.root);
        let manifest_uri = path_to_uri(&project.root.join("veln.toml"));
        let app_uri = path_to_uri(&project.root.join("app.veln"));
        let nested_uri = path_to_uri(&project.root.join("nested/hidden.veln"));
        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"outer"}}]}}}}"#
        ));
        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{nested_uri}","text":"pub fn hidden() -> Int\n  2\nend\n"}}}}}}"#
        ));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{app_uri}","version":2}},"contentChanges":[{{"text":"pub fn app() -> Int\n  1\nend\n"}}]}}}}"#
        ));

        let publish = publish_for_uri(&responses, &manifest_uri);
        assert!(
            publish.contains(r#""code":"manifest.unselected_export""#),
            "{publish}"
        );
    }

    #[test]
    fn server_clears_stale_workspace_diagnostics_after_change() {
        let mut server = Server::default();
        let project = TempProject::new("workspace-diagnostics-change-clear");
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
        ));
        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{main_uri}","text":"fn main() -> Int\n  missing\nend\n"}}}}}}"#
        ));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didChange","params":{{"textDocument":{{"uri":"{main_uri}","version":2}},"contentChanges":[{{"text":"fn main() -> Int\n  1\nend\n"}}]}}}}"#
        ));

        let publish = publish_for_uri(&responses, &main_uri);
        assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
    }

    #[test]
    fn server_analysis_respects_manifest_boundaries_and_owned_target_sources() {
        let mut server = Server::default();
        let project = TempProject::new("manifest-boundary-analysis");
        project.write("veln.toml", "[package]\nname = \"outer\"\n");
        project.write("app.veln", "pub fn app() -> Int\n\t1\nend\n");
        project.write("target/owned.veln", "pub fn owned() -> Int\n\t2\nend\n");
        project.write("nested/veln.toml", "malformed nested manifest");
        project.write("nested/hidden.veln", "this source must not be parsed");
        let root_uri = path_to_uri(&project.root);
        let app_uri = path_to_uri(&project.root.join("app.veln"));
        let target_uri = path_to_uri(&project.root.join("target/owned.veln"));
        let nested_uri = path_to_uri(&project.root.join("nested/hidden.veln"));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{root_uri}","name":"outer"}}]}}}}"#
        ));

        assert!(publish_for_uri(&responses, &app_uri).contains(r#""diagnostics":[]"#));
        assert!(publish_for_uri(&responses, &target_uri).contains(r#""diagnostics":[]"#));
        assert!(
            responses
                .iter()
                .all(|response| !response.contains(&nested_uri)),
            "nested package source should not receive outer-project diagnostics"
        );
    }

    #[test]
    fn server_clears_workspace_diagnostics_when_file_leaves_discovery() {
        let mut server = Server::default();
        let project = TempProject::new("workspace-diagnostics-left-discovery");
        project.write("main.veln", "fn main() -> Int\n  missing\nend\n");
        let root_uri = path_to_uri(&project.root);
        let main_path = project.root.join("main.veln");
        let main_uri = path_to_uri(&main_path);
        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
        ));
        fs::remove_file(main_path).expect("fixture source should be removable");

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didClose","params":{{"textDocument":{{"uri":"{main_uri}"}}}}}}"#
        ));

        let publish = publish_for_uri(&responses, &main_uri);
        assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
    }

    #[test]
    fn server_reports_cross_file_workspace_diagnostics() {
        let mut server = Server::default();
        let project = TempProject::new("cross-file-workspace-diagnostics");
        project.write(
            "app.veln",
            "use math\n\nfn main() -> Int\n  double(\"bad\")\nend\n",
        );
        project.write(
            "math.veln",
            "pub fn double(value: Int) -> Int\n  value * 2\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let app_uri = path_to_uri(&project.root.join("app.veln"));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
        ));

        let publish = publish_for_uri(&responses, &app_uri);
        assert!(publish.contains(r#""code":"type.mismatch""#), "{publish}");
    }

    #[test]
    fn server_publishes_same_leaf_files_from_multiple_roots_separately() {
        let mut server = Server::default();
        let alpha = TempProject::new("same-leaf-alpha-root");
        let beta = TempProject::new("same-leaf-beta-root");
        alpha.write("main.veln", "pub fn main() -> Int\n  1\nend\n");
        beta.write("main.veln", "pub fn main() -> Int\n  \"bad\"\nend\n");
        let alpha_root_uri = path_to_uri(&alpha.root);
        let beta_root_uri = path_to_uri(&beta.root);
        let alpha_main_uri = path_to_uri(&alpha.root.join("main.veln"));
        let beta_main_uri = path_to_uri(&beta.root.join("main.veln"));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{alpha_root_uri}","name":"alpha"}},{{"uri":"{beta_root_uri}","name":"beta"}}]}}}}"#
        ));

        let alpha_publish = publish_for_uri(&responses, &alpha_main_uri);
        let beta_publish = publish_for_uri(&responses, &beta_main_uri);
        assert!(
            alpha_publish.contains(r#""diagnostics":[]"#),
            "{alpha_publish}"
        );
        assert!(
            beta_publish.contains(r#""code":"type.mismatch""#),
            "{beta_publish}"
        );
    }

    #[test]
    fn server_keeps_same_leaf_workspace_files_in_distinct_modules() {
        let mut server = Server::default();
        let project = TempProject::new("same-leaf-workspace-diagnostics");
        project.write(
            "app.veln",
            "use alpha::item\nuse beta::item\n\npub fn main() -> Int\n  alpha::item::value() + beta::item::value()\nend\n",
        );
        project.write("alpha/item.veln", "pub fn value() -> Int\n  1\nend\n");
        project.write("beta/item.veln", "pub fn value() -> Int\n  2\nend\n");
        let root_uri = path_to_uri(&project.root);
        let app_uri = path_to_uri(&project.root.join("app.veln"));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
        ));

        let publish = publish_for_uri(&responses, &app_uri);
        assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");
        assert!(
            !publish.contains("module.duplicate_source_path"),
            "{publish}"
        );
    }

    #[test]
    fn server_overlays_same_leaf_workspace_files_by_relative_path() {
        let mut server = Server::default();
        let project = TempProject::new("same-leaf-workspace-overlay");
        project.write(
            "app.veln",
            "use alpha::item\nuse beta::item\n\npub fn main() -> Int\n  alpha::item::value() + beta::item::value()\nend\n",
        );
        project.write("alpha/item.veln", "pub fn value() -> Int\n  1\nend\n");
        project.write("beta/item.veln", "pub fn value() -> Int\n  2\nend\n");
        let root_uri = path_to_uri(&project.root);
        let app_uri = path_to_uri(&project.root.join("app.veln"));
        let beta_uri = path_to_uri(&project.root.join("beta/item.veln"));
        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
        ));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{beta_uri}","text":"pub fn value() -> String\n  \"two\"\nend\n"}}}}}}"#
        ));

        let app_publish = publish_for_uri(&responses, &app_uri);
        let beta_publish = publish_for_uri(&responses, &beta_uri);
        assert!(
            app_publish.contains(r#""code":"type.mismatch""#),
            "{app_publish}"
        );
        assert!(
            beta_publish.contains(r#""diagnostics":[]"#),
            "{beta_publish}"
        );
        assert!(
            !app_publish.contains("module.duplicate_source_path"),
            "{app_publish}"
        );
    }

    #[test]
    fn companion_private_function_definition_returns_target_declaration() {
        let mut server = Server::default();
        let project = companion_private_function_project("definition");
        let root_uri = path_to_uri(&project.root);
        let math_uri = path_to_uri(&project.root.join("math.veln"));
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&definition_request(&companion_uri, 7, 10));

        assert_eq!(responses.len(), 1);
        assert!(responses[0].contains(&format!(r#""uri":"{}""#, escape_json(&math_uri))));
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":12}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_prepare_rename_returns_reference_leaf() {
        let mut server = Server::default();
        let project = companion_private_function_project("prepare-rename");
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&prepare_rename_request(&companion_uri, 7, 10));

        assert_eq!(responses.len(), 1);
        assert!(
            responses[0].contains(
                r#""result":{"start":{"line":7,"character":8},"end":{"line":7,"character":17}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_edits_target_and_matching_companion_references() {
        let mut server = Server::default();
        let project = companion_private_function_project("rename");
        let root_uri = path_to_uri(&project.root);
        let math_uri = path_to_uri(&project.root.join("math.veln"));
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 7, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert!(
            responses[0].contains(&format!(r#""{}":["#, escape_json(&math_uri))),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(&format!(r#""{}":["#, escape_json(&companion_uri))),
            "{}",
            responses[0]
        );
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":1,"character":2},"end":{"line":1,"character":11}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":11,"character":2"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_preserves_target_symbol_identity() {
        let mut server = Server::default();
        let project = TempProject::new("rename-target-identity");
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  increment(value)\n",
                "  increment\n",
                "end\n",
                "\n",
                "fn apply(increment: fn(Int) -> Int) -> Int\n",
                "  increment(1)\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            concat!(
                "use math\n",
                "\n",
                "test companion() -> Int\n",
                "  math::increment(1)\n",
                "  math::increment\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":1,"character":2},"end":{"line":1,"character":11}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":2,"character":2},"end":{"line":2,"character":11}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":4,"character":8"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":6,"character":2"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_skips_unrelated_text_and_qualified_calls() {
        let mut server = Server::default();
        let project = TempProject::new("rename-source-isolation");
        project.write(
            "math.veln",
            concat!(
                "use support\n",
                "\n",
                "fn increment(value: Int) -> Int\n",
                "  increment(value)\n",
                "  support::increment(value)\n",
                "  \"increment(1)\"\n",
                "  value\n",
                "end\n",
            ),
        );
        project.write(
            "support.veln",
            "pub fn increment(value: Int) -> Int\n  value\nend\n",
        );
        project.write(
            "math.test.veln",
            concat!(
                "use math\n",
                "\n",
                "fn increment(value: Int) -> Int\n",
                "  value\n",
                "end\n",
                "\n",
                "test companion() -> Int\n",
                "  math::increment(1)\n",
                "  increment(1)\n",
                "  math::increment\n",
                "  \"math::increment(2)\"\n",
                "  # math::increment(3)\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 7, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":2,"character":3},"end":{"line":2,"character":12}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":3,"character":2},"end":{"line":3,"character":11}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":7,"character":8},"end":{"line":7,"character":17}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":9,"character":8"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":4,"character":11"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":5,"character":3"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":10,"character":9"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":11,"character":10"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_keeps_target_references_after_nested_blocks() {
        let mut server = Server::default();
        let project = TempProject::new("rename-nested-target-blocks");
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn use_nested(value: Int) -> Int\n",
                "  if value > 0\n",
                "    increment(value)\n",
                "  else\n",
                "    0\n",
                "  end\n",
                "  increment(value)\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":6,"character":4},"end":{"line":6,"character":13}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":10,"character":2},"end":{"line":10,"character":11}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_skips_local_callable_bindings() {
        let mut server = Server::default();
        let project = TempProject::new("rename-local-callable-shadow");
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn apply(value: Int, identity: fn(Int) -> Int) -> Int\n",
                "  increment(value)\n",
                "  let increment = identity\n",
                "  increment(value)\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":5,"character":2},"end":{"line":5,"character":11}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":6,"character":6"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":7,"character":2"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_skips_unannotated_callable_parameter_shadow() {
        let mut server = Server::default();
        let project = TempProject::new("rename-unannotated-callable-shadow");
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn apply(value: Int, increment) -> Int\n",
                "  increment(value)\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 2);
        assert!(
            !responses[0].contains(r#""line":5,"character":2"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_limits_pattern_binding_shadow_to_branch() {
        let mut server = Server::default();
        let project = TempProject::new("rename-pattern-binding-shadow");
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn branch(value: Int, identity: fn(Int) -> Int) -> Int\n",
                "  if value > 0\n",
                "    let {callback: increment} = {callback: identity}\n",
                "    increment(value)\n",
                "  else\n",
                "    increment(value)\n",
                "  end\n",
                "  increment(value)\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
        assert!(
            !responses[0].contains(r#""line":6,"character":20"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":7,"character":4"#),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":9,"character":4},"end":{"line":9,"character":13}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":11,"character":2},"end":{"line":11,"character":11}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_skips_record_fields() {
        let mut server = Server::default();
        let project = TempProject::new("rename-record-field-isolation");
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn inspect(value: Int) -> Int\n",
                "  let record = {increment: value}\n",
                "  record.increment\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 2);
        assert!(
            !responses[0].contains(r#""line":5,"character":16"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":6,"character":9"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_keeps_same_named_let_initializer_reference() {
        let mut server = Server::default();
        let project = TempProject::new("rename-let-initializer-shadow");
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn apply(value: Int) -> Int\n",
                "  let increment = increment\n",
                "  increment(value)\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":5,"character":18},"end":{"line":5,"character":27}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":5,"character":6"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":6,"character":2"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_skips_match_arm_pattern_bindings() {
        let mut server = Server::default();
        let project = TempProject::new("rename-match-arm-pattern-shadow");
        project.write(
            "math.veln",
            concat!(
                "type Choice\n",
                "  Use {callback: fn(Int) -> Int}\n",
                "  Skip\n",
                "end\n",
                "\n",
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn choose(choice: Choice, value: Int) -> Int\n",
                "  match choice\n",
                "    Use {callback: increment} => increment(value)\n",
                "    Skip => increment(value)\n",
                "  end\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
        assert!(
            !responses[0].contains(r#""line":11,"character":19"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":11,"character":33"#),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":12,"character":12},"end":{"line":12,"character":21}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_keeps_target_references_after_else_if() {
        let mut server = Server::default();
        let project = TempProject::new("rename-else-if-target-blocks");
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn choose(value: Int) -> Int\n",
                "  if value == 0\n",
                "    0\n",
                "  else if value == 1\n",
                "    increment(value)\n",
                "  else\n",
                "    2\n",
                "  end\n",
                "  increment(value)\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":8,"character":4},"end":{"line":8,"character":13}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":12,"character":2},"end":{"line":12,"character":11}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_rejects_suffix_qualified_references() {
        let mut server = Server::default();
        let project = TempProject::new("rename-qualified-path-boundary");
        project.write(
            "math.veln",
            "fn increment(value: Int) -> Int\n  value + 1\nend\n",
        );
        project.write(
            "other/math.veln",
            "pub fn increment(value: Int) -> Int\n  value\nend\n",
        );
        project.write(
            "math.test.veln",
            concat!(
                "use math\n",
                "use other::math\n",
                "\n",
                "test companion() -> Int\n",
                "  math::increment(1)\n",
                "  other::math::increment(1)\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 4, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 2);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":4,"character":8},"end":{"line":4,"character":17}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":5,"character":15"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_skips_result_binding_contract_scope() {
        let mut server = Server::default();
        let project = TempProject::new("rename-result-binding-isolation");
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> increment: Int\n",
                "  ensure increment >= value\n",
                "  increment(value)\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
        assert!(
            !responses[0].contains(r#""line":0,"character":28"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":1,"character":9"#),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":2,"character":2},"end":{"line":2,"character":11}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_skips_satisfy_candidate_scope() {
        let mut server = Server::default();
        let project = TempProject::new("rename-satisfy-candidate-isolation");
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn choose(fallback: Int) -> Int\n",
                "  _choice satisfy increment => increment > 0\n",
                "  increment(fallback)\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
        assert!(
            !responses[0].contains(r#""line":5,"character":19"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":5,"character":32"#),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":6,"character":2},"end":{"line":6,"character":11}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_includes_handler_operation_clause_calls() {
        let mut server = Server::default();
        let project = TempProject::new("rename-handler-operation-clause-call");
        project.write(
            "math.veln",
            concat!(
                "effect Adjust\n",
                "  amount(value: Int) -> Int\n",
                "end\n",
                "\n",
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "handler adjust() handles Adjust\n",
                "  amount(value) => increment(value)\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":9,"character":19},"end":{"line":9,"character":28}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_from_multiline_clause_call_covers_clause_body_calls() {
        let mut server = Server::default();
        let project = TempProject::new("rename-handler-operation-clause-multiline-call");
        project.write(
            "math.veln",
            concat!(
                "effect Adjust\n",
                "  amount(value: Int) -> Int\n",
                "end\n",
                "\n",
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "handler adjust() handles Adjust\n",
                "  amount(value) => if value == 0\n",
                "    increment(value)\n",
                "  else\n",
                "    increment(value + 1)\n",
                "  end\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("math.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&main_uri, 10, 6, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 4);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":10,"character":4},"end":{"line":10,"character":13}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":12,"character":4},"end":{"line":12,"character":13}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn handler_operation_clause_binding_rename_skips_record_fields() {
        let mut server = Server::default();
        let project = TempProject::new("rename-handler-operation-clause-field-isolation");
        project.write(
            "main.veln",
            concat!(
                "effect Adjust\n",
                "  amount(value: Int) -> Int\n",
                "end\n",
                "\n",
                "handler adjust() handles Adjust\n",
                "  amount(value) => { value: value, other: 1 }.value + value\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&main_uri, 5, 10, "amount_value"));

        assert_eq!(responses.len(), 1);
        assert_eq!(
            responses[0].matches(r#""newText":"amount_value""#).count(),
            3
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":5,"character":9},"end":{"line":5,"character":14}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":5,"character":28},"end":{"line":5,"character":33}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":5,"character":54},"end":{"line":5,"character":59}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":5,"character":21"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":5,"character":46"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn handler_operation_clause_binding_rename_covers_multiline_body_references() {
        let mut server = Server::default();
        let project = TempProject::new("rename-handler-operation-clause-multiline-body");
        project.write(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick(value: Bool) -> Int\n",
                "end\n",
                "\n",
                "handler choose() handles Choose\n",
                "  pick(value) => match value\n",
                "    true => value\n",
                "    value => value\n",
                "    false => value\n",
                "  end\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&main_uri, 5, 8, "input"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"input""#).count(), 4);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":6,"character":12},"end":{"line":6,"character":17}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":8,"character":13},"end":{"line":8,"character":18}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":7,"character":4"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":7,"character":13"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn handler_operation_clause_binding_rename_keeps_else_if_body_scope_bounded() {
        let mut server = Server::default();
        let project = TempProject::new("rename-handler-operation-clause-else-if-body");
        project.write(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick(value: Int) -> Int\n",
                "  fallback(value: Int) -> Int\n",
                "end\n",
                "\n",
                "handler choose() handles Choose\n",
                "  pick(value) => if value == 0\n",
                "    value\n",
                "  else if value == 1\n",
                "    value\n",
                "  else\n",
                "    value\n",
                "  end\n",
                "  fallback(value) => value\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&main_uri, 6, 8, "input"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"input""#).count(), 6);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":8,"character":10},"end":{"line":8,"character":15}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":11,"character":4},"end":{"line":11,"character":9}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":13,"character":11"#),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":13,"character":21"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn handler_operation_clause_binding_definition_uses_multiline_body_scope() {
        let mut server = Server::default();
        let project = TempProject::new("definition-handler-operation-clause-multiline-body");
        project.write(
            "main.veln",
            concat!(
                "effect Choose\n",
                "  pick(value: Bool) -> Int\n",
                "end\n",
                "\n",
                "handler choose() handles Choose\n",
                "  pick(value) => match value\n",
                "    true => value\n",
                "    value => value\n",
                "    false => value\n",
                "  end\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&definition_request(&main_uri, 8, 15));

        assert_eq!(responses.len(), 1);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":5,"character":7},"end":{"line":5,"character":12}}"#
            ),
            "{}",
            responses[0]
        );
        let shadowed = server.handle_message(&definition_request(&main_uri, 7, 15));
        assert_eq!(shadowed.len(), 1);
        assert!(shadowed[0].contains(r#""result":null"#), "{}", shadowed[0]);
    }

    #[test]
    fn handler_context_callable_binding_shadows_top_level_function_in_clause_body() {
        let mut server = Server::default();
        let project = TempProject::new("handler-context-callable-binding");
        project.write(
            "main.veln",
            concat!(
                "fn callback(value: Int) -> Int\n",
                "  value\n",
                "end\n",
                "\n",
                "effect Adjust\n",
                "  amount(value: Int) -> Int\n",
                "  echo(value: Int) -> Int\n",
                "  reset(value: Int) -> Int\n",
                "end\n",
                "\n",
                "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
                "  amount(value) => callback(value)\n",
                "  echo(value) => callback(value) + callback(1)\n",
                "  reset(callback) => callback\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let definition = server.handle_message(&definition_request(&main_uri, 11, 21));
        let references = server.handle_message(&references_request(&main_uri, 10, 17));
        let context_rename = server.handle_message(&rename_request(&main_uri, 10, 17, "project"));
        let clause_rename = server.handle_message(&rename_request(&main_uri, 13, 8, "value"));

        assert_eq!(definition.len(), 1);
        assert!(
            definition[0].contains(
                r#""range":{"start":{"line":10,"character":15},"end":{"line":10,"character":23}}"#
            ),
            "{}",
            definition[0]
        );
        assert_eq!(references.len(), 1);
        assert!(
            references[0].contains(
                r#""range":{"start":{"line":11,"character":19},"end":{"line":11,"character":27}}"#
            ),
            "{}",
            references[0]
        );
        assert!(
            references[0].contains(
                r#""range":{"start":{"line":12,"character":17},"end":{"line":12,"character":25}}"#
            ),
            "{}",
            references[0]
        );
        assert!(
            references[0].contains(
                r#""range":{"start":{"line":12,"character":35},"end":{"line":12,"character":43}}"#
            ),
            "{}",
            references[0]
        );
        assert!(
            !references[0].contains(r#""line":0,"character":3"#),
            "{}",
            references[0]
        );
        assert!(
            !references[0].contains(r#""line":13,"character":7"#),
            "{}",
            references[0]
        );
        assert_eq!(context_rename.len(), 1);
        assert_eq!(
            context_rename[0].matches(r#""newText":"project""#).count(),
            4
        );
        assert!(
            context_rename[0].contains(
                r#""range":{"start":{"line":12,"character":17},"end":{"line":12,"character":25}}"#
            ),
            "{}",
            context_rename[0]
        );
        assert!(
            context_rename[0].contains(
                r#""range":{"start":{"line":12,"character":35},"end":{"line":12,"character":43}}"#
            ),
            "{}",
            context_rename[0]
        );
        assert!(
            !context_rename[0].contains(r#""line":13,"character":7"#),
            "{}",
            context_rename[0]
        );
        assert_eq!(clause_rename.len(), 1);
        assert_eq!(clause_rename[0].matches(r#""newText":"value""#).count(), 2);
        assert!(
            clause_rename[0].contains(
                r#""range":{"start":{"line":13,"character":8},"end":{"line":13,"character":16}}"#
            ),
            "{}",
            clause_rename[0]
        );
        assert!(
            clause_rename[0].contains(
                r#""range":{"start":{"line":13,"character":21},"end":{"line":13,"character":29}}"#
            ),
            "{}",
            clause_rename[0]
        );
    }

    #[test]
    fn handler_context_parameter_does_not_bind_same_named_operation_heading() {
        let mut server = Server::default();
        let project = TempProject::new("handler-context-operation-heading-isolation");
        project.write(
            "main.veln",
            concat!(
                "fn callback(value: Int) -> Int\n",
                "  value\n",
                "end\n",
                "\n",
                "effect Adjust\n",
                "  callback(value: Int) -> Int\n",
                "  amount(value: Int) -> Int\n",
                "end\n",
                "\n",
                "handler adjust(callback: fn(Int) -> Int) handles Adjust\n",
                "  callback(value) => callback(value)\n",
                "  amount(value) => callback(value)\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let definition = server.handle_message(&definition_request(&main_uri, 10, 4));
        let references = server.handle_message(&references_request(&main_uri, 10, 4));
        let rename = server.handle_message(&rename_request(&main_uri, 10, 4, "project"));
        let body_definition = server.handle_message(&definition_request(&main_uri, 10, 22));

        assert_eq!(definition.len(), 1);
        assert!(
            definition[0].contains(r#""result":null"#),
            "{}",
            definition[0]
        );
        assert_eq!(references.len(), 1);
        assert!(
            references[0].contains(r#""result":[]"#),
            "{}",
            references[0]
        );
        assert_eq!(rename.len(), 1);
        assert!(rename[0].contains(r#""changes":{}"#), "{}", rename[0]);
        assert_eq!(body_definition.len(), 1);
        assert!(
            body_definition[0].contains(
                r#""range":{"start":{"line":9,"character":15},"end":{"line":9,"character":23}}"#
            ),
            "{}",
            body_definition[0]
        );
    }

    #[test]
    fn companion_private_function_rename_includes_target_function_alias_target() {
        let mut server = Server::default();
        let project = TempProject::new("rename-function-alias-target");
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  value + 1\n",
                "end\n",
                "\n",
                "pub fn advance = increment\n",
            ),
        );
        project.write(
            "math.test.veln",
            "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "bump"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"bump""#).count(), 3);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":4,"character":17},"end":{"line":4,"character":26}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_lsp_rejects_companion_function_values_and_aliases() {
        let mut server = Server::default();
        let project = TempProject::new("reject-companion-function-values");
        project.write(
            "math.veln",
            "fn increment(value: Int) -> Int\n  value + 1\nend\n",
        );
        project.write(
            "math.test.veln",
            concat!(
                "use math\n",
                "\n",
                "pub fn expose = math::increment\n",
                "\n",
                "test companion() -> ()\n",
                "  let mapper: fn(Int) -> Int = math::increment\n",
                "  ()\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let alias_definition = server.handle_message(&definition_request(&companion_uri, 2, 23));
        let value_prepare = server.handle_message(&prepare_rename_request(&companion_uri, 5, 37));
        let value_rename = server.handle_message(&rename_request(&companion_uri, 5, 37, "bump"));

        assert!(
            alias_definition[0].contains(r#""result":null"#),
            "{}",
            alias_definition[0]
        );
        assert!(
            value_prepare[0].contains(r#""result":null"#),
            "{}",
            value_prepare[0]
        );
        assert!(
            value_rename[0].contains(r#""changes":{}"#),
            "{}",
            value_rename[0]
        );
    }

    #[test]
    fn companion_private_function_requests_ignore_comment_and_string_origins() {
        let mut server = Server::default();
        let project = TempProject::new("request-origin");
        project.write(
            "math.veln",
            "fn increment(value: Int) -> Int\n  value + 1\nend\n",
        );
        project.write(
            "math.test.veln",
            concat!(
                "use math\n",
                "\n",
                "test companion() -> Int\n",
                "  math::increment(1)\n",
                "  \"math::increment(2)\"\n",
                "  # math::increment(3)\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let string_definition = server.handle_message(&definition_request(&companion_uri, 4, 10));
        let comment_prepare = server.handle_message(&prepare_rename_request(&companion_uri, 5, 11));
        let comment_rename =
            server.handle_message(&rename_request(&companion_uri, 5, 11, "advance"));

        assert!(
            string_definition[0].contains(r#""result":null"#),
            "{}",
            string_definition[0]
        );
        assert!(
            comment_prepare[0].contains(r#""result":null"#),
            "{}",
            comment_prepare[0]
        );
        assert!(
            comment_rename[0].contains(r#""changes":{}"#),
            "{}",
            comment_rename[0]
        );
    }

    #[test]
    fn companion_private_function_lsp_rejects_other_private_boundaries() {
        let mut server = Server::default();
        let project = TempProject::new("rejected-private-boundaries");
        project.write(
            "math.veln",
            "use support\n\nfn increment(value: Int) -> Int\n  value + 1\nend\n",
        );
        project.write(
            "support.veln",
            "fn private_helper(value: Int) -> Int\n  value\nend\n",
        );
        project.write(
            "other.test.veln",
            "use math\n\ntest wrong() -> Int\n  math::increment(1)\nend\n",
        );
        project.write(
            "math_test.veln",
            "use math\n\ntest integration() -> Int\n  math::increment(1)\nend\n",
        );
        project.write(
            "math.test.veln",
            "use support\n\ntest transitive() -> Int\n  support::private_helper(1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        server.handle_message(&initialize_request(&root_uri));

        let wrong_uri = path_to_uri(&project.root.join("other.test.veln"));
        let integration_uri = path_to_uri(&project.root.join("math_test.veln"));
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));

        let wrong = server.handle_message(&definition_request(&wrong_uri, 3, 10));
        let integration = server.handle_message(&definition_request(&integration_uri, 3, 10));
        let transitive = server.handle_message(&definition_request(&companion_uri, 3, 13));

        assert!(wrong[0].contains(r#""result":null"#), "{}", wrong[0]);
        assert!(
            integration[0].contains(r#""result":null"#),
            "{}",
            integration[0]
        );
        assert!(
            transitive[0].contains(r#""result":null"#),
            "{}",
            transitive[0]
        );
    }

    #[test]
    fn companion_private_function_definition_uses_open_document_overlay() {
        let mut server = Server::default();
        let project = companion_private_function_project("definition-overlay");
        let root_uri = path_to_uri(&project.root);
        let math_uri = path_to_uri(&project.root.join("math.veln"));
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));
        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{math_uri}","text":"fn bump(value: Int) -> Int\n  value + 1\nend\n"}}}}}}"#
        ));
        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{companion_uri}","text":"use math\n\ntest bump_test() -> Int\n  math::bump(1)\nend\n"}}}}}}"#
        ));

        let responses = server.handle_message(&definition_request(&companion_uri, 3, 10));

        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":7}}"#
            ),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn companion_private_function_rename_uses_open_document_overlay() {
        let mut server = Server::default();
        let project = companion_private_function_project("rename-overlay");
        let root_uri = path_to_uri(&project.root);
        let math_uri = path_to_uri(&project.root.join("math.veln"));
        let companion_uri = path_to_uri(&project.root.join("math.test.veln"));
        server.handle_message(&initialize_request(&root_uri));
        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{math_uri}","text":"fn bump(value: Int) -> Int\n  bump(value)\nend\n"}}}}}}"#
        ));
        server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{companion_uri}","text":"use math\n\ntest bump_test() -> Int\n  math::bump(1)\n  math::bump\nend\n"}}}}}}"#
        ));

        let responses = server.handle_message(&rename_request(&companion_uri, 3, 10, "advance"));

        assert_eq!(responses.len(), 1);
        assert_eq!(responses[0].matches(r#""newText":"advance""#).count(), 3);
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":0,"character":3},"end":{"line":0,"character":7}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":1,"character":2},"end":{"line":1,"character":6}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            responses[0].contains(
                r#""range":{"start":{"line":3,"character":8},"end":{"line":3,"character":12}}"#
            ),
            "{}",
            responses[0]
        );
        assert!(
            !responses[0].contains(r#""line":4,"character":8"#),
            "{}",
            responses[0]
        );
    }

    #[test]
    fn server_returns_full_semantic_tokens_for_open_document() {
        let mut server = Server::default();
        server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"fn main() -> Int\n  main()\nend\n"}}}"#,
        );

        let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/semanticTokens/full","params":{"textDocument":{"uri":"file://main.veln"}}}"#,
        );

        assert_eq!(responses.len(), 1);
        assert!(responses[0].contains(r#""id":2"#));
        assert!(responses[0].contains(r#""data":["#));
    }

    #[test]
    fn server_publishes_parse_diagnostics_for_open_document() {
        let mut server = Server::default();

        let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"fn\n"}}}"#,
        );

        assert_eq!(responses.len(), 1);
        assert!(responses[0].contains(r#""method":"textDocument/publishDiagnostics""#));
        assert!(responses[0].contains(r#""source":"veln""#));
        assert!(responses[0].contains(r#""severity":1"#));
        assert!(responses[0].contains(r#""code":"parse."#));
    }

    #[test]
    fn lsp_diagnostic_wire_fields_are_stable() {
        let diagnostic = Diagnostic::new(
            "parse.expected_item",
            Severity::Error,
            DiagnosticKind::Parse,
            "expected a function or test declaration",
            Some(SourceSpan {
                file: veln_source::SourcePath::new("main.veln"),
                start: veln_source::LineCol {
                    line: 2,
                    column: 3,
                    offset: 4,
                },
                end: veln_source::LineCol {
                    line: 2,
                    column: 5,
                    offset: 6,
                },
            }),
            JsonValue::Null,
        );

        assert_eq!(
            lsp_diagnostic_json(&diagnostic),
            concat!(
                "{\"range\":{\"start\":{\"line\":1,\"character\":2},",
                "\"end\":{\"line\":1,\"character\":4}},\"severity\":1,",
                "\"code\":\"parse.expected_item\",\"source\":\"veln\",",
                "\"message\":\"expected a function or test declaration\"}"
            )
        );
    }

    #[test]
    fn server_publishes_semantic_diagnostics_after_parse_succeeds() {
        let mut server = Server::default();

        let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"pub fn main() -> ()\n  stdio::println(\"hello\")\nend\n"}}}"#,
        );

        assert_eq!(responses.len(), 1);
        assert!(responses[0].contains(r#""code":"effect.missing_public""#));
    }

    #[test]
    fn server_clears_diagnostics_for_closed_document() {
        let mut server = Server::default();
        server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{"textDocument":{"uri":"file://main.veln","text":"fn\n"}}}"#,
        );

        let responses = server.handle_message(
            r#"{"jsonrpc":"2.0","method":"textDocument/didClose","params":{"textDocument":{"uri":"file://main.veln"}}}"#,
        );

        assert_eq!(responses.len(), 1);
        assert!(responses[0].contains(r#""diagnostics":[]"#));
    }

    #[test]
    fn server_reads_and_writes_content_length_frames() {
        let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let input = format!("Content-Length: {}\r\n\r\n{request}", request.len());
        let mut output = Vec::new();
        let mut server = Server::default();

        server.run(input.as_bytes(), &mut output).unwrap();

        let output = String::from_utf8(output).unwrap();
        assert!(output.starts_with("Content-Length: "));
        assert!(output.contains(r#""id":1"#));
    }

    fn publish_for_uri<'a>(responses: &'a [String], uri: &str) -> &'a str {
        responses
            .iter()
            .find(|response| {
                response.contains(r#""method":"textDocument/publishDiagnostics""#)
                    && response.contains(&format!(r#""uri":"{}""#, escape_json(uri)))
            })
            .map(String::as_str)
            .unwrap_or_else(|| panic!("expected publish diagnostics for {uri}: {responses:#?}"))
    }

    fn companion_private_function_project(name: &str) -> TempProject {
        let project = TempProject::new(name);
        project.write(
            "math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  increment(value - 1)\n",
                "end\n",
            ),
        );
        project.write(
            "math.test.veln",
            concat!(
                "use math\n",
                "\n",
                "fn increment(value: Int) -> Int\n",
                "  value\n",
                "end\n",
                "\n",
                "test increment_test() -> Int\n",
                "  math::increment(1)\n",
                "end\n",
                "\n",
                "test local_increment_test() -> Int\n",
                "  increment(1)\n",
                "end\n",
            ),
        );
        project
    }

    fn initialize_request(root_uri: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
        )
    }

    fn definition_request(uri: &str, line: usize, character: usize) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}}}}}}"#
        )
    }

    fn references_request(uri: &str, line: usize, character: usize) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}},"context":{{"includeDeclaration":true}}}}}}"#
        )
    }

    fn prepare_rename_request(uri: &str, line: usize, character: usize) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}}}}}}"#
        )
    }

    fn rename_request(uri: &str, line: usize, character: usize, new_name: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/rename","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}},"newName":"{new_name}"}}}}"#
        )
    }

    fn semantic_tokens_request(uri: &str) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/semanticTokens/full","params":{{"textDocument":{{"uri":"{uri}"}}}}}}"#
        )
    }

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new(name: &str) -> Self {
            let root = env::temp_dir().join(format!(
                "veln-lsp-{name}-{}-{}",
                std::process::id(),
                unique_suffix()
            ));
            fs::create_dir_all(&root).expect("temp project should be created");
            Self { root }
        }

        fn write(&self, path: &str, contents: &str) {
            let path = self.root.join(path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("fixture parent should be created");
            }
            fs::write(path, contents).expect("fixture source should be written");
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn unique_suffix() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should produce a temp suffix")
            .as_nanos()
    }
}
