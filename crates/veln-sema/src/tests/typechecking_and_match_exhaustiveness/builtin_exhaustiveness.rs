use super::*;

#[test]
fn match_exhaustiveness_accepts_finite_builtin_domains() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn bool_label(value: Bool) -> String\n",
            "  match value\n",
            "    true => \"true\"\n",
            "    false => \"false\"\n",
            "  end\n",
            "end\n",
            "fn option_label(value: Option<Int>) -> String\n",
            "  match value\n",
            "    Some(_) => \"some\"\n",
            "    None => \"none\"\n",
            "  end\n",
            "end\n",
            "fn result_label(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Ok(_) => \"ok\"\n",
            "    Err(_) => \"err\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.core.is_some());
    assert!(lowered.ir.is_some());
}

#[test]
fn match_exhaustiveness_accepts_catch_all_patterns() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn wildcard(value: Option<Int>) -> String\n",
            "  match value\n",
            "    Some(_) => \"some\"\n",
            "    _ => \"fallback\"\n",
            "  end\n",
            "end\n",
            "fn binding(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Ok(_) => \"ok\"\n",
            "    other => \"fallback\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn match_exhaustiveness_reports_missing_bool_case() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Bool) -> String\n",
            "  match value\n",
            "    true => \"yes\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing bool case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case false");
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn match_exhaustiveness_reports_empty_finite_builtin_match() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Bool) -> String\n",
            "  match value\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("empty finite-domain match should be diagnosed");
    assert_eq!(diagnostic.message, "match is missing case false");
    assert_eq!(diagnostic.related.len(), 1);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn match_exhaustiveness_reports_missing_option_case() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option<Int>) -> String\n",
            "  match value\n",
            "    Some(count) => \"some\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing option case should be diagnosed");
    assert_eq!(diagnostic.message, "match is missing case None");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Option<Int>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers Some(_)."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}

#[test]
fn match_exhaustiveness_reports_qualified_option_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option<Int>) -> String\n",
            "  match value\n",
            "    Option::Some(count) => \"some\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing option case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case None");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Option<Int>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers Some(_)."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}

#[test]
fn match_exhaustiveness_reports_qualified_option_none_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option<Int>) -> String\n",
            "  match value\n",
            "    Option::None => \"none\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing option case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Some(_)");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Option<Int>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers None."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}

#[test]
fn match_exhaustiveness_reports_missing_result_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Err(error) => error\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing result case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Ok(_)");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Result<Int, String>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers Err(_)."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}

#[test]
fn match_exhaustiveness_reports_qualified_result_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Result::Err(error) => error\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing result case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Ok(_)");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Result<Int, String>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers Err(_)."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}

#[test]
fn match_exhaustiveness_reports_qualified_result_ok_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Result::Ok(count) => \"ok\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing result case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Err(_)");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Result<Int, String>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers Ok(_)."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}
