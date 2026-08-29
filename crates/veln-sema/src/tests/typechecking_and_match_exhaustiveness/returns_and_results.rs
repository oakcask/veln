use super::*;

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
fn descriptor_routed_try_infers_omitted_local_success_type_from_operand() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result<Int, String>\n",
            "  Ok(1)\n",
            "end\n",
            "fn main(raw: String) -> Result<(), String>\n",
            "  let value = parse(raw)?\n",
            "  let incremented = value + 1\n",
            "  ensure incremented > value\n",
            "  Ok(())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn descriptor_routed_try_keeps_return_error_constraint_with_omitted_local_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result<Int, String>\n",
            "  Ok(1)\n",
            "end\n",
            "fn main(raw: String) -> Result<(), AppError>\n",
            "  let value = parse(raw)?\n",
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
    assert_diagnostic_span(diagnostic, 5, 15, 5, 25);
}
