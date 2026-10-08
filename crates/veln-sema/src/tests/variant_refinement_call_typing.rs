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
fn expected_adt_disambiguates_and_lowers_nullary_constructor() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Left\n",
            "  Ready\n",
            "end\n",
            "type Right\n",
            "  Ready\n",
            "end\n",
            "fn pick() -> Left\n",
            "  Ready\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let function = core
        .functions
        .iter()
        .find(|function| function.name == "pick")
        .expect("pick should be lowered");
    let CoreStmtKind::Return { expr } = &function.body[0].kind else {
        panic!("pick should return a constructor");
    };
    assert!(
        matches!(&expr.kind, CoreExprKind::AdtVariant { name, payloads }
            if name == &vec!["Left".to_string(), "Ready".to_string()]
                && payloads.is_empty()),
        "the expected Left type should select Left::Ready"
    );
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
fn alias_qualified_refinements_share_target_identity_and_preserve_annotations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "pub type First = State\n",
            "pub type Second = First\n",
            "type Box<A>\n",
            "  Empty\n",
            "  Boxed(A)\n",
            "end\n",
            "pub type StateBox = Box\n",
            "pub type OuterStateBox = StateBox\n",
            "fn direct(value: State::Ready) -> First::Ready\n",
            "  value\n",
            "end\n",
            "fn alias_singleton(value: First::Ready) -> State::Ready\n",
            "  value\n",
            "end\n",
            "fn alias_union(value: Second::Closed | Second::Ready | Second::Closed) -> First::Ready | State::Closed\n",
            "  value\n",
            "end\n",
            "fn generic(value: OuterStateBox<Int>::Boxed) -> Box<Int>::Boxed\n",
            "  value\n",
            "end\n",
            "fn alias_constructor()\n",
            "  First::Ready\n",
            "end\n",
            "fn alias_and_target(value: First::Ready | State::Closed) -> ()\n",
            "  ()\n",
            "end\n",
            "fn target_and_alias(value: State::Ready | First::Closed) -> ()\n",
            "  ()\n",
            "end\n",
            "fn alias_transition(value: First::Ready) -> First::Ready\n",
            "  value\n",
            "end\n",
            "fn function_value_identity() -> ()\n",
            "  let transition: fn(State::Ready) -> State::Ready = alias_transition\n",
            "  transition(Ready)\n",
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
        environment.function("direct").unwrap().return_type.render(),
        "First::Ready"
    );
    let alias_singleton = &environment.function("alias_singleton").unwrap().params[0];
    let target_singleton = &environment.function("direct").unwrap().params[0];
    assert!(crate::type_relations::is_assignable(
        alias_singleton,
        target_singleton
    ));
    assert!(crate::type_relations::is_assignable(
        target_singleton,
        alias_singleton
    ));
    assert_eq!(
        environment.function("alias_union").unwrap().params[0].render(),
        "Second::Ready | Second::Closed"
    );
    assert_eq!(
        environment
            .function("alias_union")
            .unwrap()
            .return_type
            .render(),
        "First::Ready | First::Closed"
    );
    assert_eq!(
        environment.function("generic").unwrap().params[0].render(),
        "OuterStateBox<Int>::Boxed"
    );
    assert_eq!(
        environment
            .function("alias_constructor")
            .unwrap()
            .return_type
            .render(),
        "State::Ready"
    );
    assert_eq!(
        environment.function("alias_and_target").unwrap().params[0].render(),
        "First::Ready | First::Closed"
    );
    assert_eq!(
        environment.function("target_and_alias").unwrap().params[0].render(),
        "First::Ready | First::Closed"
    );
}

#[test]
fn alias_refinement_joins_and_mismatches_keep_independent_presentation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "pub type First = State\n",
            "pub type Second = State\n",
            "type Envelope<A>\n",
            "  Left(A)\n",
            "  Right(A)\n",
            "end\n",
            "fn same_alias(flag: Bool)\n",
            "  let ready: First::Ready = Ready\n",
            "  let closed: First::Closed = Closed\n",
            "  if flag\n",
            "    ready\n",
            "  else\n",
            "    closed\n",
            "  end\n",
            "end\n",
            "fn conflicting_aliases(flag: Bool)\n",
            "  let ready: First::Ready = Ready\n",
            "  let closed: Second::Closed = Closed\n",
            "  if flag\n",
            "    ready\n",
            "  else\n",
            "    closed\n",
            "  end\n",
            "end\n",
            "fn inferred(flag: Bool)\n",
            "  if flag\n",
            "    Ready\n",
            "  else\n",
            "    Closed\n",
            "  end\n",
            "end\n",
            "fn generic_argument_union(value: Envelope<First::Ready>::Left | Envelope<Second::Ready>::Right) -> ()\n",
            "  ()\n",
            "end\n",
            "fn nested_conflicting_aliases(flag: Bool)\n",
            "  let left: Envelope<First::Ready>::Left = Left(Ready)\n",
            "  let other: Envelope<Second::Ready>::Left = Left(Ready)\n",
            "  if flag\n",
            "    left\n",
            "  else\n",
            "    other\n",
            "  end\n",
            "end\n",
            "fn needs_first(value: First::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn mismatch() -> ()\n",
            "  let actual: Second::Closed = Closed\n",
            "  needs_first(actual)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].message,
        "value of type `Second::Closed` is not assignable to variant type `First::Ready`"
    );
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("same_alias")
            .unwrap()
            .return_type
            .render(),
        "First::Ready | First::Closed"
    );
    assert_eq!(
        environment
            .function("conflicting_aliases")
            .unwrap()
            .return_type
            .render(),
        "State::Ready | State::Closed"
    );
    assert_eq!(
        environment
            .function("inferred")
            .unwrap()
            .return_type
            .render(),
        "State::Ready | State::Closed"
    );
    assert_eq!(
        environment
            .function("generic_argument_union")
            .unwrap()
            .params[0]
            .render(),
        "Envelope<State::Ready>"
    );
    assert_eq!(
        environment
            .function("nested_conflicting_aliases")
            .unwrap()
            .return_type
            .render(),
        "Envelope<State::Ready>::Left"
    );
}

#[test]
fn collapsed_refinement_joins_reconcile_alias_presentation_in_both_orders() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Single\n",
            "  Only\n",
            "end\n",
            "pub type SingleFirst = Single\n",
            "pub type SingleSecond = Single\n",
            "type Pair\n",
            "  Left\n",
            "  Right\n",
            "end\n",
            "pub type PairFirst = Pair\n",
            "pub type PairSecond = Pair\n",
            "fn single_forward(flag: Bool)\n",
            "  let first: SingleFirst::Only = Only\n",
            "  let second: SingleSecond::Only = Only\n",
            "  if flag\n",
            "    first\n",
            "  else\n",
            "    second\n",
            "  end\n",
            "end\n",
            "fn single_reverse(flag: Bool)\n",
            "  let first: SingleFirst::Only = Only\n",
            "  let second: SingleSecond::Only = Only\n",
            "  if flag\n",
            "    second\n",
            "  else\n",
            "    first\n",
            "  end\n",
            "end\n",
            "fn pair_forward(flag: Bool)\n",
            "  let first: PairFirst::Left | PairFirst::Right = Left\n",
            "  let second: PairSecond::Left | PairSecond::Right = Right\n",
            "  if flag\n",
            "    first\n",
            "  else\n",
            "    second\n",
            "  end\n",
            "end\n",
            "fn pair_reverse(flag: Bool)\n",
            "  let first: PairFirst::Left | PairFirst::Right = Left\n",
            "  let second: PairSecond::Left | PairSecond::Right = Right\n",
            "  if flag\n",
            "    second\n",
            "  else\n",
            "    first\n",
            "  end\n",
            "end\n",
            "fn single_canonical_then_alias(flag: Bool)\n",
            "  let canonical: Single = Only\n",
            "  let alias: SingleFirst::Only = Only\n",
            "  if flag\n",
            "    canonical\n",
            "  else\n",
            "    alias\n",
            "  end\n",
            "end\n",
            "fn pair_canonical_then_alias(flag: Bool)\n",
            "  let canonical: Pair = Left\n",
            "  let alias: PairFirst::Left | PairFirst::Right = Right\n",
            "  if flag\n",
            "    canonical\n",
            "  else\n",
            "    alias\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);

    for function in ["single_forward", "single_reverse"] {
        assert_eq!(
            environment.function(function).unwrap().return_type.render(),
            "Single"
        );
    }
    for function in ["pair_forward", "pair_reverse"] {
        assert_eq!(
            environment.function(function).unwrap().return_type.render(),
            "Pair"
        );
    }
    assert_eq!(
        environment
            .function("single_canonical_then_alias")
            .unwrap()
            .return_type
            .render(),
        "SingleFirst"
    );
    assert_eq!(
        environment
            .function("pair_canonical_then_alias")
            .unwrap()
            .return_type
            .render(),
        "PairFirst"
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
