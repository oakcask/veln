use super::*;

#[test]
fn unknown_wrong_kind_and_wrong_owner_finals_use_the_focused_diagnostic() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "end\n",
        "type Marker\n",
        "  Marked\n",
        "end\n",
        "type Other\n",
        "  OtherOnly\n",
        "end\n",
        "fn unknown(value: State::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn wrong_kind(value: State::Marker) -> ()\n",
        "  ()\n",
        "end\n",
        "fn wrong_owner(value: State::OtherOnly) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.variant_refinement_unknown"),
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.details.to_json())
            .collect::<Vec<_>>(),
        [
            "{\"base_type\":\"State\",\"variant\":\"Missing\"}",
            "{\"base_type\":\"State\",\"variant\":\"Marker\"}",
            "{\"base_type\":\"State\",\"variant\":\"OtherOnly\"}",
        ]
    );
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.span.as_ref().unwrap().start.column)
            .collect::<Vec<_>>(),
        [26, 29, 30]
    );
    for diagnostic in &diagnostics {
        let related = diagnostic
            .related
            .iter()
            .map(|value| value.to_json())
            .collect::<Vec<_>>();
        assert_eq!(related.len(), 3, "{related:#?}");
        assert!(related[0].contains("\"kind\":\"base_type_declaration\""));
        assert!(related[1].contains("\"variant\":\"Ready\""));
        assert!(related[2].contains("\"variant\":\"Closed\""));
    }
}

#[test]
fn unknown_final_retains_only_the_base_for_independent_union_comparison() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "end\n",
        "type Other\n",
        "  OtherOnly\n",
        "end\n",
        "fn rejected(value: State::Missing | Other::OtherOnly) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        ["type.variant_refinement_unknown", "type.variant_union_base"],
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics[1].details.to_json(),
        "{\"expected_base_type\":\"State\",\"actual_base_type\":\"Other\"}"
    );
}

#[test]
fn unknown_final_quarantines_dependent_failures_and_invalid_children_suppress_outer_finals() {
    let diagnostics = diagnostics_for(concat!(
        "pub type State\n",
        "  pub Ready\n",
        "  Hidden\n",
        "end\n",
        "type Box<A>\n",
        "  Boxed(A)\n",
        "  Empty\n",
        "end\n",
        "fn needs_string(value: String) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn rejected(value: State::Missing) -> State::Missing\n",
        "  needs_string(value)\n",
        "  value\n",
        "end\n",
        "fn nested(value: Box<State::Missing>::Absent) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.variant_refinement_unknown",
            "type.variant_refinement_unknown",
            "type.variant_refinement_unknown",
        ],
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.variant_refinement_private"
            && diagnostic.id != "type.invalid_annotation"
            && diagnostic.id != "type.mismatch"
    }));
    assert_eq!(
        diagnostics[2].details.to_json(),
        "{\"base_type\":\"State\",\"variant\":\"Missing\"}"
    );
}
