use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

use veln_diagnostics::Severity;
use veln_language_service::{
    NavigationLocation, NavigationResult, NavigationSource, RenameFailure,
};
use veln_project::discover_source_paths;
use veln_source::{SourceFile, SourceSpan};
use veln_syntax::{format_tree, parse};

use crate::{legend, semantic_tokens_full};

mod diagnostic;

pub(crate) use diagnostic::*;

pub(crate) fn document_uri_and_text(message: &str) -> Option<(String, String)> {
    Some((
        extract_string_field(message, "uri")?,
        extract_string_field(message, "text")?,
    ))
}

pub(crate) fn initialize_result() -> String {
    let legend = legend();
    format!(
        "{{\"capabilities\":{{\"textDocumentSync\":1,\"definitionProvider\":true,\"referencesProvider\":true,\"documentFormattingProvider\":true,\"renameProvider\":{{\"prepareProvider\":true}},\"semanticTokensProvider\":{{\"legend\":{{\"tokenTypes\":[{}],\"tokenModifiers\":[{}]}},\"full\":true,\"range\":false}}}}}}",
        json_string_list(&legend.token_types),
        json_string_list(&legend.token_modifiers),
    )
}

pub(crate) fn semantic_tokens_result(uri: &str, text: String) -> String {
    let source = SourceFile::new(display_path(uri), text);
    let data = semantic_tokens_full(&source)
        .data
        .into_iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"data\":[{data}]}}")
}

pub(crate) fn formatting_result(uri: &str, text: String) -> String {
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

pub(crate) fn range_json(span: Option<&SourceSpan>) -> String {
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

pub(crate) fn position_json(line: usize, column: usize) -> String {
    format!(
        "{{\"line\":{},\"character\":{}}}",
        line.saturating_sub(1),
        column.saturating_sub(1),
    )
}

pub(crate) fn full_document_range_json(text: &str) -> String {
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

pub(crate) fn severity_code(severity: Severity) -> u8 {
    match severity {
        Severity::Error => 1,
        Severity::Warning => 2,
        Severity::Info => 3,
        Severity::Hint => 4,
    }
}

#[derive(Debug, Default)]
pub(crate) struct WorkspaceSelection {
    pub(crate) roots: Vec<PathBuf>,
    pub(crate) aliases: BTreeMap<PathBuf, Vec<PathBuf>>,
}

#[derive(Debug)]
pub(crate) struct WorkspaceFolderRoot {
    identity: PathBuf,
    visible: PathBuf,
}

#[derive(Debug)]
pub(crate) struct WorkspaceDocumentRoot<'a> {
    pub(crate) root: &'a Path,
    pub(crate) relative: PathBuf,
}

pub(crate) fn resolve_workspace_roots(message: &str) -> WorkspaceSelection {
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

pub(crate) fn filesystem_workspace_root(root: PathBuf) -> Option<WorkspaceFolderRoot> {
    let visible = absolute_workspace_path(root);
    let identity = fs::canonicalize(&visible).ok()?;
    Some(WorkspaceFolderRoot { identity, visible })
}

pub(crate) fn absolute_workspace_path(root: PathBuf) -> PathBuf {
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

pub(crate) fn add_workspace_project_roots(
    selection: &mut WorkspaceSelection,
    folder: &WorkspaceFolderRoot,
) {
    for root in resolve_workspace_project_roots(&folder.identity) {
        let visible = folder
            .visible
            .join(root.strip_prefix(&folder.identity).unwrap_or(Path::new("")));
        selection.roots.push(root.clone());
        selection.aliases.entry(root).or_default().push(visible);
    }
}

pub(crate) fn resolve_workspace_project_roots(root: &Path) -> Vec<PathBuf> {
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

pub(crate) fn collect_manifest_project_roots(dir: &Path, roots: &mut Vec<PathBuf>) {
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

pub(crate) fn has_regular_manifest(root: &Path) -> bool {
    fs::symlink_metadata(root.join("veln.toml"))
        .is_ok_and(|metadata| metadata.file_type().is_file())
}

pub(crate) fn extract_workspace_folder_uris(message: &str) -> Vec<String> {
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

pub(crate) fn visible_workspace_root<'a>(
    root: &'a Path,
    aliases: &'a BTreeMap<PathBuf, Vec<PathBuf>>,
) -> &'a Path {
    aliases
        .get(root)
        .and_then(|aliases| aliases.first())
        .map(PathBuf::as_path)
        .unwrap_or(root)
}

pub(crate) fn workspace_root_for_uri<'a>(
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

pub(crate) fn owned_workspace_source_file(
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

pub(crate) fn owned_workspace_relative_source_path(root: &Path, relative: &Path) -> Option<String> {
    let relative = workspace_relative_source_path(relative)?;
    let input = PathBuf::from(&relative);
    discover_source_paths(root, std::slice::from_ref(&input)).ok()?;
    Some(relative)
}

pub(crate) fn workspace_relative_source_path(relative: &Path) -> Option<String> {
    if relative
        .extension()
        .is_none_or(|extension| extension != "veln")
    {
        return None;
    }
    Some(relative.to_string_lossy().replace('\\', "/"))
}

#[derive(Clone, Debug)]
pub(crate) struct Position {
    pub(crate) line: usize,
    pub(crate) character: usize,
}

#[derive(Debug)]
pub(crate) struct NavigationRequest {
    pub(crate) root: PathBuf,
    pub(crate) result: NavigationResult,
}

pub(crate) fn location_json(root: &Path, location: &NavigationLocation) -> String {
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

pub(crate) fn is_workspace_location(location: &NavigationLocation) -> bool {
    matches!(location.source, NavigationSource::Workspace)
}

pub(crate) fn references_json(
    root: &Path,
    result: &NavigationResult,
    include_declaration: bool,
) -> String {
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

pub(crate) fn workspace_edit_json(
    root: &Path,
    result: &NavigationResult,
    new_name: &str,
) -> String {
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

pub(crate) fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

pub(crate) fn path_to_uri(path: &Path) -> String {
    let path = path.to_string_lossy().replace('\\', "/");
    format!("file://{}", percent_encode_uri_path(&path))
}

pub(crate) fn percent_encode_uri_path(path: &str) -> String {
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

pub(crate) fn read_message(input: &mut impl BufRead) -> io::Result<Option<String>> {
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

pub(crate) fn write_message(output: &mut impl Write, body: &str) -> io::Result<()> {
    write!(output, "Content-Length: {}\r\n\r\n{body}", body.len())?;
    output.flush()
}

pub(crate) fn response(id: &str, result: &str) -> String {
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"result\":{result}}}")
}

pub(crate) fn error_response(id: &str, code: i32, message: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":{code},\"message\":\"{}\"}}}}",
        escape_json(message)
    )
}

pub(crate) fn rename_failure_response(id: &str, failure: &RenameFailure) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"id\":{id},\"error\":{{\"code\":-32602,\"message\":\"{}\",\"data\":{{\"code\":\"{}\",\"details\":{{\"symbol_class\":\"{}\",\"requested_name\":\"{}\",\"required_initial\":\"{}\"}}}}}}}}",
        escape_json(failure.code),
        escape_json(failure.code),
        failure.symbol_class.as_str(),
        escape_json(&failure.requested_name),
        failure.required_initial.as_str(),
    )
}

pub(crate) fn extract_id(message: &str) -> Option<String> {
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

pub(crate) fn extract_position(message: &str) -> Option<Position> {
    let position_index = message.find("\"position\"")?;
    let position = &message[position_index..];
    Some(Position {
        line: extract_usize_field(position, "line")?,
        character: extract_usize_field(position, "character")?,
    })
}

pub(crate) fn extract_usize_field(message: &str, field: &str) -> Option<usize> {
    let key = format!("\"{field}\"");
    let index = message.find(&key)?;
    let after_key = &message[index + key.len()..];
    let after_colon = after_key[after_key.find(':')? + 1..].trim_start();
    let end = after_colon
        .find(|ch: char| !ch.is_ascii_digit())
        .unwrap_or(after_colon.len());
    after_colon[..end].parse().ok()
}

pub(crate) fn extract_bool_field(message: &str, field: &str) -> Option<bool> {
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

pub(crate) fn extract_string_field(message: &str, field: &str) -> Option<String> {
    let key = format!("\"{field}\"");
    let index = message.find(&key)?;
    let after_key = &message[index + key.len()..];
    let after_colon = after_key[after_key.find(':')? + 1..].trim_start();
    parse_json_string(after_colon)
}

pub(crate) fn parse_json_string(input: &str) -> Option<String> {
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

pub(crate) fn json_string_list(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| format!("\"{}\"", escape_json(value)))
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn escape_json(value: &str) -> String {
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

pub(crate) fn display_path(uri: &str) -> String {
    uri_to_path(uri)
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|| uri.to_string())
}

pub(crate) fn uri_to_path(uri: &str) -> Option<PathBuf> {
    let path = uri.strip_prefix("file://")?;
    Some(PathBuf::from(percent_decode(path)))
}

pub(crate) fn percent_decode(value: &str) -> String {
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
