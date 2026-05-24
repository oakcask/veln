use std::io;
use std::path::Path;

use veln_source::{SourceFile, SourcePath, SourceSpan, TextRange};

#[derive(Clone, Debug)]
pub struct ProjectManifest {
    pub path: SourcePath,
    pub modules: Vec<ManifestModule>,
}

#[derive(Clone, Debug)]
pub struct ManifestModule {
    pub path: String,
    pub name: String,
    pub path_span: SourceSpan,
    pub name_span: SourceSpan,
}

pub fn read_manifest(root: &Path) -> io::Result<Option<ProjectManifest>> {
    let path = root.join("veln.toml");
    if !path.exists() {
        return Ok(None);
    }
    let source = SourceFile::read(root, &path)?;
    Ok(Some(parse_manifest(&source)))
}

fn parse_manifest(source: &SourceFile) -> ProjectManifest {
    let mut modules = Vec::new();
    let mut in_modules = false;
    let mut offset = 0;

    for line in source.text().split_inclusive('\n') {
        let line_without_newline = line.trim_end_matches(['\r', '\n']);
        let trimmed = line_without_newline.trim();
        if trimmed == "[modules]" {
            in_modules = true;
        } else if trimmed.starts_with('[') {
            in_modules = false;
        } else if in_modules {
            if let Some(module) = parse_module_entry(source, offset, line_without_newline) {
                modules.push(module);
            }
        }
        offset += line.len();
    }

    ProjectManifest {
        path: source.path().clone(),
        modules,
    }
}

fn parse_module_entry(
    source: &SourceFile,
    line_offset: usize,
    line: &str,
) -> Option<ManifestModule> {
    let trimmed_start = line.len() - line.trim_start().len();
    let text = line.trim();
    if text.is_empty() || text.starts_with('#') {
        return None;
    }
    let (path, after_path, path_range) = parse_quoted(text, line_offset + trimmed_start)?;
    let after_path = after_path.trim_start();
    let equals_len = after_path
        .strip_prefix('=')
        .map(|remaining| after_path.len() - remaining.len())?;
    let name_offset = line_offset + line.find(after_path)? + equals_len;
    let name_text = after_path.strip_prefix('=')?.trim_start();
    let leading = after_path.strip_prefix('=')?.len() - name_text.len();
    let (name, _, name_range) = parse_quoted(name_text, name_offset + leading)?;

    Some(ManifestModule {
        path,
        name,
        path_span: source.span(path_range),
        name_span: source.span(name_range),
    })
}

fn parse_quoted(text: &str, absolute_offset: usize) -> Option<(String, &str, TextRange)> {
    let rest = text.strip_prefix('"')?;
    let end = rest.find('"')?;
    let value = rest[..end].to_string();
    let start = absolute_offset + 1;
    let end_offset = start + end;
    let remaining = &rest[end + 1..];
    Some((value, remaining, TextRange::new(start, end_offset)))
}
