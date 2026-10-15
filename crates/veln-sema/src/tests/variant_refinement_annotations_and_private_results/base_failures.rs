use super::*;

#[test]
fn invalid_refinement_annotations_are_diagnosed_instead_of_becoming_unknown() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn missing(value: State::Missing) -> State::Missing\n",
            "  value\n",
            "end\n",
            "fn wrong_arity(value: Option<Int, String>::Some) -> ()\n",
            "  ()\n",
            "end\n",
        )
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.invalid_annotation")
            .count(),
        3,
        "{diagnostics:#?}"
    );
}

#[test]
fn non_adt_base_reports_declaration_and_preserves_arity_precedence() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "fn rejected(value: Empty::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn arity(value: Empty<Int>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let base_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
        .collect::<Vec<_>>();
    assert_eq!(base_diagnostics.len(), 1, "{diagnostics:#?}");
    let observed = veln_diagnostics::diagnostic_to_json(base_diagnostics[0]).to_json();
    assert!(
        observed.contains("\"details\":{\"written_type\":\"Empty\",\"reason\":\"not_adt\"}"),
        "{observed}"
    );
    assert!(
        observed.contains("\"kind\":\"base_type_declaration\""),
        "{observed}"
    );
    assert!(
        observed.contains("\"resolved_identity\":\"Empty\""),
        "{observed}"
    );
    assert!(observed.contains("\"line\":1,\"column\":1"), "{observed}");
    assert_eq!(base_diagnostics[0].span.as_ref().unwrap().start.column, 20);

    let arity = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "type.invalid_annotation"
                && diagnostic
                    .message
                    .contains("expects 0 type argument(s), found 1")
        })
        .count();
    assert_eq!(arity, 1, "{diagnostics:#?}");
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
}

#[test]
fn non_adt_alias_bases_report_the_canonical_target_declaration() {
    let diagnostics = diagnostics_for(concat!(
        "pub type Empty\n",
        "end\n",
        "pub type Alias = Empty\n",
        "pub type Transitive = Alias\n",
        "fn direct(value: Alias::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn chained(value: Transitive::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let base = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert_eq!(base.len(), 2, "{diagnostics:#?}");
    for (observed, written_type) in base.iter().zip(["Alias", "Transitive"]) {
        assert!(
            observed.contains(&format!(
                "\"details\":{{\"written_type\":\"{written_type}\",\"reason\":\"not_adt\"}}"
            )),
            "{observed}"
        );
        assert!(
            observed.contains(
                "\"kind\":\"base_type_declaration\",\"message\":\"Type `Empty` is declared here without finite ADT variants.\",\"resolved_identity\":\"Empty\""
            ),
            "{observed}"
        );
        assert!(observed.contains("\"line\":1,\"column\":5"), "{observed}");
        assert!(!observed.contains("Type `Alias` is declared"), "{observed}");
        assert!(
            !observed.contains("Type `Transitive` is declared"),
            "{observed}"
        );
    }
    let primary_spans = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.span.as_ref().expect("base failure span"))
        .map(|span| {
            (
                span.start.line,
                span.start.column,
                span.end.line,
                span.end.column,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(primary_spans, [(5, 18, 5, 23), (8, 19, 8, 29)]);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
}

#[test]
fn source_less_non_adt_and_opaque_bases_report_closed_provenance() {
    let diagnostics = diagnostics_for(concat!(
        "fn scalar(value: Int::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn stream(value: NetStream::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let base = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert_eq!(base.len(), 2, "{diagnostics:#?}");
    assert!(base[0].contains("\"details\":{\"written_type\":\"Int\",\"reason\":\"not_adt\"}"));
    assert!(base[0].contains("\"kind\":\"base_type_origin\""));
    assert!(base[0].contains("\"resolved_identity\":\"Int\""));
    assert!(base[0].contains("\"provider\":\"type_syntax\"}]"));
    assert!(base[1].contains("\"details\":{\"written_type\":\"NetStream\",\"reason\":\"opaque\"}"));
    assert!(base[1].contains("\"kind\":\"base_type_origin\""));
    assert!(base[1].contains("\"resolved_identity\":\"NetStream\""));
    assert!(base[1].contains("\"provider\":\"adt\"}]"));
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
}

#[test]
fn invalid_cased_bases_recover_one_identity_and_keep_independent_failures() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "end\n",
        "type Empty\n",
        "end\n",
        "fn refinable(value: state::Ready) -> ()\n",
        "  ()\n",
        "end\n",
        "fn missing(value: state::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn non_adt(value: empty::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let relevant = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.id.as_str(),
                "name.invalid_case" | "type.variant_refinement_base"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(relevant.len(), 4, "{diagnostics:#?}");
    assert_eq!(
        relevant
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "name.invalid_case",
            "name.invalid_case",
            "type.variant_refinement_base",
        ]
    );
    let base_json = relevant
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert!(base_json[0].contains("\"reason\":\"not_adt\""));
    assert!(base_json[0].contains("\"resolved_identity\":\"Empty\""));
    let recovered_final = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "type.invalid_annotation"
                && diagnostic.message.contains("state::Missing")
        })
        .count();
    assert_eq!(recovered_final, 1, "{diagnostics:#?}");
}

#[test]
fn explicit_call_type_arguments_keep_casing_silent_while_checking_eligibility() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "fn sink(value: Int) -> ()\n",
        "  ()\n",
        "end\n",
        "fn main() -> ()\n",
        "  sink<Empty::Missing>(1)\n",
        "  sink<empty::Missing>(1)\n",
        "end\n",
    ));

    let relevant = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.id.as_str(),
                "name.invalid_case" | "type.variant_refinement_base"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(relevant.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        relevant
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.variant_refinement_base",
            "type.variant_refinement_base"
        ]
    );
    let recovered = veln_diagnostics::diagnostic_to_json(relevant[1]).to_json();
    assert!(recovered.contains("\"written_type\":\"empty\""));
    assert!(recovered.contains("\"resolved_identity\":\"Empty\""));
}

#[test]
fn base_failures_are_merged_at_their_annotation_positions() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "type GenericEmpty<A>\n",
        "end\n",
        "pub fn unrelated_before()\n",
        "  ()\n",
        "end\n",
        "fn recovered(value: empty::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn unrelated_between()\n",
        "  ()\n",
        "end\n",
        "fn first(value: Empty::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn arity(value: GenericEmpty<Int, String>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn later(value: Empty::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn later_recovered(value: empty::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn unrelated_after()\n",
        "  ()\n",
        "end\n",
    ));

    let actual = diagnostics
        .iter()
        .map(|diagnostic| {
            (
                diagnostic.id.as_str(),
                diagnostic.span.as_ref().unwrap().start.line,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        actual,
        [
            ("name.invalid_case", 8),
            ("name.invalid_case", 23),
            ("type.public_signature_missing", 5),
            ("type.variant_refinement_base", 8),
            ("type.public_signature_missing", 11),
            ("type.variant_refinement_base", 14),
            ("type.invalid_annotation", 17),
            ("type.variant_refinement_base", 20),
            ("type.variant_refinement_base", 23),
            ("type.public_signature_missing", 26),
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn mixed_union_failures_follow_written_alternative_order() {
    for (annotation, expected) in [
        (
            "Empty::Missing | GenericEmpty<Int, String>::Missing",
            ["type.variant_refinement_base", "type.invalid_annotation"],
        ),
        (
            "GenericEmpty<Int, String>::Missing | Empty::Missing",
            ["type.invalid_annotation", "type.variant_refinement_base"],
        ),
    ] {
        let diagnostics = diagnostics_for(&format!(
            "type Empty\nend\ntype GenericEmpty<A>\nend\nfn mixed(value: {annotation}) -> ()\n  ()\nend\n"
        ));
        let relevant = diagnostics
            .iter()
            .filter(|diagnostic| {
                matches!(
                    diagnostic.id.as_str(),
                    "type.variant_refinement_base" | "type.invalid_annotation"
                )
            })
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(relevant, expected, "{diagnostics:#?}");
    }
}

#[test]
fn base_failure_skips_only_its_union_alternative() {
    for (annotation, expected_ids) in [
        (
            "Empty::Missing | State::Ready | Other::Go",
            ["type.variant_refinement_base", "type.variant_union_base"],
        ),
        (
            "State::Ready | Empty::Missing | Other::Go",
            ["type.variant_refinement_base", "type.variant_union_base"],
        ),
        (
            "State::Ready | Other::Go | Empty::Missing",
            ["type.variant_union_base", "type.variant_refinement_base"],
        ),
    ] {
        let diagnostics = diagnostics_for(&format!(
            concat!(
                "type Empty\nend\n",
                "type State\n  Ready\nend\n",
                "type Other\n  Go\nend\n",
                "fn mixed(value: {}) -> ()\n  ()\nend\n",
            ),
            annotation
        ));
        let relevant = diagnostics
            .iter()
            .filter(|diagnostic| {
                matches!(
                    diagnostic.id.as_str(),
                    "type.variant_refinement_base" | "type.variant_union_base"
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            relevant
                .iter()
                .map(|diagnostic| diagnostic.id.as_str())
                .collect::<Vec<_>>(),
            expected_ids,
            "{diagnostics:#?}"
        );
        let union_error = relevant
            .iter()
            .find(|diagnostic| diagnostic.id == "type.variant_union_base")
            .expect("surviving alternatives must still be compared");
        let union_json = veln_diagnostics::diagnostic_to_json(union_error).to_json();
        assert!(union_json.contains("\"expected_base_type\":\"State\""));
        assert!(union_json.contains("\"actual_base_type\":\"Other\""));
        let span = union_error.span.as_ref().expect("union error span");
        let expected_column = 17 + annotation.find("Other::Go").unwrap();
        assert_eq!(span.start.line, 9);
        assert_eq!(span.start.column, expected_column);
        assert_eq!(span.end.column, expected_column + "Other::Go".len());
    }
}

#[test]
fn recovered_union_compares_canonical_generic_arguments() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "type State\n",
        "  Ready\n",
        "end\n",
        "pub type StateAlias = State\n",
        "type Box<A>\n",
        "  Present(A)\n",
        "  Absent\n",
        "end\n",
        "fn mixed(value: Empty::Missing | Box<State>::Present | Box<StateAlias>::Absent) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.invalid_annotation"),
        "{diagnostics:#?}"
    );
}

#[test]
fn base_failures_preserve_independent_sibling_and_nested_annotation_errors() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "end\n",
        "type Empty<A>\n",
        "end\n",
        "type Box<A>\n",
        "  Boxed(A)\n",
        "end\n",
        "fn sibling(value: State::Missing | Empty<Int>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn nested(value: Empty<Box::Boxed>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    let annotation_messages = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.invalid_annotation")
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(annotation_messages.len(), 2, "{diagnostics:#?}");
    assert!(
        annotation_messages
            .iter()
            .any(|message| message.contains("State::Missing")),
        "{diagnostics:#?}"
    );
    assert!(
        annotation_messages
            .iter()
            .any(|message| message.contains("`Box` expects 1 type argument(s), found 0")),
        "{diagnostics:#?}"
    );
}

#[test]
fn base_recovery_traverses_record_field_siblings() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "fn failed(value: {bad: Empty::Missing, sibling: Vec<Int, Int>}) -> ()\n",
        "  ()\n",
        "end\n",
        "fn control(value: {sibling: Vec<Int, Int>}) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.variant_refinement_base",
            "type.invalid_annotation",
            "type.invalid_annotation",
        ],
        "{diagnostics:#?}"
    );
    for diagnostic in diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.invalid_annotation")
    {
        assert!(
            diagnostic
                .message
                .contains("`Vec` expects 1 type argument(s), found 2"),
            "{diagnostics:#?}"
        );
    }
    assert!(diagnostics[1].message.contains("`Vec<Int, Int>`"));
    let recovered_span = diagnostics[1].span.as_ref().expect("recovered arity span");
    assert_eq!(
        recovered_span.end.column - recovered_span.start.column,
        "Vec<Int, Int>".len()
    );
}

#[test]
fn nested_base_recovery_preserves_independent_errors_without_derivative_parents() {
    let diagnostics = diagnostics_for(concat!(
        "type GenericEmpty<A>\n",
        "end\n",
        "type Empty\n",
        "end\n",
        "type Box<A>\n",
        "  Boxed(A)\n",
        "end\n",
        "fn outer(value: GenericEmpty<Vec<Int, String>>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn control(value: Box<Vec<Int, String>>::Boxed) -> ()\n",
        "  ()\n",
        "end\n",
        "fn invalid_child(value: Box<Empty::Missing>::Boxed) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let relevant = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.id.as_str(),
                "type.variant_refinement_base" | "type.invalid_annotation"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(relevant.len(), 4, "{diagnostics:#?}");
    assert_eq!(
        relevant
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.variant_refinement_base",
            "type.invalid_annotation",
            "type.invalid_annotation",
            "type.variant_refinement_base",
        ],
        "{diagnostics:#?}"
    );
    assert_eq!(
        relevant
            .iter()
            .map(|diagnostic| {
                let span = diagnostic.span.as_ref().unwrap();
                (span.start.line, span.start.column, span.end.column)
            })
            .collect::<Vec<_>>(),
        [(8, 17, 29), (8, 30, 46), (11, 12, 47), (14, 29, 34)],
        "{diagnostics:#?}"
    );
    assert!(
        relevant[1].message.contains(
            "invalid type annotation `Vec<Int, String>`: `Vec` expects 1 type argument(s), found 2"
        ),
        "{diagnostics:#?}"
    );
    assert!(
        relevant[2]
            .message
            .contains("`Vec` expects 1 type argument(s), found 2"),
        "{diagnostics:#?}"
    );
}

#[test]
fn nested_recovery_visits_each_source_path_occurrence_once() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty<A>\n",
        "end\n",
        "type State\n",
        "  Ready\n",
        "end\n",
        "fn f(value: Empty<state::Missing>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let relevant = diagnostics
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic.id.as_str(),
                "name.invalid_case" | "type.variant_refinement_base" | "type.invalid_annotation"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        relevant
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "type.variant_refinement_base",
            "type.invalid_annotation",
        ],
        "{diagnostics:#?}"
    );
    let invalid = relevant[2];
    assert!(invalid.message.contains("state::Missing"), "{invalid:#?}");
    assert_diagnostic_span(invalid, 6, 19, 6, 33);
}

#[test]
fn equal_nested_recovery_failures_at_distinct_spans_are_each_reported_once() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty<A>\n",
        "end\n",
        "type State\n",
        "  Ready\n",
        "end\n",
        "fn f(first: Empty<state::Missing>::Missing, second: Empty<state::Missing>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let invalid = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "type.invalid_annotation"
                && diagnostic.message.contains("state::Missing")
        })
        .collect::<Vec<_>>();
    assert_eq!(invalid.len(), 2, "{diagnostics:#?}");
    assert_diagnostic_span(invalid[0], 6, 19, 6, 33);
    assert_diagnostic_span(invalid[1], 6, 59, 6, 73);
}

#[test]
fn non_unique_invalid_case_recovery_does_not_select_a_base() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "end\n",
        "type State\n",
        "  Ready\n",
        "end\n",
        "fn main(value: state::Ready) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.variant_refinement_base"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.invalid_case"),
        "{diagnostics:#?}"
    );
}

#[test]
fn non_unique_generic_invalid_case_recovery_does_not_select_a_base() {
    let diagnostics = diagnostics_for(concat!(
        "type GenericEmpty<A>\n",
        "end\n",
        "type GenericEmpty<A>\n",
        "end\n",
        "fn main(value: genericEmpty<Int>::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert!(
        diagnostics.iter().all(|diagnostic| !matches!(
            diagnostic.id.as_str(),
            "name.invalid_case" | "type.variant_refinement_base"
        )),
        "{diagnostics:#?}"
    );
}

#[test]
fn ambiguous_exact_and_builtin_colliding_bases_suppress_only_unresolved_eligibility() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "type Empty\n",
        "end\n",
        "type Int\n",
        "end\n",
        "type Int\n",
        "end\n",
        "fn exact(value: Empty::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn recovered(value: int::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn opaque_follow_up(value: NetStream::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let base_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
        .collect::<Vec<_>>();
    assert_eq!(base_diagnostics.len(), 1, "{diagnostics:#?}");
    assert!(
        base_diagnostics[0]
            .details
            .to_json()
            .contains("\"reason\":\"opaque\"")
    );
}

#[test]
fn declaration_and_nested_carriers_report_base_failures_once() {
    let source = concat!(
        "type State\n",
        "  Ready\n",
        "end\n",
        "type Empty\n",
        "end\n",
        "type GenericEmpty<A>\n",
        "end\n",
        "type Wrap\n",
        "  Wrapped(Empty::Missing)\n",
        "end\n",
        "effect Store\n",
        "  put(value: Empty::Missing) -> Empty::Missing\n",
        "end\n",
        "handler store(context: Empty::Missing) for Store\n",
        "  put(value) => ()\n",
        "end\n",
        "schema Packet\n",
        "  value: Empty::Missing\n",
        "end\n",
        "fn sink(value: Int) -> ()\n",
        "  ()\n",
        "end\n",
        "fn main(value: GenericEmpty<State::Ready>::Missing) -> ()\n",
        "  sink<Empty::Missing>(1)\n",
        "end\n",
    );
    let source_file = SourceFile::new("main.veln", source);
    let parsed = parse(&source_file);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    let base_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
        .collect::<Vec<_>>();
    assert_eq!(base_diagnostics.len(), 7, "{diagnostics:#?}");
    let written = base_diagnostics
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert!(
        written
            .iter()
            .any(|json| { json.contains("\"written_type\":\"GenericEmpty<State::Ready>\"") }),
        "{written:#?}"
    );
    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered.core.is_none(),
        "invalid declaration must not be published"
    );
    assert!(
        lowered.ir.is_none(),
        "invalid declaration must not be lowered"
    );
}

#[test]
fn source_names_colliding_with_builtin_opaque_names_keep_source_identity() {
    let diagnostics = diagnostics_for(concat!(
        "type NetListener\n",
        "end\n",
        "type NetStream\n",
        "end\n",
        "fn listener(value: NetListener::Missing) -> ()\n",
        "  ()\n",
        "end\n",
        "fn stream(value: NetStream::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let base = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert_eq!(base.len(), 2, "{diagnostics:#?}");
    assert!(base[0].contains("\"reason\":\"not_adt\""));
    assert!(base[0].contains("\"resolved_identity\":\"NetListener\""));
    assert!(base[0].contains("\"span\":"), "{base:#?}");
    assert!(base[1].contains("\"reason\":\"not_adt\""));
    assert!(base[1].contains("\"resolved_identity\":\"NetStream\""));
}

#[test]
fn qualified_source_names_precede_colliding_compiler_known_names() {
    let source = SourceFile::new(
        "collision.veln",
        concat!(
            "type Int\n",
            "end\n",
            "type NetStream\n",
            "end\n",
            "fn scalar(value: collision::Int::Missing) -> ()\n",
            "  ()\n",
            "end\n",
            "fn stream(value: collision::NetStream::Missing) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let module = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&source).tree,
        "collision".to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    );
    let diagnostics = analyze_surface_module(&module);

    let base = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert_eq!(base.len(), 2, "{diagnostics:#?}");
    assert!(
        base.iter()
            .all(|json| json.contains("\"reason\":\"not_adt\""))
    );
    assert!(base[0].contains("\"written_type\":\"collision::Int\""));
    assert!(base[0].contains("\"resolved_identity\":\"collision::Int\""));
    assert!(base[0].contains("\"line\":1,\"column\":1"));
    assert!(base[1].contains("\"written_type\":\"collision::NetStream\""));
    assert!(base[1].contains("\"resolved_identity\":\"collision::NetStream\""));
    assert!(base[1].contains("\"line\":3,\"column\":1"));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.invalid_annotation"),
        "{diagnostics:#?}"
    );
}

#[test]
fn compiler_known_refinements_require_base_type_arguments() {
    let diagnostics = diagnostics_for(concat!(
        "fn keep(value: Option::Some) -> Option::Some\n",
        "  value\n",
        "end\n",
        "fn main() -> Option<Int>\n",
        "  keep(None)\n",
        "end\n",
    ));

    let invalid_annotations = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.invalid_annotation")
        .collect::<Vec<_>>();
    assert_eq!(invalid_annotations.len(), 2, "{diagnostics:#?}");
    assert!(invalid_annotations.iter().all(|diagnostic| {
        diagnostic
            .message
            .contains("`Option` expects 1 type argument(s), found 0")
    }));
}

#[test]
fn source_defined_refinements_require_base_type_arguments() {
    let diagnostics = diagnostics_for(concat!(
        "type Choice<A>\n",
        "  Present(A)\n",
        "  Absent\n",
        "end\n",
        "fn keep(value: Choice::Absent) -> Choice::Absent\n",
        "  value\n",
        "end\n",
        "fn main() -> ()\n",
        "  keep(1)\n",
        "end\n",
    ));

    let invalid_annotations = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.invalid_annotation")
        .collect::<Vec<_>>();
    assert_eq!(invalid_annotations.len(), 2, "{diagnostics:#?}");
    assert!(invalid_annotations.iter().all(|diagnostic| {
        diagnostic
            .message
            .contains("`Choice` expects 1 type argument(s), found 0")
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.invalid_annotation"),
        "{diagnostics:#?}"
    );
}
