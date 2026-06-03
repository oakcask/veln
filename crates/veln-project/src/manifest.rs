use std::io;
use std::path::Path;

use veln_source::{SourceFile, SourcePath, SourceSpan, TextRange};

#[derive(Clone, Debug)]
pub struct ProjectManifest {
    pub path: SourcePath,
    pub package: ManifestPackage,
    pub lib: ManifestLib,
    pub unsupported_sections: Vec<ManifestUnsupportedSection>,
    pub tools: Vec<ManifestTool>,
}

#[derive(Clone, Debug, Default)]
pub struct ManifestPackage {
    pub fields: Vec<ManifestField>,
}

#[derive(Clone, Debug)]
pub struct ManifestLib {
    pub exports: Vec<ManifestExport>,
}

#[derive(Clone, Debug)]
pub struct ManifestExport {
    pub path: String,
    pub path_span: SourceSpan,
}

#[derive(Clone, Debug)]
pub struct ManifestUnsupportedSection {
    pub name: String,
    pub span: SourceSpan,
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
    let mut lib = ManifestLib {
        exports: Vec::new(),
    };
    let mut unsupported_sections = Vec::new();
    let mut tools = Vec::<ManifestTool>::new();
    let mut section = ManifestSection::Other;
    let mut array = ManifestArray::None;
    let mut offset = 0;

    for line in source.text().split_inclusive('\n') {
        let line_without_newline = line.trim_end_matches(['\r', '\n']);
        if matches!(array, ManifestArray::LibExports) {
            if parse_export_array_items(source, offset, line_without_newline, &mut lib.exports) {
                array = ManifestArray::None;
            }
        } else if let Some((section_name, name_start, name_end)) =
            section_header(line_without_newline, offset)
        {
            section = ManifestSection::from_name(section_name);
            if matches!(section, ManifestSection::Modules) {
                unsupported_sections.push(ManifestUnsupportedSection {
                    name: section_name.to_string(),
                    span: source.span(TextRange::new(name_start, name_end)),
                });
            }
        } else if matches!(section, ManifestSection::Lib) {
            array = parse_lib_entry(source, offset, line_without_newline, &mut lib.exports);
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
        lib,
        unsupported_sections,
        tools,
    }
}

#[derive(Clone)]
enum ManifestSection {
    Package,
    Lib,
    Modules,
    Tool(String),
    Other,
}

impl ManifestSection {
    fn from_name(name: &str) -> Self {
        if name == "package" {
            Self::Package
        } else if name == "lib" {
            Self::Lib
        } else if name == "modules" {
            Self::Modules
        } else if let Some(tool_name) = name.strip_prefix("tool.") {
            Self::Tool(tool_name.to_string())
        } else {
            Self::Other
        }
    }
}

#[derive(Clone, Copy)]
enum ManifestArray {
    LibExports,
    None,
}

fn section_header(line: &str, line_offset: usize) -> Option<(&str, usize, usize)> {
    let trimmed_start = line.len() - line.trim_start().len();
    let text = line.trim_start();
    let rest = text.strip_prefix('[')?;
    let end = rest.find(']')?;
    let start = line_offset + trimmed_start + 1;
    Some((&rest[..end], start, start + end))
}

fn parse_lib_entry(
    source: &SourceFile,
    line_offset: usize,
    line: &str,
    exports: &mut Vec<ManifestExport>,
) -> ManifestArray {
    let text = line.trim();
    if text.is_empty() || text.starts_with('#') {
        return ManifestArray::None;
    }

    let Some((key, after_equals, value_offset)) = parse_field_start(line_offset, line) else {
        return ManifestArray::None;
    };
    if key != "exports" {
        return ManifestArray::None;
    }
    let Some(open) = after_equals.find('[') else {
        return ManifestArray::None;
    };
    let item_offset = value_offset + open + 1;
    let closed = parse_export_array_items(source, item_offset, &after_equals[open + 1..], exports);
    if closed {
        ManifestArray::None
    } else {
        ManifestArray::LibExports
    }
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

    let (key_text, value_text, value_offset) = parse_field_start(line_offset, line)?;
    let key_start = line_offset + trimmed_start + text.find(key_text)?;
    let key_end = key_start + key_text.len();
    let (value, _, value_range) = parse_quoted(value_text, value_offset)?;

    Some(ManifestField {
        key: key_text.to_string(),
        value,
        key_span: source.span(TextRange::new(key_start, key_end)),
        value_span: source.span(value_range),
    })
}

fn parse_field_start(line_offset: usize, line: &str) -> Option<(&str, &str, usize)> {
    let trimmed_start = line.len() - line.trim_start().len();
    let text = line.trim();
    let equals = text.find('=')?;
    let key_text = text[..equals].trim();
    if key_text.is_empty() || !key_text.chars().all(is_bare_key_char) {
        return None;
    }

    let value_text = text[equals + 1..].trim_start();
    let value_leading = text[equals + 1..].len() - value_text.len();
    let value_offset = line_offset + trimmed_start + equals + 1 + value_leading;
    Some((key_text, value_text, value_offset))
}

fn parse_export_array_items(
    source: &SourceFile,
    absolute_offset: usize,
    text: &str,
    exports: &mut Vec<ManifestExport>,
) -> bool {
    let mut index = 0;
    while index < text.len() {
        let remaining = &text[index..];
        if remaining.starts_with('#') {
            return false;
        }
        if remaining.starts_with(']') {
            return true;
        }
        if remaining.starts_with('"') {
            if let Some((path, after_path, path_range)) =
                parse_quoted(remaining, absolute_offset + index)
            {
                exports.push(ManifestExport {
                    path,
                    path_span: source.span(path_range),
                });
                index = text.len() - after_path.len();
                continue;
            }
            return false;
        }
        index += 1;
    }
    false
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
