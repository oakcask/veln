use super::*;

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
    assert_eq!(mismatches[0].related.len(), 1);
    assert!(mismatches[1].message.contains("State`"));
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
