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
