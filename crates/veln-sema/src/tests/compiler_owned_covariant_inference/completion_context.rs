use super::*;

#[test]
fn complete_constructor_contributions_preserve_aliases_until_joined() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "end\n",
            "pub type First = State\n",
            "pub type Second = State\n",
            "type Pair<T>\n",
            "  Paired(T, T)\n",
            "end\n",
            "fn list_forward(first: Option<First>, second: Second::Closed)\n",
            "  [first, Some(second)]\n",
            "end\n",
            "fn list_reverse(first: Option<First>, second: Second::Closed)\n",
            "  [Some(second), first]\n",
            "end\n",
            "fn payload_forward(first: Option<First>, second: Second::Closed)\n",
            "  Paired(first, Some(second))\n",
            "end\n",
            "fn payload_reverse(first: Option<First>, second: Second::Closed)\n",
            "  Paired(Some(second), first)\n",
            "end\n",
            "fn dictionary_forward(first: Option<First>, second: Second::Closed)\n",
            "  {1: first, 2: Some(second)}\n",
            "end\n",
            "fn dictionary_reverse(first: Option<First>, second: Second::Closed)\n",
            "  {2: Some(second), 1: first}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    for function in ["list_forward", "list_reverse"] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("missing function {function}"))
                .return_type
                .render(),
            "Vec<Option<State>>",
        );
    }
    for function in ["payload_forward", "payload_reverse"] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("missing function {function}"))
                .return_type
                .render(),
            "Pair<Option<State>>::Paired",
        );
    }
    for function in ["dictionary_forward", "dictionary_reverse"] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("missing function {function}"))
                .return_type
                .render(),
            "Dict<Int, Option<State>>",
        );
    }
}

#[test]
fn incomplete_constructor_calls_use_only_admitted_completion_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Pair<T>\n",
            "  Paired(T, T)\n",
            "end\n",
            "fn private_dictionary(first: Result<Int, String>)\n",
            "  {1: first, 2: Ok(1)}\n",
            "end\n",
            "fn private_payload(first: Result<Int, String>)\n",
            "  Paired(first, Ok(1))\n",
            "end\n",
            "fn nested_dictionary(first: Option<Result<Int, String>>)\n",
            "  {1: first, 2: Some(Ok(1))}\n",
            "end\n",
            "fn nested_payload(first: Option<Result<Int, String>>)\n",
            "  Paired(first, Some(Ok(1)))\n",
            "end\n",
            "fn needs_int(value: Int) -> ()\n",
            "  ()\n",
            "end\n",
            "fn ordinary_dictionary(first: Result<Int, String>) -> ()\n",
            "  needs_int({1: first, 2: Ok(1)})\n",
            "end\n",
            "fn ordinary_payload(first: Result<Int, String>) -> ()\n",
            "  needs_int(Paired(first, Ok(1)))\n",
            "end\n",
            "fn no_context()\n",
            "  Ok(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 4, "{diagnostics:#?}");
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        2,
        "{diagnostics:#?}",
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.inference_ambiguous"),
        "{diagnostics:#?}",
    );
    let rendered = format!("{diagnostics:#?}");
    assert!(
        rendered.contains("Dict<Int, Result<Int, String>>")
            && rendered.contains("Pair<Result<Int, String>>::Paired"),
        "{diagnostics:#?}",
    );

    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("private_dictionary")
            .expect("private_dictionary function")
            .return_type
            .render(),
        "Dict<Int, Result<Int, String>>",
    );
    assert_eq!(
        environment
            .function("private_payload")
            .expect("private_payload function")
            .return_type
            .render(),
        "Pair<Result<Int, String>>::Paired",
    );
    assert_eq!(
        environment
            .function("nested_dictionary")
            .expect("nested_dictionary function")
            .return_type
            .render(),
        "Dict<Int, Option<Result<Int, String>>>",
    );
    assert_eq!(
        environment
            .function("nested_payload")
            .expect("nested_payload function")
            .return_type
            .render(),
        "Pair<Option<Result<Int, String>>>::Paired",
    );
}

#[test]
fn diagnostically_failed_control_flow_results_do_not_contribute() {
    let source_text = concat!(
        "type State\n",
        "  Ready\n",
        "  Failed\n",
        "  Closed\n",
        "end\n",
        "fn failed(value: Int) -> Option<State::Failed>\n",
        "  Some(Failed)\n",
        "end\n",
        "fn selected(first: Bool, second: Bool, ready: Option<State::Ready>, closed: Option<State::Closed>)\n",
        "  if first\n",
        "    ready\n",
        "  else if second\n",
        "    failed(\"bad\")\n",
        "  else\n",
        "    closed\n",
        "  end\n",
        "end\n",
        "fn matched(state: State, ready: Option<State::Ready>, closed: Option<State::Closed>)\n",
        "  match state\n",
        "    Ready => ready\n",
        "    Failed => failed(\"bad\")\n",
        "    Closed => closed\n",
        "  end\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}",
    );

    let environment = TypeEnvironment::from_module(&module);
    for function in ["selected", "matched"] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("missing function {function}"))
                .return_type
                .render(),
            "Option<State::Ready | State::Closed>",
        );
    }
}

#[test]
fn projected_context_is_per_argument_and_template_complete() {
    let source_text = concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "end\n",
        "pub type First = State\n",
        "pub type Second = State\n",
        "type ConcreteWrap<T>\n",
        "  Built(Vec<T>)\n",
        "end\n",
        "type SelfWrap<T>\n",
        "  Nested(SelfWrap<T>)\n",
        "  Leaf(T)\n",
        "end\n",
        "fn concrete_forward(first: ConcreteWrap<First>, closed: Second::Closed)\n",
        "  [first, Built([closed])]\n",
        "end\n",
        "fn concrete_reverse(first: ConcreteWrap<First>, closed: Second::Closed)\n",
        "  [Built([closed]), first]\n",
        "end\n",
        "fn self_forward(first: SelfWrap<First>, closed: Second::Closed)\n",
        "  [first, Nested(Leaf(closed))]\n",
        "end\n",
        "fn self_reverse(first: SelfWrap<First>, closed: Second::Closed)\n",
        "  [Nested(Leaf(closed)), first]\n",
        "end\n",
        "fn result_private(first: Result<First::Ready, String>, closed: Second::Closed)\n",
        "  [first, Ok(closed)]\n",
        "end\n",
        "fn complementary_result(first: Result<First::Ready, String>, closed: Second::Closed)\n",
        "  [first, Ok(closed), Err(\"failed\")]\n",
        "end\n",
        "fn dict_private(first: Dict<First::Ready, Vec<Int>>, closed: Second::Closed)\n",
        "  [first, {(closed): []}]\n",
        "end\n",
        "fn rollback(first: Result<First::Ready, String>, closed: Second::Closed)\n",
        "  [first, Err(1), Ok(closed)]\n",
        "end\n",
        "fn needs_int(value: Int) -> ()\n",
        "  ()\n",
        "end\n",
        "fn result_ordinary(first: Result<First::Ready, String>, closed: Second::Closed) -> ()\n",
        "  needs_int([first, Ok(closed)])\n",
        "end\n",
        "fn dict_ordinary(first: Dict<First::Ready, Vec<Int>>, closed: Second::Closed) -> ()\n",
        "  needs_int([first, {(closed): []}])\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 3, "{diagnostics:#?}");
    let rendered = format!("{diagnostics:#?}");
    assert!(
        rendered.contains("Vec<Result<State, String>>")
            && rendered.contains("Vec<Dict<State, Vec<Int>>>")
            && rendered.contains("expected `String`, but found `Int`"),
        "{diagnostics:#?}",
    );

    let environment = TypeEnvironment::from_module(&module);
    for function in ["concrete_forward", "concrete_reverse"] {
        assert_eq!(
            environment.function(function).unwrap().return_type.render(),
            "Vec<ConcreteWrap<State>>",
            "{function} must preserve order-independent Concrete payload inference",
        );
    }
    for function in ["self_forward", "self_reverse"] {
        assert_eq!(
            environment.function(function).unwrap().return_type.render(),
            "Vec<SelfWrap<State>>",
            "{function} must preserve order-independent SelfType payload inference",
        );
    }
    assert_eq!(
        environment
            .function("result_private")
            .unwrap()
            .return_type
            .render(),
        "Vec<Result<State, String>>",
    );
    assert_eq!(
        environment
            .function("complementary_result")
            .unwrap()
            .return_type
            .render(),
        "Vec<Result<State, String>>",
        "context required by separate elements must combine per type argument",
    );
    assert_eq!(
        environment
            .function("dict_private")
            .unwrap()
            .return_type
            .render(),
        "Vec<Dict<State, Vec<Int>>>",
    );
    assert_eq!(
        environment
            .function("rollback")
            .unwrap()
            .return_type
            .render(),
        "Vec<Result<State, String>>",
        "the rejected completed contribution must not change type or alias state",
    );
}

struct PartialJoinShape {
    name: &'static str,
    partial_first: &'static str,
    complete_first: &'static str,
    leading_parameter: Option<&'static str>,
    bind_ordinary_result: bool,
    expected_index: usize,
}

const PARTIAL_JOIN_SHAPES: [PartialJoinShape; 4] = [
    PartialJoinShape {
        name: "collection",
        partial_first: "[{partial}, first]",
        complete_first: "[{first}, {partial}]",
        leading_parameter: None,
        bind_ordinary_result: false,
        expected_index: 0,
    },
    PartialJoinShape {
        name: "payload",
        partial_first: "Paired({partial}, first)",
        complete_first: "Paired(first, {partial})",
        leading_parameter: None,
        bind_ordinary_result: false,
        expected_index: 1,
    },
    PartialJoinShape {
        name: "if_result",
        partial_first: "if flag\n    {partial}\n  else\n    first\n  end",
        complete_first: "if flag\n    first\n  else\n    {partial}\n  end",
        leading_parameter: Some("flag: Bool"),
        bind_ordinary_result: true,
        expected_index: 2,
    },
    PartialJoinShape {
        name: "match_result",
        partial_first: "match choice\n    One => {partial}\n    Two => first\n  end",
        complete_first: "match choice\n    One => first\n    Two => {partial}\n  end",
        leading_parameter: Some("choice: Choice"),
        bind_ordinary_result: true,
        expected_index: 2,
    },
];

struct PartialJoinKind {
    name: &'static str,
    first_type: &'static str,
    partial: &'static str,
    expected: [&'static str; 3],
}

const PARTIAL_JOIN_KINDS: [PartialJoinKind; 2] = [
    PartialJoinKind {
        name: "result",
        first_type: "Result<Option<First::Ready>, List<String>>",
        partial: "Ok(Some(closed))",
        expected: [
            "Vec<Result<Option<State>, List<String>>>",
            "Pair<Result<Option<State>, List<String>>>::Paired",
            "Result<Option<State>, List<String>>",
        ],
    },
    PartialJoinKind {
        name: "dict",
        first_type: "Dict<First::Ready, Vec<Int>>",
        partial: "{(closed): []}",
        expected: [
            "Vec<Dict<State, Vec<Int>>>",
            "Pair<Dict<State, Vec<Int>>>::Paired",
            "Dict<State, Vec<Int>>",
        ],
    },
];

struct PartialJoinFixture {
    source_text: String,
    expected_private: Vec<(String, &'static str)>,
    ordinary_checks: usize,
}

fn partial_join_parameters(kind: &PartialJoinKind, shape: &PartialJoinShape) -> String {
    let shared = format!("first: {}, closed: Second::Closed", kind.first_type);
    match shape.leading_parameter {
        Some(leading) => format!("{leading}, {shared}"),
        None => shared,
    }
}

fn append_partial_join_case(
    fixture: &mut PartialJoinFixture,
    kind: &PartialJoinKind,
    shape: &PartialJoinShape,
    order: &str,
    expression_template: &str,
) {
    let expression = expression_template
        .replace("{partial}", kind.partial)
        .replace("{first}", "first");
    let parameters = partial_join_parameters(kind, shape);
    let private_name = format!("{}_{}_{}", kind.name, shape.name, order);
    fixture.source_text.push_str(&format!(
        "fn {private_name}({parameters})\n  {expression}\nend\n"
    ));
    fixture
        .expected_private
        .push((private_name, kind.expected[shape.expected_index]));

    let check_name = format!("check_{}_{}_{}", kind.name, shape.name, order);
    let ordinary_body = if shape.bind_ordinary_result {
        format!("let joined = {expression}\n  needs_int(joined)")
    } else {
        format!("needs_int({expression})")
    };
    fixture.source_text.push_str(&format!(
        "fn {check_name}({parameters}) -> ()\n  {ordinary_body}\nend\n"
    ));
    fixture.ordinary_checks += 1;
}

fn partial_join_fixture() -> PartialJoinFixture {
    let mut fixture = PartialJoinFixture {
        source_text: concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "end\n",
            "pub type First = State\n",
            "pub type Second = State\n",
            "pub type Third = State\n",
            "type Choice\n",
            "  One\n",
            "  Two\n",
            "end\n",
            "type Pair<T>\n",
            "  Paired(T, T)\n",
            "end\n",
            "fn needs_int(value: Int) -> ()\n",
            "  ()\n",
            "end\n",
        )
        .to_string(),
        expected_private: Vec::new(),
        ordinary_checks: 0,
    };
    for kind in &PARTIAL_JOIN_KINDS {
        for shape in &PARTIAL_JOIN_SHAPES {
            append_partial_join_case(
                &mut fixture,
                kind,
                shape,
                "partial_first",
                shape.partial_first,
            );
            append_partial_join_case(
                &mut fixture,
                kind,
                shape,
                "complete_first",
                shape.complete_first,
            );
        }
    }
    fixture.source_text.push_str(concat!(
        "fn partial_rollback(first: Result<Option<Second::Ready>, List<String>>, rejected: Result<Option<Third::Ready>, Int>, closed: Second::Closed)\n",
        "  [Ok(Some(closed)), first, rejected]\n",
        "end\n",
    ));
    fixture.expected_private.push((
        "partial_rollback".to_string(),
        "Vec<Result<Option<Second>, List<String>>>",
    ));
    fixture.ordinary_checks += 1;
    fixture
}

fn assert_partial_join_results(
    environment: &TypeEnvironment,
    expected_private: Vec<(String, &'static str)>,
) {
    for (function, expected) in expected_private {
        assert_eq!(
            environment
                .function(&function)
                .unwrap_or_else(|| panic!("missing function {function}"))
                .return_type
                .render(),
            expected,
            "unexpected inferred result for {function}",
        );
    }
}

#[test]
fn partial_compiler_owned_arguments_join_in_both_orders() {
    let fixture = partial_join_fixture();
    let source = SourceFile::new("main.veln", fixture.source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    let mismatch_count = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .count();
    assert_eq!(mismatch_count, fixture.ordinary_checks, "{diagnostics:#?}");
    assert_eq!(
        diagnostics.len(),
        fixture.ordinary_checks,
        "{diagnostics:#?}"
    );

    let environment = TypeEnvironment::from_module(&module);
    assert_partial_join_results(&environment, fixture.expected_private);
}
