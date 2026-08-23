use super::support::*;

#[test]
fn invalid_source_name_classes_have_exact_diagnostics_without_a_missing_name_cascade() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type item\n",
            "  made\n",
            "  _other\n",
            "end\n",
            "fn Build(_value: Int) -> Result: Int\n",
            "  Build(1)\n",
            "end\n",
        ),
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        6
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved")
    );
    let function = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.message == "function name must start with an ASCII lowercase letter"
        })
        .expect("function casing diagnostic");
    assert_diagnostic_span(function, 5, 4, 5, 9);
    assert!(function.details.to_json().contains(
        "\"origin\":\"source\",\"occurrence\":\"declaration\",\"name\":\"Build\",\"name_class\":\"function\",\"required_initial\":\"ascii_lowercase\",\"observed_initial\":\"ascii_uppercase\""
    ));
}

#[test]
fn invalid_declarations_never_produce_checked_artifacts() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        "fn Broken() -> Int\n  1\nend\n",
    ));
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
    assert_eq!(lowered.diagnostics[0].id, "name.invalid_case");
}

#[test]
fn multiple_invalid_functions_do_not_select_a_recovery_symbol() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn Broken(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "fn Broken() -> Int\n",
            "  1\n",
            "end\n",
            "fn main() -> Int\n",
            "  Broken()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved")
    );
}

#[test]
fn a_valid_constructor_wins_over_an_invalid_same_spelled_function() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type Choice\n",
            "  Build\n",
            "end\n",
            "fn Build() -> Int\n",
            "  1\n",
            "end\n",
            "fn main() -> Choice\n",
            "  Build()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.invalid_case");
    assert_eq!(
        diagnostics[0].message,
        "function name must start with an ASCII lowercase letter"
    );
}

#[test]
fn accepted_declaration_and_binding_names_keep_checked_artifacts() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type Choice\n",
            "  Made(Int)\n",
            "end\n",
            "fn build(value: Int) -> result: Choice\n",
            "  let local = value\n",
            "  Made(local)\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.core.is_some());
    assert!(lowered.ir.is_some());
}

#[test]
fn incompatible_recovery_class_does_not_suppress_unresolved_call() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type item\n",
            "  Made\n",
            "end\n",
            "fn main() -> Int\n",
            "  item()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved")
    );
}
