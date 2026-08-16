//! LSP-facing semantic token helpers for Veln editors.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use veln_analysis::{
    DoctestMode, checked_project_diagnostics, parse_diagnostic_to_envelope,
    validate_manifest_exports,
};
use veln_ast::{SurfaceModule, lower_surface_ast};
use veln_diagnostics::{Diagnostic, Severity};
use veln_editor::{encode_lsp_semantic_tokens, semantic_token_legend};
use veln_language_service::{
    DirectDependencySnapshot, EffectiveProjectSnapshot, NavigationLocation, NavigationResult,
    NavigationSource, SourcePosition, navigate,
};
use veln_project::{
    PackageIdentity, PackageSnapshotSource, Project, ProjectManifest,
    capture_embedded_package_snapshot, capture_package_snapshot, discover_source_paths,
    parse_manifest_text,
};
use veln_source::{SourceFile, SourcePath, SourceSpan};
use veln_syntax::{format_tree, parse};

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
    workspace_root_aliases: BTreeMap<PathBuf, Vec<PathBuf>>,
    published_diagnostic_uris: BTreeSet<String>,
    project_snapshots: BTreeMap<PathBuf, EffectiveProjectSnapshot>,
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
            "veln/virtualDocument" => self.handle_virtual_document(message, id),
            "textDocument/references" => self.handle_references(message, id),
            "textDocument/formatting" => self.handle_formatting(message, id),
            "textDocument/prepareRename" => self.handle_prepare_rename(message, id),
            "textDocument/rename" => self.handle_rename(message, id),
            _ => self.handle_unknown_method(id),
        }
    }

    fn handle_initialize(&mut self, message: &str, id: Option<String>) -> Vec<String> {
        let selection = resolve_workspace_roots(message);
        self.workspace_roots = selection.roots;
        self.workspace_root_aliases = selection.aliases;
        self.capture_project_snapshots();
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
                .map(|request| location_json(&request.root, &request.result.definition))
                .unwrap_or_else(|| "null".to_string());
            response(&id, &result)
        })
        .into_iter()
        .collect()
    }

    fn handle_virtual_document(&self, message: &str, id: Option<String>) -> Vec<String> {
        let Some(id) = id else {
            return Vec::new();
        };
        let Some(uri) = extract_string_field(message, "uri") else {
            return vec![error_response(
                &id,
                -32602,
                "virtual document URI is required",
            )];
        };
        let Some(bytes) = self
            .project_snapshots
            .values()
            .find_map(|snapshot| snapshot.resolve_virtual_source(&uri))
        else {
            return vec![error_response(
                &id,
                -32602,
                "virtual document was not found",
            )];
        };
        let Ok(text) = std::str::from_utf8(bytes) else {
            return vec![error_response(&id, -32603, "virtual document is not UTF-8")];
        };
        vec![response(&id, &format!("\"{}\"", escape_json(text)))]
    }

    fn handle_references(&self, message: &str, id: Option<String>) -> Vec<String> {
        id.map(|id| {
            let result = self
                .symbol_at_request(message)
                .map(|request| {
                    references_json(
                        &request.root,
                        &request.result,
                        extract_bool_field(message, "includeDeclaration").unwrap_or(false),
                    )
                })
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
                .filter(|request| is_workspace_location(&request.result.definition))
                .map(|request| range_json(Some(&request.result.selection)))
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
                    self.symbol_at_request(message)
                        .filter(|request| is_workspace_location(&request.result.definition))
                        .map(|request| {
                            workspace_edit_json(&request.root, &request.result, &new_name)
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
                let uri = path_to_uri(
                    &visible_workspace_root(&root, &self.workspace_root_aliases).join(&source_path),
                );
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
            let Some(source) =
                owned_workspace_source_file(root, &self.workspace_root_aliases, uri, text)
            else {
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

    fn open_workspace_sources(&self, root: &Path) -> Vec<SourceFile> {
        self.documents
            .iter()
            .filter_map(|(uri, text)| {
                owned_workspace_source_file(root, &self.workspace_root_aliases, uri, text)
            })
            .collect()
    }

    fn workspace_source_path(&self, uri: &str) -> Option<String> {
        let document_root =
            workspace_root_for_uri(&self.workspace_roots, &self.workspace_root_aliases, uri)?;
        owned_workspace_relative_source_path(document_root.root, &document_root.relative)
    }

    fn capture_project_snapshots(&mut self) {
        self.project_snapshots.clear();
        for root in &self.workspace_roots {
            if let Some(snapshot) = retained_project_snapshot(root) {
                self.project_snapshots.insert(root.clone(), snapshot);
            }
        }
    }

    fn symbol_at_request(&self, message: &str) -> Option<NavigationRequest> {
        let uri = extract_string_field(message, "uri")?;
        let position = extract_position(message)?;
        let document_root =
            workspace_root_for_uri(&self.workspace_roots, &self.workspace_root_aliases, &uri)?;
        let root = document_root.root;
        let source_path = workspace_relative_source_path(&document_root.relative)?;
        let visible_root = visible_workspace_root(root, &self.workspace_root_aliases);
        let snapshot = self.project_snapshots.get(root)?;
        let overlays = self.open_workspace_sources(root);
        let overlaid_snapshot;
        let snapshot = if overlays.is_empty() {
            snapshot
        } else {
            overlaid_snapshot = snapshot.with_workspace_overlays(overlays);
            &overlaid_snapshot
        };
        let result = navigate(
            snapshot,
            SourcePosition {
                source: SourcePath::new(source_path),
                line: position.line.checked_add(1)?,
                column: position.character.checked_add(1)?,
            },
        )?;
        Some(NavigationRequest {
            root: visible_root.to_path_buf(),
            result,
        })
    }
}

fn retained_project_snapshot(root: &Path) -> Option<EffectiveProjectSnapshot> {
    let project = Project::discover(root.to_path_buf(), &[]).ok()?;
    let direct_dependencies = project
        .manifest
        .as_ref()
        .map(|manifest| retained_direct_dependencies(root, manifest))
        .unwrap_or_default();
    let standard_library = retained_standard_library()?;
    Some(
        EffectiveProjectSnapshot::with_direct_dependencies(project.files, direct_dependencies)
            .with_standard_library(standard_library),
    )
}

fn retained_standard_library() -> Option<DirectDependencySnapshot> {
    let bundle = veln_stdlib::package_bundle();
    let snapshot = capture_embedded_package_snapshot(
        bundle.manifest.as_bytes(),
        bundle
            .files
            .iter()
            .map(|file| PackageSnapshotSource::new(file.path, file.text.as_bytes())),
    )
    .ok()?;
    let manifest = parse_manifest_text("veln.toml", bundle.manifest);
    let project = Project {
        root: PathBuf::new(),
        files: snapshot
            .sources()
            .iter()
            .map(|source| {
                SourceFile::new(
                    source.path(),
                    std::str::from_utf8(source.bytes())
                        .expect("embedded standard package source text is valid UTF-8"),
                )
            })
            .collect(),
        manifest: Some(manifest),
    };
    if !validate_manifest_exports(&project).is_empty() {
        return None;
    }
    DirectDependencySnapshot::from_validated_standard_library(snapshot, project.manifest?).ok()
}

fn retained_direct_dependencies(
    root: &Path,
    manifest: &ProjectManifest,
) -> Vec<DirectDependencySnapshot> {
    manifest
        .dependencies
        .iter()
        .filter_map(|dependency| {
            let identity = PackageIdentity::new(&dependency.package).ok()?;
            let dependency_root = dependency
                .direct_analysis_source_root(root)
                .ok()
                .flatten()?;
            let snapshot = capture_package_snapshot(&dependency_root).ok()?;
            let manifest_text = std::str::from_utf8(snapshot.manifest_bytes()).ok()?;
            let dependency_manifest = parse_manifest_text("veln.toml", manifest_text);
            let dependency_project = Project {
                root: dependency_root,
                files: snapshot
                    .sources()
                    .iter()
                    .map(|source| {
                        SourceFile::new(
                            source.path(),
                            std::str::from_utf8(source.bytes())
                                .expect("captured package source text is valid UTF-8"),
                        )
                    })
                    .collect(),
                manifest: Some(dependency_manifest),
            };
            if !validate_manifest_exports(&dependency_project).is_empty() {
                return None;
            }
            let dependency_manifest = dependency_project.manifest?;
            DirectDependencySnapshot::from_validated_manifest(
                &identity,
                snapshot,
                dependency_manifest,
            )
            .ok()
        })
        .collect()
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

#[derive(Debug, Default)]
struct WorkspaceSelection {
    roots: Vec<PathBuf>,
    aliases: BTreeMap<PathBuf, Vec<PathBuf>>,
}

#[derive(Debug)]
struct WorkspaceFolderRoot {
    identity: PathBuf,
    visible: PathBuf,
}

#[derive(Debug)]
struct WorkspaceDocumentRoot<'a> {
    root: &'a Path,
    relative: PathBuf,
}

fn resolve_workspace_roots(message: &str) -> WorkspaceSelection {
    let client_roots = extract_workspace_folder_uris(message)
        .into_iter()
        .filter_map(|uri| uri_to_path(&uri))
        .filter_map(filesystem_workspace_root)
        .collect::<Vec<_>>();
    let mut selection = WorkspaceSelection::default();
    for root in client_roots {
        add_workspace_project_roots(&mut selection, &root);
    }
    if selection.roots.is_empty()
        && let Some(root) =
            extract_string_field(message, "rootUri").and_then(|uri| uri_to_path(&uri))
    {
        let Some(root) = filesystem_workspace_root(root) else {
            return selection;
        };
        add_workspace_project_roots(&mut selection, &root);
    }
    selection.roots.sort();
    selection.roots.dedup();
    for aliases in selection.aliases.values_mut() {
        aliases.sort();
        aliases.dedup();
    }
    selection
}

fn filesystem_workspace_root(root: PathBuf) -> Option<WorkspaceFolderRoot> {
    let visible = absolute_workspace_path(root);
    let identity = fs::canonicalize(&visible).ok()?;
    Some(WorkspaceFolderRoot { identity, visible })
}

fn absolute_workspace_path(root: PathBuf) -> PathBuf {
    let absolute = if root.is_absolute() {
        root
    } else {
        env::current_dir()
            .map(|current| current.join(&root))
            .unwrap_or(root)
    };
    absolute
        .components()
        .filter(|component| !matches!(component, std::path::Component::CurDir))
        .collect()
}

fn add_workspace_project_roots(selection: &mut WorkspaceSelection, folder: &WorkspaceFolderRoot) {
    for root in resolve_workspace_project_roots(&folder.identity) {
        let visible = folder
            .visible
            .join(root.strip_prefix(&folder.identity).unwrap_or(Path::new("")));
        selection.roots.push(root.clone());
        selection.aliases.entry(root).or_default().push(visible);
    }
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

fn visible_workspace_root<'a>(
    root: &'a Path,
    aliases: &'a BTreeMap<PathBuf, Vec<PathBuf>>,
) -> &'a Path {
    aliases
        .get(root)
        .and_then(|aliases| aliases.first())
        .map(PathBuf::as_path)
        .unwrap_or(root)
}

fn workspace_root_for_uri<'a>(
    roots: &'a [PathBuf],
    aliases: &'a BTreeMap<PathBuf, Vec<PathBuf>>,
    uri: &str,
) -> Option<WorkspaceDocumentRoot<'a>> {
    let uri_path = uri_to_path(uri)?;
    let absolute = if uri_path.is_absolute() {
        uri_path
    } else {
        env::current_dir().ok()?.join(uri_path)
    };
    let mut selected = None;
    let mut selected_depth = 0;
    for root in roots {
        for visible in std::iter::once(root.as_path()).chain(
            aliases
                .get(root)
                .into_iter()
                .flatten()
                .map(PathBuf::as_path),
        ) {
            let Ok(relative) = absolute.strip_prefix(visible) else {
                continue;
            };
            let depth = visible.components().count();
            if depth >= selected_depth {
                selected = Some(WorkspaceDocumentRoot {
                    root,
                    relative: relative.to_path_buf(),
                });
                selected_depth = depth;
            }
        }
    }
    selected
}

fn owned_workspace_source_file(
    root: &Path,
    aliases: &BTreeMap<PathBuf, Vec<PathBuf>>,
    uri: &str,
    text: &str,
) -> Option<SourceFile> {
    let root_buf = root.to_path_buf();
    let document_root = workspace_root_for_uri(std::slice::from_ref(&root_buf), aliases, uri)?;
    if document_root.root != root {
        return None;
    }
    owned_workspace_relative_source_path(root, &document_root.relative)
        .map(|path| SourceFile::new(path, text.to_string()))
}

fn owned_workspace_relative_source_path(root: &Path, relative: &Path) -> Option<String> {
    let relative = workspace_relative_source_path(relative)?;
    let input = PathBuf::from(&relative);
    discover_source_paths(root, std::slice::from_ref(&input)).ok()?;
    Some(relative)
}

fn workspace_relative_source_path(relative: &Path) -> Option<String> {
    if relative
        .extension()
        .is_none_or(|extension| extension != "veln")
    {
        return None;
    }
    Some(relative.to_string_lossy().replace('\\', "/"))
}

#[derive(Clone, Debug)]
struct Position {
    line: usize,
    character: usize,
}

#[derive(Debug)]
struct NavigationRequest {
    root: PathBuf,
    result: NavigationResult,
}

fn location_json(root: &Path, location: &NavigationLocation) -> String {
    let uri = match &location.source {
        NavigationSource::Workspace => path_to_uri(&root.join(location.span.file.as_str())),
        NavigationSource::Package { uri } => uri.clone(),
    };
    format!(
        "{{\"uri\":\"{}\",\"range\":{}}}",
        escape_json(&uri),
        range_json(Some(&location.span))
    )
}

fn is_workspace_location(location: &NavigationLocation) -> bool {
    matches!(location.source, NavigationSource::Workspace)
}

fn references_json(root: &Path, result: &NavigationResult, include_declaration: bool) -> String {
    if !is_workspace_location(&result.definition) {
        return "[]".to_string();
    }
    let mut locations = Vec::new();
    if include_declaration {
        locations.push(location_json(root, &result.definition));
    }
    locations.extend(result.references.iter().map(|span| {
        location_json(
            root,
            &NavigationLocation {
                source: NavigationSource::Workspace,
                span: span.clone(),
            },
        )
    }));
    format!("[{}]", locations.join(","))
}

fn workspace_edit_json(root: &Path, result: &NavigationResult, new_name: &str) -> String {
    let mut changes = BTreeMap::<String, Vec<&SourceSpan>>::new();
    let NavigationSource::Workspace = result.definition.source else {
        return "{\"changes\":{}}".to_string();
    };
    for span in std::iter::once(&result.definition.span).chain(&result.references) {
        changes
            .entry(path_to_uri(&root.join(span.file.as_str())))
            .or_default()
            .push(span);
    }
    let changes = changes
        .into_iter()
        .map(|(uri, spans)| {
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
            format!("\"{}\":[{edits}]", escape_json(&uri))
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"changes\":{{{changes}}}}}")
}

fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
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

fn extract_bool_field(message: &str, field: &str) -> Option<bool> {
    let key = format!("\"{field}\"");
    let index = message.find(&key)?;
    let after_key = &message[index + key.len()..];
    let after_colon = after_key[after_key.find(':')? + 1..].trim_start();
    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
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
    use veln_project::materialized_git_repository_root;

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
    fn server_keeps_symlink_workspace_alias_documents_in_project() {
        use std::os::unix::fs::symlink;

        let mut server = Server::default();
        let workspace = TempProject::new("workspace-alias-document-identity");
        workspace.write("package/veln.toml", "[package]\nname = \"package\"\n");
        workspace.write(
            "package/math.veln",
            concat!(
                "fn increment(value: Int) -> Int\n",
                "  increment(value - 1)\n",
                "end\n",
            ),
        );
        workspace.write(
            "package/math.test.veln",
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
            ),
        );
        workspace.write("package/main.veln", "pub fn main() -> Int\n  1\nend\n");
        symlink(workspace.root.join("package"), workspace.root.join("alias"))
            .expect("workspace alias should be created");
        let alias_uri = path_to_uri(&workspace.root.join("alias"));
        let alias_main_uri = path_to_uri(&workspace.root.join("alias/main.veln"));
        let alias_math_uri = path_to_uri(&workspace.root.join("alias/math.veln"));
        let alias_companion_uri = path_to_uri(&workspace.root.join("alias/math.test.veln"));

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"workspaceFolders":[{{"uri":"{alias_uri}","name":"alias"}}]}}}}"#
        ));

        assert_eq!(server.workspace_roots, vec![workspace.root.join("package")]);
        let publish = publish_for_uri(&responses, &alias_main_uri);
        assert!(publish.contains(r#""diagnostics":[]"#), "{publish}");

        let responses = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","method":"textDocument/didOpen","params":{{"textDocument":{{"uri":"{alias_main_uri}","text":"pub fn main() -> Int\n  \"bad\"\nend\n"}}}}}}"#
        ));
        let publish = publish_for_uri(&responses, &alias_main_uri);
        assert!(publish.contains(r#""code":"type.mismatch""#), "{publish}");

        let responses = server.handle_message(&semantic_tokens_request(&alias_main_uri));
        assert_eq!(responses.len(), 1);
        assert!(responses[0].contains(r#""id":2,"result":{"data":["#));
        assert!(!responses[0].contains(r#""data":[]"#), "{}", responses[0]);

        let responses = server.handle_message(&definition_request(&alias_companion_uri, 7, 10));
        assert_eq!(responses.len(), 1);
        assert!(
            responses[0].contains(&escape_json(&alias_math_uri)),
            "{}",
            responses[0]
        );
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
    fn dependency_definition_round_trips_through_retained_virtual_document() {
        let mut server = Server::default();
        let project = TempProject::new("dependency-virtual-document");
        project.write(
            "veln.toml",
            concat!(
                "[package]\nname = \"app\"\n\n",
                "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
            ),
        );
        project.write(
            "main.veln",
            concat!(
                "use math from \"example/pkg\"\n\n",
                "pub fn main() -> Int\n",
                "  math::exposed(1)\n",
                "  math::secret(1)\n",
                "end\n",
            ),
        );
        project.write(
            "vendor/lib/veln.toml",
            concat!(
                "[package]\nname = \"example/pkg\"\n\n",
                "[lib]\nexports = [\"./math.veln\"]\n",
            ),
        );
        let retained_text = concat!(
            "pub fn exposed(value: Int) -> Int\r\n",
            "  value + 1\r\n",
            "end\r\n\r\n",
            "fn secret(value: Int) -> Int\r\n",
            "  value\r\n",
            "end\r\n",
        );
        project.write("vendor/lib/math.veln", retained_text);
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        project.write(
            "vendor/lib/math.veln",
            "pub fn changed() -> Int\n  0\nend\n",
        );
        project.write(
            "main.veln",
            "use math from \"example/pkg\"\n\npub fn main() -> Int\n  math::changed()\nend\n",
        );
        let definition = server.handle_message(&definition_request(&main_uri, 3, 10));
        assert_eq!(definition.len(), 1);
        let virtual_uri = extract_string_field(&definition[0], "uri").unwrap();
        assert!(
            virtual_uri.starts_with("veln-pkg:///example%2Fpkg/snapshot/"),
            "{}",
            definition[0]
        );
        assert!(virtual_uri.ends_with("/math.veln"), "{}", definition[0]);
        assert!(
            !virtual_uri.contains("vendor") && !virtual_uri.contains("veln-lsp-"),
            "{}",
            definition[0]
        );
        assert!(
            definition[0].contains(
                r#""range":{"start":{"line":0,"character":7},"end":{"line":0,"character":14}}"#
            ),
            "{}",
            definition[0]
        );

        let read = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"veln/virtualDocument","params":{{"uri":"{virtual_uri}"}}}}"#
        ));
        assert_eq!(
            read,
            [response(
                "3",
                &format!(r#""{}""#, escape_json(retained_text))
            )]
        );

        let prepare_rename = server.handle_message(&prepare_rename_request(&main_uri, 3, 10));
        assert!(
            prepare_rename[0].contains(r#""result":null"#),
            "{}",
            prepare_rename[0]
        );
        let rename = server.handle_message(&rename_request(&main_uri, 3, 10, "renamed"));
        assert!(rename[0].contains(r#""changes":{}"#), "{}", rename[0]);

        let private_definition = server.handle_message(&definition_request(&main_uri, 4, 10));
        assert!(
            private_definition[0].contains(r#""result":null"#),
            "{}",
            private_definition[0]
        );

        for include_declaration in [false, true] {
            let references = server.handle_message(&references_request_with_declaration(
                &main_uri,
                3,
                10,
                include_declaration,
            ));
            assert!(
                references[0].contains(r#""result":[]"#),
                "{}",
                references[0]
            );
        }
        for rejected_uri in [
            format!("{virtual_uri}/missing"),
            virtual_uri.replacen("%2F", "%2f", 1),
        ] {
            let rejected = server.handle_message(&format!(
                r#"{{"jsonrpc":"2.0","id":4,"method":"veln/virtualDocument","params":{{"uri":"{rejected_uri}"}}}}"#
            ));
            assert!(rejected[0].contains(r#""code":-32602"#), "{}", rejected[0]);
        }
    }

    #[test]
    fn path_vendor_and_mirror_dependencies_share_retained_virtual_uris() {
        let mut observed = Vec::new();

        for (source_field, source_root) in [
            ("path", "path/lib"),
            ("vendor", "vendor/lib"),
            ("mirror", "mirror/example/pkg"),
        ] {
            let mut server = Server::default();
            let project = TempProject::new(&format!("dependency-virtual-document-{source_field}"));
            project.write(
                "veln.toml",
                &format!(
                    concat!(
                        "[package]\nname = \"app\"\n\n",
                        "[dependencies.\"example/pkg\"]\n",
                        "{} = \"{}\"\n",
                    ),
                    source_field, source_root
                ),
            );
            project.write(
                "main.veln",
                concat!(
                    "use math from \"example/pkg\"\n\n",
                    "use hidden from \"example/pkg\"\n\n",
                    "pub fn main() -> Int\n",
                    "  math::exposed(1)\n",
                    "  math::secret(1)\n",
                    "  hidden::published(1)\n",
                    "end\n",
                ),
            );
            project.write(
                &format!("{source_root}/veln.toml"),
                concat!(
                    "[package]\nname = \"example/pkg\"\n\n",
                    "[lib]\nexports = [\"math.veln\"]\n",
                ),
            );
            let retained_text = "pub fn exposed(value: Int) -> Int\r\n  value + 1\r\nend\r\n";
            project.write(&format!("{source_root}/math.veln"), retained_text);
            project.write(
                &format!("{source_root}/hidden.veln"),
                "pub fn published(value: Int) -> Int\n  value\nend\n",
            );
            let root_uri = path_to_uri(&project.root);
            let main_uri = path_to_uri(&project.root.join("main.veln"));
            server.handle_message(&initialize_request(&root_uri));

            let definition = server.handle_message(&definition_request(&main_uri, 5, 10));
            let virtual_uri = extract_string_field(&definition[0], "uri").unwrap();
            assert!(
                virtual_uri.starts_with("veln-pkg:///example%2Fpkg/snapshot/")
                    && virtual_uri.ends_with("/math.veln"),
                "{}",
                definition[0]
            );
            assert!(!virtual_uri.contains(source_root), "{}", definition[0]);

            let read = server.handle_message(&format!(
                r#"{{"jsonrpc":"2.0","id":3,"method":"veln/virtualDocument","params":{{"uri":"{virtual_uri}"}}}}"#
            ));
            assert_eq!(
                read,
                [response(
                    "3",
                    &format!(r#""{}""#, escape_json(retained_text))
                )],
                "{source_field}"
            );
            let private_definition = server.handle_message(&definition_request(&main_uri, 6, 10));
            assert!(
                private_definition[0].contains(r#""result":null"#),
                "{source_field}: {}",
                private_definition[0]
            );
            let unexported_definition =
                server.handle_message(&definition_request(&main_uri, 7, 12));
            assert!(
                unexported_definition[0].contains(r#""result":null"#),
                "{source_field}: {}",
                unexported_definition[0]
            );
            observed.push(virtual_uri);
        }

        assert_eq!(observed[0], observed[1]);
        assert_eq!(observed[0], observed[2]);
    }

    #[test]
    fn git_dependency_subdir_definition_uses_retained_virtual_document() {
        let mut server = Server::default();
        let project = TempProject::new("git-dependency-virtual-document");
        let remote_url = "https://example.invalid/mono.git";
        let repository_root = materialized_git_repository_root(&project.root, remote_url);
        project.write(
            "veln.toml",
            concat!(
                "[package]\nname = \"app\"\n\n",
                "[dependencies.\"example/pkg\"]\n",
                "git = \"https://example.invalid/mono.git\"\n",
                "rev = \"abc123\"\n",
                "subdir = \"packages/lib\"\n",
            ),
        );
        project.write(
            "main.veln",
            concat!(
                "use math from \"example/pkg\"\n\n",
                "pub fn main() -> Int\n",
                "  math::exposed(1)\n",
                "  math::secret(1)\n",
                "end\n",
            ),
        );
        let package_root = repository_root.join("packages/lib");
        project.write(
            &package_root.join("veln.toml").display().to_string(),
            concat!(
                "[package]\nname = \"example/pkg\"\n\n",
                "[lib]\nexports = [\"math.veln\"]\n",
            ),
        );
        let retained_text = concat!(
            "pub fn exposed(value: Int) -> Int\r\n",
            "  value + 1\r\n",
            "end\r\n\r\n",
            "fn secret(value: Int) -> Int\r\n",
            "  value\r\n",
            "end\r\n",
        );
        project.write(
            &package_root.join("math.veln").display().to_string(),
            retained_text,
        );

        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));
        project.write(
            &package_root.join("math.veln").display().to_string(),
            "pub fn changed() -> Int\n  0\nend\n",
        );

        let definition = server.handle_message(&definition_request(&main_uri, 3, 10));
        let virtual_uri = extract_string_field(&definition[0], "uri").unwrap();
        assert!(
            virtual_uri.starts_with("veln-pkg:///example%2Fpkg/snapshot/")
                && virtual_uri.ends_with("/math.veln"),
            "{}",
            definition[0]
        );
        assert!(
            !virtual_uri.contains(".veln/package/git") && !virtual_uri.contains("packages/lib"),
            "{}",
            definition[0]
        );

        let read = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"veln/virtualDocument","params":{{"uri":"{virtual_uri}"}}}}"#
        ));
        assert_eq!(
            read,
            [response(
                "3",
                &format!(r#""{}""#, escape_json(retained_text))
            )]
        );

        let private_definition = server.handle_message(&definition_request(&main_uri, 4, 10));
        assert!(
            private_definition[0].contains(r#""result":null"#),
            "{}",
            private_definition[0]
        );
    }

    #[test]
    fn standard_library_definition_round_trips_through_embedded_virtual_document() {
        let mut server = Server::default();
        let project = TempProject::new("standard-library-virtual-document");
        project.write(
            "main.veln",
            concat!(
                "use http2::diagnostic from \"std\"\n\n",
                "pub fn implicit() -> Result<Byte, String>\n",
                "  byte(1)\n",
                "end\n\n",
                "pub fn qualified() -> Result<Byte, String>\n",
                "  prelude::byte(1)\n",
                "end\n\n",
                "pub fn parameter_shadow(byte: fn(Int) -> Result<Byte, String>) -> Result<Byte, String>\n",
                "  byte(1)\n",
                "end\n\n",
                "pub fn local_shadow() -> Result<Byte, String>\n",
                "  let byte: fn(Int) -> Result<Byte, String> = prelude::byte\n",
                "  byte(1)\n",
                "end\n\n",
                "pub fn imported() -> Result<(), RuntimeDiagnostic>\n",
                "  http2::diagnostic::protocol_invalid_frame_kind(0, 0, 0, 0, \"open\", \"rule\", byte_view(byte_chunk([]), ByteOffset(0), ByteCount(0)))\n",
                "end\n\n",
                "pub fn private_helper() -> Vec<Int>\n",
                "  prelude::vec_append([], 1)\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let implicit = server.handle_message(&definition_request(&main_uri, 3, 4));
        let qualified = server.handle_message(&definition_request(&main_uri, 7, 12));
        let shadowed_parameter = server.handle_message(&definition_request(&main_uri, 11, 2));
        let shadowed_local = server.handle_message(&definition_request(&main_uri, 16, 2));
        let imported = server.handle_message(&definition_request(&main_uri, 20, 31));
        let prelude_uri = extract_string_field(&implicit[0], "uri").unwrap();

        assert_eq!(
            extract_string_field(&qualified[0], "uri"),
            Some(prelude_uri.clone())
        );
        assert!(
            prelude_uri.starts_with("veln-pkg:///std/snapshot/")
                && prelude_uri.ends_with("/prelude.veln"),
            "{}",
            implicit[0]
        );
        assert!(
            implicit[0].contains(
                r#""range":{"start":{"line":97,"character":7},"end":{"line":97,"character":11}}"#
            ),
            "{}",
            implicit[0]
        );
        assert!(shadowed_parameter[0].contains(r#""result":null"#));
        assert!(shadowed_local[0].contains(r#""result":null"#));
        let diagnostic_uri = extract_string_field(&imported[0], "uri").unwrap();
        assert!(
            diagnostic_uri.starts_with("veln-pkg:///std/snapshot/")
                && diagnostic_uri.ends_with("/http2/diagnostic.veln"),
            "{}",
            imported[0]
        );

        let read = server.handle_message(&format!(
            r#"{{"jsonrpc":"2.0","id":3,"method":"veln/virtualDocument","params":{{"uri":"{prelude_uri}"}}}}"#
        ));
        assert_eq!(
            read,
            [response(
                "3",
                &format!(
                    r#""{}""#,
                    escape_json(
                        veln_stdlib::package_bundle()
                            .files
                            .iter()
                            .find(|file| file.path == "prelude.veln")
                            .unwrap()
                            .text
                    )
                )
            )]
        );

        let private_definition = server.handle_message(&definition_request(&main_uri, 24, 12));
        assert!(private_definition[0].contains(r#""result":null"#));
        let prepare_rename = server.handle_message(&prepare_rename_request(&main_uri, 3, 4));
        let rename = server.handle_message(&rename_request(&main_uri, 3, 4, "renamed"));
        assert!(prepare_rename[0].contains(r#""result":null"#));
        assert!(rename[0].contains(r#""changes":{}"#));

        for rejected_uri in [
            format!("{prelude_uri}/missing"),
            prelude_uri.replacen("veln-pkg", "VELN-pkg", 1),
        ] {
            let rejected = server.handle_message(&format!(
                r#"{{"jsonrpc":"2.0","id":4,"method":"veln/virtualDocument","params":{{"uri":"{rejected_uri}"}}}}"#
            ));
            assert!(rejected[0].contains(r#""code":-32602"#), "{}", rejected[0]);
        }
    }

    #[test]
    fn ambiguous_bare_prelude_fallback_returns_no_definition() {
        let mut server = Server::default();
        let project = TempProject::new("ambiguous-bare-prelude-definition");
        project.write(
            "veln.toml",
            concat!(
                "[package]\nname = \"app\"\n\n",
                "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
            ),
        );
        project.write(
            "math.veln",
            concat!(
                "use math from \"example/pkg\"\n\n",
                "pub fn main(items: Vec<Int>) -> Int\n",
                "  vec_len(items)\n",
                "end\n",
            ),
        );
        project.write(
            "vendor/lib/veln.toml",
            concat!(
                "[package]\nname = \"example/pkg\"\n\n",
                "[lib]\nexports = [\"math.veln\"]\n",
            ),
        );
        project.write(
            "vendor/lib/math.veln",
            "pub fn vec_len(items: Vec<Int>) -> Int\n  0\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("math.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let definition = server.handle_message(&definition_request(&main_uri, 3, 4));

        assert_eq!(definition.len(), 1);
        assert!(
            definition[0].contains(r#""result":null"#),
            "{}",
            definition[0]
        );
    }

    #[test]
    fn private_workspace_import_does_not_hide_bare_prelude_definition() {
        let mut server = Server::default();
        let project = TempProject::new("private-import-bare-prelude-definition");
        project.write(
            "main.veln",
            concat!(
                "use math\n\n",
                "pub fn main() -> Result<Byte, String>\n",
                "  byte(1)\n",
                "end\n",
            ),
        );
        project.write(
            "math.veln",
            "fn byte(value: Int) -> Result<Byte, String>\n  Ok(Byte(value))\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let definition = server.handle_message(&definition_request(&main_uri, 3, 4));

        assert_eq!(definition.len(), 1);
        let prelude_uri = extract_string_field(&definition[0], "uri").unwrap();
        assert!(
            prelude_uri.starts_with("veln-pkg:///std/snapshot/")
                && prelude_uri.ends_with("/prelude.veln"),
            "{}",
            definition[0]
        );
        assert!(
            definition[0].contains(
                r#""range":{"start":{"line":97,"character":7},"end":{"line":97,"character":11}}"#
            ),
            "{}",
            definition[0]
        );
    }

    #[test]
    fn imported_constructor_definition_wins_over_bare_prelude_fallback() {
        let mut server = Server::default();
        let project = TempProject::new("imported-constructor-bare-prelude-definition");
        project.write(
            "main.veln",
            concat!(
                "use model\n\n",
                "pub fn main() -> Token\n",
                "  byte(1)\n",
                "end\n\n",
                "fn byte(value: Int) -> Int\n",
                "  value\n",
                "end\n",
            ),
        );
        project.write(
            "model.veln",
            concat!("pub type Token\n", "  pub byte(Int)\n", "end\n"),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let definition = server.handle_message(&definition_request(&main_uri, 3, 4));

        assert_eq!(definition.len(), 1);
        assert!(definition[0].contains("/model.veln"), "{}", definition[0]);
        assert!(
            definition[0].contains(
                r#""range":{"start":{"line":1,"character":6},"end":{"line":1,"character":10}}"#
            ),
            "{}",
            definition[0]
        );
        assert!(
            !definition[0].contains("veln-pkg:///std/snapshot/"),
            "{}",
            definition[0]
        );

        let references = server.handle_message(&references_request(&main_uri, 6, 4));
        assert!(
            references[0].contains(
                r#""range":{"start":{"line":6,"character":3},"end":{"line":6,"character":7}}"#
            ),
            "{}",
            references[0]
        );
        assert!(
            !references[0].contains(
                r#""range":{"start":{"line":3,"character":2},"end":{"line":3,"character":6}}"#
            ),
            "{}",
            references[0]
        );
    }

    #[test]
    fn callable_binding_shadows_constructor_for_bare_call_navigation() {
        let mut server = Server::default();
        let project = TempProject::new("callable-shadow-constructor-navigation");
        project.write(
            "main.veln",
            concat!(
                "type Token\n",
                "  byte(Int)\n",
                "end\n",
                "\n",
                "pub fn parameter_shadow(byte: fn(Int) -> Token) -> Token\n",
                "  byte(1)\n",
                "end\n",
                "\n",
                "pub fn local_shadow(identity: fn(Int) -> Token) -> Token\n",
                "  let byte = identity\n",
                "  byte(1)\n",
                "end\n",
            ),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        for (line, character) in [(5, 4), (10, 4)] {
            let definition = server.handle_message(&definition_request(&main_uri, line, character));
            let references = server.handle_message(&references_request(&main_uri, line, character));
            let rename = server.handle_message(&rename_request(&main_uri, line, character, "pack"));

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
        }
    }

    #[test]
    fn ambiguous_current_module_constructor_definition_does_not_fall_back_to_import() {
        let mut server = Server::default();
        let project = TempProject::new("current-constructor-ambiguity-blocks-import-definition");
        project.write(
            "main.veln",
            concat!(
                "use model\n\n",
                "type LocalToken\n",
                "  byte(Int)\n",
                "end\n\n",
                "type OtherToken\n",
                "  byte(Int)\n",
                "end\n\n",
                "pub fn main() -> LocalToken\n",
                "  byte(1)\n",
                "end\n\n",
                "fn byte(value: Int) -> Int\n",
                "  value\n",
                "end\n",
            ),
        );
        project.write(
            "model.veln",
            concat!("pub type ImportedToken\n", "  pub byte(Int)\n", "end\n"),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let definition = server.handle_message(&definition_request(&main_uri, 11, 4));

        assert_eq!(definition.len(), 1);
        assert!(definition[0].contains(r#""id":2,"result":null"#));

        let references = server.handle_message(&references_request(&main_uri, 14, 4));
        assert!(
            references[0].contains(
                r#""range":{"start":{"line":14,"character":3},"end":{"line":14,"character":7}}"#
            ),
            "{}",
            references[0]
        );
        assert!(
            !references[0].contains(
                r#""range":{"start":{"line":11,"character":2},"end":{"line":11,"character":6}}"#
            ),
            "{}",
            references[0]
        );
        assert!(!references[0].contains("/model.veln"), "{}", references[0]);
    }

    #[test]
    fn reexported_constructor_definition_wins_over_bare_prelude_fallback() {
        let mut server = Server::default();
        let project = TempProject::new("reexported-constructor-bare-prelude-definition");
        project.write(
            "main.veln",
            concat!(
                "use facade\n\n",
                "pub fn bare() -> Token\n",
                "  byte(1)\n",
                "end\n\n",
                "pub fn qualified() -> Token\n",
                "  facade::byte(2)\n",
                "end\n",
            ),
        );
        project.write(
            "facade.veln",
            concat!("use model\n\n", "pub type Token = model::Token\n"),
        );
        project.write(
            "model.veln",
            concat!("pub type Token\n", "  pub byte(Int)\n", "end\n"),
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        for (line, character) in [(3, 4), (7, 10)] {
            let definition = server.handle_message(&definition_request(&main_uri, line, character));

            assert_eq!(definition.len(), 1);
            assert!(definition[0].contains("/model.veln"), "{}", definition[0]);
            assert!(
                definition[0].contains(
                    r#""range":{"start":{"line":1,"character":6},"end":{"line":1,"character":10}}"#
                ),
                "{}",
                definition[0]
            );
            assert!(
                !definition[0].contains("veln-pkg:///std/snapshot/"),
                "{}",
                definition[0]
            );
        }
    }

    #[test]
    fn retained_direct_dependencies_use_the_supplied_workspace_manifest() {
        let project = TempProject::new("retained-dependency-supplied-manifest");
        project.write(
            "vendor/lib/veln.toml",
            concat!(
                "[package]\nname = \"example/pkg\"\n\n",
                "[lib]\nexports = [\"math.veln\"]\n",
            ),
        );
        project.write(
            "vendor/lib/math.veln",
            "pub fn exposed(value: Int) -> Int\n  value + 1\nend\n",
        );
        let manifest = parse_manifest_text(
            "veln.toml",
            concat!(
                "[package]\nname = \"app\"\n\n",
                "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
            ),
        );

        let dependencies = retained_direct_dependencies(&project.root, &manifest);

        assert_eq!(dependencies.len(), 1);
        let snapshot = EffectiveProjectSnapshot::with_direct_dependencies(
            vec![SourceFile::new(
                "main.veln",
                concat!(
                    "use math from \"example/pkg\"\n\n",
                    "pub fn main() -> Int\n",
                    "  math::exposed(1)\n",
                    "end\n",
                ),
            )],
            dependencies,
        );
        assert!(
            navigate(
                &snapshot,
                SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 4,
                    column: 10,
                }
            )
            .is_some()
        );
    }

    #[test]
    fn dependency_definition_requires_external_import_path() {
        let mut server = Server::default();
        let project = TempProject::new("dependency-import-boundary");
        project.write(
            "veln.toml",
            concat!(
                "[package]\nname = \"app\"\n\n",
                "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
            ),
        );
        project.write(
            "main.veln",
            concat!(
                "use math from \"other/pkg\"\n",
                "use other from \"example/pkg\"\n\n",
                "pub fn missing_import() -> Int\n",
                "  exposed(1)\n",
                "end\n\n",
                "pub fn wrong_package() -> Int\n",
                "  math::exposed(1)\n",
                "end\n\n",
                "pub fn wrong_module() -> Int\n",
                "  other::exposed(1)\n",
                "end\n",
            ),
        );
        project.write(
            "vendor/lib/veln.toml",
            concat!(
                "[package]\nname = \"example/pkg\"\n\n",
                "[lib]\nexports = [\"math.veln\"]\n",
            ),
        );
        project.write(
            "vendor/lib/math.veln",
            "pub fn exposed(value: Int) -> Int\n  value + 1\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let main_uri = path_to_uri(&project.root.join("main.veln"));
        server.handle_message(&initialize_request(&root_uri));

        for (line, character) in [(4, 4), (8, 10), (12, 11)] {
            let response = server.handle_message(&definition_request(&main_uri, line, character));
            assert!(response[0].contains(r#""result":null"#), "{}", response[0]);
        }
    }

    #[test]
    fn workspace_references_and_rename_ignore_dependency_sources() {
        let mut server = Server::default();
        let project = TempProject::new("dependency-reference-isolation");
        project.write(
            "veln.toml",
            concat!(
                "[package]\nname = \"app\"\n\n",
                "[dependencies.\"example/pkg\"]\npath = \"vendor/lib\"\n",
            ),
        );
        project.write(
            "math.veln",
            "pub fn exposed(value: Int) -> Int\n  value + 1\nend\n",
        );
        project.write(
            "vendor/lib/veln.toml",
            concat!(
                "[package]\nname = \"example/pkg\"\n\n",
                "[lib]\nexports = [\"math.veln\"]\n",
            ),
        );
        project.write(
            "vendor/lib/math.veln",
            "pub fn exposed(value: Int) -> Int\n  exposed(value - 1)\nend\n",
        );
        let root_uri = path_to_uri(&project.root);
        let math_uri = path_to_uri(&project.root.join("math.veln"));
        server.handle_message(&initialize_request(&root_uri));

        let references = server.handle_message(&references_request(&math_uri, 0, 7));
        assert!(
            references[0].contains(
                r#""range":{"start":{"line":0,"character":7},"end":{"line":0,"character":14}}"#
            ),
            "{}",
            references[0]
        );
        assert!(
            !references[0].contains(r#""line":1,"character":2"#),
            "{}",
            references[0]
        );

        let rename = server.handle_message(&rename_request(&math_uri, 0, 7, "renamed"));
        assert!(
            rename[0].contains(
                r#""range":{"start":{"line":0,"character":7},"end":{"line":0,"character":14}}"#
            ),
            "{}",
            rename[0]
        );
        assert!(
            !rename[0].contains(r#""line":1,"character":2"#),
            "{}",
            rename[0]
        );
        assert!(!rename[0].contains("vendor"), "{}", rename[0]);
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
        references_request_with_declaration(uri, line, character, true)
    }

    fn references_request_with_declaration(
        uri: &str,
        line: usize,
        character: usize,
        include_declaration: bool,
    ) -> String {
        format!(
            r#"{{"jsonrpc":"2.0","id":2,"method":"textDocument/references","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}},"context":{{"includeDeclaration":{include_declaration}}}}}}}"#
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
