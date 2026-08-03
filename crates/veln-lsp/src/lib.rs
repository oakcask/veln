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
use veln_project::{Project, classify_companion_source};
use veln_source::{SourceFile, SourceSpan, TextRange};
use veln_syntax::parse;

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
                .and_then(|request| request.index.location_json(&request.symbol.declaration))
                .unwrap_or_else(|| "null".to_string());
            response(&id, &result)
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
                            .workspace_edit_json(&request.symbol, &new_name)
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
            let Some(source) = workspace_source_file(root, uri, text) else {
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
        workspace_relative_source_path(root, uri)
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
        "{{\"capabilities\":{{\"textDocumentSync\":1,\"definitionProvider\":true,\"renameProvider\":{{\"prepareProvider\":true}},\"semanticTokensProvider\":{{\"legend\":{{\"tokenTypes\":[{}],\"tokenModifiers\":[{}]}},\"full\":true,\"range\":false}}}}}}",
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
        .collect::<Vec<_>>();
    let mut roots = Vec::new();
    for root in client_roots {
        roots.extend(resolve_workspace_project_roots(&root));
    }
    if roots.is_empty()
        && let Some(root) =
            extract_string_field(message, "rootUri").and_then(|uri| uri_to_path(&uri))
    {
        roots.extend(resolve_workspace_project_roots(&root));
    }
    roots.sort();
    roots.dedup();
    roots
}

fn resolve_workspace_project_roots(root: &Path) -> Vec<PathBuf> {
    if root.join("veln.toml").is_file() {
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
        if file_name == ".git" || file_name == "target" || !path.is_dir() {
            continue;
        }
        if path.join("veln.toml").is_file() {
            roots.push(path);
        } else {
            collect_manifest_project_roots(&path, roots);
        }
    }
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

fn workspace_source_file(root: &Path, uri: &str, text: &str) -> Option<SourceFile> {
    workspace_relative_source_path(root, uri).map(|path| SourceFile::new(path, text.to_string()))
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
    symbol: FunctionSymbol,
    selection: SourceSpan,
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
        let functions = files
            .iter()
            .flat_map(|file| function_declarations(file))
            .collect();
        Self { files, functions }
    }

    fn symbol_at_position(self, source_path: &str, position: Position) -> Option<SymbolRequest> {
        let file = self
            .files
            .iter()
            .find(|file| file.source.path().as_str() == source_path)?;
        let offset = offset_for_position(file.source.text(), position)?;
        let selection = identifier_span_at(&file.source, offset)?;
        let name = file
            .source
            .text()
            .get(selection.start.offset..selection.end.offset)?
            .to_string();
        let symbol = self.symbol_for_selection(file, &name, &selection)?;
        Some(SymbolRequest {
            index: self,
            symbol,
            selection,
        })
    }

    fn symbol_for_selection(
        &self,
        file: &LspFile,
        name: &str,
        selection: &SourceSpan,
    ) -> Option<FunctionSymbol> {
        if let Some(symbol) = self.functions.iter().find(|symbol| {
            symbol.name == name
                && symbol.declaration.file == selection.file
                && symbol.declaration.start.offset == selection.start.offset
                && symbol.declaration.end.offset == selection.end.offset
        }) {
            return Some(symbol.clone());
        }

        let qualifier = qualifier_before(file.source.text(), selection.start.offset)?;
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
    let mut line_start = 0;
    for line in file.source.text().split_inclusive('\n') {
        let trimmed = line.trim_start();
        let leading = line.len() - trimmed.len();
        let head = trimmed.strip_prefix("fn ");
        if let Some(rest) = head
            && let Some(name) = leading_identifier(rest)
        {
            let start = line_start + leading + "fn ".len();
            let end = start + name.len();
            functions.push(FunctionSymbol {
                module: file.module.clone(),
                name: name.to_string(),
                declaration: file.source.span(TextRange::new(start, end)),
            });
        }
        line_start += line.len();
    }
    functions
}

fn call_references(source: &SourceFile, name: &str) -> Vec<SourceSpan> {
    identifier_occurrences(source, name)
        .into_iter()
        .filter(|span| following_non_space(source.text(), span.end.offset) == Some('('))
        .collect()
}

fn qualified_references(source: &SourceFile, module: &str, name: &str) -> Vec<SourceSpan> {
    let needle = format!("{module}::{name}");
    let mut spans = Vec::new();
    let mut start = 0;
    while let Some(relative) = source.text()[start..].find(&needle) {
        let qualified_start = start + relative;
        let name_start = qualified_start + module.len() + "::".len();
        let name_end = name_start + name.len();
        if identifier_boundary(source.text(), qualified_start, name_end)
            && following_non_space(source.text(), name_end) == Some('(')
        {
            spans.push(source.span(TextRange::new(name_start, name_end)));
        }
        start = name_end;
    }
    spans
}

fn identifier_occurrences(source: &SourceFile, name: &str) -> Vec<SourceSpan> {
    let mut spans = Vec::new();
    let mut start = 0;
    while let Some(relative) = source.text()[start..].find(name) {
        let name_start = start + relative;
        let name_end = name_start + name.len();
        if identifier_boundary(source.text(), name_start, name_end) {
            spans.push(source.span(TextRange::new(name_start, name_end)));
        }
        start = name_end;
    }
    spans
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

fn leading_identifier(input: &str) -> Option<&str> {
    let end = input
        .char_indices()
        .take_while(|(_, ch)| is_identifier_char(*ch))
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    Some(&input[..end])
}

fn leading_module_path(input: &str) -> Option<&str> {
    let end = input
        .char_indices()
        .take_while(|(_, ch)| is_identifier_char(*ch) || *ch == ':')
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    Some(&input[..end])
}

fn identifier_span_at(source: &SourceFile, offset: usize) -> Option<SourceSpan> {
    let text = source.text();
    let start = text[..offset]
        .char_indices()
        .rev()
        .find(|(_, ch)| !is_identifier_char(*ch))
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    let end = text[offset..]
        .char_indices()
        .find(|(_, ch)| !is_identifier_char(*ch))
        .map(|(index, _)| offset + index)
        .unwrap_or(text.len());
    (start < end).then(|| source.span(TextRange::new(start, end)))
}

fn qualifier_before(text: &str, name_start: usize) -> Option<String> {
    let before = text.get(..name_start)?;
    let qualifier_end = before.strip_suffix("::")?.len();
    let qualifier_start = before[..qualifier_end]
        .char_indices()
        .rev()
        .find(|(_, ch)| !is_identifier_char(*ch) && *ch != ':')
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    Some(before[qualifier_start..qualifier_end].to_string())
}

fn following_non_space(text: &str, offset: usize) -> Option<char> {
    text[offset..].chars().find(|ch| !ch.is_whitespace())
}

fn identifier_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(|ch| !is_identifier_char(ch))
        && after.is_none_or(|ch| !is_identifier_char(ch))
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
    fn server_initializes_workspace_root_from_workspace_folders() {
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
