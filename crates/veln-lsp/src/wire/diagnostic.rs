use std::collections::BTreeMap;

use veln_diagnostics::{Diagnostic, JsonValue};
use veln_source::{SourceFile, SourceSpan};

use super::{display_path, escape_json, range_json, severity_code};
use crate::diagnostics;

#[derive(Clone, Debug)]
pub(crate) struct DiagnosticSource {
    uri: String,
    text: String,
    line_starts: Vec<usize>,
}

impl DiagnosticSource {
    pub(crate) fn new(uri: String, text: String) -> Self {
        let line_starts = if text.is_empty() {
            Vec::new()
        } else {
            let mut starts = vec![0];
            starts.extend(text.bytes().enumerate().filter_map(|(index, byte)| {
                let next = index + 1;
                (byte == b'\n' && next < text.len()).then_some(next)
            }));
            starts
        };
        Self {
            uri,
            text,
            line_starts,
        }
    }

    pub(crate) fn utf16_position_json(&self, line: usize, column: usize) -> String {
        let character = self
            .line_text(line)
            .map(|text| {
                text.chars()
                    .take(column.saturating_sub(1))
                    .map(char::len_utf16)
                    .sum()
            })
            .unwrap_or_else(|| column.saturating_sub(1));
        format!(
            "{{\"line\":{},\"character\":{character}}}",
            line.saturating_sub(1),
        )
    }

    fn line_text(&self, line: usize) -> Option<&str> {
        let start = *self.line_starts.get(line.saturating_sub(1))?;
        let rest = self.text.get(start..)?;
        let line = rest.find('\n').map_or(rest, |end| &rest[..end]);
        line.strip_suffix('\r').or(Some(line))
    }
}

pub(crate) type DiagnosticSources = BTreeMap<String, DiagnosticSource>;

pub(crate) fn publish_diagnostics(uri: &str, text: String) -> String {
    let path = display_path(uri);
    let source = SourceFile::new(path.clone(), text.clone());
    let diagnostics = diagnostics(&source)
        .into_iter()
        .filter(|diagnostic| diagnostic_applies_to_uri(diagnostic, uri))
        .collect::<Vec<_>>();
    let sources = BTreeMap::from([(path, DiagnosticSource::new(uri.to_string(), text))]);
    publish_diagnostics_for_uri(uri, &diagnostics, &sources)
}

pub(crate) fn publish_diagnostics_for_uri(
    uri: &str,
    diagnostics: &[Diagnostic],
    sources: &DiagnosticSources,
) -> String {
    let diagnostics = diagnostics
        .iter()
        .map(|diagnostic| lsp_diagnostic_json(diagnostic, sources))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{{\"uri\":\"{}\",\"diagnostics\":[{diagnostics}]}}}}",
        escape_json(uri)
    )
}

pub(crate) fn empty_publish_diagnostics(uri: &str) -> String {
    format!(
        "{{\"jsonrpc\":\"2.0\",\"method\":\"textDocument/publishDiagnostics\",\"params\":{{\"uri\":\"{}\",\"diagnostics\":[]}}}}",
        escape_json(uri)
    )
}

pub(crate) fn diagnostic_applies_to_uri(diagnostic: &Diagnostic, uri: &str) -> bool {
    diagnostic
        .span
        .as_ref()
        .is_none_or(|span| span.file.as_str() == display_path(uri))
}

pub(crate) fn lsp_diagnostic_json(diagnostic: &Diagnostic, sources: &DiagnosticSources) -> String {
    let data = lsp_diagnostic_data_json(diagnostic)
        .map(|data| format!(",\"data\":{data}"))
        .unwrap_or_default();
    let related_information = lsp_related_information_json(diagnostic, sources)
        .map(|related| format!(",\"relatedInformation\":[{related}]"))
        .unwrap_or_default();
    format!(
        "{{\"range\":{},\"severity\":{},\"code\":\"{}\",\"source\":\"veln\",\"message\":\"{}\"{related_information}{data}}}",
        diagnostic_range_json(diagnostic.span.as_ref(), sources),
        severity_code(diagnostic.severity),
        escape_json(&diagnostic.id),
        escape_json(diagnostic.message.as_str()),
    )
}

fn diagnostic_range_json(span: Option<&SourceSpan>, sources: &DiagnosticSources) -> String {
    let Some(span) = span else {
        return range_json(None);
    };
    let Some(source) = sources.get(span.file.as_str()) else {
        return range_json(Some(span));
    };
    format!(
        "{{\"start\":{},\"end\":{}}}",
        source.utf16_position_json(span.start.line, span.start.column),
        source.utf16_position_json(span.end.line, span.end.column),
    )
}

fn lsp_related_information_json(
    diagnostic: &Diagnostic,
    sources: &DiagnosticSources,
) -> Option<String> {
    let related = diagnostic
        .related
        .iter()
        .filter_map(|note| {
            let message = object_text(note, "message")?;
            let span = object_value(note, "span")?;
            let file = object_text(span, "file")?;
            let source = sources.get(file)?;
            let start = object_value(span, "start")?;
            let end = object_value(span, "end")?;
            let start_line = object_usize(start, "line")?;
            let start_column = object_usize(start, "column")?;
            let end_line = object_usize(end, "line")?;
            let end_column = object_usize(end, "column")?;
            let range = format!(
                "{{\"start\":{},\"end\":{}}}",
                source.utf16_position_json(start_line, start_column),
                source.utf16_position_json(end_line, end_column),
            );
            Some(format!(
                "{{\"location\":{{\"uri\":\"{}\",\"range\":{range}}},\"message\":\"{}\"}}",
                escape_json(&source.uri),
                escape_json(message),
            ))
        })
        .collect::<Vec<_>>();
    (!related.is_empty()).then(|| related.join(","))
}

fn object_value<'a>(value: &'a JsonValue, key: &str) -> Option<&'a JsonValue> {
    value
        .as_object()?
        .iter()
        .find_map(|(candidate, value)| (candidate == key).then_some(value))
}

fn object_text<'a>(value: &'a JsonValue, key: &str) -> Option<&'a str> {
    object_value(value, key)?.as_text()
}

fn object_usize(value: &JsonValue, key: &str) -> Option<usize> {
    let JsonValue::Number(value) = object_value(value, key)? else {
        return None;
    };
    usize::try_from(*value).ok()
}

fn lsp_diagnostic_data_json(diagnostic: &Diagnostic) -> Option<String> {
    if diagnostic.id != "name.invalid_case" {
        return None;
    }
    if detail_string(diagnostic, "origin")? != "source_path" {
        return None;
    }
    Some(
        JsonValue::object([
            ("origin", detail(diagnostic, "origin")?.clone()),
            ("occurrence", detail(diagnostic, "occurrence")?.clone()),
            ("source_path", detail(diagnostic, "source_path")?.clone()),
            ("source_kind", detail(diagnostic, "source_kind")?.clone()),
            ("segment", detail(diagnostic, "segment")?.clone()),
            (
                "segment_index",
                detail(diagnostic, "segment_index")?.clone(),
            ),
        ])
        .to_json(),
    )
}

fn detail<'a>(diagnostic: &'a Diagnostic, key: &str) -> Option<&'a JsonValue> {
    let JsonValue::Object(entries) = &diagnostic.details else {
        return None;
    };
    entries
        .iter()
        .find_map(|(candidate, value)| (candidate == key).then_some(value))
}

fn detail_string<'a>(diagnostic: &'a Diagnostic, key: &str) -> Option<&'a str> {
    detail(diagnostic, key)?.as_text()
}

pub(crate) fn diagnostics_by_path(
    diagnostics: Vec<Diagnostic>,
) -> BTreeMap<String, Vec<Diagnostic>> {
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
