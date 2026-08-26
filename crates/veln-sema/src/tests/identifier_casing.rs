use super::*;
use crate::types::environment::TypeEnvironment;

#[test]
fn covered_source_names_report_exact_casing_contract_details() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type item\n",
            "  value\n",
            "  _Payload(Int)\n",
            "end\n",
            "fn Build(Input: Int) -> Output: Int\n",
            "  let Local = Input\n",
            "  match Local\n",
            "    _bound => _bound\n",
            "  end\n",
            "end\n",
            "test Verify() -> ()\n",
            "  ()\n",
            "end\n",
            "pub fn Exported = Build\n",
            "pub type exported = item\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let mut diagnostics = analyze_surface_module(&module)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();
    diagnostics.sort_by_key(|diagnostic| {
        diagnostic
            .span
            .as_ref()
            .map(|span| (span.start.offset, span.end.offset))
    });

    let expected = [
        ("item", "type", "declaration", 1, 6, 10),
        ("value", "constructor", "declaration", 2, 3, 8),
        ("_Payload", "constructor", "declaration", 3, 3, 11),
        ("Build", "function", "declaration", 5, 4, 9),
        ("Input", "value_binding", "binding", 5, 10, 15),
        ("Output", "value_binding", "binding", 5, 25, 31),
        ("Local", "value_binding", "pattern_head", 6, 7, 12),
        ("_bound", "value_binding", "pattern_head", 8, 5, 11),
        ("Verify", "function", "declaration", 11, 6, 12),
        ("Exported", "function", "declaration", 14, 8, 16),
        ("exported", "type", "declaration", 15, 10, 18),
    ];
    assert_eq!(diagnostics.len(), expected.len(), "{diagnostics:#?}");
    for (diagnostic, (name, class, occurrence, line, start, end)) in
        diagnostics.iter().zip(expected)
    {
        let span = diagnostic.span.as_ref().expect("name diagnostic span");
        assert_eq!(
            (span.start.line, span.start.column, span.end.column),
            (line, start, end)
        );
        let details = diagnostic.details.to_json();
        assert!(details.contains(&format!("\"name\":\"{name}\"")));
        assert!(details.contains(&format!("\"name_class\":\"{class}\"")));
        assert!(details.contains(&format!("\"occurrence\":\"{occurrence}\"")));
        assert!(details.contains("\"phase\":\"name\""));
        assert!(details.contains("\"origin\":\"source\""));
    }

    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn accepted_names_and_expression_holes_keep_existing_behavior() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  Value(Int)\n",
            "end\n",
            "fn build(input: Int) -> output: Int\n",
            "  let local = input\n",
            "  local\n",
            "end\n",
            "fn incomplete() -> Int\n",
            "  _missing\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    assert!(module.invalid_names.is_empty());
    assert!(
        analyze_surface_module(&module)
            .iter()
            .all(|diagnostic| diagnostic.id != "name.invalid_case")
    );
}

#[test]
fn underscore_led_binding_recovers_without_missing_identifier_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        "fn _build(_input: Int) -> _output: Int\n  let _local = _input\n  _local\nend\n",
    );
    let parsed = parse(&source);

    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    assert_eq!(module.invalid_names.len(), 4);
    assert!(
        module
            .invalid_names
            .iter()
            .all(|name| name.name.starts_with('_'))
    );
}

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

#[test]
fn recovery_is_not_visible_through_an_import() {
    let module = merged_modules(vec![
        SourceFile::new(
            "main.veln",
            concat!(
                "mod main\n",
                "use helper\n",
                "fn main() -> Int\n",
                "  helper::Bad()\n",
                "end\n",
            ),
        ),
        SourceFile::new("helper.veln", "mod helper\npub fn Bad() -> Int\n  1\nend\n"),
    ]);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `helper::Bad`"
    }));
}

#[test]
fn recovery_is_not_visible_through_public_alias_targets() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn Bad() -> Int\n",
            "  1\n",
            "end\n",
            "type item\n",
            "  Value\n",
            "end\n",
            "pub fn exposed = Bad\n",
            "pub type Exposed = item\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "function name `Bad` must start with an ASCII lowercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "type name `item` must start with an ASCII uppercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved function alias target `Bad`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved type alias target `item`"
    }));

    let environment = TypeEnvironment::from_module(&module);
    assert!(environment.function("exposed").is_none());
}

#[test]
fn invalid_public_function_alias_recovery_suppresses_derivative_unresolved_without_lookup() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  Exported()\n",
            "end\n",
            "fn good() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn Exported = good\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message
                == "function name `Exported` must start with an ASCII lowercase letter"
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
    assert!(
        TypeEnvironment::from_module(&module)
            .function("Exported")
            .is_none()
    );
}
