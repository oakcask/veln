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
fn compiler_known_prelude_arguments_report_variant_mismatches() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "fn main() -> Vec<State::Ready>\n",
            "  vec_push([Ready], Closed)\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let mismatch = &diagnostics[0];
    assert_eq!(mismatch.id, "type.variant_mismatch", "{diagnostics:#?}");
    let json = veln_diagnostics::diagnostic_to_json(mismatch).to_json();
    assert!(json.contains("\"actual_type\":\"State::Closed\""), "{json}");
    assert!(
        json.contains("\"expected_type\":\"State::Ready\""),
        "{json}"
    );
    assert!(json.contains("\"expected_variants\":[\"Ready\"]"), "{json}");
    assert!(json.contains("\"variants\":[\"Closed\"]"), "{json}");
    assert!(json.contains("\"constraint\":\"call_argument\""), "{json}");
    assert!(json.contains("\"kind\":\"variant_exclusion\""), "{json}");
    assert!(json.contains("\"kind\":\"expected_type_origin\""), "{json}");
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
fn alias_qualified_refinement_annotations_are_rejected() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "end\n",
        "pub type First = State\n",
        "pub type Second = State\n",
        "fn direct(value: State::Ready) -> State::Ready\n",
        "  value\n",
        "end\n",
        "fn alias_singleton(value: First::Ready) -> ()\n",
        "  ()\n",
        "end\n",
        "fn alias_union(value: State::Ready | Second::Closed) -> ()\n",
        "  ()\n",
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
            .contains("variant refinement annotations cannot use a type alias as their base")
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.invalid_annotation"),
        "{diagnostics:#?}"
    );
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
fn aggregate_construction_retains_record_fields_and_rejects_excluded_variants() {
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
            "  accept_ready(retained.state)\n",
            "  let bound: {state: State} = retained\n",
            "end\n",
            "fn accept_ready(value: State::Ready) -> ()\n",
            "  ()\n",
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

    assert_eq!(diagnostics.len(), 8, "{diagnostics:#?}");
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
            .count(),
        0,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        8,
        "{diagnostics:#?}"
    );
}

#[test]
fn non_record_aggregate_inference_retains_and_joins_variant_refinements() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Box<A>\n",
            "  Boxed(A)\n",
            "end\n",
            "fn main() -> ()\n",
            "  let states = [Ready, Closed]\n",
            "  let table = {\"ready\": Ready, \"closed\": Closed}\n",
            "  let boxed = Boxed(Ready)\n",
            "  let states_exact: Vec<State::Ready | State::Closed> = states\n",
            "  let table_exact: Dict<String, State::Ready | State::Closed> = table\n",
            "  let boxed_exact: Box<State::Ready> = boxed\n",
            "  let contextual_states: Vec<State> = [Ready, Closed]\n",
            "  let contextual_table: Dict<String, State> = {\"ready\": Ready, \"closed\": Closed}\n",
            "  let contextual_box: Box<State> = Boxed(Ready)\n",
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
fn inferred_generic_payload_retains_its_refinement() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Box<A>\n",
            "  Boxed(A)\n",
            "end\n",
            "fn main() -> ()\n",
            "  let retained = Boxed(Ready)\n",
            "  let exact: Box<State::Ready> = retained\n",
            "end\n",
        )
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}
