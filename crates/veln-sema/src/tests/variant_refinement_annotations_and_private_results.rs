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

mod base_failures;
mod casing_base_failures;
mod declaration_quarantine;
mod lowercase_final_recovery;
mod public_visibility;
mod scaling;
mod unknown_finals;

#[test]
fn records_are_covariant_while_named_arguments_remain_invariant() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Box<A>\n",
            "  Boxed(A)\n",
            "end\n",
            "fn widen_record(value: {state: State::Ready}) -> ()\n",
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

    assert_eq!(diagnostics.len(), 5, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn vec_arguments_are_covariant_at_complete_assignment_boundaries() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Box<A>\n",
            "  Boxed(A)\n",
            "end\n",
            "fn state_vectors(value: Vec<State>) -> Vec<State::Ready>\n",
            "  [Ready]\n",
            "end\n",
            "fn ready_vectors(value: Vec<State::Ready>) -> Vec<State>\n",
            "  value\n",
            "end\n",
            "fn boundaries(ready: Vec<State::Ready>, states: Vec<State>, boxed: Box<State::Ready>, record: {items: Vec<State::Ready>}) -> ()\n",
            "  let widened: Vec<State> = ready\n",
            "  let widened_record: {items: Vec<State>} = record\n",
            "  let composed: fn(Vec<State::Ready>) -> Vec<State> = state_vectors\n",
            "  let narrowed: Vec<State::Ready> = states\n",
            "  let boxed_widening: Box<State> = boxed\n",
            "  let reversed: fn(Vec<State>) -> Vec<State::Ready> = ready_vectors\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn standard_immutable_constructor_arguments_are_covariant_by_slot() {
    let cases = [
        ("Option<State::Ready>", "Option<State>"),
        ("Result<State::Ready, String>", "Result<State, String>"),
        ("Result<Int, State::Ready>", "Result<Int, State>"),
        ("List<State::Ready>", "List<State>"),
        ("Dict<State::Ready, Int>", "Dict<State, Int>"),
        ("Dict<String, State::Ready>", "Dict<String, State>"),
    ];

    for (narrow, wide) in cases {
        let diagnostics = diagnostics_for(&format!(
            "{STATE_DECL}{}",
            format!(
                "fn wide_to_narrow(value: {wide}) -> {narrow}\n  value\nend\n\
                 fn narrow_to_wide(value: {narrow}) -> {wide}\n  value\nend\n\
                 fn nested(actual: {{value: {narrow}}}, wide: {wide}, narrow: {narrow}) -> ()\n\
                   let widened: {wide} = narrow\n\
                   let widened_record: {{value: {wide}}} = actual\n\
                   let composed: fn({narrow}) -> {wide} = wide_to_narrow\n\
                   let reversed: {narrow} = wide\n\
                   let reversed_composed: fn({wide}) -> {narrow} = narrow_to_wide\n\
                 end\n"
            )
        ));

        assert_eq!(diagnostics.len(), 3, "{narrow} -> {wide}: {diagnostics:#?}");
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id == "type.mismatch"),
            "{narrow} -> {wide}: {diagnostics:#?}"
        );
    }
}

#[test]
fn standard_immutable_constructor_aliases_retain_covariance() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "pub type Optional = Option\n",
            "pub type Outcome = Result\n",
            "pub type Sequence = List\n",
            "fn boundaries(optional: Optional<State::Ready>, outcome: Outcome<State::Ready, String>, sequence: Sequence<State::Ready>) -> ()\n",
            "  let widened_optional: Option<State> = optional\n",
            "  let widened_outcome: Result<State, String> = outcome\n",
            "  let widened_sequence: List<State> = sequence\n",
            "end\n",
        )
    ));

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn standard_adt_refinements_compose_argument_covariance_with_variant_subsets() {
    let cases = [
        (
            "Option<State::Ready>::Some",
            "Option<State>::Some | Option<State>::None",
        ),
        (
            "Result<State::Ready, String>::Ok",
            "Result<State, String>::Ok | Result<State, String>::Err",
        ),
        (
            "Result<Int, State::Ready>::Err",
            "Result<Int, State>::Ok | Result<Int, State>::Err",
        ),
        (
            "List<State::Ready>::Cons",
            "List<State>::Nil | List<State>::Cons",
        ),
    ];

    for (narrow, wide) in cases {
        let diagnostics = diagnostics_for(&format!(
            "{STATE_DECL}fn boundaries(narrow: {narrow}, wide: {wide}) -> ()\n\
               let accepted: {wide} = narrow\n\
               let rejected: {narrow} = wide\n\
             end\n"
        ));

        assert_eq!(diagnostics.len(), 1, "{narrow} -> {wide}: {diagnostics:#?}");
        assert_eq!(
            diagnostics[0].id, "type.mismatch",
            "{narrow} -> {wide}: {diagnostics:#?}"
        );
    }
}

#[test]
fn same_spelled_source_dict_and_other_generics_remain_invariant_and_nominal() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Dict<K, V>\n  SourceDict(K, V)\nend\n",
            "type Box<A>\n  Boxed(A)\nend\n",
            "fn boundaries(source_dict: Dict<String, State::Ready>, boxed: Box<State::Ready>) -> ()\n",
            "  let invariant_dict: Dict<String, State> = source_dict\n",
            "  let invariant_box: Box<State> = boxed\n",
            "  let standard_dict = {\"ready\": Ready}\n",
            "  let nominal_dict: Dict<String, State> = standard_dict\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}"
    );
}

#[test]
fn identity_free_source_vec_cannot_acquire_compiler_vec_covariance() {
    let diagnostics = diagnostics_for(&format!(
        "{STATE_DECL}{}",
        concat!(
            "type Vec<A>\n",
            "  SourceVec(A)\n",
            "end\n",
            "type Box<A>\n",
            "  Boxed(A)\n",
            "end\n",
            "fn boundaries(source_ready: Vec<State::Ready>) -> ()\n",
            "  let invariant_source: Vec<State> = source_ready\n",
            "  let compiler_value: Vec<State::Ready> = [Ready]\n",
            "  let compiler_box = Boxed([Ready])\n",
            "  let nested_source: Box<Vec<State::Ready>> = compiler_box\n",
            "end\n",
        )
    ));

    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.id.as_str(), diagnostic.message.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (
                "type.mismatch",
                "expected `Vec<State>`, but found `Vec<State::Ready>`"
            ),
            (
                "type.mismatch",
                "expected `Vec<State::Ready>`, but found `Vec<State::Ready>`"
            ),
            (
                "type.mismatch",
                "expected `Box<Vec<State::Ready>>`, but found `Box<Vec<State::Ready>>::Boxed`"
            ),
        ],
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
