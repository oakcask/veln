use super::*;

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

    let mut diagnostics = analyze_surface_module(&module)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();
    diagnostics.sort_by_key(|diagnostic| {
        diagnostic
            .span
            .as_ref()
            .map(|span| (span.start.offset, span.end.offset))
    });

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
        ("Build", "function", "alias_target", 14, 19, 24),
        ("exported", "type", "declaration", 15, 10, 18),
        ("item", "type", "alias_target", 15, 21, 25),
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
fn import_path_segments_report_module_casing_with_retained_spans() {
    let source = SourceFile::new("main.veln", "use HTTP::_tls\n");
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].message,
        "module name `HTTP` must start with an ASCII lowercase letter"
    );
    assert_diagnostic_span(&diagnostics[0], 1, 5, 1, 9);
    let first_details = diagnostics[0].details.to_json();
    assert!(first_details.contains("\"occurrence\":\"path_segment\""));
    assert!(first_details.contains("\"name_class\":\"module\""));
    assert!(first_details.contains("\"observed_initial\":\"ascii_uppercase\""));
    assert!(first_details.contains("\"segment_index\":0"));

    assert_eq!(
        diagnostics[1].message,
        "module name `_tls` must start with an ASCII lowercase letter"
    );
    assert_diagnostic_span(&diagnostics[1], 1, 11, 1, 15);
    let second_details = diagnostics[1].details.to_json();
    assert!(second_details.contains("\"observed_initial\":\"underscore\""));
    assert!(second_details.contains("\"segment_index\":1"));
}

#[test]
fn import_path_casing_diagnostics_follow_source_order_with_declarations() {
    let source = SourceFile::new(
        "app.veln",
        concat!("use HTTP\n", "\n", "fn Bad() -> Int\n", "  1\n", "end\n",),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        [
            "module name `HTTP` must start with an ASCII lowercase letter",
            "function name `Bad` must start with an ASCII lowercase letter",
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn let_annotation_reports_qualified_type_path_segments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(input: Int) -> Int\n",
            "  let value: prelude::option = input\n",
            "  input\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].message,
        "type name `option` must start with an ASCII uppercase letter"
    );
    assert_diagnostic_span(&diagnostics[0], 2, 24, 2, 30);
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"occurrence\":\"path_segment\""));
    assert!(details.contains("\"name_class\":\"type\""));
    assert!(details.contains("\"segment_index\":1"));
}

#[test]
fn unresolved_three_segment_call_does_not_guess_intermediate_type_role() {
    let source = SourceFile::new(
        "main.veln",
        concat!("fn main() -> Int\n", "  Missing::bad::Value()\n", "end\n",),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        ["module name `Missing` must start with an ASCII lowercase letter"],
        "{diagnostics:#?}"
    );
}
