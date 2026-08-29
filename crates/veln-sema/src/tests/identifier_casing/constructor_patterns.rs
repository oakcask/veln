use super::*;

#[test]
fn qualified_lowercase_constructor_pattern_reports_leaf_path_segment() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  None\n",
            "  Some(Int)\n",
            "end\n",
            "fn main(input: Item) -> Int\n",
            "  match input\n",
            "    Item::some(value) => value\n",
            "    Item::None => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.invalid_case")
        .expect("qualified constructor casing diagnostic");
    assert_eq!(
        diagnostic.message,
        "constructor name `some` must start with an ASCII uppercase letter"
    );
    let span = diagnostic.span.as_ref().expect("diagnostic span");
    assert_eq!(
        (span.start.line, span.start.column, span.end.column),
        (7, 11, 15)
    );
    let details = diagnostic.details.to_json();
    assert!(
        details.contains("\"occurrence\":\"path_segment\""),
        "{details}"
    );
    assert!(
        details.contains("\"name_class\":\"constructor\""),
        "{details}"
    );
    assert!(
        details.contains("\"required_initial\":\"ascii_uppercase\""),
        "{details}"
    );
    assert!(
        details.contains("\"observed_initial\":\"ascii_lowercase\""),
        "{details}"
    );
    assert!(details.contains("\"segment_index\":1"), "{details}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "type.match_non_exhaustive"
                && diagnostic.id != "type.mismatch"
                && diagnostic.id != "name.unresolved"
        }),
        "{diagnostics:#?}"
    );
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn qualified_lowercase_nullary_constructor_pattern_suppresses_exhaustiveness_cascade() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  None\n",
            "  Some(Int)\n",
            "end\n",
            "fn main(input: Item) -> Int\n",
            "  match input\n",
            "    Item::none => 0\n",
            "    Item::Some(value) => value\n",
            "  end\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.match_non_exhaustive"),
        "{diagnostics:#?}"
    );
}

#[test]
fn qualified_lowercase_constructor_pattern_keeps_direct_nested_and_body_diagnostics() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  None\n",
            "  Some(Int)\n",
            "end\n",
            "fn main(input: Item) -> Int\n",
            "  match input\n",
            "    Item::some(BadBinding) => missing_value\n",
            "    Item::None => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);

    let ids = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        ["name.invalid_case", "name.invalid_case", "name.unresolved"]
    );
    assert_eq!(
        diagnostics[0].message,
        "constructor name `some` must start with an ASCII uppercase letter"
    );
    assert_eq!(
        diagnostics[1].message,
        "binding name `BadBinding` must start with an ASCII lowercase letter"
    );
    assert_eq!(diagnostics[2].message, "unresolved value `missing_value`");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "core.constructor_arity_mismatch"
                && diagnostic.id != "type.match_non_exhaustive"
                && diagnostic.id != "type.mismatch"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn qualified_lowercase_constructor_pattern_recovery_is_initial_only() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  Some\n",
            "  SOME\n",
            "end\n",
            "fn main(input: Item) -> Int\n",
            "  match input\n",
            "    Item::some => 1\n",
            "  end\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    let non_exhaustive = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("SOME remains independently missing");
    assert_eq!(non_exhaustive.message, "match is missing case SOME");
}

#[test]
fn qualified_lowercase_constructor_pattern_recovery_preserves_remaining_spelling() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  Some\n",
            "end\n",
            "fn main(input: Item) -> Int\n",
            "  match input\n",
            "    Item::something => 1\n",
            "  end\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case"),
        "{diagnostics:#?}"
    );
    let non_exhaustive = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("Some remains independently missing");
    assert_eq!(non_exhaustive.message, "match is missing case Some");
}

#[test]
fn qualified_uppercase_constructor_pattern_remains_valid_control() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  None\n",
            "  Some(Int)\n",
            "end\n",
            "fn main(input: Item) -> Int\n",
            "  match input\n",
            "    Item::Some(value) => value\n",
            "    Item::None => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.invalid_case"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.match_non_exhaustive"),
        "{diagnostics:#?}"
    );
}
