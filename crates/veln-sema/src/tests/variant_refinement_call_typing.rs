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
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
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
fn nested_aggregate_widening_is_rejected_for_inline_and_bound_values() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Box<A>\n",
            "  Boxed(A)\n",
            "end\n",
            "fn records() -> ()\n",
            "  let inline: {state: State} = {state: Ready}\n",
            "  let retained = {state: Ready}\n",
            "  let bound: {state: State} = retained\n",
            "end\n",
            "fn payloads() -> ()\n",
            "  let inline: Box<State> = Boxed(Ready)\n",
            "  let retained = Boxed(Ready)\n",
            "  let bound: Box<State> = retained\n",
            "end\n",
            "fn collections() -> ()\n",
            "  let inline: Vec<State> = [Ready]\n",
            "  let retained = [Ready]\n",
            "  let bound: Vec<State> = retained\n",
            "end\n",
        )
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
            .count(),
        6,
        "{diagnostics:#?}"
    );
}

#[test]
fn recursive_adt_payloads_reject_nested_refinement_widening() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Wrapped(State)\n",
        "end\n",
        "fn main() -> ()\n",
        "  let invalid = Wrapped(Ready)\n",
        "end\n",
    ));

    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert!(mismatches[0].message.contains("State::Ready"));
    assert!(mismatches[0].message.contains("State`"));
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
fn base_mismatch_diagnostic_output_grows_linearly_with_variant_count() {
    fn diagnostic_bytes(variant_count: usize) -> (usize, usize) {
        let mut source = String::from("type State\n");
        for index in 0..variant_count {
            source.push_str(&format!("  Variant{index:03}\n"));
        }
        source.push_str("end\n");
        for index in 0..variant_count {
            source.push_str(&format!(
                "fn reject{index:03}(value: State) -> State::Variant{index:03}\n  value\nend\n"
            ));
        }
        let diagnostics = diagnostics_for(&source);
        let mismatches = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_mismatch")
            .collect::<Vec<_>>();
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

    let (small_count, small_bytes) = diagnostic_bytes(32);
    let (large_count, large_bytes) = diagnostic_bytes(64);
    eprintln!(
        "variant mismatch diagnostic bytes at 32 and 64 variants: {small_bytes}, {large_bytes}"
    );
    assert_eq!((small_count, large_count), (32, 64));
    assert!(
        large_bytes <= small_bytes * 3,
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
