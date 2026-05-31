use std::io;
use std::path::Path;

use veln_source::{SourceFile, SourcePath, SourceSpan, TextRange};

#[derive(Clone, Debug)]
pub struct ProjectManifest {
    pub path: SourcePath,
    pub package: ManifestPackage,
    pub modules: Vec<ManifestModule>,
    pub tools: Vec<ManifestTool>,
}

#[derive(Clone, Debug, Default)]
pub struct ManifestPackage {
    pub fields: Vec<ManifestField>,
}

#[derive(Clone, Debug)]
pub struct ManifestModule {
    pub path: String,
    pub name: String,
    pub path_span: SourceSpan,
    pub name_span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct ManifestTool {
    pub name: String,
    pub fields: Vec<ManifestField>,
}

#[derive(Clone, Debug)]
pub struct ManifestField {
    pub key: String,
    pub value: String,
    pub key_span: SourceSpan,
    pub value_span: SourceSpan,
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
    let mut package = ManifestPackage::default();
    let mut modules = Vec::new();
    let mut tools = Vec::<ManifestTool>::new();
    let mut section = ManifestSection::Other;
    let mut offset = 0;

    for line in source.text().split_inclusive('\n') {
        let line_without_newline = line.trim_end_matches(['\r', '\n']);
        let trimmed = line_without_newline.trim();
        if let Some(section_name) = section_name(trimmed) {
            section = ManifestSection::from_name(section_name);
        } else if matches!(section, ManifestSection::Modules)
            && let Some(module) = parse_module_entry(source, offset, line_without_newline)
        {
            modules.push(module);
        } else if matches!(section, ManifestSection::Package)
            && let Some(field) = parse_string_field(source, offset, line_without_newline)
        {
            package.fields.push(field);
        } else if let ManifestSection::Tool(tool_name) = &section
            && let Some(field) = parse_string_field(source, offset, line_without_newline)
        {
            let index = tools
                .iter()
                .position(|tool| tool.name == *tool_name)
                .unwrap_or_else(|| {
                    tools.push(ManifestTool {
                        name: tool_name.clone(),
                        fields: Vec::new(),
                    });
                    tools.len() - 1
                });
            let tool = &mut tools[index];
            tool.fields.push(field);
        }
        offset += line.len();
    }

    ProjectManifest {
        path: source.path().clone(),
        package,
        modules,
        tools,
    }
}

#[derive(Clone)]
enum ManifestSection {
    Package,
    Modules,
    Tool(String),
    Other,
}

impl ManifestSection {
    fn from_name(name: &str) -> Self {
        if name == "package" {
            Self::Package
        } else if name == "modules" {
            Self::Modules
        } else if let Some(tool_name) = name.strip_prefix("tool.") {
            Self::Tool(tool_name.to_string())
        } else {
            Self::Other
        }
    }
}

fn section_name(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('[')?;
    let end = rest.find(']')?;
    Some(&rest[..end])
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

fn parse_string_field(
    source: &SourceFile,
    line_offset: usize,
    line: &str,
) -> Option<ManifestField> {
    let trimmed_start = line.len() - line.trim_start().len();
    let text = line.trim();
    if text.is_empty() || text.starts_with('#') {
        return None;
    }

    let equals = text.find('=')?;
    let key_text = text[..equals].trim();
    if key_text.is_empty() || !key_text.chars().all(is_bare_key_char) {
        return None;
    }
    let key_start = line_offset + trimmed_start + text[..equals].find(key_text)?;
    let key_end = key_start + key_text.len();

    let value_text = text[equals + 1..].trim_start();
    let value_leading = text[equals + 1..].len() - value_text.len();
    let value_offset = line_offset + trimmed_start + equals + 1 + value_leading;
    let (value, _, value_range) = parse_quoted(value_text, value_offset)?;

    Some(ManifestField {
        key: key_text.to_string(),
        value,
        key_span: source.span(TextRange::new(key_start, key_end)),
        value_span: source.span(value_range),
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

fn is_bare_key_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}
