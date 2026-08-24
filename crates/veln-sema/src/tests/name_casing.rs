use super::support::*;

#[test]
fn invalid_source_name_classes_have_exact_diagnostics_without_a_missing_name_cascade() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type item\n",
            "  made\n",
            "  _other\n",
            "end\n",
            "fn Build(_value: Int) -> Result: Int\n",
            "  Build(1)\n",
            "end\n",
        ),
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        6
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved")
    );
    let function = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.message == "function name must start with an ASCII lowercase letter"
        })
        .expect("function casing diagnostic");
    assert_diagnostic_span(function, 5, 4, 5, 9);
    assert!(function.details.to_json().contains(
        "\"origin\":\"source\",\"occurrence\":\"declaration\",\"name\":\"Build\",\"name_class\":\"function\",\"required_initial\":\"ascii_lowercase\",\"observed_initial\":\"ascii_uppercase\""
    ));
}

#[test]
fn invalid_declarations_never_produce_checked_artifacts() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        "fn Broken() -> Int\n  1\nend\n",
    ));
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
    assert_eq!(lowered.diagnostics[0].id, "name.invalid_case");
}

#[test]
fn test_declaration_names_use_function_identifier_casing() {
    let parsed = parse(&SourceFile::new(
        "main_test.veln",
        "test BrokenTest() -> ()\n  ()\nend\n",
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.invalid_case")
        .expect("test declaration casing diagnostic");
    assert_eq!(
        diagnostic.message,
        "function name must start with an ASCII lowercase letter"
    );
    assert_diagnostic_span(diagnostic, 1, 6, 1, 16);
    assert!(diagnostic.details.to_json().contains(
        "\"origin\":\"source\",\"occurrence\":\"declaration\",\"name\":\"BrokenTest\",\"name_class\":\"function\",\"required_initial\":\"ascii_lowercase\",\"observed_initial\":\"ascii_uppercase\""
    ));
}

#[test]
fn invalid_tests_do_not_recover_function_calls() {
    let parsed = parse(&SourceFile::new(
        "main_test.veln",
        concat!(
            "test Broken() -> ()\n",
            "  ()\n",
            "end\n",
            "test caller() -> ()\n",
            "  Broken()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "function name must start with an ASCII lowercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `Broken`"
    }));
}

#[test]
fn valid_test_declaration_names_do_not_emit_identifier_casing_diagnostics() {
    let parsed = parse(&SourceFile::new(
        "main_test.veln",
        "test valid_test() -> ()\n  ()\nend\n",
    ));
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
fn invalid_public_alias_declaration_names_use_type_and_function_classes() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "pub type Valid\n",
            "  Made\n",
            "end\n",
            "pub fn good() -> Int\n",
            "  1\n",
            "end\n",
            "pub type exposed = Valid\n",
            "pub fn Build = good\n",
        ),
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let invalid_case = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.invalid_case")
        .collect::<Vec<_>>();
    assert_eq!(invalid_case.len(), 2, "{diagnostics:#?}");
    assert!(invalid_case.iter().any(|diagnostic| {
        diagnostic.message == "type name must start with an ASCII uppercase letter"
            && diagnostic
                .details
                .to_json()
                .contains("\"name\":\"exposed\"")
            && diagnostic
                .details
                .to_json()
                .contains("\"name_class\":\"type\"")
    }));
    assert!(invalid_case.iter().any(|diagnostic| {
        diagnostic.message == "function name must start with an ASCII lowercase letter"
            && diagnostic.details.to_json().contains("\"name\":\"Build\"")
            && diagnostic
                .details
                .to_json()
                .contains("\"name_class\":\"function\"")
    }));
}

#[test]
fn invalid_public_alias_declaration_names_report_duplicates() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type exposed\n",
            "  Made\n",
            "end\n",
            "fn Build() -> Int\n",
            "  1\n",
            "end\n",
            "pub type exposed = Int\n",
            "pub fn Build = good\n",
            "fn good() -> Int\n",
            "  1\n",
            "end\n",
        ),
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count()
            >= 2
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.duplicate"
                && diagnostic.message == "duplicate type alias name `exposed`"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.duplicate"
                && diagnostic.message == "duplicate function alias name `Build`"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_function_declarations_do_not_recover_public_alias_targets() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn Broken() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn exposed = Broken\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case")
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved function alias target `Broken`"
    }));
}

#[test]
fn invalid_function_alias_preserves_missing_target_diagnostic() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "pub fn Bad = missing\n",
            "\n",
            "pub fn main() -> Int\n",
            "  1\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "function name must start with an ASCII lowercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved function alias target `missing`"
    }));
}

#[test]
fn invalid_function_alias_preserves_wrong_kind_target_diagnostic() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type Target\n",
            "  Made\n",
            "end\n",
            "\n",
            "pub fn Bad = Target\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "function name must start with an ASCII lowercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message == "public alias target `Target` is a type, not a function"
    }));
}

#[test]
fn invalid_function_alias_declaration_name_recovers_unique_same_file_call() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn good() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "pub fn Bad = good\n",
            "\n",
            "pub fn main() -> Int\n",
            "  Bad()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "function name must start with an ASCII lowercase letter"
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message != "unresolved call_target `Bad`"),
        "{diagnostics:#?}"
    );
}

#[test]
fn same_spelled_invalid_function_aliases_do_not_recover_call() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn good() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "pub fn Bad = good\n",
            "pub fn Bad = good\n",
            "\n",
            "pub fn main() -> Int\n",
            "  Bad()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved call_target `Bad`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate function alias name `Bad`"
    }));
}

#[test]
fn invalid_function_and_alias_candidates_do_not_recover_call() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn good() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "fn Bad() -> Int\n",
            "  2\n",
            "end\n",
            "\n",
            "pub fn Bad = good\n",
            "\n",
            "pub fn main() -> Int\n",
            "  Bad()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved call_target `Bad`"
    }));
}

#[test]
fn invalid_type_declarations_do_not_recover_public_alias_targets() {
    let broken = parse(&SourceFile::new(
        "broken.veln",
        concat!(
            "mod broken\n",
            "type broken\n",
            "  pub Made\n",
            "end\n",
            "pub type Exposed = broken\n",
        ),
    ));
    let main = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "use broken\n",
            "fn main(value: broken::Exposed) -> Int\n",
            "  1\n",
            "end\n",
        ),
    ));
    let broken = lower_surface_ast(&broken.tree);
    let main = lower_surface_ast(&main.tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: broken.aliases,
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: broken.types,
        functions: main.functions,
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "type name must start with an ASCII uppercase letter"
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.mismatch"),
        "{diagnostics:#?}"
    );
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn invalid_callable_local_and_function_candidates_do_not_select_recovery() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn Callback(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "\n",
            "fn main(Callback: fn(Int) -> Int) -> Int\n",
            "  Callback(1)\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `Callback`"
    }));
}

#[test]
fn invalid_type_declaration_and_same_spelled_alias_report_duplicate() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type bad\n",
            "  Made\n",
            "end\n",
            "\n",
            "type Valid\n",
            "  Other\n",
            "end\n",
            "\n",
            "pub type bad = Valid\n",
            "\n",
            "fn main(value: bad) -> Int\n",
            "  1\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate" && diagnostic.message == "duplicate type alias name `bad`"
    }));
}

#[test]
fn invalid_result_binding_recovers_only_ensure_contracts() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn require_cannot_read_result(value: Int) -> Output: Int\n",
            "  require Output >= 0\n",
            "  value\n",
            "end\n",
            "\n",
            "fn invariant_cannot_read_result(value: Int) -> Output: Int\n",
            "  invariant Output >= 0\n",
            "  value\n",
            "end\n",
            "\n",
            "fn ensure_can_read_result(value: Int) -> Output: Int\n",
            "  ensure Output >= 0\n",
            "  value\n",
            "end\n",
            "\n",
            "fn body_cannot_read_result(value: Int) -> Output: Int\n",
            "  Output\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        4
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.message == "unresolved contract_predicate `Output`")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved value `Output`"
    }));
}

#[test]
fn multiple_invalid_functions_do_not_select_a_recovery_symbol() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn Broken(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "fn Broken() -> Int\n",
            "  1\n",
            "end\n",
            "fn main() -> Int\n",
            "  Broken()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved")
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate function declaration name `Broken`"
    }));
}

#[test]
fn invalid_type_and_constructor_declarations_report_same_kind_duplicates() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type item\n",
            "  made\n",
            "  made\n",
            "end\n",
            "type item\n",
            "  other\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        5
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate constructor declaration name `made`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate type declaration name `item`"
    }));
}

#[test]
fn a_valid_constructor_wins_over_an_invalid_same_spelled_function() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type Choice\n",
            "  Build\n",
            "end\n",
            "fn Build() -> Int\n",
            "  1\n",
            "end\n",
            "fn main() -> Choice\n",
            "  Build()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.invalid_case");
    assert_eq!(
        diagnostics[0].message,
        "function name must start with an ASCII lowercase letter"
    );
}

#[test]
fn accepted_declaration_and_binding_names_keep_checked_artifacts() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type Choice\n",
            "  Made(Int)\n",
            "end\n",
            "fn build(value: Int) -> result: Choice\n",
            "  let local = value\n",
            "  Made(local)\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.core.is_some());
    assert!(lowered.ir.is_some());
}

#[test]
fn incompatible_recovery_class_does_not_suppress_unresolved_call() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type item\n",
            "  Made\n",
            "end\n",
            "fn main() -> Int\n",
            "  item()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved")
    );
}

#[test]
fn invalid_constructor_does_not_suppress_unresolved_value() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type Box\n",
            "  item(value: Int)\n",
            "end\n",
            "fn main() -> Int\n",
            "  item\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "constructor name must start with an ASCII uppercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved value `item`"
    }));
}

#[test]
fn invalid_value_binding_is_quarantined_but_unique_value_use_recovers() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!("fn main(Value: Int) -> Int\n", "  Value\n", "end\n",),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.invalid_case");
    assert_eq!(
        diagnostics[0].message,
        "binding name must start with an ASCII lowercase letter"
    );
}

#[test]
fn invalid_pattern_bindings_recover_unique_same_scope_uses() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type Pair\n",
            "  Pair(Int, Int)\n",
            "end\n",
            "\n",
            "fn from_match(input: Pair) -> Int\n",
            "  match input\n",
            "    Pair(Head, tail) => Head + tail\n",
            "  end\n",
            "end\n",
            "\n",
            "fn from_destructure(input: Pair) -> Int\n",
            "  let Pair(First, second) = input\n",
            "  First + second\n",
            "end\n",
        ),
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let invalid_bindings = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.invalid_case"
                && diagnostic.message == "binding name must start with an ASCII lowercase letter"
        })
        .collect::<Vec<_>>();

    assert_eq!(invalid_bindings.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_handler_clause_bindings_report_exact_spans() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value(item: Int) -> Int\n",
            "end\n",
            "handler recover(Value: Int) handles Ask\n",
            "  value(_item) => Value\n",
            "end\n",
        ),
    ));
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let mut binding_diagnostics = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.invalid_case"
                && diagnostic.message == "binding name must start with an ASCII lowercase letter"
        })
        .collect::<Vec<_>>();
    binding_diagnostics.sort_by_key(|diagnostic| {
        diagnostic
            .span
            .as_ref()
            .map(|span| span.start.offset)
            .unwrap_or_default()
    });

    assert_eq!(binding_diagnostics.len(), 2, "{diagnostics:#?}");
    assert_diagnostic_span(binding_diagnostics[0], 4, 17, 4, 22);
    assert_diagnostic_span(binding_diagnostics[1], 5, 9, 5, 14);
}

#[test]
fn invalid_value_bindings_report_same_scope_duplicates_without_normal_lookup() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn main(Value: Int, Value: Int) -> Int\n",
            "  Value\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate parameter name `Value`"
    }));
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved")
    );
}

#[test]
fn multiple_invalid_value_bindings_do_not_select_recovery() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn main(Value: Int, Value: Int) -> Int\n",
            "  Value\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved")
    );
}

#[test]
fn invalid_callable_binding_recovers_only_for_function_typed_calls() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn main(Callback: fn(Int) -> Int, Number: Int) -> Int\n",
            "  Callback(1)\n",
            "  Number()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `Number`")
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message != "unresolved value `Number`")
    );
}

#[test]
fn invalid_inferred_callable_let_binding_recovers_call_target() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "\n",
            "fn main() -> String\n",
            "  let Callback = stringify\n",
            "  Callback(1)\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

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
            .all(|diagnostic| diagnostic.message != "unresolved call_target `Callback`"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_inferred_callable_pattern_binding_recovers_call_target() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type CallbackBox\n",
            "  CallbackBox(fn(Int) -> String)\n",
            "end\n",
            "\n",
            "fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "\n",
            "fn main(input: CallbackBox) -> String\n",
            "  let CallbackBox(Callback) = input\n",
            "  Callback(1)\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

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
            .all(|diagnostic| diagnostic.message != "unresolved call_target `Callback`"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_inferred_callable_binding_and_function_candidates_do_not_select_recovery() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn Callback(value: Int) -> String\n",
            "  \"bad\"\n",
            "end\n",
            "\n",
            "fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "\n",
            "fn main() -> String\n",
            "  let Callback = stringify\n",
            "  Callback(1)\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `Callback`"
    }));
}

#[test]
fn invalid_callable_handler_context_binding_recovers_call_target() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value(item: Int) -> Int\n",
            "end\n",
            "handler recover(Callback: fn(Int) -> Int, Number: Int) handles Ask\n",
            "  value(item) => Callback(item) + Number()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message != "unresolved call_target `Callback`"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `Number`"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_callable_handler_operation_binding_recovers_call_target() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value(callback: fn(Int) -> Int, number: Int) -> Int\n",
            "end\n",
            "handler recover() handles Ask\n",
            "  value(Callback, Number) => Callback(1) + Number()\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        2
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message != "unresolved call_target `Callback`"),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `Number`"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_satisfy_candidate_does_not_enter_predicate_bindings() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "fn main(fallback: Int) -> Int\n",
            "  _value satisfy Candidate => Candidate == fallback\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.invalid_case")
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.message != "unresolved satisfy_predicate `Candidate`")
    );
}

#[test]
fn quarantined_alias_does_not_suppress_unrelated_type_mismatch() {
    let module = merged_modules(vec![
        SourceFile::new(
            "broken.veln",
            concat!("type bad\n", "  Made\n", "end\n", "pub type E = bad\n",),
        ),
        SourceFile::new(
            "main.veln",
            concat!(
                "use broken\n",
                "type Error\n",
                "  Failure\n",
                "end\n",
                "fn main() -> Error\n",
                "  1\n",
                "end\n",
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "type name must start with an ASCII uppercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch" && diagnostic.details.to_json().contains("\"Error\"")
    }));
}

#[test]
fn quarantined_import_alias_does_not_suppress_same_leaf_expected_type_mismatch() {
    let module = merged_modules(vec![
        SourceFile::new(
            "broken.veln",
            concat!("type bad\n", "  Made\n", "end\n", "pub type E = bad\n",),
        ),
        SourceFile::new(
            "main.veln",
            concat!(
                "use broken\n",
                "type E\n",
                "  Failure\n",
                "end\n",
                "fn main() -> E\n",
                "  1\n",
                "end\n",
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "type name must start with an ASCII uppercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch" && diagnostic.details.to_json().contains("\"E\"")
    }));
}

#[test]
fn quarantined_same_file_alias_does_not_suppress_valid_type_mismatch() {
    let parsed = parse(&SourceFile::new(
        "main.veln",
        concat!(
            "type bad\n",
            "  Made\n",
            "end\n",
            "pub type E = bad\n",
            "type E\n",
            "  Failure\n",
            "end\n",
            "fn main() -> E\n",
            "  1\n",
            "end\n",
        ),
    ));
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "type name must start with an ASCII uppercase letter"
    }));
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.mismatch"
                && diagnostic
                    .details
                    .to_json()
                    .contains("\"expected_type\":\"E\"")),
        "{diagnostics:#?}"
    );
}

#[test]
fn quarantined_import_alias_does_not_suppress_same_leaf_actual_type_mismatch() {
    let module = merged_modules(vec![
        SourceFile::new(
            "broken.veln",
            concat!("type bad\n", "  Made\n", "end\n", "pub type E = bad\n",),
        ),
        SourceFile::new(
            "main.veln",
            concat!(
                "use broken\n",
                "type E\n",
                "  Failure\n",
                "end\n",
                "fn main() -> Int\n",
                "  Failure\n",
                "end\n",
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "type name must start with an ASCII uppercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch" && diagnostic.details.to_json().contains("\"E\"")
    }));
}
