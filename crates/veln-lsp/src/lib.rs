//! LSP-facing semantic token helpers for Veln editors.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use veln_analysis::{
    DoctestMode, checked_project_diagnostics, parse_diagnostic_to_envelope,
    validate_manifest_exports,
};
use veln_ast::{SurfaceModule, lower_surface_ast};
use veln_diagnostics::Diagnostic;
use veln_editor::{encode_lsp_semantic_tokens, semantic_token_legend};
use veln_language_service::{
    DirectDependencySnapshot, EffectiveProjectSnapshot, SourcePosition, navigate, validate_rename,
};
use veln_project::{
    PackageIdentity, PackageSnapshotSource, Project, ProjectManifest,
    capture_embedded_package_snapshot, capture_package_snapshot, parse_manifest_text,
};
use veln_source::{SourceFile, SourcePath};
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
        invalid_names: lowered.invalid_names,
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
            let Some(new_name) = extract_string_field(message, "newName") else {
                return response(&id, "{\"changes\":{}}");
            };
            if !is_identifier(&new_name) {
                return response(&id, "{\"changes\":{}}");
            }
            let Some(request) = self
                .symbol_at_request(message)
                .filter(|request| is_workspace_location(&request.result.definition))
            else {
                return response(&id, "{\"changes\":{}}");
            };
            match validate_rename(&request.result, &new_name) {
                Ok(()) => response(
                    &id,
                    &workspace_edit_json(&request.root, &request.result, &new_name),
                ),
                Err(failure) => rename_failure_response(&id, &failure),
            }
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

mod wire;

use wire::*;

#[cfg(test)]
mod tests;
