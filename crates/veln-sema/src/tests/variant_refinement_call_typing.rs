use super::*;
use crate::types::TypeEnvironment;
use veln_diagnostics::JsonValue;

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

#[test]
fn aggregate_refinement_inference_covers_joins_context_and_projection() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "fn ready() -> State::Ready\n",
            "  Ready\n",
            "end\n",
            "fn closed() -> State::Closed\n",
            "  Closed\n",
            "end\n",
            "type Pair<A>\n",
            "  Paired(A, A)\n",
            "end\n",
            "type Container<A>\n",
            "  Built(A, Vec<A>)\n",
            "end\n",
            "type First<A>\n",
            "  Some(A)\n",
            "  Empty\n",
            "end\n",
            "fn forward()\n",
            "  [Ready, Closed]\n",
            "end\n",
            "fn reverse()\n",
            "  [Closed, Ready]\n",
            "end\n",
            "fn complete()\n",
            "  [Failed, Ready, Closed]\n",
            "end\n",
            "fn with_base(value: State)\n",
            "  [Ready, value]\n",
            "end\n",
            "fn optional()\n",
            "  [Option::Some(1), None]\n",
            "end\n",
            "fn table()\n",
            "  {ready(): Closed, closed(): Ready}\n",
            "end\n",
            "fn table_reverse()\n",
            "  {closed(): Ready, ready(): Closed}\n",
            "end\n",
            "fn paired()\n",
            "  Paired(Closed, Ready)\n",
            "end\n",
            "fn paired_reverse()\n",
            "  Paired(Ready, Closed)\n",
            "end\n",
            "fn retained_pair()\n",
            "  Paired(Ready, Ready)\n",
            "end\n",
            "fn mixed_occurrence()\n",
            "  Built(Ready, [Ready])\n",
            "end\n",
            "fn mixed_payload() -> Vec<State::Ready>\n",
            "  match mixed_occurrence()\n",
            "    Built(_, items) => items\n",
            "  end\n",
            "end\n",
            "fn ambiguous_private_result()\n",
            "  Paired(First::Some(1), Empty)\n",
            "end\n",
            "fn payload() -> State::Ready\n",
            "  match retained_pair()\n",
            "    Paired(item, _) => item\n",
            "  end\n",
            "end\n",
            "fn accept_ready(value: State::Ready) -> Bool\n",
            "  true\n",
            "end\n",
            "fn projected() -> Vec<State::Ready>\n",
            "  vec_filter([Ready], accept_ready)\n",
            "end\n",
            "fn main() -> ()\n",
            "  let first: Vec<State::Ready | State::Closed> = forward()\n",
            "  let second: Vec<State::Ready | State::Closed> = reverse()\n",
            "  let all: Vec<State> = complete()\n",
            "  let base_join: Vec<State> = with_base(Closed)\n",
            "  let optional_values: Vec<Option<Int>> = optional()\n",
            "  let entries: Dict<State::Ready | State::Closed, State::Ready | State::Closed> = table()\n",
            "  let reverse_entries: Dict<State::Ready | State::Closed, State::Ready | State::Closed> = table_reverse()\n",
            "  let pair: Pair<State::Ready | State::Closed>::Paired = paired()\n",
            "  let reverse_pair: Pair<State::Ready | State::Closed>::Paired = paired_reverse()\n",
            "  let contextual_vec: Vec<State> = [Ready, Closed]\n",
            "  let contextual_dict: Dict<String, State> = {\"ready\": Ready}\n",
            "  let contextual_pair: Pair<State> = Paired(Ready, Closed)\n",
            "  let item: State::Ready = payload()\n",
            "  let items: Vec<State::Ready> = projected()\n",
            "  let nested_items: Vec<State::Ready> = mixed_payload()\n",
            "  let inferred_private: Pair<First<Int>>::Paired = ambiguous_private_result()\n",
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
        ("forward", "Vec<State::Ready | State::Closed>"),
        ("reverse", "Vec<State::Ready | State::Closed>"),
        ("complete", "Vec<State>"),
        ("with_base", "Vec<State>"),
        ("optional", "Vec<Option<Int>>"),
        (
            "table",
            "Dict<State::Ready | State::Closed, State::Ready | State::Closed>",
        ),
        (
            "table_reverse",
            "Dict<State::Ready | State::Closed, State::Ready | State::Closed>",
        ),
        ("paired", "Pair<State::Ready | State::Closed>::Paired"),
        (
            "paired_reverse",
            "Pair<State::Ready | State::Closed>::Paired",
        ),
        ("retained_pair", "Pair<State::Ready>::Paired"),
        ("mixed_occurrence", "Container<State::Ready>::Built"),
        ("ambiguous_private_result", "Pair<First<Int>>::Paired"),
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
fn retained_aggregate_refinement_rejections_are_table_driven() {
    for (name, source, rejected_expression, expected, actual, constraint) in [
        (
            "later nested widening",
            concat!(
                "type State\n",
                "  Ready\n",
                "  Closed\n",
                "end\n",
                "fn main() -> ()\n",
                "  let inferred = [Ready]\n",
                "  let widened: Vec<State> = inferred\n",
                "end\n",
            ),
            "inferred",
            "Vec<State>",
            "Vec<State::Ready>",
            "assignable",
        ),
        (
            "later dictionary value widening",
            concat!(
                "type State\n",
                "  Ready\n",
                "  Closed\n",
                "end\n",
                "fn main() -> ()\n",
                "  let inferred = {\"ready\": Ready}\n",
                "  let widened: Dict<String, State> = inferred\n",
                "end\n",
            ),
            "inferred",
            "Dict<String, State>",
            "Dict<String, State::Ready>",
            "assignable",
        ),
        (
            "later generic payload widening",
            concat!(
                "type State\n",
                "  Ready\n",
                "  Closed\n",
                "end\n",
                "type Pair<A>\n",
                "  Paired(A, A)\n",
                "end\n",
                "fn main() -> ()\n",
                "  let inferred = Paired(Ready, Ready)\n",
                "  let widened: Pair<State> = inferred\n",
                "end\n",
            ),
            "inferred",
            "Pair<State>",
            "Pair<State::Ready>::Paired",
            "assignable",
        ),
        (
            "mixed direct and nested type parameter",
            concat!(
                "type State\n",
                "  Ready\n",
                "  Closed\n",
                "end\n",
                "type Container<A>\n",
                "  Built(A, Vec<A>)\n",
                "end\n",
                "fn main() -> ()\n",
                "  let value = Built(Ready, [Closed])\n",
                "end\n",
            ),
            "Closed",
            "State::Ready",
            "State::Closed",
            "list_element",
        ),
        (
            "mixed nested and direct type parameter",
            concat!(
                "type State\n",
                "  Ready\n",
                "  Closed\n",
                "end\n",
                "type Container<A>\n",
                "  Built(Vec<A>, A)\n",
                "end\n",
                "fn main() -> ()\n",
                "  let value = Built([Ready], Closed)\n",
                "end\n",
            ),
            "Closed",
            "State::Ready",
            "State::Closed",
            "call_argument",
        ),
        (
            "generic argument mismatch",
            concat!(
                "fn int_some() -> Option<Int>::Some\n",
                "  Some(1)\n",
                "end\n",
                "fn string_some() -> Option<String>::Some\n",
                "  Some(\"wrong\")\n",
                "end\n",
                "fn main() -> ()\n",
                "  let values = [int_some(), string_some()]\n",
                "end\n",
            ),
            "string_some()",
            "Option<Int>::Some",
            "Option<String>::Some",
            "list_element",
        ),
        (
            "different resolved ADTs",
            concat!(
                "type First\n",
                "  Value\n",
                "  Other\n",
                "end\n",
                "type Second\n",
                "  Value\n",
                "  Other\n",
                "end\n",
                "fn first() -> First::Value\n",
                "  First::Value\n",
                "end\n",
                "fn second() -> Second::Value\n",
                "  Second::Value\n",
                "end\n",
                "fn main() -> ()\n",
                "  let values = [first(), second()]\n",
                "end\n",
            ),
            "second()",
            "First::Value",
            "Second::Value",
            "list_element",
        ),
        (
            "repeated generic payload mismatch",
            concat!(
                "type Pair<A>\n",
                "  Paired(A, A)\n",
                "end\n",
                "fn int_some() -> Option<Int>::Some\n",
                "  Some(1)\n",
                "end\n",
                "fn string_some() -> Option<String>::Some\n",
                "  Some(\"wrong\")\n",
                "end\n",
                "fn main() -> ()\n",
                "  let value = Paired(int_some(), string_some())\n",
                "end\n",
            ),
            "string_some()",
            "Option<Int>::Some",
            "Option<String>::Some",
            "call_argument",
        ),
    ] {
        let source_file = SourceFile::new("main.veln", source);
        let parsed = parse(&source_file);
        assert!(
            parsed.diagnostics.is_empty(),
            "{name}: {:#?}",
            parsed.diagnostics
        );
        let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
        assert_eq!(diagnostics.len(), 1, "{name}: {diagnostics:#?}");
        let mismatch = &diagnostics[0];
        assert_eq!(mismatch.id, "type.mismatch", "{name}: {diagnostics:#?}");
        assert_eq!(
            mismatch.message.to_string(),
            format!("expected `{expected}`, but found `{actual}`"),
            "{name}: {diagnostics:#?}"
        );
        let span = mismatch.span.as_ref().expect("mismatch span");
        assert_eq!(
            &source[span.start.offset..span.end.offset],
            rejected_expression,
            "{name}: {span:#?}"
        );
        let details = mismatch.details.to_json();
        assert!(
            details.contains(&format!("\"expected_type\":\"{expected}\"")),
            "{name}: {details}"
        );
        assert!(
            details.contains(&format!("\"actual_type\":\"{actual}\"")),
            "{name}: {details}"
        );
        assert!(
            details.contains(&format!("\"constraint\":\"{constraint}\"")),
            "{name}: {details}"
        );
    }
}

fn aggregate_join_scaling_source(variant_count: usize) -> String {
    let variants = (0..variant_count)
        .map(|index| format!("Variant{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in &variants {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("end\ntype Repeated<A>\n  Many(");
    source.push_str(
        &(0..variant_count)
            .map(|_| "A")
            .collect::<Vec<_>>()
            .join(", "),
    );
    source.push_str(")\nend\n");
    for variant in &variants {
        source.push_str(&format!(
            "fn make_{variant}() -> State::{variant}\n  {variant}\nend\n"
        ));
    }
    let calls = variants
        .iter()
        .map(|variant| format!("make_{variant}()"))
        .collect::<Vec<_>>();
    source.push_str(&format!("fn vector()\n  [{}]\nend\n", calls.join(", ")));
    source.push_str(&format!(
        "fn dictionary()\n  {{{}}}\nend\n",
        calls
            .iter()
            .zip(calls.iter().rev())
            .map(|(key, value)| format!("{key}: {value}"))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    source.push_str(&format!(
        "fn repeated()\n  Many({})\nend\n",
        calls.join(", ")
    ));
    source.push_str(concat!(
        "fn main() -> ()\n",
        "  let items: Vec<State> = vector()\n",
        "  let entries: Dict<State, State> = dictionary()\n",
        "  let payloads: Repeated<State>::Many = repeated()\n",
        "end\n",
    ));
    source
}

fn independent_aggregate_site_scaling_source(variant_count: usize) -> String {
    let mut source = String::from("type State\n");
    for index in 0..variant_count {
        source.push_str(&format!("  Variant{index:04}\n"));
    }
    source.push_str("end\ntype Boxed<A>\n  One(A)\nend\n");
    for index in 0..variant_count {
        let variant = format!("State::Variant{index:04}");
        source.push_str(&format!(
            "fn vector_{index:04}()\n  [{variant}]\nend\n\
             fn dictionary_{index:04}()\n  {{{variant}: {variant}}}\nend\n\
             fn boxed_{index:04}()\n  Boxed::One({variant})\nend\n"
        ));
    }
    source
}

fn aggregate_rejection_scaling_source(variant_count: usize) -> String {
    let mut source = String::from("type State\n");
    for index in 0..variant_count {
        source.push_str(&format!("  Variant{index:04}\n"));
    }
    source.push_str("end\ntype Foreign\n  Other\nend\ntype Repeated<A>\n  Many(");
    source.push_str(
        &(0..=variant_count)
            .map(|_| "A")
            .collect::<Vec<_>>()
            .join(", "),
    );
    source.push_str(")\nend\nfn main() -> ()\n");
    let rejected = (0..variant_count)
        .map(|_| "Foreign::Other")
        .collect::<Vec<_>>();
    source.push_str(&format!(
        "  let vector = [State::Variant0000, {}]\n",
        rejected.join(", ")
    ));
    source.push_str(&format!(
        "  let dictionary = {{State::Variant0000: State::Variant0000, {}}}\n",
        (0..variant_count)
            .map(|_| "Foreign::Other: Foreign::Other")
            .collect::<Vec<_>>()
            .join(", ")
    ));
    source.push_str(&format!(
        "  let repeated = Repeated::Many(State::Variant0000, {})\n",
        rejected.join(", ")
    ));
    source.push_str("end\n");
    source
}

fn wide_generic_constructor_scaling_source(width: usize) -> String {
    let parameters = (0..width)
        .map(|index| format!("A{index}"))
        .collect::<Vec<_>>();
    let payloads = parameters
        .iter()
        .map(|parameter| format!("Vec<{parameter}>"))
        .collect::<Vec<_>>();
    let arguments = (0..width).map(|_| "[1]").collect::<Vec<_>>().join(", ");
    let result_args = (0..width).map(|_| "Int").collect::<Vec<_>>().join(", ");
    format!(
        "type Wide<{}>\n  Made({})\nend\nfn inferred()\n  Made({arguments})\nend\npub fn declared() -> Wide<{result_args}>::Made\n  Made({arguments})\nend\n",
        parameters.join(", "),
        payloads.join(", "),
    )
}

#[test]
fn aggregate_join_work_grows_linearly_through_all_inference_paths() {
    let work = [200, 400, 800, 1600].map(|variant_count| {
        let source = SourceFile::new("main.veln", aggregate_join_scaling_source(variant_count));
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::aggregate_type_join::reset_work();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        eprintln!(
            "{variant_count}-variant aggregate analysis: {:?}",
            started.elapsed()
        );
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let environment = TypeEnvironment::from_module(&module);
        assert_eq!(
            environment.function("vector").unwrap().return_type.render(),
            "Vec<State>"
        );
        assert_eq!(
            environment
                .function("dictionary")
                .unwrap()
                .return_type
                .render(),
            "Dict<State, State>"
        );
        assert_eq!(
            environment
                .function("repeated")
                .unwrap()
                .return_type
                .render(),
            "Repeated<State>::Many"
        );
        crate::aggregate_type_join::take_work()
    });
    eprintln!("aggregate join work units at doubled sizes: {work:?}");
    assert!(work[0] > 0, "the metric must observe aggregate join work");
    assert!(
        work[1] <= work[0] * 2 + 64 && work[2] <= work[1] * 2 + 64 && work[3] <= work[2] * 2 + 64,
        "doubling aggregate input must add only linear join work: {work:?}"
    );
}

#[test]
fn aggregate_join_initialization_and_materialization_scale_with_independent_sites() {
    let work = [50, 100, 200, 400].map(|variant_count| {
        let source = SourceFile::new(
            "main.veln",
            independent_aggregate_site_scaling_source(variant_count),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::aggregate_type_join::reset_work();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        eprintln!(
            "{variant_count} variants and {} independent aggregate sites: {:?}",
            variant_count * 3,
            started.elapsed()
        );
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        crate::aggregate_type_join::take_work()
    });
    eprintln!("independent aggregate-site work units at doubled sizes: {work:?}");
    assert!(
        work[0] >= 50 * 3,
        "the metric must include initialization and materialization at each site: {work:?}"
    );
    assert!(
        work[1] <= work[0] * 2 + 64 && work[2] <= work[1] * 2 + 64 && work[3] <= work[2] * 2 + 64,
        "doubling variants and independent sites must add only linear aggregate work: {work:?}"
    );
}

#[test]
fn unchanged_aggregate_join_work_grows_linearly_for_rejected_contributions() {
    let work = [50, 100, 200, 400].map(|variant_count| {
        let source = SourceFile::new(
            "main.veln",
            aggregate_rejection_scaling_source(variant_count),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::aggregate_type_join::reset_work();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        eprintln!(
            "{variant_count}-variant rejected aggregate analysis: {:?}",
            started.elapsed()
        );
        assert_eq!(diagnostics.len(), variant_count * 4, "{diagnostics:#?}");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id == "type.mismatch"),
            "{diagnostics:#?}"
        );
        let details = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.details.to_json())
            .collect::<Vec<_>>();
        assert!(
            details.iter().all(|details| {
                details.contains("\"expected_type\":\"State::Variant0000\"")
                    && details.contains("\"actual_type\":\"Foreign::Other\"")
            }),
            "{details:#?}"
        );
        for (constraint, expected_count) in [
            ("list_element", variant_count),
            ("dict_key", variant_count),
            ("dict_value", variant_count),
            ("call_argument", variant_count),
        ] {
            assert_eq!(
                details
                    .iter()
                    .filter(|details| details.contains(&format!("\"constraint\":\"{constraint}\"")))
                    .count(),
                expected_count,
                "{constraint}: {details:#?}"
            );
        }
        crate::aggregate_type_join::take_work()
    });
    eprintln!("aggregate rejection join work units at doubled sizes: {work:?}");
    assert!(work[0] > 0, "the metric must observe aggregate join work");
    assert!(
        work[1] <= work[0] * 2 + 64 && work[2] <= work[1] * 2 + 64 && work[3] <= work[2] * 2 + 64,
        "doubling rejected aggregate input must add only linear join work: {work:?}"
    );
}

#[test]
fn wide_generic_constructor_inference_work_grows_linearly_in_parameters_and_payloads() {
    let work = [100, 200, 400, 800].map(|width| {
        let source = SourceFile::new("main.veln", wide_generic_constructor_scaling_source(width));
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::aggregate_type_join::reset_work();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        eprintln!(
            "{width}-parameter generic constructor analysis: {:?}",
            started.elapsed()
        );
        assert!(diagnostics.is_empty(), "{diagnostics:#?}");
        let environment = TypeEnvironment::from_module(&module);
        let inferred = environment.function("inferred").expect("inferred function");
        assert_eq!(inferred.return_type.render().matches("Int").count(), width);
        crate::aggregate_type_join::take_work()
    });
    eprintln!("generic constructor inference work units at doubled widths: {work:?}");
    assert!(
        work[0] > 0,
        "the metric must observe constructor inference work"
    );
    assert!(
        work[1] <= work[0] * 2 + 64 && work[2] <= work[1] * 2 + 64 && work[3] <= work[2] * 2 + 64,
        "doubling generic constructor width must add only linear inference work: {work:?}"
    );
}

#[test]
fn aggregate_join_cache_invalidates_after_a_successful_variant_change() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  First\n",
        "  Second\n",
        "  Third\n",
        "end\n",
        "type Foreign\n",
        "  Other\n",
        "end\n",
        "fn foreign() -> Foreign::Other\n",
        "  Other\n",
        "end\n",
        "fn main() -> ()\n",
        "  let values = [Second, foreign(), First, foreign()]\n",
        "end\n",
    ));
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].message.to_string(),
        "expected `State::Second`, but found `Foreign`"
    );
    assert_eq!(
        diagnostics[1].message.to_string(),
        "expected `State::First | State::Second`, but found `Foreign`"
    );
}

#[test]
fn failed_payloads_do_not_contribute_to_private_or_body_recovery_joins() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "type Triple<A>\n",
            "  Made(A, A, A)\n",
            "end\n",
            "fn recovered()\n",
            "  Triple::Made(Ready, begin\n",
            "    let bad: Int = \"bad\"\n",
            "    Closed\n",
            "  end, Failed)\n",
            "end\n",
            "fn recovered_vector()\n",
            "  [Ready, begin\n",
            "    let bad: Int = \"bad\"\n",
            "    Closed\n",
            "  end, Failed]\n",
            "end\n",
            "fn recovered_dict_values()\n",
            "  {1: Ready, 2: begin\n",
            "    let bad: Int = \"bad\"\n",
            "    Closed\n",
            "  end, 3: Failed}\n",
            "end\n",
            "fn accept_triple(value: Triple<State::Ready | State::Failed>::Made) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_vector(value: Vec<State::Ready | State::Failed>) -> ()\n",
            "  ()\n",
            "end\n",
            "fn accept_dict_values(value: Dict<Int, State::Ready | State::Failed>) -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  accept_triple(recovered())\n",
            "  accept_vector(recovered_vector())\n",
            "  accept_dict_values(recovered_dict_values())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    for diagnostic in &diagnostics {
        assert_eq!(diagnostic.id, "type.mismatch", "{diagnostics:#?}");
        assert_eq!(
            diagnostic.message.to_string(),
            "expected `Int`, but found `String`"
        );
        let span = diagnostic.span.as_ref().expect("initializer mismatch span");
        assert_eq!(
            &source.text()[span.start.offset..span.end.offset],
            "\"bad\""
        );
    }

    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("recovered")
            .expect("private omitted result should be published")
            .return_type
            .render(),
        "Triple<State::Ready | State::Failed>::Made"
    );
    for (function, expected) in [
        ("recovered_vector", "Vec<State::Ready | State::Failed>"),
        (
            "recovered_dict_values",
            "Dict<Int, State::Ready | State::Failed>",
        ),
    ] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("{function} should be present"))
                .return_type
                .render(),
            expected
        );
    }
}

#[test]
fn rejected_expressions_never_contribute_to_private_aggregate_results() {
    let failures = [
        (
            "annotated local",
            concat!(
                "keep_closed(begin\n",
                "    let bad: Int = \"bad\"\n",
                "    Closed(1)\n",
                "  end)"
            ),
        ),
        ("declared call argument", "takes_int(\"bad\")"),
        ("constructor payload", "Closed(\"bad\")"),
    ];
    let aggregates = [
        (
            "vector element",
            "[Ready, {bad}, Failed]",
            "Vec<State::Ready | State::Failed>",
        ),
        (
            "dictionary key",
            "{{State::Ready: 1, {bad}: 2, State::Failed: 3}}",
            "Dict<State::Ready | State::Failed, Int>",
        ),
        (
            "dictionary value",
            "{{1: Ready, 2: {bad}, 3: Failed}}",
            "Dict<Int, State::Ready | State::Failed>",
        ),
        (
            "generic payload",
            "Triple::Made(Ready, {bad}, Failed)",
            "Triple<State::Ready | State::Failed>::Made",
        ),
    ];

    for (failure_name, rejected) in failures {
        for (aggregate_name, template, expected) in aggregates {
            if failure_name == "annotated local" && aggregate_name == "dictionary key" {
                continue;
            }
            let expression = template.replace("{bad}", rejected);
            let source = SourceFile::new(
                "main.veln",
                format!(
                    concat!(
                        "type State\n",
                        "  Ready\n",
                        "  Closed(Int)\n",
                        "  Failed\n",
                        "end\n",
                        "type Triple<A>\n",
                        "  Made(A, A, A)\n",
                        "end\n",
                        "fn takes_int(value: Int) -> State::Closed\n",
                        "  Closed(value)\n",
                        "end\n",
                        "fn keep_closed(value: State::Closed) -> State::Closed\n",
                        "  value\n",
                        "end\n",
                        "fn recovered()\n",
                        "  {}\n",
                        "end\n",
                        "fn accept(value: {}) -> ()\n",
                        "  ()\n",
                        "end\n",
                        "fn main() -> ()\n",
                        "  accept(recovered())\n",
                        "end\n",
                    ),
                    expression, expected,
                ),
            );
            let parsed = parse(&source);
            assert!(
                parsed.diagnostics.is_empty(),
                "{failure_name}, {aggregate_name}: {:#?}",
                parsed.diagnostics
            );
            let module = lower_surface_ast(&parsed.tree);
            let diagnostics = analyze_surface_module(&module);
            assert_eq!(
                diagnostics.len(),
                1,
                "{failure_name}, {aggregate_name}: {diagnostics:#?}"
            );
            assert_eq!(
                diagnostics[0].message.to_string(),
                "expected `Int`, but found `String`",
                "{failure_name}, {aggregate_name}: {diagnostics:#?}"
            );
            assert_eq!(
                TypeEnvironment::from_module(&module)
                    .function("recovered")
                    .expect("private omitted result should be published")
                    .return_type
                    .render(),
                expected,
                "{failure_name}, {aggregate_name}"
            );
        }
    }
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
fn private_control_flow_results_keep_only_a_common_singleton() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "end\n",
            "fn same(value: Bool)\n",
            "  if value\n",
            "    Ready\n",
            "  else\n",
            "    Ready\n",
            "  end\n",
            "end\n",
            "fn mixed_if(value: Bool)\n",
            "  if value\n",
            "    Ready\n",
            "  else\n",
            "    Closed\n",
            "  end\n",
            "end\n",
            "fn mixed_match(value: Bool)\n",
            "  match value\n",
            "    true => Ready\n",
            "    false => Closed\n",
            "  end\n",
            "end\n",
            "fn accept(value: State) -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  accept(mixed_if(true))\n",
            "  accept(mixed_match(true))\n",
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
        environment.function("same").unwrap().return_type.render(),
        "State::Ready"
    );
    assert_eq!(
        environment
            .function("mixed_if")
            .unwrap()
            .return_type
            .render(),
        "State"
    );
    assert_eq!(
        environment
            .function("mixed_match")
            .unwrap()
            .return_type
            .render(),
        "State"
    );
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

fn repeated_broad_variant_mismatch_source(variant_count: usize, use_count: usize) -> String {
    let variants = (0..variant_count)
        .map(|index| format!("Variant{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in &variants {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("  Excluded\nend\n");
    let broad = variants
        .iter()
        .map(|variant| format!("State::{variant}"))
        .collect::<Vec<_>>()
        .join(" | ");
    source.push_str(&format!(
        "fn accept(value: {broad}) -> ()\n  ()\nend\nfn check(value: State) -> ()\n"
    ));
    for _ in 0..use_count {
        source.push_str("  accept(value)\n");
    }
    source.push_str("end\n");
    source
}

fn distinct_variant_mismatch_source(expected_count: usize, actual_count: usize) -> String {
    let expected_variants = (0..expected_count)
        .map(|index| format!("Expected{index:04}"))
        .collect::<Vec<_>>();
    let actual_variants = (0..actual_count)
        .map(|index| format!("Actual{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in expected_variants.iter().chain(&actual_variants) {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("end\n");
    let broad = expected_variants
        .iter()
        .map(|variant| format!("State::{variant}"))
        .collect::<Vec<_>>()
        .join(" | ");
    source.push_str(&format!(
        "fn accept(value: {broad}) -> ()\n  ()\nend\nfn check() -> ()\n"
    ));
    for variant in &actual_variants {
        source.push_str(&format!("  accept({variant})\n"));
    }
    source.push_str("end\n");
    source
}

fn detail_field<'a>(diagnostic: &'a Diagnostic, name: &str) -> &'a JsonValue {
    let JsonValue::Object(entries) = &diagnostic.details else {
        panic!("diagnostic details must be an object")
    };
    entries
        .iter()
        .find_map(|(field, value)| (field == name).then_some(value))
        .unwrap_or_else(|| panic!("missing diagnostic detail `{name}`"))
}

#[test]
fn repeated_broad_variant_mismatches_share_retained_diagnostic_facts() {
    for (variant_count, use_count) in [(64, 64), (64, 128), (128, 64), (128, 128)] {
        let diagnostics = diagnostics_for(&repeated_broad_variant_mismatch_source(
            variant_count,
            use_count,
        ));
        assert_eq!(diagnostics.len(), use_count, "{diagnostics:#?}");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id == "type.variant_mismatch"),
            "{diagnostics:#?}"
        );
        let first = &diagnostics[0];
        let JsonValue::Shared(first_expected_variants) = detail_field(first, "expected_variants")
        else {
            panic!("expected variants must use shared retained storage")
        };
        let JsonValue::Shared(first_excluded_variants) = detail_field(first, "excluded_variants")
        else {
            panic!("excluded variants must use shared retained storage")
        };
        for diagnostic in diagnostics.iter().skip(1) {
            assert!(diagnostic.message.shares_storage_with(&first.message));
            let JsonValue::Shared(expected_variants) =
                detail_field(diagnostic, "expected_variants")
            else {
                panic!("expected variants must use shared retained storage")
            };
            let JsonValue::Shared(excluded_variants) =
                detail_field(diagnostic, "excluded_variants")
            else {
                panic!("excluded variants must use shared retained storage")
            };
            assert!(std::sync::Arc::ptr_eq(
                expected_variants,
                first_expected_variants
            ));
            assert!(std::sync::Arc::ptr_eq(
                excluded_variants,
                first_excluded_variants
            ));
        }
        for diagnostic in [first, diagnostics.last().unwrap()] {
            let json = veln_diagnostics::diagnostic_to_json(diagnostic).to_json();
            assert!(json.contains("\"form\":\"all_except_expected\""), "{json}");
            assert!(json.contains("State::Variant0000"), "{json}");
            assert!(
                json.contains(&format!("State::Variant{:04}", variant_count - 1)),
                "{json}"
            );
        }
    }
}

#[test]
fn distinct_variant_mismatches_retain_linear_diagnostic_cache_keys() {
    for (expected_count, actual_count) in [(64, 64), (128, 128)] {
        let source = SourceFile::new(
            "main.veln",
            distinct_variant_mismatch_source(expected_count, actual_count),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::analysis::reset_retained_variant_diagnostic_key_variants();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        let elapsed = started.elapsed();
        let retained = crate::analysis::take_retained_variant_diagnostic_key_variants();
        eprintln!(
            "{expected_count} expected and {actual_count} distinct actual variants: {elapsed:?}, {retained} retained cache-key variants"
        );

        assert_eq!(diagnostics.len(), actual_count, "{diagnostics:#?}");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id == "type.variant_mismatch"),
            "{diagnostics:#?}"
        );
        assert_eq!(
            retained,
            expected_count + actual_count,
            "one broad expected key and each singleton actual key should be retained once"
        );
    }
}

fn large_variant_union_source(variant_count: usize) -> String {
    let variants = (0..=variant_count)
        .map(|index| format!("Variant{index:04}"))
        .collect::<Vec<_>>();
    let mut source = String::from("type State\n");
    for variant in &variants {
        source.push_str(&format!("  {variant}\n"));
    }
    source.push_str("end\n");
    let broad = variants[..variant_count]
        .iter()
        .map(|variant| format!("State::{variant}"))
        .collect::<Vec<_>>()
        .join(" | ");
    let narrow = variants[..variant_count - 1]
        .iter()
        .map(|variant| format!("State::{variant}"))
        .collect::<Vec<_>>()
        .join(" | ");
    source.push_str(&format!(
        "fn accept(value: {narrow}) -> ()\n  ()\nend\nfn check(value: {broad}) -> ()\n  accept(value)\nend\n"
    ));
    source
}

#[test]
fn large_variant_union_semantic_analysis_work_grows_linearly() {
    let work = [128, 256, 512].map(|variant_count| {
        let source = SourceFile::new("main.veln", large_variant_union_source(variant_count));
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::type_relations::reset_variant_set_lookups();
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        eprintln!(
            "{variant_count}-variant semantic analysis: {:?}",
            started.elapsed()
        );
        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
        assert_eq!(diagnostics[0].id, "type.variant_mismatch");
        crate::type_relations::take_variant_set_lookups()
    });
    assert!(work[0] > 0, "the metric must observe variant-set work");
    assert!(
        work[1] <= work[0] * 2 + 32 && work[2] <= work[1] * 2 + 32,
        "doubling the variant count must add only linear set work: {work:?}"
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
