use super::*;
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
fn constructors_retain_singletons_and_calls_apply_subset_and_direct_widening() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn accept_one(value: State::Ready) -> State::Ready\n",
            "  value\n",
            "end\n",
            "fn accept_two(value: State::Ready | State::Closed) -> State\n",
            "  value\n",
            "end\n",
            "fn main() -> State\n",
            "  let exact = Ready\n",
            "  let same: State::Ready = exact\n",
            "  let widened: State = accept_one(same)\n",
            "  accept_two(exact)\n",
            "end\n",
        )
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn compiler_known_generic_refinements_resolve_and_widen() {
    let diagnostics = diagnostics_for(concat!(
        "fn keep(value: Option<Int>::Some) -> Option<Int>::Some\n",
        "  value\n",
        "end\n",
        "fn main() -> Option<Int>\n",
        "  let exact: Option<Int>::Some = Some(1)\n",
        "  keep(exact)\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn compiler_known_variant_unions_resolve_and_canonicalize() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn accept_pair(value: DecodeStep<Int>::NeedMore | DecodeStep<Int>::Decoded | DecodeStep<Int>::NeedMore) -> DecodeStep<Int>\n",
            "  value\n",
            "end\n",
            "fn accept_complete(value: DecodeStep<Int>::Invalid | DecodeStep<Int>::NeedMore | DecodeStep<Int>::Decoded) -> DecodeStep<Int>\n",
            "  value\n",
            "end\n",
            "fn pass_singleton(value: DecodeStep<Int>::Decoded) -> DecodeStep<Int>\n",
            "  accept_pair(value)\n",
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
        environment.function("accept_pair").unwrap().params[0].render(),
        "DecodeStep<Int>::Decoded | DecodeStep<Int>::NeedMore"
    );
    assert_eq!(
        environment.function("accept_complete").unwrap().params[0].render(),
        "DecodeStep<Int>"
    );
}

#[test]
fn unresolved_generic_constructor_locals_preserve_their_variant() {
    let diagnostics = diagnostics_for(concat!(
        "type Choice<A>\n",
        "  Present(A)\n",
        "  Absent\n",
        "end\n",
        "fn needs_some(value: Option<Int>::Some) -> ()\n",
        "  ()\n",
        "end\n",
        "fn needs_present(value: Choice<Int>::Present) -> ()\n",
        "  ()\n",
        "end\n",
        "fn main() -> ()\n",
        "  let builtin = None\n",
        "  let source = Absent\n",
        "  needs_some(builtin)\n",
        "  needs_present(source)\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id == "type.variant_mismatch"
            && diagnostic.details.to_json().contains("\"form\":\"listed\"")
    }));
}

#[test]
fn expected_adt_disambiguates_nullary_constructors() {
    let diagnostics = diagnostics_for(concat!(
        "type Left\n",
        "  Ready\n",
        "end\n",
        "type Right\n",
        "  Ready\n",
        "end\n",
        "fn pick() -> Left\n",
        "  Ready\n",
        "end\n",
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn expected_imported_adt_overrides_local_constructor_preference() {
    let module = merged_modules_with_identities(vec![
        (
            "left",
            SourceFile::new(
                "left.veln",
                concat!(
                    "pub type State\n",
                    "  pub Ready\n",
                    "  pub Wrapped(Int)\n",
                    "end\n",
                ),
            ),
        ),
        (
            "main",
            SourceFile::new(
                "main.veln",
                concat!(
                    "use left\n",
                    "type Other\n",
                    "  Ready\n",
                    "  Wrapped(String)\n",
                    "end\n",
                    "fn ready() -> left::State::Ready\n",
                    "  Ready\n",
                    "end\n",
                    "fn wrapped() -> left::State::Wrapped\n",
                    "  Wrapped(1)\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for (function_name, variant_name, payload_count) in
        [("ready", "Ready", 0), ("wrapped", "Wrapped", 1)]
    {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .unwrap_or_else(|| panic!("{function_name} should be lowered"));
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("{function_name} should return a constructor");
        };
        assert!(
            matches!(&expr.kind, CoreExprKind::AdtVariant { name, payloads }
                if name == &vec!["State".to_string(), variant_name.to_string()]
                    && payloads.len() == payload_count),
            "{function_name} should lower to the imported State constructor"
        );
    }
}

#[test]
fn variant_refinement_casing_stays_in_semantic_analysis() {
    let module = merged_modules_with_identities(vec![
        (
            "helper",
            SourceFile::new("helper.veln", "pub type Item\n  pub Ready\nend\n"),
        ),
        (
            "main",
            SourceFile::new(
                "main.veln",
                concat!(
                    "use helper\n",
                    "fn inspect(value: Helper::Item::Ready) -> ()\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
        ),
    ]);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic
                .details
                .to_json()
                .contains("\"name_class\":\"module\"")
    }));
}

#[test]
fn rejected_calls_and_results_report_variant_mismatches() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn needs_ready(value: State::Ready) -> State::Ready\n",
            "  value\n",
            "end\n",
            "fn wrong_call() -> State::Ready\n",
            "  needs_ready(Closed)\n",
            "end\n",
            "fn wrong_result(value: State) -> State::Ready\n",
            "  value\n",
            "end\n",
        )
    ));

    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 2, "{diagnostics:#?}");
    assert!(mismatches[0].message.contains("State::Closed"));
    assert!(mismatches[0].message.contains("State::Ready"));
    assert_eq!(mismatches[0].related.len(), 2);
    assert!(mismatches[1].message.contains("State`"));
}

#[test]
fn resolved_base_identity_rejects_same_spelled_adt_from_another_module() {
    let module = merged_modules_with_identities(vec![
        (
            "left",
            SourceFile::new(
                "left.veln",
                "pub type State\n  pub Ready\n  pub Closed\nend\n",
            ),
        ),
        (
            "right",
            SourceFile::new(
                "right.veln",
                "pub type State\n  pub Ready\n  pub Closed\nend\n",
            ),
        ),
        (
            "main",
            SourceFile::new(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n",
                    "fn wrong() -> left::State\n",
                    "  right::Ready\n",
                    "end\n",
                    "fn wrong_refinement() -> left::State::Ready\n",
                    "  right::Ready\n",
                    "end\n",
                    "fn wrong_base_to_refinement(value: right::State) -> left::State::Ready\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        3,
        "{diagnostics:#?}"
    );
}

#[test]
fn resolved_named_types_reject_same_spelled_adts_from_another_module() {
    let module = merged_modules_with_identities(vec![
        (
            "left",
            SourceFile::new(
                "left.veln",
                "pub type State\n  pub Ready\n  pub Closed\nend\n",
            ),
        ),
        (
            "right",
            SourceFile::new(
                "right.veln",
                "pub type State\n  pub Ready\n  pub Closed\nend\n",
            ),
        ),
        (
            "main",
            SourceFile::new(
                "main.veln",
                concat!(
                    "use left\n",
                    "use right\n",
                    "fn consume(value: left::State) -> ()\n",
                    "  ()\n",
                    "end\n",
                    "fn wrong_assignment(value: right::State) -> ()\n",
                    "  let local: left::State = value\n",
                    "end\n",
                    "fn wrong_argument(value: right::State) -> ()\n",
                    "  consume(value)\n",
                    "end\n",
                    "fn wrong_result(value: right::State) -> left::State\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        3,
        "{diagnostics:#?}"
    );
}

#[test]
fn qualified_and_unqualified_union_bases_resolve_before_identity_comparison() {
    let module = merged_modules_with_identities(vec![(
        "state",
        SourceFile::new(
            "state.veln",
            concat!(
                "pub type State\n",
                "  pub Ready\n",
                "  pub Closed\n",
                "end\n",
                "fn keep(value: state::State::Ready | State::Closed) -> State\n",
                "  value\n",
                "end\n",
            ),
        ),
    )]);

    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn variant_unions_reject_mixed_resolved_adts() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Other\n",
            "  Ready\n",
            "end\n",
            "fn invalid(value: State::Ready | Other::Ready) -> ()\n",
            "  ()\n",
            "end\n",
        )
    ));

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.invalid_annotation"),
        "{diagnostics:#?}"
    );
}

#[test]
fn aggregate_construction_erases_base_components_and_rejects_excluded_variants() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Box<A>\n",
            "  Boxed(A)\n",
            "end\n",
            "fn records() -> ()\n",
            "  let inline: {state: State} = {state: Ready}\n",
            "  let exact: {state: State::Ready} = {state: Closed}\n",
            "  let retained = {state: Ready}\n",
            "  let bound: {state: State} = retained\n",
            "end\n",
            "fn payloads() -> ()\n",
            "  let inline: Box<State> = Boxed(Ready)\n",
            "  let exact: Box<State::Ready> = Boxed(Closed)\n",
            "  let retained = Boxed(Ready)\n",
            "  let bound: Box<State> = retained\n",
            "end\n",
            "fn collections() -> ()\n",
            "  let inline: Vec<State> = [Ready]\n",
            "  let exact: Vec<State::Ready> = [Closed]\n",
            "  let retained = [Ready]\n",
            "  let bound: Vec<State> = retained\n",
            "end\n",
            "fn dictionaries() -> ()\n",
            "  let value_inline: Dict<String, State> = {\"state\": Ready}\n",
            "  let value_exact: Dict<String, State::Ready> = {\"state\": Closed}\n",
            "  let key_inline: Dict<State, String> = {ready(): \"state\"}\n",
            "  let key_exact: Dict<State::Ready, String> = {closed(): \"state\"}\n",
            "end\n",
            "fn ready() -> State::Ready\n",
            "  Ready\n",
            "end\n",
            "fn closed() -> State::Closed\n",
            "  Closed\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 5, "{diagnostics:#?}");
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
            .count(),
        5,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        0,
        "{diagnostics:#?}"
    );
}

#[test]
fn aggregate_inference_uses_base_types_for_mixed_variants() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Box<A>\n",
            "  Boxed(A)\n",
            "end\n",
            "fn main() -> ()\n",
            "  let record = {state: Ready}\n",
            "  let states = [Ready, Closed]\n",
            "  let table = {\"ready\": Ready, \"closed\": Closed}\n",
            "  let boxed = Boxed(Ready)\n",
            "  let record_base: {state: State} = record\n",
            "  let states_base: Vec<State> = states\n",
            "  let table_base: Dict<String, State> = table\n",
            "  let boxed_base: Box<State> = boxed\n",
            "end\n",
        )
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn record_fields_report_ordinary_nested_type_mismatches() {
    let diagnostics = diagnostics_for(concat!(
        "fn main() -> ()\n",
        "  let value: {n: Int} = {n: \"wrong\"}\n",
        "end\n",
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
fn inferred_generic_payload_uses_its_base_type() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Box<A>\n",
            "  Boxed(A)\n",
            "end\n",
            "fn main() -> ()\n",
            "  let retained = Boxed(Ready)\n",
            "  let widened: Box<State> = retained\n",
            "end\n",
        )
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

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
fn source_annotations_canonicalize_variant_sets() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "fn finite(value: State::Closed | State::Ready | State::Closed) -> ()\n",
            "  ()\n",
            "end\n",
            "fn complete(value: State::Failed | State::Ready | State::Closed) -> ()\n",
            "  ()\n",
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
        environment.function("finite").unwrap().params[0].render(),
        "State::Ready | State::Closed"
    );
    assert_eq!(
        environment.function("complete").unwrap().params[0].render(),
        "State"
    );
}

#[test]
fn nested_mismatch_diagnostic_output_grows_linearly_with_variant_count() {
    fn diagnostic_bytes(variant_count: usize) -> (usize, usize) {
        let mut source = String::from("type State\n");
        for index in 0..variant_count {
            source.push_str(&format!("  Variant{index:03}\n"));
        }
        source.push_str("end\n");
        for index in 0..variant_count {
            source.push_str(&format!(
                "fn reject{index:03}(value: {{state: State::Variant{index:03}}}) -> ()\n"
            ));
            source.push_str("  let rejected: {state: State} = value\n");
            source.push_str("end\n");
        }
        let diagnostics = diagnostics_for(&source);
        let mismatches = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .collect::<Vec<_>>();
        assert_eq!(mismatches.len(), diagnostics.len(), "{diagnostics:#?}");
        let bytes = mismatches
            .iter()
            .map(|diagnostic| {
                veln_diagnostics::diagnostic_to_json(diagnostic)
                    .to_json()
                    .len()
            })
            .sum();
        (mismatches.len(), bytes)
    }

    let (small_count, small_bytes) = diagnostic_bytes(64);
    let (large_count, large_bytes) = diagnostic_bytes(128);
    eprintln!(
        "nested mismatch diagnostic bytes at 64 and 128 variants: {small_bytes}, {large_bytes}"
    );
    assert_eq!((small_count, large_count), (64, 128));
    assert!(
        large_bytes * 2 <= small_bytes * 5,
        "diagnostic bytes grew too quickly: {small_bytes} -> {large_bytes}"
    );
}

#[test]
fn final_if_checks_every_branch_and_untyped_expressions_do_not_cascade() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn branch() -> State::Ready\n",
            "  if true\n",
            "    Ready\n",
            "  else\n",
            "    Closed\n",
            "  end\n",
            "end\n",
            "fn unresolved() -> State::Ready\n",
            "  missing\n",
            "end\n",
        )
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved")
    );
}

#[test]
fn untyped_final_if_branch_does_not_report_a_variant_mismatch() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn branch() -> State::Ready\n",
            "  if true\n",
            "    missing\n",
            "  else\n",
            "    Ready\n",
            "  end\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.unresolved", "{diagnostics:#?}");
}

#[test]
fn final_match_checks_every_typed_arm_even_for_a_refined_scrutinee() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn branch(value: State::Ready) -> State::Ready\n",
            "  match value\n",
            "    Ready => Ready\n",
            "    Closed => Closed\n",
            "    Failed => Ready\n",
            "  end\n",
            "end\n",
        )
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn untyped_final_match_arm_does_not_report_a_variant_mismatch() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn branch(value: State) -> State::Ready\n",
            "  match value\n",
            "    Ready => missing\n",
            "    Closed => Ready\n",
            "    Failed => Ready\n",
            "  end\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.unresolved", "{diagnostics:#?}");
}
