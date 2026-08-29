use super::*;

#[test]
fn lowers_runnable_checked_program_to_core_and_typed_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(raw: String) -> Result<Int, AppError>\n",
            "  Ok(1)\n",
            "end\n",
            "pub fn main(raw: String) -> Result<(), AppError> effects [stdio]\n",
            "  let value: Int = parse(raw)?\n",
            "  stdio::println(\"ok\")\n",
            "  Ok(())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    assert!(matches!(main.body[0].kind, CoreStmtKind::Let { .. }));
    let CoreStmtKind::Expr { expr } = &main.body[1].kind else {
        panic!("stdio call should lower as an expression statement");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::StdioBuiltin(symbol),
            ..
        } if symbol == "stdio::println"
    ));
    assert!(matches!(main.body[2].kind, CoreStmtKind::Return { .. }));

    let ir = lowered.ir.expect("complete core should lower to typed IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    assert!(matches!(main.body[0].kind, IrStmtKind::Let { .. }));
    let IrStmtKind::Expr { value } = &main.body[1].kind else {
        panic!("stdio call should stay an expression statement in IR");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StdioBuiltin(symbol),
            ..
        } if symbol == "stdio::println"
    ));
    let IrStmtKind::Return { value } = &main.body[2].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(value.kind, IrExprKind::ResultOk(_)));
}

#[test]
fn constructor_arity_diagnostics_keep_constructor_source_spans() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn make_result() -> Result<Int, AppError>\n",
            "  Ok()\n",
            "end\n",
            "fn make_option() -> Option<Int>\n",
            "  Some(1, 2)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let result_arity = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "core.result_constructor_arity_mismatch")
        .expect("result constructor arity should be diagnosed");
    assert_eq!(
        result_arity.message,
        "result constructor expects 1 argument, but got 0"
    );
    assert_diagnostic_span(result_arity, 2, 3, 2, 7);

    let missing_argument = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "core.missing_expression")
        .expect("missing result constructor argument should be diagnosed");
    assert_eq!(missing_argument.message, "expression is missing");
    assert!(
        missing_argument
            .details
            .to_json()
            .contains("\"expected_type\":\"Int\"")
    );
    assert_diagnostic_span(missing_argument, 2, 3, 2, 7);

    let option_arity = lowered
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "core.option_constructor_arity_mismatch")
        .expect("option constructor arity should be diagnosed");
    assert_eq!(
        option_arity.message,
        "option constructor expects 1 argument, but got 2"
    );
    assert_diagnostic_span(option_arity, 5, 3, 5, 13);
}

#[test]
fn wildcard_let_lowers_to_discarding_expression_statement() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Int) -> ()\n",
            "  let _: Int = value\n",
            "  ()\n",
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
    let CoreStmtKind::Expr { expr } = &main.body[0].kind else {
        panic!("wildcard let should lower as a discarded expression");
    };
    assert_eq!(expr.ty, CoreType::int());
    assert!(matches!(expr.kind, CoreExprKind::Local(ref name) if name == "value"));
    assert!(matches!(main.body[1].kind, CoreStmtKind::Return { .. }));

    let ir = lowered.ir.expect("complete core should lower to typed IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    assert!(matches!(main.body[0].kind, IrStmtKind::Expr { .. }));
}

#[test]
fn record_let_pattern_binds_field_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: {count: Int, label: String}) -> Int\n",
            "  let {count: amount}: {count: Int, label: String} = value\n",
            "  amount\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    assert_eq!(main.body.len(), 3);
    let CoreStmtKind::Let { name, expr, .. } = &main.body[1].kind else {
        panic!("record field binding should lower as a let statement");
    };
    assert_eq!(name, "amount");
    assert!(matches!(
        &expr.kind,
        CoreExprKind::FieldAccess { field, .. } if field == "count"
    ));
    assert_eq!(expr.ty, CoreType::int());
    let CoreStmtKind::Return { expr } = &main.body[2].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(&expr.kind, CoreExprKind::Local(name) if name == "amount"));
}

#[test]
fn constructor_let_pattern_lowers_payload_binding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Option<Int>) -> Int\n",
            "  let Some(amount) = value\n",
            "  amount\n",
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
    assert_eq!(main.body.len(), 3);
    let CoreStmtKind::Let { name, ty, expr } = &main.body[1].kind else {
        panic!("payload binding should lower as a let statement");
    };
    assert_eq!(name, "amount");
    assert_eq!(ty, &CoreType::int());
    assert!(matches!(expr.kind, CoreExprKind::Match { .. }));
}

#[test]
fn match_expression_binds_constructor_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Option<Int>) -> Int\n",
            "  match value\n",
            "    Some(count) => count + 1\n",
            "    None => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::int());
    assert!(matches!(expr.kind, CoreExprKind::Match { .. }));
    let ir = lowered.ir.expect("complete core should lower to typed IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(value.kind, IrExprKind::Match { .. }));
}

#[test]
fn match_expression_binds_qualified_constructor_payloads() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Result<Int, String>) -> Int\n",
            "  match value\n",
            "    Result::Ok(count) => count + 1\n",
            "    Result::Err(_) => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(core.readiness, CoreReadiness::Complete);
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::int());
    assert!(matches!(expr.kind, CoreExprKind::Match { .. }));
    assert!(lowered.ir.is_some());
}

#[test]
fn lowercase_qualified_constructor_pattern_reports_independent_descriptor_mismatch() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  Some\n",
            "end\n",
            "\n",
            "type Other\n",
            "  Some\n",
            "end\n",
            "\n",
            "fn main(input: Item) -> Int\n",
            "  match input\n",
            "    Other::some => 1\n",
            "    _ => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let ids = lowered
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["name.invalid_case", "type.mismatch"]);
    assert_eq!(
        lowered.diagnostics[0].message,
        "constructor name `some` must start with an ASCII uppercase letter"
    );
    assert_eq!(lowered.diagnostics[0].span.as_ref().unwrap().start.line, 11);
    assert_eq!(
        lowered.diagnostics[0].span.as_ref().unwrap().start.column,
        12
    );
    assert_eq!(lowered.diagnostics[0].span.as_ref().unwrap().end.column, 16);
    assert_eq!(
        lowered.diagnostics[1].message,
        "expected `Item`, but found `Other`"
    );
    assert_eq!(lowered.diagnostics[1].span.as_ref().unwrap().start.line, 11);
    assert_eq!(
        lowered.diagnostics[1].span.as_ref().unwrap().start.column,
        5
    );
    assert_eq!(lowered.diagnostics[1].span.as_ref().unwrap().end.column, 16);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn lowering_suppresses_nullary_lowercase_qualified_constructor_recovery() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  None\n",
            "  Some(Int)\n",
            "end\n",
            "\n",
            "fn main(input: Item) -> Int\n",
            "  match input\n",
            "    Item::none => 0\n",
            "    Item::Some(value) => value\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let ids = lowered
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["name.invalid_case"]);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn lowering_suppresses_payload_lowercase_qualified_constructor_recovery() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  None\n",
            "  Some(Int)\n",
            "end\n",
            "\n",
            "fn main(input: Item) -> Int\n",
            "  match input\n",
            "    Item::some(value) => value\n",
            "    Item::None => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let ids = lowered
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["name.invalid_case"]);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn uppercase_qualified_constructor_pattern_reports_descriptor_mismatch_control() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Item\n",
            "  Some\n",
            "end\n",
            "\n",
            "type Other\n",
            "  Some\n",
            "end\n",
            "\n",
            "fn main(input: Item) -> Int\n",
            "  match input\n",
            "    Other::Some => 1\n",
            "    _ => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let ids = lowered
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(ids, ["type.mismatch"]);
    assert_eq!(
        lowered.diagnostics[0].message,
        "expected `Item`, but found `Other`"
    );
    assert_eq!(lowered.diagnostics[0].span.as_ref().unwrap().start.line, 11);
    assert_eq!(
        lowered.diagnostics[0].span.as_ref().unwrap().start.column,
        5
    );
    assert_eq!(lowered.diagnostics[0].span.as_ref().unwrap().end.column, 16);
}

#[test]
fn holes_build_blocked_core_but_not_executable_ir() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> Result<(), AppError>\n  _\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 1);
    assert_eq!(lowered.diagnostics[0].id, "hole.unfilled");
    let core = lowered.core.expect("partial checked core should be built");
    assert!(matches!(
        core.readiness,
        CoreReadiness::Blocked(ref blockers) if matches!(blockers.as_slice(), [CoreBlocker::Hole { .. }])
    ));
    assert!(lowered.ir.is_none());
}

#[test]
fn semantic_errors_block_core_and_ir() {
    let source = SourceFile::new("main.veln", "pub fn main() -> Int\n  \"no\"\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.mismatch")
    );
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}
