use super::*;

#[test]
fn marks_static_sept_case_split_satisfy_predicate_as_tautology_repair() {
    let predicate =
        exhaustive_case_split_predicate("candidate", &["a", "b", "c", "d", "e", "f", "g"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn main(primary: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool}}, fallback: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool}}) -> {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool}}\n  _value satisfy candidate => {predicate}\nend\n"
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}

#[test]
fn marks_static_oct_case_split_satisfy_predicate_as_tautology_repair() {
    let predicate =
        exhaustive_case_split_predicate("candidate", &["a", "b", "c", "d", "e", "f", "g", "h"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "fn main(primary: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool}}, fallback: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool}}) -> {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool}}\n  _value satisfy candidate => {predicate}\nend\n"
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}

#[test]
fn marks_distinct_literal_equality_contradiction_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: String, fallback: String) -> String\n",
            "  _value satisfy candidate => not (candidate == \"ready\" and candidate == \"done\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    let details = diagnostics[0].details.to_json();
    assert!(details.contains(
        "{\"candidate_id\":\"symbol-1\",\"name\":\"fallback\",\
         \"type\":\"String\",\"rank\":1,\"reason\":\"satisfy_tautology\",\
         \"application_policy\":\"safe_repair_candidate\","
    ));
    assert!(details.contains(
        "{\"candidate_id\":\"symbol-2\",\"name\":\"primary\",\
         \"type\":\"String\",\"rank\":2,\"reason\":\"satisfy_tautology\",\
         \"application_policy\":\"safe_repair_candidate\","
    ));
    assert_eq!(
        details
            .matches("\"satisfy_status\":\"statically_satisfied\"")
            .count(),
        2
    );
}

#[test]
fn does_not_mark_invalid_static_satisfy_predicate_as_tautology_repair() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(primary: Int, fallback: Int) -> Int\n",
            "  _value satisfy candidate => candidate.ready or (not candidate.ready and true)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_field_missing"
            && diagnostic.details.to_json().contains("\"field\":\"ready\"")
    }));
    let hole = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "hole.unfilled")
        .expect("unfilled hole diagnostic");
    let details = hole.details.to_json();
    assert!(!details.contains("\"reason\":\"satisfy_tautology\""));
    assert!(!details.contains("\"satisfy_status\":\"statically_satisfied\""));
    assert!(!details.contains("\"application_policy\":\"safe_repair_candidate\""));
}

#[test]
fn reports_return_type_mismatch() {
    let source = SourceFile::new("main.veln", "fn bad() -> Int\n  \"no\"\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"expr-3\",\"expected_type\":\"Int\",",
            "\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",",
            "\"actual_type_source\":\"inferred_expression\",",
            "\"constraint\":\"return_value\",",
            "\"origin_node_ids\":[\"fn-1\",\"expr-2\",\"expr-3\"]}"
        )
    );
}

#[test]
fn omitted_tail_expression_returns_unit() {
    let source = SourceFile::new("main.veln", "fn main() -> ()\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn omitted_tail_expression_checks_declared_return_type() {
    let source = SourceFile::new("main.veln", "fn main() -> Int\n  let value = 1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `()`");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"fn-1\",\"expected_type\":\"Int\",",
            "\"actual_type\":\"()\",\"expected_type_source\":\"declared_return\",",
            "\"actual_type_source\":\"implicit_unit\",",
            "\"constraint\":\"return_value\",",
            "\"origin_node_ids\":[\"fn-1\",\"fn-1\"]}"
        )
    );
}

#[test]
fn omitted_tail_expression_lowers_to_unit_return() {
    let source = SourceFile::new(
        "main.veln",
        concat!("fn main() -> ()\n", "  let value = 1\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    assert!(matches!(main.body[0].kind, CoreStmtKind::Let { .. }));
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("omitted tail should lower as unit return");
    };
    assert!(matches!(expr.kind, CoreExprKind::Unit));

    let ir = lowered.ir.expect("complete core should lower to typed IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[1].kind else {
        panic!("omitted tail should lower as IR unit return");
    };
    assert!(matches!(value.kind, IrExprKind::Unit));
}

#[test]
fn ok_constructor_accepts_declared_result_return() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result<(), AppError>\n  Ok(())\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty());
}

#[test]
fn result_constructor_checks_expected_value_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result<(), AppError>\n  Ok(\"no\")\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].details.to_json(),
        concat!(
            "{\"phase\":\"type\",\"node_id\":\"expr-5\",\"expected_type\":\"()\",",
            "\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",",
            "\"actual_type_source\":\"inferred_expression\",",
            "\"constraint\":\"call_argument\",",
            "\"origin_node_ids\":[\"fn-1\",\"expr-2\",\"expr-5\"]}"
        )
    );
}

#[test]
fn descriptor_routed_err_constructor_checks_expected_error_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result<Int, AppError>\n  Err(1)\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `AppError`, but found `Int`"
    );
    assert_diagnostic_span(&diagnostics[0], 2, 7, 2, 8);
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"expected_type\":\"AppError\""));
    assert!(details.contains("\"actual_type\":\"Int\""));
    assert!(details.contains("\"constraint\":\"call_argument\""));
}

#[test]
fn descriptor_routed_option_constructor_checks_expected_item_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Option<Int>\n  Some(\"no\")\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
    assert_diagnostic_span(&diagnostics[0], 2, 8, 2, 12);
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"expected_type\":\"Int\""));
    assert!(details.contains("\"actual_type\":\"String\""));
    assert!(details.contains("\"constraint\":\"call_argument\""));
}

#[test]
fn descriptor_routed_qualified_option_constructor_checks_expected_item_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Option<Int>\n  Option::Some(\"no\")\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
    assert_diagnostic_span(&diagnostics[0], 2, 16, 2, 20);
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"expected_type\":\"Int\""));
    assert!(details.contains("\"actual_type\":\"String\""));
    assert!(details.contains("\"constraint\":\"call_argument\""));
}

#[test]
fn descriptor_routed_qualified_result_constructor_checks_expected_error_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result<Int, AppError>\n  Result::Err(1)\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `AppError`, but found `Int`"
    );
    assert_diagnostic_span(&diagnostics[0], 2, 15, 2, 16);
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"expected_type\":\"AppError\""));
    assert!(details.contains("\"actual_type\":\"Int\""));
    assert!(details.contains("\"constraint\":\"call_argument\""));
}

#[test]
fn descriptor_routed_qualified_result_constructor_checks_expected_value_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result<Int, AppError>\n  Result::Ok(\"no\")\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
    assert_diagnostic_span(&diagnostics[0], 2, 14, 2, 18);
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"expected_type\":\"Int\""));
    assert!(details.contains("\"actual_type\":\"String\""));
    assert!(details.contains("\"constraint\":\"call_argument\""));
}

#[test]
fn descriptor_routed_qualified_list_constructor_checks_expected_head_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main() -> List<Int>\n",
            "  List::Cons(\"no\", List::Nil)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
    assert_diagnostic_span(&diagnostics[0], 6, 14, 6, 18);
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"expected_type\":\"Int\""));
    assert!(details.contains("\"actual_type\":\"String\""));
    assert!(details.contains("\"constraint\":\"call_argument\""));
}

#[test]
fn descriptor_routed_qualified_list_constructor_checks_expected_tail_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main() -> List<Int>\n",
            "  List::Cons(1, None)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `List<Int>`, but found `Option<unknown>`"
    );
    assert_diagnostic_span(&diagnostics[0], 6, 17, 6, 21);
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"expected_type\":\"List<Int>\""));
    assert!(details.contains("\"actual_type\":\"Option<unknown>\""));
    assert!(details.contains("\"constraint\":\"call_argument\""));
}

#[test]
fn descriptor_routed_result_arity_diagnostic_keeps_call_span() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result<Int, AppError>\n  Ok()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "core.result_constructor_arity_mismatch")
        .expect("result constructor arity should be diagnosed");
    assert_eq!(
        diagnostic.message,
        "result constructor expects 1 argument, but got 0"
    );
    assert_diagnostic_span(diagnostic, 2, 3, 2, 7);
}

#[test]
fn descriptor_routed_option_arity_diagnostic_keeps_call_span() {
    let source = SourceFile::new("main.veln", "fn main() -> Option<Int>\n  Some(1, 2)\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "core.option_constructor_arity_mismatch")
        .expect("option constructor arity should be diagnosed");
    assert_eq!(
        diagnostic.message,
        "option constructor expects 1 argument, but got 2"
    );
    assert_diagnostic_span(diagnostic, 2, 3, 2, 13);
}

#[test]
fn descriptor_routed_try_checks_result_error_type_at_operand() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result<Int, String>\n",
            "  Ok(1)\n",
            "end\n",
            "fn main(raw: String) -> Result<(), AppError>\n",
            "  let value: Int = parse(raw)?\n",
            "  Ok(())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.mismatch")
        .expect("try operand result error type should be diagnosed");
    assert_eq!(
        diagnostic.message,
        "expected `Result<Int, AppError>`, but found `Result<Int, String>`"
    );
    assert_diagnostic_span(diagnostic, 5, 20, 5, 30);
    let details = diagnostic.details.to_json();
    assert!(details.contains("\"constraint\":\"return_value\""));
    assert!(details.contains("\"expected_type\":\"Result<Int, AppError>\""));
    assert!(details.contains("\"actual_type\":\"Result<Int, String>\""));
}

#[test]
fn accepts_supported_type_forms_and_record_expected_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> {score: Float, names: Vec<String>, table: Dict<String, Int>, ",
            "callback: fn(Int) -> String}\n",
            "  {score: _, names: [], table: _, callback: _}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 3);
    let rendered = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.details.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(rendered.contains("\"expected_type\":\"Float\""));
    assert!(rendered.contains("\"expected_type\":\"Dict<String, Int>\""));
    assert!(rendered.contains("\"expected_type\":\"fn(Int) -> String\""));
    assert!(rendered.contains("\"candidate_queries\":[{\"kind\":\"symbol\""));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.related.is_empty())
    );
}

#[test]
fn accepts_dictionary_literals_with_expected_key_and_value_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Dict<String, Int>\n",
            "  {\"one\": 1, \"two\": 2}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::dict(CoreType::string(), CoreType::int()));
    let CoreExprKind::Dict(entries) = &expr.kind else {
        panic!("tail expression should lower as dictionary");
    };
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].key.ty, CoreType::string());
    assert_eq!(entries[0].value.ty, CoreType::int());
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Dict(entries) = &value.kind else {
        panic!("tail expression should lower as IR dictionary");
    };
    assert_eq!(entries.len(), 2);
}

#[test]
fn accepts_dictionary_literals_with_identifier_led_expression_keys() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(seed: Int) -> Dict<Int, String>\n",
            "  {seed + 1: \"next\"}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::dict(CoreType::int(), CoreType::string()));
    let CoreExprKind::Dict(entries) = &expr.kind else {
        panic!("tail expression should lower as dictionary");
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].key.ty, CoreType::int());
    assert_eq!(entries[0].value.ty, CoreType::string());
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Dict(entries) = &value.kind else {
        panic!("tail expression should lower as IR dictionary");
    };
    assert_eq!(entries.len(), 1);
}

#[test]
fn record_patterns_bind_field_types_through_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: {count: Int, label: String}) -> String\n",
            "  match value\n",
            "    {count: 0, label: name} => name\n",
            "    {count: count, label: _} => \"many\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Match { arms, .. } = &expr.kind else {
        panic!("tail expression should lower as match");
    };
    let CorePatternKind::Record(fields) = &arms[0].pattern.kind else {
        panic!("first arm should lower as record pattern");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "count");
    assert_eq!(fields[1].name, "label");

    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Match { arms, .. } = &value.kind else {
        panic!("tail expression should lower as IR match");
    };
    assert!(matches!(
        &arms[1].pattern.kind,
        IrPatternKind::Record(fields)
            if fields.iter().any(|field| field.name == "count")
    ));
}

#[test]
fn match_expression_type_checks_inside_call_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn wrap(value: String) -> String\n",
            "  value\n",
            "end\n",
            "fn describe(value: Option<Int>) -> String\n",
            "  wrap(match value\n",
            "    Some(count) => \"some\"\n",
            "    None => \"none\"\n",
            "  end)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let describe = core
        .functions
        .iter()
        .find(|function| function.name == "describe")
        .expect("describe should be lowered");
    let CoreStmtKind::Return { expr } = &describe.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert!(matches!(args[0].kind, CoreExprKind::Match { .. }));
}

#[test]
fn descriptor_routed_constructor_patterns_bind_payload_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option<Int>) -> Int\n",
            "  match value\n",
            "    Option::Some(count) => count + 1\n",
            "    Option::None => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Match { arms, .. } = &expr.kind else {
        panic!("tail expression should lower as match");
    };
    assert_eq!(arms[0].expr.ty, CoreType::int());
    assert_eq!(arms[1].expr.ty, CoreType::int());
}

#[test]
fn descriptor_routed_result_pattern_reports_payload_type_mismatch_at_branch_expr() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Result<Int, String>) -> Int\n",
            "  match value\n",
            "    Result::Ok(count) => count\n",
            "    Result::Err(error) => error\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.mismatch")
        .expect("result error branch payload mismatch should be diagnosed");
    assert_eq!(diagnostic.message, "expected `Int`, but found `String`");
    assert_diagnostic_span(diagnostic, 4, 27, 4, 32);
    let details = diagnostic.details.to_json();
    assert!(details.contains("\"expected_type\":\"Int\""));
    assert!(details.contains("\"actual_type\":\"String\""));
    assert!(details.contains("\"constraint\":\"match_arm\""));
}

#[test]
fn match_exhaustiveness_accepts_finite_builtin_domains() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn bool_label(value: Bool) -> String\n",
            "  match value\n",
            "    true => \"true\"\n",
            "    false => \"false\"\n",
            "  end\n",
            "end\n",
            "fn option_label(value: Option<Int>) -> String\n",
            "  match value\n",
            "    Some(_) => \"some\"\n",
            "    None => \"none\"\n",
            "  end\n",
            "end\n",
            "fn result_label(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Ok(_) => \"ok\"\n",
            "    Err(_) => \"err\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.core.is_some());
    assert!(lowered.ir.is_some());
}

#[test]
fn match_exhaustiveness_accepts_catch_all_patterns() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn wildcard(value: Option<Int>) -> String\n",
            "  match value\n",
            "    Some(_) => \"some\"\n",
            "    _ => \"fallback\"\n",
            "  end\n",
            "end\n",
            "fn binding(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Ok(_) => \"ok\"\n",
            "    other => \"fallback\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn match_exhaustiveness_reports_missing_bool_case() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Bool) -> String\n",
            "  match value\n",
            "    true => \"yes\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing bool case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case false");
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn match_exhaustiveness_reports_empty_finite_builtin_match() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Bool) -> String\n",
            "  match value\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("empty finite-domain match should be diagnosed");
    assert_eq!(diagnostic.message, "match is missing case false");
    assert_eq!(diagnostic.related.len(), 1);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn match_exhaustiveness_reports_missing_option_case() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option<Int>) -> String\n",
            "  match value\n",
            "    Some(count) => \"some\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing option case should be diagnosed");
    assert_eq!(diagnostic.message, "match is missing case None");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Option<Int>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers Some(_)."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}

#[test]
fn match_exhaustiveness_reports_qualified_option_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option<Int>) -> String\n",
            "  match value\n",
            "    Option::Some(count) => \"some\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing option case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case None");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Option<Int>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers Some(_)."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}

#[test]
fn match_exhaustiveness_reports_qualified_option_none_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Option<Int>) -> String\n",
            "  match value\n",
            "    Option::None => \"none\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing option case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Some(_)");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Option<Int>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers None."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}

#[test]
fn match_exhaustiveness_reports_missing_result_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Err(error) => error\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing result case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Ok(_)");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Result<Int, String>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers Err(_)."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}

#[test]
fn match_exhaustiveness_reports_qualified_result_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Result::Err(error) => error\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing result case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Ok(_)");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Result<Int, String>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers Err(_)."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}

#[test]
fn match_exhaustiveness_reports_qualified_result_ok_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Result::Ok(count) => \"ok\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing result case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Err(_)");
    assert_diagnostic_span(diagnostic, 2, 3, 4, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `Result<Int, String>`."));
    assert!(related.contains("\"start\":{\"line\":2,\"column\":9,"));
    assert!(related.contains("This arm covers Ok(_)."));
    assert!(related.contains("\"start\":{\"line\":3,\"column\":5,"));
}

#[test]
fn minimal_list_adt_declaration_type_checks_constructor_patterns() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main(value: List<Int>) -> Int\n",
            "  match value\n",
            "    Nil => 0\n",
            "    Cons(head, _) => head\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn minimal_list_adt_qualified_constructors_type_check_and_bind_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main(value: List<Int>) -> Int\n",
            "  match value\n",
            "    List::Nil => 0\n",
            "    List::Cons(head, tail) => head + length(tail)\n",
            "  end\n",
            "end\n",
            "fn length(value: List<Int>) -> Int\n",
            "  match value\n",
            "    List::Nil => 0\n",
            "    List::Cons(_, tail) => 1 + length(tail)\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn minimal_list_adt_constructor_calls_lower_with_declared_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main() -> List<Int>\n",
            "  List::Cons(1, List::Nil)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::named("List", vec![CoreType::int()]));
    let CoreExprKind::ListCons { head, tail } = &expr.kind else {
        panic!("List::Cons call should lower to a list constructor");
    };
    assert_eq!(head.ty, CoreType::int());
    assert_eq!(tail.ty, CoreType::named("List", vec![CoreType::int()]));
    assert!(matches!(tail.kind, CoreExprKind::ListNil));
}

#[test]
fn arbitrary_source_adt_constructors_type_check_and_lower() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Maybe<A>\n",
            "  Missing\n",
            "  Just(value: A)\n",
            "end\n",
            "fn empty() -> Maybe<Int>\n",
            "  Maybe::Missing\n",
            "end\n",
            "fn filled() -> Maybe<Int>\n",
            "  Maybe::Just(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let empty = core
        .functions
        .iter()
        .find(|function| function.name == "empty")
        .expect("empty should be lowered");
    let CoreStmtKind::Return { expr } = &empty.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::named("Maybe", vec![CoreType::int()]));
    assert!(
        matches!(&expr.kind, CoreExprKind::AdtVariant { name, payloads }
        if name == &vec!["Maybe".to_string(), "Missing".to_string()] && payloads.is_empty())
    );

    let filled = core
        .functions
        .iter()
        .find(|function| function.name == "filled")
        .expect("filled should be lowered");
    let CoreStmtKind::Return { expr } = &filled.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::named("Maybe", vec![CoreType::int()]));
    assert!(
        matches!(&expr.kind, CoreExprKind::AdtVariant { name, payloads }
        if name == &vec!["Maybe".to_string(), "Just".to_string()] && payloads.len() == 1)
    );
}

#[test]
fn record_shaped_source_adt_constructor_calls_lower_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Shape\n",
            "  Point {x: Int, y: Int}\n",
            "  Origin\n",
            "end\n",
            "fn main() -> Shape\n",
            "  Shape::Point(2, 5)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::named("Shape", Vec::new()));
    let CoreExprKind::AdtVariant { name, payloads } = &expr.kind else {
        panic!("Shape::Point call should lower to a source ADT variant");
    };
    assert_eq!(name, &vec!["Shape".to_string(), "Point".to_string()]);
    assert_eq!(payloads.len(), 2);
    assert_eq!(payloads[0].ty, CoreType::int());
    assert_eq!(payloads[1].ty, CoreType::int());
}

#[test]
fn nullary_generic_source_adt_constructor_requires_type_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Maybe<A>\n",
            "  Missing\n",
            "  Just(A)\n",
            "end\n",
            "fn main()\n",
            "  let value = Missing\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.inference_ambiguous"
            && diagnostic.message == "constructor `Missing` needs type context"
    }));
}

#[test]
fn duplicate_source_adt_constructor_names_are_rejected_per_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!("type Left\n", "  Same\n", "  Same\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate constructor declaration name `Same`"
    }));
}

#[test]
fn same_module_constructor_leaf_conflicts_resolve_through_type_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Left\n",
            "  Same\n",
            "end\n",
            "type Right\n",
            "  Same\n",
            "end\n",
            "fn left() -> Left\n",
            "  Left::Same\n",
            "end\n",
            "fn right() -> Right\n",
            "  Right::Same\n",
            "end\n",
            "fn ambiguous() -> Left\n",
            "  Same\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.ambiguous" && diagnostic.message == "ambiguous value `Same`"
    }));
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate constructor declaration name `Same`"
    }));
}

#[test]
fn same_module_payload_constructor_leaf_conflicts_resolve_through_type_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Left\n",
            "  Build(value: Int)\n",
            "end\n",
            "type Right\n",
            "  Build(value: String)\n",
            "end\n",
            "fn left() -> Left\n",
            "  Left::Build(1)\n",
            "end\n",
            "fn right() -> Right\n",
            "  Right::Build(\"ok\")\n",
            "end\n",
            "fn ambiguous() -> Left\n",
            "  Build(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.ambiguous" && diagnostic.message == "ambiguous call_target `Build`"
    }));
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate constructor declaration name `Build`"
    }));
}

#[test]
fn ambiguous_unqualified_imported_source_adt_constructor_is_rejected() {
    let first = SourceFile::new(
        "first.veln",
        concat!("mod first\n", "pub type Left\n", "  pub Same\n", "end\n",),
    );
    let second = SourceFile::new(
        "second.veln",
        concat!("mod second\n", "pub type Right\n", "  pub Same\n", "end\n",),
    );
    let app = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use first\n",
            "use second\n",
            "fn main() -> Left\n",
            "  Same\n",
            "end\n",
        ),
    );
    let first = lower_surface_ast(&parse(&first).tree);
    let second = lower_surface_ast(&parse(&second).tree);
    let app = lower_surface_ast(&parse(&app).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: first.types.into_iter().chain(second.types).collect(),
        functions: app.functions,
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.ambiguous" && diagnostic.message == "ambiguous value `Same`"
    }));
}

#[test]
fn imported_source_adt_constructor_resolves_through_module_and_type_paths() {
    let types = SourceFile::new(
        "types.veln",
        concat!(
            "mod types\n",
            "pub type Maybe<A>\n",
            "  pub Missing\n",
            "  pub Just(A)\n",
            "end\n",
        ),
    );
    let app = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use types\n",
            "fn module_qualified() -> Maybe<Int>\n",
            "  types::Just(1)\n",
            "end\n",
            "fn type_qualified() -> Maybe<Int>\n",
            "  types::Maybe::Just(1)\n",
            "end\n",
            "fn label(value: Maybe<Int>) -> String\n",
            "  match value\n",
            "    types::Missing => \"missing\"\n",
            "    types::Maybe::Just(_) => \"value\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let types = lower_surface_ast(&parse(&types).tree);
    let app = lower_surface_ast(&parse(&app).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: types.types,
        functions: app.functions,
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn public_type_alias_reexports_imported_constructors() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod spec.app\n",
            "use spec.api\n",
            "pub fn main() -> Int\n",
            "  match api::Circle(3)\n",
            "    api::Circle(radius) => radius\n",
            "  end\n",
            "end\n",
        ),
    );
    let api_source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "use spec.impl\n",
            "pub type Shape = impl::Shape\n",
        ),
    );
    let impl_source = SourceFile::new(
        "impl.veln",
        concat!(
            "mod spec.impl\n",
            "type Shape\n",
            "  pub Circle(Int)\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let api = lower_surface_ast(&parse(&api_source).tree);
    let implementation = lower_surface_ast(&parse(&impl_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses.into_iter().chain(api.uses).collect(),
        aliases: api.aliases,
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: implementation.types,
        functions: app.functions,
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn private_source_adt_constructor_is_hidden_from_importing_module() {
    let shapes = SourceFile::new(
        "shapes.veln",
        concat!("mod shapes\n", "pub type Rect\n", "  Rect\n", "end\n",),
    );
    let app = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use shapes\n",
            "fn main() -> Rect\n",
            "  Rect\n",
            "end\n",
        ),
    );
    let shapes = lower_surface_ast(&parse(&shapes).tree);
    let app = lower_surface_ast(&parse(&app).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: shapes.types,
        functions: app.functions,
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved value `Rect`"
    }));
}

#[test]
fn private_source_adt_constructor_pattern_does_not_satisfy_imported_exhaustiveness() {
    let shapes = SourceFile::new(
        "shapes.veln",
        concat!(
            "mod shapes\n",
            "pub type Shape\n",
            "  Hidden\n",
            "  pub Shown\n",
            "end\n",
        ),
    );
    let app = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use shapes\n",
            "fn label(value: Shape) -> String\n",
            "  match value\n",
            "    shapes::Hidden => \"hidden\"\n",
            "    shapes::Shown => \"shown\"\n",
            "  end\n",
            "end\n",
        ),
    );
    let shapes = lower_surface_ast(&parse(&shapes).tree);
    let app = lower_surface_ast(&parse(&app).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: shapes.types,
        functions: app.functions,
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.match_non_exhaustive"
            && diagnostic.message == "match is missing case Hidden"
    }));
}

#[test]
fn private_source_adt_constructor_remains_usable_in_declaring_module() {
    let shapes = SourceFile::new(
        "shapes.veln",
        concat!(
            "mod shapes\n",
            "pub type Rect\n",
            "  Rect\n",
            "end\n",
            "fn make() -> Rect\n",
            "  Rect\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&shapes).tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn private_source_adt_constructor_type_path_remains_usable_in_declaring_module() {
    let shapes = SourceFile::new(
        "shapes.veln",
        concat!(
            "mod shapes\n",
            "pub type Rect\n",
            "  Rect\n",
            "end\n",
            "fn make() -> Rect\n",
            "  Rect::Rect\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&shapes).tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn minimal_list_adt_match_reports_missing_cons_case() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main(value: List<Int>) -> Int\n",
            "  match value\n",
            "    Nil => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing list case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Cons(_)");
    assert_diagnostic_span(diagnostic, 6, 3, 8, 6);
}

#[test]
fn minimal_list_adt_match_reports_missing_qualified_nil_case_with_source_anchors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "fn main(value: List<Int>) -> Int\n",
            "  match value\n",
            "    List::Cons(head, _) => head\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let diagnostic = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.match_non_exhaustive")
        .expect("missing list case should be diagnosed");
    assert_eq!(diagnostic.kind, DiagnosticKind::Type);
    assert_eq!(diagnostic.message, "match is missing case Nil");
    assert_diagnostic_span(diagnostic, 6, 3, 8, 6);
    assert_eq!(diagnostic.related.len(), 2);
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(related.contains("Scrutinee has type `List<Int>`."));
    assert!(related.contains("\"start\":{\"line\":6,\"column\":9,"));
    assert!(related.contains("This arm covers Cons(_)."));
    assert!(related.contains("\"start\":{\"line\":7,\"column\":5,"));
}
