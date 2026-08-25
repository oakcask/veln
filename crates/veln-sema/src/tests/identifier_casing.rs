use super::*;
use crate::types::environment::TypeEnvironment;

#[test]
fn covered_source_names_report_exact_casing_contract_details() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type item\n",
            "  value\n",
            "  _Payload(Int)\n",
            "end\n",
            "fn Build(Input: Int) -> Output: Int\n",
            "  let Local = Input\n",
            "  match Local\n",
            "    _bound => _bound\n",
            "  end\n",
            "end\n",
            "test Verify() -> ()\n",
            "  ()\n",
            "end\n",
            "pub fn Exported = Build\n",
            "pub type exported = item\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();

    let expected = [
        ("item", "type", "declaration", 1, 6, 10),
        ("value", "constructor", "declaration", 2, 3, 8),
        ("_Payload", "constructor", "declaration", 3, 3, 11),
        ("Build", "function", "declaration", 5, 4, 9),
        ("Input", "value_binding", "binding", 5, 10, 15),
        ("Output", "value_binding", "binding", 5, 25, 31),
        ("Local", "value_binding", "pattern_head", 6, 7, 12),
        ("_bound", "value_binding", "pattern_head", 8, 5, 11),
        ("Verify", "function", "declaration", 11, 6, 12),
        ("Exported", "function", "declaration", 14, 8, 16),
        ("exported", "type", "declaration", 15, 10, 18),
    ];
    assert_eq!(diagnostics.len(), expected.len(), "{diagnostics:#?}");
    for (diagnostic, (name, class, occurrence, line, start, end)) in
        diagnostics.iter().zip(expected)
    {
        let span = diagnostic.span.as_ref().expect("name diagnostic span");
        assert_eq!(
            (span.start.line, span.start.column, span.end.column),
            (line, start, end)
        );
        let details = diagnostic.details.to_json();
        assert!(details.contains(&format!("\"name\":\"{name}\"")));
        assert!(details.contains(&format!("\"name_class\":\"{class}\"")));
        assert!(details.contains(&format!("\"occurrence\":\"{occurrence}\"")));
        assert!(details.contains("\"phase\":\"name\""));
        assert!(details.contains("\"origin\":\"source\""));
    }

    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn accepted_names_and_expression_holes_keep_existing_behavior() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  Value(Int)\n",
            "end\n",
            "fn build(input: Int) -> output: Int\n",
            "  let local = input\n",
            "  local\n",
            "end\n",
            "fn incomplete() -> Int\n",
            "  _missing\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    assert!(module.invalid_names.is_empty());
    assert!(
        analyze_surface_module(&module)
            .iter()
            .all(|diagnostic| diagnostic.id != "name.invalid_case")
    );
}

#[test]
fn underscore_led_binding_recovers_without_missing_identifier_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        "fn _build(_input: Int) -> _output: Int\n  let _local = _input\n  _local\nend\n",
    );
    let parsed = parse(&source);

    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    assert_eq!(module.invalid_names.len(), 4);
    assert!(
        module
            .invalid_names
            .iter()
            .all(|name| name.name.starts_with('_'))
    );
}

#[test]
fn unique_same_source_recovery_suppresses_only_derivative_missing_name() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Int\n  Bad()\nend\nfn Bad() -> Int\n  1\nend\n",
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        1
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved")
    );
    assert!(
        TypeEnvironment::from_module(&module)
            .function("Bad")
            .is_none()
    );
}

#[test]
fn ambiguous_recovery_does_not_resolve() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  Bad()\n",
            "end\n",
            "fn Bad() -> Int\n",
            "  1\n",
            "end\n",
            "fn Bad() -> Int\n",
            "  2\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved call_target `Bad`"
    }));
}

#[test]
fn recovery_is_not_visible_through_an_import() {
    let module = merged_modules(vec![
        SourceFile::new(
            "main.veln",
            concat!(
                "mod main\n",
                "use helper\n",
                "fn main() -> Int\n",
                "  helper::Bad()\n",
                "end\n",
            ),
        ),
        SourceFile::new("helper.veln", "mod helper\npub fn Bad() -> Int\n  1\nend\n"),
    ]);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `helper::Bad`"
    }));
}
