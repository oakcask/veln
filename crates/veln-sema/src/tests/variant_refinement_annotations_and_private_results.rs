use super::*;
use crate::semantic_model::Type;
use crate::types::TypeEnvironment;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

const STATE_DECL: &str = concat!(
    "type State\n",
    "  Ready\n",
    "  Closed\n",
    "  Failed\n",
    "end\n",
);

#[test]
fn explicitly_refined_nested_positions_remain_invariant() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Box<A>\n",
            "  Boxed(A)\n",
            "end\n",
            "fn reject_record(value: {state: State::Ready}) -> ()\n",
            "  let widened: {state: State} = value\n",
            "end\n",
            "fn reject_record_narrowing(value: {state: State}) -> ()\n",
            "  let narrowed: {state: State::Ready} = value\n",
            "end\n",
            "fn reject_named(value: Box<State::Ready>) -> ()\n",
            "  let widened: Box<State> = value\n",
            "end\n",
            "fn reject_named_narrowing(value: Box<State>) -> ()\n",
            "  let narrowed: Box<State::Ready> = value\n",
            "end\n",
            "fn accepts_base(value: State) -> State\n",
            "  value\n",
            "end\n",
            "fn accepts_ready(value: State::Ready) -> State::Ready\n",
            "  value\n",
            "end\n",
            "fn reject_function_positions() -> ()\n",
            "  let narrowed: fn(State::Ready) -> State::Ready = accepts_base\n",
            "  let widened: fn(State) -> State = accepts_ready\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 6, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn base_with_different_generic_arguments_uses_an_ordinary_mismatch() {
    let diagnostics = diagnostics_for(concat!(
        "fn needs_some(value: Option<Int>::Some) -> ()\n",
        "  ()\n",
        "end\n",
        "fn main(value: Option<String>) -> ()\n",
        "  needs_some(value)\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch", "{diagnostics:#?}");
}

#[test]
fn refinements_require_identical_resolved_generic_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Box<A>\n",
            "  Boxed(A)\n",
            "  Empty\n",
            "end\n",
            "fn needs_refined(value: Box<{x: Int}>::Boxed) -> ()\n",
            "  ()\n",
            "end\n",
            "fn needs_base(value: Box<{x: Int}>) -> ()\n",
            "  ()\n",
            "end\n",
            "fn reject(value: Box<{x: Int, y: Int}>::Boxed) -> ()\n",
            "  needs_refined(value)\n",
            "  needs_base(value)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment.function("needs_refined").unwrap().params[0].render(),
        "Box<{x: Int}>::Boxed"
    );
    assert_eq!(
        environment.function("reject").unwrap().params[0].render(),
        "Box<{x: Int, y: Int}>::Boxed"
    );
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn refinement_function_variadics_are_invariant() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn ready_values(first: Int, values: ...State::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  let exact: fn(Int, ...State::Ready) -> () = ready_values\n",
            "  let base: fn(Int, ...State) -> () = ready_values\n",
            "end\n",
        )
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn variant_mismatch_lists_multiple_variants_in_declaration_order() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn needs_ready(value: State::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn wrong(value: State::Failed | State::Closed) -> ()\n",
            "  needs_ready(value)\n",
            "end\n",
        )
    ));

    let mismatch = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.variant_mismatch")
        .unwrap_or_else(|| panic!("{diagnostics:#?}"));
    let json = veln_diagnostics::diagnostic_to_json(mismatch).to_json();
    assert!(json.contains("\"expected_variants\":[\"Ready\"]"), "{json}");
    assert!(
        json.contains("\"variants\":[\"Closed\",\"Failed\"]"),
        "{json}"
    );
}

#[test]
fn recursive_adt_payload_construction_uses_the_nested_constructor_base() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Wrapped(State)\n",
        "end\n",
        "fn main() -> State::Wrapped\n",
        "  Wrapped(Ready)\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn recursive_generic_payload_preserves_base_arguments_under_refined_expectation() {
    let diagnostics = diagnostics_for(concat!(
        "type Chain<A>\n",
        "  End(A)\n",
        "  More(Chain<A>)\n",
        "end\n",
        "fn main() -> Chain<Int>::More\n",
        "  More(End(1))\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

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
            ("type.public_signature_missing", 5),
            ("type.variant_refinement_base", 8),
            ("type.public_signature_missing", 11),
            ("type.variant_refinement_base", 14),
            ("type.invalid_annotation", 17),
            ("type.variant_refinement_base", 20),
            ("type.public_signature_missing", 23),
        ],
        "{diagnostics:#?}"
    );
}

#[test]
fn many_independent_base_failures_have_linear_end_to_end_work() {
    let measurements = [128, 256, 512].map(|count| {
        let mut source = String::from("type Empty\nend\npub fn unrelated_before()\n  ()\nend\n");
        for index in 0..count {
            source.push_str(&format!(
                "fn invalid_{index}(value: empty::Missing) -> ()\n  ()\nend\n\
                 pub fn unrelated_after_{index}()\n  ()\nend\n"
            ));
        }
        crate::pipeline::reset_variant_refinement_base_work();
        let diagnostics = diagnostics_for(&source);
        let work = crate::pipeline::take_variant_refinement_base_work();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
                .count(),
            count
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.public_signature_missing")
                .count(),
            count + 1
        );
        assert!(
            work.annotation_collection <= count * 16 + 16,
            "count={count} work={work:?}"
        );
        assert!(
            work.casing_collection <= count * 256 + 20_000,
            "count={count} work={work:?}"
        );
        assert!(
            work.enclosing_context <= count * 8 + 16,
            "count={count} work={work:?}"
        );
        assert!(
            work.diagnostic_ordering <= count * 64 + 10_000,
            "count={count} work={work:?}"
        );
        eprintln!("variant refinement base pipeline: annotations={count} work={work:?}");
        work
    });
    for stage in [
        |work: crate::pipeline::VariantRefinementBaseWork| work.annotation_collection,
        |work: crate::pipeline::VariantRefinementBaseWork| work.casing_collection,
        |work: crate::pipeline::VariantRefinementBaseWork| work.enclosing_context,
        |work: crate::pipeline::VariantRefinementBaseWork| work.diagnostic_ordering,
    ] {
        let first_growth = stage(measurements[1]) - stage(measurements[0]);
        let second_growth = stage(measurements[2]) - stage(measurements[1]);
        assert_eq!(
            second_growth,
            first_growth * 2,
            "stage work must have exact affine growth: measurements={measurements:?}"
        );
    }
}

#[test]
fn declaration_quarantine_work_is_linear_across_consumers() {
    let measurements = [16, 32, 64].map(|count| {
        let mut source = String::from("type Empty\nend\n");
        for index in 0..count {
            source.push_str(&format!(
                "type Carrier{index}\n  Built{index}(invalid: Empty::Missing, sibling: Int)\nend\n\
                 effect Effect{index}\n  apply(invalid: Empty::Missing, sibling: Int) -> Int\nend\n\
                 handler handler_{index}(invalid: Empty::Missing, sibling: Int) for Effect{index}\n\
                   apply(invalid, sibling) => sibling\nend\n"
            ));
        }
        crate::pipeline::reset_variant_refinement_base_work();
        let diagnostics = diagnostics_for(&source);
        let work = crate::pipeline::take_variant_refinement_base_work();
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
                .count(),
            count * 3,
            "count={count} diagnostics={diagnostics:#?}"
        );
        eprintln!("variant refinement declaration quarantine: declarations={count} work={work:?}");
        work.diagnostic_ordering
    });

    let first_growth = measurements[1] - measurements[0];
    let second_growth = measurements[2] - measurements[1];
    assert_eq!(
        second_growth,
        first_growth * 2,
        "quarantine work must have exact affine growth: measurements={measurements:?}"
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
fn invalid_declaration_annotations_quarantine_body_and_caller_types() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "fn needs_string(value: String) -> ()\n",
        "  ()\n",
        "end\n",
        "fn invalid_param(value: Empty::Missing) -> ()\n",
        "  needs_string(value)\n",
        "end\n",
        "fn invalid_return() -> Empty::Missing\n",
        "  \"not an int\"\n",
        "end\n",
        "fn caller() -> ()\n",
        "  invalid_param(\"not an int\")\n",
        "  let value: String = invalid_return()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.variant_refinement_base"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_effect_operation_quarantines_all_parameter_and_result_types() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "effect Broken\n",
        "  apply(Bad: Empty::Missing, sibling: String) -> String\n",
        "end\n",
        "fn caller() -> () effects [Broken]\n",
        "  let result: Int = perform Broken::apply(1, 2)\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.variant_refinement_base");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_handler_quarantines_sibling_arguments_and_body_bindings() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "handler broken(Bad: Empty::Missing, sibling: String) for Ask\n",
        "  value() => sibling\n",
        "end\n",
        "fn caller() -> Int\n",
        "  handle perform Ask::value() with broken(1, 2)\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.invalid_case");
    assert_eq!(diagnostics[1].id, "type.variant_refinement_base");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_constructor_quarantines_every_payload_type() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "type Broken\n",
        "  Built(Bad: Empty::Missing, sibling: String)\n",
        "  Other\n",
        "  Other\n",
        "end\n",
        "fn caller() -> Broken\n",
        "  Built(1, 2)\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.variant_refinement_base");
    assert_eq!(diagnostics[1].id, "name.duplicate");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_schema_refinement_does_not_publish_generated_helpers() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Empty\n",
            "end\n",
            "schema Packet\n",
            "  invalid: Empty::Missing\n",
            "  sibling: Int\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);

    for helper in [
        "byte_decode_packet",
        "byte_encode_packet",
        "validate_packet",
    ] {
        assert!(
            environment
                .function_path(&[helper.to_string()], None)
                .is_none(),
            "invalid schema must not expose partially typed helper `{helper}`"
        );
    }
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_base")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_return_refinement_keeps_unknown_result_binding_for_contract_validation() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "fn invalid_return() -> output: Empty::Missing\n",
        "ensure output.missing == 0\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.variant_refinement_base");
    assert_eq!(diagnostics[1].id, "contract.field_missing");
    assert!(
        diagnostics[1]
            .details
            .to_json()
            .contains("\"reason\":\"missing_field\"")
    );
}

#[test]
fn invalid_return_is_quarantined_before_private_return_inference() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Empty\n",
            "end\n",
            "fn invalid_return() -> Empty::Missing\n",
            "  \"not an int\"\n",
            "end\n",
            "fn inferred_caller()\n",
            "  invalid_return()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let inferred = module
        .functions
        .iter()
        .find(|function| function.name.as_deref() == Some("inferred_caller"))
        .and_then(|function| environment.function_for(function))
        .expect("inferred caller signature");
    assert_eq!(inferred.return_type, Type::Unknown);
}

#[test]
fn nested_invalid_cased_bases_keep_independent_eligibility_failures() {
    let diagnostics = diagnostics_for(concat!(
        "type Empty\n",
        "end\n",
        "type Box<A>\n",
        "  Boxed(A)\n",
        "end\n",
        "fn main(value: Box<empty::Missing>::Boxed) -> ()\n",
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
    assert_eq!(relevant.len(), 2, "{diagnostics:#?}");
    assert_eq!(relevant[0].id, "name.invalid_case");
    assert_eq!(relevant[1].id, "type.variant_refinement_base");
    let base_json = veln_diagnostics::diagnostic_to_json(relevant[1]).to_json();
    assert!(base_json.contains("\"written_type\":\"empty\""));
    assert!(base_json.contains("\"reason\":\"not_adt\""));
    assert!(base_json.contains("\"resolved_identity\":\"Empty\""));
}

#[test]
fn invalid_cased_bases_keep_independent_generic_arity_failures() {
    let diagnostics = diagnostics_for(concat!(
        "type GenericEmpty<A>\n",
        "end\n",
        "fn missing(value: genericEmpty::Missing) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let casing = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .count();
    assert_eq!(casing, 1, "{diagnostics:#?}");
    let arity_messages = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.invalid_annotation")
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert_eq!(arity_messages.len(), 1, "{diagnostics:#?}");
    assert!(
        arity_messages
            .iter()
            .any(|message| message.contains("`GenericEmpty` expects 1 type argument(s), found 0")),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.variant_refinement_base"),
        "{diagnostics:#?}"
    );
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

#[test]
fn nested_refinement_canonicalization_handles_increasing_depths() {
    fn canonicalization_work(depth: usize) -> usize {
        let mut annotation = "Int".to_string();
        for _ in 0..depth {
            annotation = format!("Choice<{annotation}>::Present");
        }
        let source = format!(
            "type Choice<A>\n  Present(A)\n  Absent\nend\nfn keep(value: {annotation}) -> {annotation}\n  value\nend\n"
        );
        crate::types::reset_type_canonicalization_visits();
        let diagnostics = diagnostics_for(&source);
        assert!(diagnostics.is_empty(), "depth {depth}: {diagnostics:#?}");
        crate::types::take_type_canonicalization_visits()
    }

    let work = [16, 32, 64].map(canonicalization_work);
    eprintln!("type canonicalization visits at depths 16, 32, and 64: {work:?}");
    assert!(work[0] > 0, "the metric must observe canonicalization work");
    assert!(
        work[1] <= work[0] * 2 + 32 && work[2] <= work[1] * 2 + 32,
        "doubling refinement depth must add only linear canonicalization work: {work:?}"
    );
}

#[test]
fn unresolved_nested_annotations_do_not_repeat_canonicalization() {
    fn canonicalization_work(depth: usize) -> usize {
        let mut annotation = "Int".to_string();
        for _ in 0..depth {
            annotation = format!("missing::Outer<{annotation}>");
        }
        let source = format!("fn keep(value: {annotation}) -> ()\n  ()\nend\n");
        crate::types::reset_type_canonicalization_visits();
        let diagnostics = diagnostics_for(&source);
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "type.variant_mismatch"),
            "depth {depth}: {diagnostics:#?}"
        );
        crate::types::take_type_canonicalization_visits()
    }

    let work = [16, 32, 64].map(canonicalization_work);
    eprintln!("invalid annotation canonicalization visits at depths 16, 32, and 64: {work:?}");
    assert!(work[0] > 0, "the metric must observe canonicalization work");
    assert!(
        work[0] == work[1] && work[1] == work[2],
        "an invalid outer annotation must stop validation before child recanonicalization: {work:?}"
    );
}

#[test]
fn private_constructor_results_retain_singleton_refinements() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "end\n",
            "type GenericState<A>\n",
            "  Ready(A)\n",
            "end\n",
            "fn ready()\n",
            "  State::Ready\n",
            "end\n",
            "fn generic_ready()\n",
            "  GenericState::Ready(1)\n",
            "end\n",
            "fn accept_ready(value: State::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_generic_ready(value: GenericState<Int>::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  accept_ready(ready())\n",
            "  accept_generic_ready(generic_ready())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("ready")
            .expect("private helper should be present")
            .return_type
            .render(),
        "State::Ready"
    );
    assert_eq!(
        environment
            .function("generic_ready")
            .expect("private generic helper should be present")
            .return_type
            .render(),
        "GenericState<Int>::Ready"
    );
}

#[test]
fn private_results_retain_refinements_in_aggregate_positions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "end\n",
            "type Box<A>\n",
            "  Boxed(A)\n",
            "end\n",
            "fn states()\n",
            "  [State::Ready]\n",
            "end\n",
            "fn state_record()\n",
            "  {state: State::Ready}\n",
            "end\n",
            "fn state_dict()\n",
            "  {\"state\": State::Ready}\n",
            "end\n",
            "fn boxed_state()\n",
            "  Box::Boxed(State::Ready)\n",
            "end\n",
            "fn accept_states(value: Vec<State::Ready>) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_ready(value: State::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_dict(value: Dict<String, State::Ready>) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_box(value: Box<State::Ready>) -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  accept_states(states())\n",
            "  accept_ready(state_record().state)\n",
            "  accept_dict(state_dict())\n",
            "  accept_box(boxed_state())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    for (function, expected) in [
        ("states", "Vec<State::Ready>"),
        ("state_record", "{state: State::Ready}"),
        ("state_dict", "Dict<String, State::Ready>"),
        ("boxed_state", "Box<State::Ready>::Boxed"),
    ] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("{function} should be present"))
                .return_type
                .render(),
            expected,
        );
    }
}

#[test]
fn private_results_canonicalize_explicit_local_refinements() {
    let module = merged_modules_with_identities(vec![
        (
            "states",
            SourceFile::new(
                "states.veln",
                concat!(
                    "pub type ImportedState\n",
                    "  pub Ready\n",
                    "  pub Closed\n",
                    "end\n",
                ),
            ),
        ),
        (
            "main",
            SourceFile::new(
                "main.veln",
                concat!(
                    "use states\n",
                    "type State\n",
                    "  Ready\n",
                    "  Closed\n",
                    "end\n",
                    "fn local_ready()\n",
                    "  let state: State::Ready = State::Ready\n",
                    "  state\n",
                    "end\n",
                    "fn imported_ready()\n",
                    "  let state: states::ImportedState::Ready = states::ImportedState::Ready\n",
                    "  state\n",
                    "end\n",
                    "fn some()\n",
                    "  let value: Option<Int>::Some = Some(1)\n",
                    "  value\n",
                    "end\n",
                    "fn accept_local(value: State::Ready) -> ()\n",
                    "  ()\n",
                    "end\n",
                    "fn accept_imported(value: states::ImportedState::Ready) -> ()\n",
                    "  ()\n",
                    "end\n",
                    "fn accept_some(value: Option<Int>::Some) -> ()\n",
                    "  ()\n",
                    "end\n",
                    "fn main() -> ()\n",
                    "  accept_local(local_ready())\n",
                    "  accept_imported(imported_ready())\n",
                    "  accept_some(some())\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    for (function, expected) in [
        ("local_ready", "State::Ready"),
        ("imported_ready", "ImportedState::Ready"),
        ("some", "Option<Int>::Some"),
    ] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("{function} should be present"))
                .return_type
                .render(),
            expected,
        );
    }
}

#[test]
fn source_adt_payload_refinement_annotations_are_canonical() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "end\n",
        "type Envelope\n",
        "  ReadyOnly(State::Ready)\n",
        "end\n",
        "fn main() -> Envelope\n",
        "  ReadyOnly(Ready)\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "end\n",
        "type Envelope\n",
        "  ReadyOnly(State::Ready)\n",
        "end\n",
        "fn reject(value: State) -> Envelope\n",
        "  ReadyOnly(value)\n",
        "end\n",
    ));
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch", "{diagnostics:#?}");
}
