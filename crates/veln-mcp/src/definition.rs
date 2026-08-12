use std::path::Path;

use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use veln_language_service::{EffectiveProjectSnapshot, NavigationSource, SourcePosition, navigate};
use veln_source::SourcePath;

use crate::check_project::{CheckProjectOutcome, capture_navigation_source};
use crate::workspace::{Selection, WorkspaceBase};

pub(crate) enum DefinitionOutcome {
    Success(Value),
    DomainFailure {
        code: &'static str,
        message: &'static str,
        details: Value,
    },
}

pub(crate) fn definition(
    base: &WorkspaceBase,
    selection: &Selection,
    arguments: &Value,
) -> DefinitionOutcome {
    let source = arguments["source"]
        .as_str()
        .expect("definition input schema requires a string source");
    let line = arguments["line"]
        .as_u64()
        .expect("definition input schema requires a positive line");
    let column = arguments["column"]
        .as_u64()
        .expect("definition input schema requires a positive column");
    let (captured, captured_source) = match capture_navigation_source(base, selection, source) {
        Ok(captured) => captured,
        Err(failure) => return failure.into(),
    };
    let source_file = captured
        .project
        .files
        .iter()
        .find(|file| file.path().as_str() == captured_source)
        .expect("navigation capture contains the requested source");
    if !valid_position(source_file.text(), line, column) {
        return domain_failure(
            "invalid_position",
            "position is outside the selected source",
            json!({"source": source, "line": line, "column": column}),
        );
    }

    let root = captured.project.root.clone();
    let snapshot = EffectiveProjectSnapshot::new(captured.project.files);
    let result = navigate(
        &snapshot,
        SourcePosition {
            source: SourcePath::new(captured_source),
            line: line
                .try_into()
                .expect("valid definition line should fit in usize"),
            column: column
                .try_into()
                .expect("valid definition column should fit in usize"),
        },
    );
    let definition = result.and_then(|result| match result.definition.source {
        NavigationSource::Workspace => Some(json!({
            "uri": path_to_uri(&root.join(result.definition.span.file.as_str())),
            "range": {
                "start": {
                    "line": result.definition.span.start.line,
                    "column": result.definition.span.start.column
                },
                "end": {
                    "line": result.definition.span.end.line,
                    "column": result.definition.span.end.column
                }
            }
        })),
        NavigationSource::Package { .. } => None,
    });
    DefinitionOutcome::Success(json!({"definition": definition}))
}

fn valid_position(text: &str, line: u64, column: u64) -> bool {
    let Ok(line_index) = usize::try_from(line.saturating_sub(1)) else {
        return false;
    };
    let mut lines = text.split('\n').peekable();
    let Some(selected_line) = lines.nth(line_index) else {
        return false;
    };
    let selected_line = if lines.peek().is_some() {
        selected_line.strip_suffix('\r').unwrap_or(selected_line)
    } else {
        selected_line
    };
    column <= selected_line.chars().count() as u64 + 1
}

fn path_to_uri(path: &Path) -> String {
    #[cfg(unix)]
    let bytes = path.as_os_str().as_bytes();
    #[cfg(not(unix))]
    let path_text = path.to_string_lossy();
    #[cfg(not(unix))]
    let bytes = path_text.as_bytes();
    let mut encoded = String::new();
    for &byte in bytes {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' => {
                encoded.push(byte as char);
            }
            byte => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    format!("file://{encoded}")
}

fn domain_failure(code: &'static str, message: &'static str, details: Value) -> DefinitionOutcome {
    DefinitionOutcome::DomainFailure {
        code,
        message,
        details,
    }
}

impl From<CheckProjectOutcome> for DefinitionOutcome {
    fn from(outcome: CheckProjectOutcome) -> Self {
        match outcome {
            CheckProjectOutcome::DomainFailure {
                code,
                message,
                details,
            } => Self::DomainFailure {
                code,
                message,
                details,
            },
            CheckProjectOutcome::Success(_) => {
                unreachable!("navigation capture failures are domain failures")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::valid_position;

    #[test]
    fn positions_follow_lf_crlf_terminal_empty_and_scalar_rules() {
        let cases = [
            ("", (1, 1), true),
            ("", (1, 2), false),
            ("a\n", (1, 2), true),
            ("a\n", (2, 1), true),
            ("a\n", (2, 2), false),
            ("a\r\n", (1, 2), true),
            ("a\r\n", (1, 3), false),
            ("a\r", (1, 3), true),
            ("a\r", (1, 4), false),
            ("😀x", (1, 3), true),
            ("😀x", (1, 4), false),
            ("x", (2, 1), false),
        ];
        for (text, (line, column), expected) in cases {
            assert_eq!(
                valid_position(text, line, column),
                expected,
                "{text:?} {line}:{column}"
            );
        }
    }
}
