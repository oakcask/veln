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
    derive_visible_with_diagnostics(source).and_then(|module| {
        module.ok_or_else(|| {
            vec![*invalid_source_path(
                source,
                source.path().as_str(),
                "generated source has no source-visible module origin",
            )]
        })
    })
}

pub(crate) fn derive_visible_with_diagnostics(
    source: &SourceFile,
) -> Result<Option<String>, Vec<Diagnostic>> {
    derive_visible_with_source_kind(source, "regular")
}

pub(crate) fn derive_visible_with_source_kind(
    source: &SourceFile,
    regular_source_kind: &'static str,
) -> Result<Option<String>, Vec<Diagnostic>> {
    if let Some(origin_path) = source.generated_origin_path() {
        return origin_path.map_or(Ok(None), |origin_path| {
            derive_generated(source, origin_path.as_str()).map(Some)
        });
    }
    let path = source.path().as_str();
    if path.contains("#doctest-") {
        return derive_doctest(source, path).map(Some);
    }
    if let Some(companion) = classify_companion_source(path) {
        return if companion.chained {
            Ok(None)
        } else {
            derive_test_companion(source, path, &companion.target_path).map(Some)
        };
    }
    derive_regular(source, path, regular_source_kind).map(Some)
}

pub(crate) fn invalid_case_rejected_visible_module_path(source: &SourceFile) -> Option<String> {
    let Err(diagnostics) = derive_visible_with_diagnostics(source) else {
        return None;
    };
    diagnostics
        .iter()
        .all(is_source_path_invalid_case_diagnostic)
        .then(|| unvalidated_visible_module_path(source))
        .flatten()
}

fn is_source_path_invalid_case_diagnostic(diagnostic: &Diagnostic) -> bool {
    diagnostic.id == "name.invalid_case"
        && json_string_field(&diagnostic.details, "origin") == Some("source_path")
}

fn json_string_field<'a>(value: &'a JsonValue, key: &str) -> Option<&'a str> {
    let JsonValue::Object(fields) = value else {
        return None;
    };
    fields.iter().find_map(|(field, value)| {
        if field == key {
            match value {
                JsonValue::String(value) => Some(value.as_str()),
                _ => None,
            }
        } else {
            None
        }
    })
}

fn unvalidated_visible_module_path(source: &SourceFile) -> Option<String> {
    if let Some(origin_path) = source.generated_origin_path() {
        return origin_path
            .and_then(|origin_path| unvalidated_regular_module_path(origin_path.as_str()));
    }
    let path = source.path().as_str();
    if path.contains("#doctest-") {
        return path
            .split_once("#doctest-")
            .and_then(|(source_path, _)| unvalidated_regular_module_path(source_path));
    }
    if let Some(companion) = classify_companion_source(path) {
        if companion.chained {
            return None;
        }
        let mut module_path = unvalidated_regular_module_path(&companion.target_path)?;
        module_path.push_str("__test_companion");
        return Some(module_path);
    }
    unvalidated_regular_module_path(path)
}

fn unvalidated_regular_module_path(path: &str) -> Option<String> {
    path.strip_suffix(".veln")
        .map(|stem| stem.replace('/', "::"))
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

fn derive_regular(
    source: &SourceFile,
    path: &str,
    source_kind: &'static str,
) -> Result<String, Vec<Diagnostic>> {
    let stem = source_path_stem(source, path).map_err(|diagnostic| vec![*diagnostic])?;
    Ok(validated_segments(source, stem, source_kind)?.join("::"))
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
        } else if has_invalid_module_initial(segment) {
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

fn derive_generated(source: &SourceFile, origin_path: &str) -> Result<String, Vec<Diagnostic>> {
    let source_stem =
        source_path_stem(source, origin_path).map_err(|diagnostic| vec![*diagnostic])?;
    Ok(validated_segments(source, source_stem, "generated")?.join("::"))
}

fn is_valid_module_segment(segment: &str) -> bool {
    let mut chars = segment.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase() && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn has_invalid_module_initial(segment: &str) -> bool {
    segment
        .as_bytes()
        .first()
        .is_some_and(|initial| !initial.is_ascii_lowercase())
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
    use veln_source::{SourceFile, SourcePath};

    use super::{
        derive, derive_visible_with_diagnostics, derive_visible_with_source_kind,
        derive_with_diagnostics,
    };

    #[test]
    fn preserves_each_source_kind() {
        let cases = [
            ("app/math.veln", "app::math"),
            ("app/math.veln#doctest-example", "app::math"),
            ("app/math.test.veln", "app::math__test_companion"),
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
    fn validates_generated_origin_metadata_and_skips_source_less_generated_modules() {
        let generated = SourceFile::generated(
            "target/bookkeeping.veln",
            "",
            Some(SourcePath::new("App/_net.veln")),
        );
        let diagnostics =
            derive_visible_with_diagnostics(&generated).expect_err("origin should be rejected");

        assert_eq!(diagnostics.len(), 2);
        assert_invalid_case(
            &diagnostics[0],
            "App",
            "ascii_uppercase",
            "generated",
            0,
            "target/bookkeeping.veln",
        );
        assert_invalid_case(
            &diagnostics[1],
            "_net",
            "underscore",
            "generated",
            1,
            "target/bookkeeping.veln",
        );

        let generated = SourceFile::generated("target/bookkeeping.veln", "", None::<SourcePath>);
        assert_eq!(
            derive_visible_with_diagnostics(&generated).expect("source-less generated source"),
            None
        );
    }

    #[test]
    fn generated_origin_precedes_export_source_kind() {
        let generated = SourceFile::generated(
            "Target/_bookkeeping.veln",
            "",
            Some(SourcePath::new("src/generated_api.veln")),
        );
        assert_eq!(
            derive_visible_with_source_kind(&generated, "export")
                .expect("generated origin should derive")
                .as_deref(),
            Some("src::generated_api")
        );

        let generated = SourceFile::generated(
            "target/bookkeeping.veln",
            "",
            Some(SourcePath::new("App/generated_api.veln")),
        );
        let diagnostics = derive_visible_with_source_kind(&generated, "export")
            .expect_err("invalid origin should be rejected");

        assert_eq!(diagnostics.len(), 1);
        assert_invalid_case(
            &diagnostics[0],
            "App",
            "ascii_uppercase",
            "generated",
            0,
            "target/bookkeeping.veln",
        );
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
    fn preserves_structural_source_path_errors_after_valid_module_initial() {
        let source = SourceFile::new("appé.veln", "");
        let diagnostics = derive_with_diagnostics(&source).expect_err("path should be rejected");

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "module.invalid_source_path");
        assert_eq!(
            diagnostics[0].message,
            "source path segment cannot be used as a module identifier: `appé`"
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
    fn chained_companion_has_no_visible_module_identity() {
        let source = SourceFile::new("App/_math.test.test.veln", "");

        assert_eq!(
            derive_visible_with_diagnostics(&source).expect("chained companion"),
            None
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
