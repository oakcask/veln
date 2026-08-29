use super::*;

#[test]
fn reports_hole_with_declared_return_expected_type() {
    let source = SourceFile::new("main.veln", "fn todo() -> Result<(), AppError>\n  _\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Hole);
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"hole\",\"node_id\":\"hole-3\",\"label\":null,",
            "\"expected_type\":\"Result<(), AppError>\",",
            "\"expected_type_source\":\"declared\",",
            "\"constraints\":[],\"local_bindings\":[],",
            "\"candidate_queries\":[{\"kind\":\"symbol\",",
            "\"candidate_status\":\"query_only\",",
            "\"application_policy\":\"manual_review_required\",",
            "\"query\":\"fn() -> Result<(), AppError>\"}]}"
        )
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn ranks_visible_symbol_candidates_for_hole_expected_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let fallback = 1\n",
            "  _\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"candidates\":["));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",",
        "\"type\":\"Int\",\"rank\":1,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\",",
        "\"edits\":[{\"kind\":\"replace\","
    )));
    assert!(details.contains(concat!(
        "\"span\":{\"file\":\"main.veln\",",
        "\"start\":{\"line\":3,\"column\":3,\"offset\":48},",
        "\"end\":{\"line\":3,\"column\":4,\"offset\":49}}"
    )));
    assert!(details.contains("\"replacement\":\"fallback\""));
    assert!(details.contains(concat!(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"limit\",",
        "\"type\":\"Int\",\"rank\":2,\"reason\":\"exact_type_match\",",
        "\"application_policy\":\"manual_review_required\",",
        "\"edits\":[{\"kind\":\"replace\","
    )));
    assert!(details.contains("\"replacement\":\"limit\""));
    assert!(details.contains("\"target\":{\"node_id\":\"hole-"));
    assert!(details.contains("\"edit_summary\":\"Replace hole with `fallback`\""));
    assert!(details.contains(concat!(
        "\"evidence\":[{\"kind\":\"type\",\"status\":\"passed\",",
        "\"expected_type\":\"Int\",\"candidate_type\":\"Int\"},",
        "{\"kind\":\"ranking\",\"status\":\"ranked\",\"rank\":1,"
    )));
    assert!(details.contains(concat!(
        "\"known_limits\":[\"edit is advisory and unapplied\",",
        "\"tests and examples have not been run\"]"
    )));
    assert!(details.contains(concat!(
        "\"blocking_obligations\":[\"manual_review_required\",",
        "\"verification.not_run\"]"
    )));
    assert!(details.contains(concat!(
        "\"verification_hint\":{\"command\":\"veln check --json main.veln\",",
        "\"scope\":\"after_applying_candidate_edit\"}"
    )));
    assert!(details.contains("\"application_status\":\"unapplied\""));
}

#[test]
fn holes_receive_expected_types_from_expression_contexts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Box<A>\n",
            "  Box(value: A)\n",
            "end\n",
            "\n",
            "fn accept(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "fn return_context(candidate: Int) -> Int\n",
            "  _\n",
            "end\n",
            "\n",
            "fn call_context(candidate: Int) -> Int\n",
            "  accept(_)\n",
            "end\n",
            "\n",
            "fn record_context(candidate: Int) -> {value: Int}\n",
            "  {value: _}\n",
            "end\n",
            "\n",
            "fn if_context(flag: Bool, candidate: Int) -> Int\n",
            "  if flag\n",
            "    _\n",
            "  else\n",
            "    candidate\n",
            "  end\n",
            "end\n",
            "\n",
            "fn match_context(flag: Bool, candidate: Int) -> Int\n",
            "  match flag\n",
            "    true => _\n",
            "    false => candidate\n",
            "  end\n",
            "end\n",
            "\n",
            "fn constructor_context(candidate: Int) -> Box<Int>\n",
            "  Box(_)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let holes = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "hole.unfilled")
        .collect::<Vec<_>>();
    assert_eq!(holes.len(), 6, "{diagnostics:#?}");
    assert_eq!(diagnostics.len(), holes.len(), "{diagnostics:#?}");
    for hole in holes {
        assert_eq!(hole.message, "hole requires a `Int` value");
        let details = hole.details.to_json();
        assert!(details.contains("\"expected_type\":\"Int\""), "{details}");
        assert!(
            details.contains("\"expected_type_source\":\"declared\""),
            "{details}"
        );
        assert!(
            details.contains("\"candidate_queries\":[{\"kind\":\"symbol\""),
            "{details}"
        );
        assert!(
            details.contains("\"name\":\"candidate\",\"type\":\"Int\""),
            "{details}"
        );
    }
}
