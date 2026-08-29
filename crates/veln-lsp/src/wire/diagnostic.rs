use std::collections::BTreeMap;

use veln_diagnostics::{Diagnostic, JsonValue};
use veln_source::SourceFile;

use super::{display_path, escape_json, range_json, severity_code};
use crate::diagnostics;

pub(crate) fn publish_diagnostics(uri: &str, text: String) -> String {
    let source = SourceFile::new(display_path(uri), text);
    let diagnostics = diagnostics(&source)
        .into_iter()
        .filter(|diagnostic| diagnostic_applies_to_uri(diagnostic, uri))
        .collect::<Vec<_>>();
    publish_diagnostics_for_uri(uri, &diagnostics)
}

pub(crate) fn publish_diagnostics_for_uri(uri: &str, diagnostics: &[Diagnostic]) -> String {
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

pub(crate) fn lsp_diagnostic_json(diagnostic: &Diagnostic) -> String {
    let data = lsp_diagnostic_data_json(diagnostic)
        .map(|data| format!(",\"data\":{data}"))
        .unwrap_or_default();
    format!(
        "{{\"range\":{},\"severity\":{},\"code\":\"{}\",\"source\":\"veln\",\"message\":\"{}\"{data}}}",
        range_json(diagnostic.span.as_ref()),
        severity_code(diagnostic.severity),
        escape_json(&diagnostic.id),
        escape_json(&diagnostic.message),
    )
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
    let JsonValue::String(value) = detail(diagnostic, key)? else {
        return None;
    };
    Some(value)
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
