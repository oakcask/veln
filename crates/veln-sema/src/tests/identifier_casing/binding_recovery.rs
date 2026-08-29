use super::*;

#[test]
fn invalid_value_bindings_suppress_derivative_unresolved_without_lookup() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(_input: Int) -> _output: Int\n",
            "  let Local = _input\n",
            "  Local\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        3,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn later_invalid_value_binding_does_not_suppress_unresolved_call() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  Bad()\n",
            "  let Bad = 1\n",
            "  1\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "binding name `Bad` must start with an ASCII lowercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved call_target `Bad`"
    }));
}

#[test]
fn non_callable_invalid_value_binding_does_not_suppress_unresolved_call() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  let Bad = 1\n",
            "  Bad()\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "binding name `Bad` must start with an ASCII lowercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved call_target `Bad`"
    }));
}

#[test]
fn nullary_constructor_let_pattern_does_not_become_invalid_binding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Option\n",
            "  None\n",
            "  Some(Int)\n",
            "end\n",
            "fn main(input: Option) -> Option\n",
            "  let None = input\n",
            "  None\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.invalid_case"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_value_bindings_do_not_enter_repair_candidates() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(limit: Int) -> Int\n",
            "  let Candidate = limit\n",
            "  _value satisfy candidate => candidate == Candidate\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);
    let hole = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "hole.unfilled")
        .expect("hole diagnostic");
    let details = hole.details.to_json();

    assert!(!details.contains("\"name\":\"Candidate\""), "{details}");
    assert!(details.contains("\"name\":\"limit\""), "{details}");
}

#[test]
fn invalid_handler_bindings_do_not_enter_hole_repair_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value(input: Int) -> Int\n",
            "end\n",
            "handler ask(Context: Int) handles Ask\n",
            "  value(Result) => _missing\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);
    let hole = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "hole.unfilled")
        .expect("hole diagnostic");
    let details = hole.details.to_json();

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(!details.contains("\"name\":\"Context\""), "{details}");
    assert!(!details.contains("\"name\":\"Result\""), "{details}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn unique_same_source_recovery_suppresses_only_derivative_missing_name() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Int\n  Bad()\nend\nfn Bad() -> Int\n  1\nend\n",
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        1
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved")
    );
    assert!(
        TypeEnvironment::from_module(&module)
            .function("Bad")
            .is_none()
    );
}

#[test]
fn invalid_type_with_valid_nullary_constructor_suppresses_derivative_missing_constructor() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type item\n",
            "  Value\n",
            "end\n",
            "fn main() -> Int\n",
            "  Value\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "type name `item` must start with an ASCII uppercase letter"
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
    assert!(
        TypeEnvironment::from_module(&module)
            .adts
            .descriptors()
            .iter()
            .all(|descriptor| descriptor.type_name != "item")
    );
}

#[test]
fn invalid_type_with_valid_payload_constructor_suppresses_derivative_missing_constructor() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type item\n",
            "  Value(Int)\n",
            "end\n",
            "fn main() -> Int\n",
            "  Value(1)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "type name `item` must start with an ASCII uppercase letter"
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_type_annotation_and_valid_constructor_use_leave_only_casing_failure() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type item\n",
            "  Value\n",
            "end\n",
            "fn main() -> item\n",
            "  Value\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "type name `item` must start with an ASCII uppercase letter"
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_function_value_recovery_flows_through_valid_local_callable() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn Bad(input: Int) -> Int\n",
            "  input\n",
            "end\n",
            "fn main() -> Int\n",
            "  let callable = Bad\n",
            "  callable(1)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "function name `Bad` must start with an ASCII lowercase letter"
    }));
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved" && diagnostic.id != "type.local_inference_incomplete"
        }),
        "{diagnostics:#?}"
    );
    let environment = TypeEnvironment::from_module(&module);
    assert!(environment.function("Bad").is_none());
    assert!(lower_checked_surface_module(&module).core.is_none());
}

#[test]
fn ambiguous_function_value_recovery_preserves_downstream_failures() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn Bad(input: Int) -> Int\n",
            "  input\n",
            "end\n",
            "fn Bad(input: Int) -> Int\n",
            "  input\n",
            "end\n",
            "fn main() -> Int\n",
            "  let callable = Bad\n",
            "  callable(1)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved value `Bad`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `callable`"
    }));
}

#[test]
fn incompatible_function_value_recovery_preserves_unresolved_flow() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "test Bad() -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> Int\n",
            "  let callable = Bad\n",
            "  callable(1)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved value `Bad`")
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `callable`"
    }));
}

#[test]
fn ambiguous_recovery_does_not_resolve() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  Bad()\n",
            "end\n",
            "fn Bad() -> Int\n",
            "  1\n",
            "end\n",
            "fn Bad() -> Int\n",
            "  2\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved call_target `Bad`"
    }));
}

#[test]
fn cross_class_recovery_ambiguity_preserves_unresolved_call() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  Bad(1)\n",
            "end\n",
            "type item\n",
            "  Bad(Int)\n",
            "end\n",
            "fn Bad(value: Int) -> Int\n",
            "  value\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "type name `item` must start with an ASCII uppercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "function name `Bad` must start with an ASCII lowercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved call_target `Bad`"
    }));
}

#[test]
fn incompatible_recovery_does_not_suppress_unresolved_call() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  Bad()\n",
            "end\n",
            "test Bad() -> ()\n",
            "  ()\n",
            "end\n",
            "fn WrongArity(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "fn caller() -> Int\n",
            "  WrongArity()\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved call_target `Bad`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `WrongArity`"
    }));
}

#[test]
fn valid_function_lookup_ignores_same_source_recovery_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  good()\n",
            "end\n",
            "fn good() -> Int\n",
            "  1\n",
            "end\n",
            "fn Bad() -> Int\n",
            "  2\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "name.unresolved" || diagnostic.message != "unresolved call_target `good`"
    }));
    let environment = TypeEnvironment::from_module(&module);
    assert!(environment.function("good").is_some());
    assert!(environment.function("Bad").is_none());
}

#[test]
fn valid_constructor_lookup_wins_over_same_spelled_function_recovery() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Bad\n",
            "  Bad\n",
            "end\n",
            "fn main() -> Bad\n",
            "  Bad\n",
            "end\n",
            "fn Bad() -> Bad\n",
            "  Bad\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        1
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic.message
            == "function name `Bad` must start with an ASCII lowercase letter"),
        "{diagnostics:#?}"
    );

    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}
