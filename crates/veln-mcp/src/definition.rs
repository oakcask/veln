use std::path::Path;

use serde_json::{Value, json};
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use veln_language_service::{EffectiveProjectSnapshot, NavigationSource, SourcePosition, navigate};
use veln_source::SourcePath;

use crate::check_project::{CheckProjectOutcome, capture_navigation_source};
use crate::workspace::{Selection, WorkspaceBase};

#[derive(Clone, Copy)]
enum Coordinate {
    Addressable(usize),
    OutOfRange,
}

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
    let line = coordinate(&arguments["line"]);
    let column = coordinate(&arguments["column"]);
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
            json!({"source": source, "line": arguments["line"].clone(), "column": arguments["column"].clone()}),
        );
    }
    let (Coordinate::Addressable(line), Coordinate::Addressable(column)) = (line, column) else {
        unreachable!("valid positions are addressable")
    };

    let root = captured.project.root.clone();
    let snapshot = EffectiveProjectSnapshot::new(captured.project.files);
    let result = navigate(
        &snapshot,
        SourcePosition {
            source: SourcePath::new(captured_source),
            line,
            column,
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

fn coordinate(value: &Value) -> Coordinate {
    let number = value
        .as_number()
        .expect("definition input schema requires a positive integer coordinate");
    match parse_coordinate_integer(&number.to_string()) {
        Some(value) if value >= 1 => Coordinate::Addressable(value),
        _ => Coordinate::OutOfRange,
    }
}

fn parse_coordinate_integer(text: &str) -> Option<usize> {
    if text.starts_with('-') {
        return None;
    }
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(index) => (&text[..index], parse_json_exponent(&text[index + 1..])?),
        None => (text, 0),
    };
    let (integer, fraction) = match mantissa.split_once('.') {
        Some((integer, fraction)) => (integer, fraction),
        None => (mantissa, ""),
    };
    let mut digits = String::with_capacity(integer.len() + fraction.len());
    digits.push_str(integer);
    digits.push_str(fraction);
    let scale = i64::try_from(fraction.len())
        .ok()
        .and_then(|fraction_len| fraction_len.checked_sub(exponent))?;
    if scale > 0 {
        let scale = scale as usize;
        if scale > digits.len()
            || !digits[digits.len() - scale..]
                .bytes()
                .all(|byte| byte == b'0')
        {
            return None;
        }
        digits.truncate(digits.len() - scale);
    }
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() {
        return Some(0);
    }
    let trailing_zeros = if scale < 0 {
        usize::try_from(scale.unsigned_abs()).ok()?
    } else {
        0
    };
    if trimmed.len() + trailing_zeros > usize::MAX.to_string().len() {
        return None;
    }
    let mut parsed = trimmed.bytes().try_fold(0usize, |parsed, byte| {
        parsed.checked_mul(10)?.checked_add((byte - b'0') as usize)
    })?;
    for _ in 0..trailing_zeros {
        parsed = parsed.checked_mul(10)?;
    }
    Some(parsed)
}

fn parse_json_exponent(text: &str) -> Option<i64> {
    let digits = match text.as_bytes().first() {
        Some(b'+') | Some(b'-') => &text[1..],
        _ => text,
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse::<i64>().ok()
}

fn valid_position(text: &str, line: Coordinate, column: Coordinate) -> bool {
    let (Coordinate::Addressable(line), Coordinate::Addressable(column)) = (line, column) else {
        return false;
    };
    let mut lines = text.split('\n').peekable();
    let Some(selected_line) = lines.nth(line.saturating_sub(1)) else {
        return false;
    };
    let selected_line = if lines.peek().is_some() {
        selected_line.strip_suffix('\r').unwrap_or(selected_line)
    } else {
        selected_line
    };
    column <= selected_line.chars().count() + 1
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
    use super::{Coordinate, coordinate, valid_position};
    use serde_json::json;

    #[test]
    fn coordinates_accept_integral_json_number_spellings() {
        for value in [json!(1), json!(1.0), json!(1e0)] {
            assert!(matches!(coordinate(&value), Coordinate::Addressable(1)));
        }
        let non_integer = serde_json::from_str("6.0000000000000001").unwrap();
        assert!(matches!(coordinate(&non_integer), Coordinate::OutOfRange));
        let above_u64 = serde_json::from_str("18446744073709551616").unwrap();
        assert!(matches!(coordinate(&above_u64), Coordinate::OutOfRange));
    }

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
                valid_position(
                    text,
                    Coordinate::Addressable(line),
                    Coordinate::Addressable(column)
                ),
                expected,
                "{text:?} {line}:{column}"
            );
        }
        assert!(!valid_position(
            "x",
            Coordinate::OutOfRange,
            Coordinate::Addressable(1)
        ));
    }
}
