use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_project::classify_companion_source;
use veln_source::{SourceFile, TextRange};

pub fn derive(source: &SourceFile) -> Result<String, Box<Diagnostic>> {
    derive_with_diagnostics(source).map_err(|diagnostics| {
        Box::new(
            diagnostics
                .into_iter()
                .next()
                .expect("failed module derivation should have a diagnostic"),
        )
    })
}

pub(crate) fn derive_with_diagnostics(source: &SourceFile) -> Result<String, Vec<Diagnostic>> {
    let path = source.path().as_str();
    if path.contains("#doctest-") {
        return derive_doctest(source, path);
    }
    if let Some(companion) = classify_companion_source(path) {
        return if companion.chained {
            derive_chained_companion(source, path).map_err(|diagnostic| vec![*diagnostic])
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
) -> Result<String, Vec<Diagnostic>> {
    let target_stem =
        source_path_stem(source, target_path).map_err(|diagnostic| vec![*diagnostic])?;
    let mut segments = validated_segments(source, target_stem, "companion")?;
    let Some(last) = segments.last_mut() else {
        return Err(vec![*invalid_source_path(
            source,
            path,
            "source path segment cannot be used as a module identifier",
        )]);
    };
    *last = format!("{last}__test_companion");
    Ok(segments.join("::"))
}

fn derive_regular(source: &SourceFile, path: &str) -> Result<String, Vec<Diagnostic>> {
    let stem = source_path_stem(source, path).map_err(|diagnostic| vec![*diagnostic])?;
    Ok(validated_segments(source, stem, "regular")?.join("::"))
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

fn validated_segments(
    source: &SourceFile,
    stem: &str,
    source_kind: &'static str,
) -> Result<Vec<String>, Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    let mut segments = Vec::new();
    for (index, segment) in stem.split('/').enumerate() {
        if is_valid_module_segment(segment) {
            segments.push(segment.to_string());
        } else if has_segment_name_characters(segment) {
            diagnostics.push(invalid_source_path_module_case_diagnostic(
                source,
                source_kind,
                segment,
                index,
            ));
        } else {
            diagnostics.push(*invalid_source_path(
                source,
                segment,
                "source path segment cannot be used as a module identifier",
            ));
        }
    }
    if diagnostics.is_empty() {
        Ok(segments)
    } else {
        Err(diagnostics)
    }
}

fn derive_doctest(source: &SourceFile, path: &str) -> Result<String, Vec<Diagnostic>> {
    let Some((source_path, _)) = path.split_once("#doctest-") else {
        return Err(vec![*invalid_source_path(
            source,
            path,
            "doctest source path must carry origin metadata",
        )]);
    };
    let source_stem =
        source_path_stem(source, source_path).map_err(|diagnostic| vec![*diagnostic])?;
    Ok(validated_segments(source, source_stem, "doctest")?.join("::"))
}

fn is_valid_module_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase() && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn has_segment_name_characters(segment: &str) -> bool {
    let mut chars = segment.chars();
    if chars.next().is_none() {
        return false;
    }
    segment.chars().all(|ch| ch.is_alphanumeric() || ch == '_')
}

fn observed_initial(segment: &str) -> &'static str {
    segment.as_bytes().first().map_or("other", |initial| {
        if initial.is_ascii_uppercase() {
            "ascii_uppercase"
        } else if initial.is_ascii_lowercase() {
            "ascii_lowercase"
        } else if *initial == b'_' {
            "underscore"
        } else {
            "other"
        }
    })
}

fn invalid_source_path_module_case_diagnostic(
    source: &SourceFile,
    source_kind: &'static str,
    segment: &str,
    segment_index: usize,
) -> Diagnostic {
    Diagnostic::new(
        "name.invalid_case",
        Severity::Error,
        DiagnosticKind::Name,
        format!("module name `{segment}` must start with an ASCII lowercase letter"),
        Some(source.span(TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("origin", JsonValue::string("source_path")),
            ("occurrence", JsonValue::string("path_segment")),
            ("name", JsonValue::string(segment)),
            ("name_class", JsonValue::string("module")),
            ("required_initial", JsonValue::string("ascii_lowercase")),
            (
                "observed_initial",
                JsonValue::string(observed_initial(segment)),
            ),
            ("source_path", JsonValue::string(source.path().as_str())),
            ("source_kind", JsonValue::string(source_kind)),
            ("segment", JsonValue::string(segment)),
            ("segment_index", JsonValue::Number(segment_index as i64)),
        ]),
    )
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
    use veln_diagnostics::diagnostic_to_json;
    use veln_source::SourceFile;

    use super::{derive, derive_with_diagnostics};

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

    #[test]
    fn reports_each_invalid_regular_origin_segment_without_module_identity() {
        let source = SourceFile::new("App/_net/éclair.veln", "");
        let diagnostics = derive_with_diagnostics(&source).expect_err("path should be rejected");

        assert_eq!(diagnostics.len(), 3);
        assert_invalid_case(
            &diagnostics[0],
            "App",
            "ascii_uppercase",
            "regular",
            0,
            "App/_net/éclair.veln",
        );
        assert_invalid_case(
            &diagnostics[1],
            "_net",
            "underscore",
            "regular",
            1,
            "App/_net/éclair.veln",
        );
        assert_invalid_case(
            &diagnostics[2],
            "éclair",
            "other",
            "regular",
            2,
            "App/_net/éclair.veln",
        );
    }

    #[test]
    fn validates_companion_and_doctest_origin_segments_before_synthetic_text() {
        let companion = SourceFile::new("App/math.test.veln", "");
        let doctest = SourceFile::new("App/math.veln#doctest-1_test.veln", "");

        let companion_diagnostics =
            derive_with_diagnostics(&companion).expect_err("companion should be rejected");
        let doctest_diagnostics =
            derive_with_diagnostics(&doctest).expect_err("doctest should be rejected");

        assert_eq!(companion_diagnostics.len(), 1);
        assert_invalid_case(
            &companion_diagnostics[0],
            "App",
            "ascii_uppercase",
            "companion",
            0,
            "App/math.test.veln",
        );
        assert_eq!(doctest_diagnostics.len(), 1);
        assert_invalid_case(
            &doctest_diagnostics[0],
            "App",
            "ascii_uppercase",
            "doctest",
            0,
            "App/math.veln#doctest-1_test.veln",
        );
    }

    #[test]
    fn chained_companion_keeps_structural_recovery_identity() {
        let source = SourceFile::new("App/_math.test.test.veln", "");
        let module = derive_with_diagnostics(&source)
            .expect("chained companion should skip origin casing validation");

        assert_eq!(
            module,
            "App__chained_companion::_math_test_test__chained_companion"
        );
    }

    fn assert_invalid_case(
        diagnostic: &veln_diagnostics::Diagnostic,
        segment: &str,
        observed_initial: &str,
        source_kind: &str,
        segment_index: usize,
        source_path: &str,
    ) {
        assert_eq!(diagnostic.id, "name.invalid_case");
        assert_eq!(
            diagnostic.message,
            format!("module name `{segment}` must start with an ASCII lowercase letter")
        );
        let span = diagnostic.span.as_ref().expect("span");
        assert_eq!(span.start.offset, 0);
        assert_eq!(span.end.offset, 0);
        assert_eq!(span.start.line, 1);
        assert_eq!(span.start.column, 1);
        assert_eq!(span.end.line, 1);
        assert_eq!(span.end.column, 1);
        assert_eq!(
            diagnostic_to_json(diagnostic).to_json(),
            format!(
                "{{\"id\":\"name.invalid_case\",\"severity\":\"error\",\"kind\":\"name\",\
                 \"message\":\"module name `{segment}` must start with an ASCII lowercase letter\",\
                 \"span\":{{\"file\":\"{source_path}\",\"start\":{{\"line\":1,\"column\":1,\
                 \"offset\":0}},\"end\":{{\"line\":1,\"column\":1,\"offset\":0}}}},\
                 \"details\":{{\"phase\":\"name\",\"origin\":\"source_path\",\
                 \"occurrence\":\"path_segment\",\"name\":\"{segment}\",\
                 \"name_class\":\"module\",\"required_initial\":\"ascii_lowercase\",\
                 \"observed_initial\":\"{observed_initial}\",\"source_path\":\"{source_path}\",\
                 \"source_kind\":\"{source_kind}\",\"segment\":\"{segment}\",\
                 \"segment_index\":{segment_index}}},\"related\":[]}}"
            )
        );
    }
}
