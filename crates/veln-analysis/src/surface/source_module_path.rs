use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_project::classify_companion_source;
use veln_source::{SourceFile, TextRange};

pub fn derive(source: &SourceFile) -> Result<String, Box<Diagnostic>> {
    let path = source.path().as_str();
    if let Some(module_name) = derive_doctest(path) {
        return Ok(module_name);
    }
    if let Some(companion) = classify_companion_source(path) {
        return if companion.chained {
            derive_chained_companion(source, path)
        } else {
            derive_test_companion(source, path, &companion.target_path)
        };
    }
    derive_regular(source, path)
}

fn derive_chained_companion(source: &SourceFile, path: &str) -> Result<String, Box<Diagnostic>> {
    let stem = source_path_stem(source, path)?;
    Ok(stem
        .split('/')
        .map(|segment| {
            let sanitized = segment
                .chars()
                .map(|ch| {
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        ch
                    } else {
                        '_'
                    }
                })
                .collect::<String>();
            format!("{sanitized}__chained_companion")
        })
        .collect::<Vec<_>>()
        .join("::"))
}

fn derive_test_companion(
    source: &SourceFile,
    path: &str,
    target_path: &str,
) -> Result<String, Box<Diagnostic>> {
    let target_stem = source_path_stem(source, target_path)?;
    let mut segments = validated_segments(source, target_stem)?;
    let Some(last) = segments.last_mut() else {
        return Err(invalid_source_path(
            source,
            path,
            "source path segment cannot be used as a module identifier",
        ));
    };
    *last = format!("{last}__test_companion");
    Ok(segments.join("::"))
}

fn derive_regular(source: &SourceFile, path: &str) -> Result<String, Box<Diagnostic>> {
    let stem = source_path_stem(source, path)?;
    Ok(validated_segments(source, stem)?.join("::"))
}

fn source_path_stem<'a>(source: &SourceFile, path: &'a str) -> Result<&'a str, Box<Diagnostic>> {
    path.strip_suffix(".veln").ok_or_else(|| {
        invalid_source_path(
            source,
            path,
            "source module files must use the `.veln` extension",
        )
    })
}

fn validated_segments(source: &SourceFile, stem: &str) -> Result<Vec<String>, Box<Diagnostic>> {
    stem.split('/')
        .map(|segment| {
            if is_module_identifier(segment) {
                Ok(segment.to_string())
            } else {
                Err(invalid_source_path(
                    source,
                    segment,
                    "source path segment cannot be used as a module identifier",
                ))
            }
        })
        .collect()
}

fn derive_doctest(path: &str) -> Option<String> {
    let (source_path, _) = path.split_once("#doctest-")?;
    let source_stem = source_path.strip_suffix(".veln")?;
    let mut segments = Vec::new();
    for segment in source_stem.split('/') {
        if is_module_identifier(segment) {
            segments.push(segment.to_string());
        } else {
            return None;
        }
    }
    Some(segments.join("::"))
}

fn is_module_identifier(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn invalid_source_path(
    source: &SourceFile,
    segment: &str,
    message: &'static str,
) -> Box<Diagnostic> {
    Box::new(Diagnostic::new(
        "module.invalid_source_path",
        Severity::Error,
        DiagnosticKind::Module,
        format!("{message}: `{segment}`"),
        Some(source.span(TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("source_path", JsonValue::string(source.path().as_str())),
            ("segment", JsonValue::string(segment)),
        ]),
    ))
}

#[cfg(test)]
mod tests {
    use veln_source::SourceFile;

    use super::derive;

    #[test]
    fn preserves_each_source_kind() {
        let cases = [
            ("app/math.veln", "app::math"),
            ("app/math.veln#doctest-example", "app::math"),
            ("app/math.test.veln", "app::math__test_companion"),
            (
                "app/math.test.test.veln",
                "app__chained_companion::math_test_test__chained_companion",
            ),
        ];

        for (path, expected) in cases {
            let source = SourceFile::new(path, "");
            assert_eq!(
                derive(&source).unwrap_or_else(|diagnostic| panic!("{diagnostic:#?}")),
                expected,
                "unexpected derived module for {path}"
            );
        }
    }
}
