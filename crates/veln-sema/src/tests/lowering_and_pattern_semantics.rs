use super::*;

#[test]
fn satisfy_candidate_reports_shadowing_and_unused_predicates() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn default_port(max: Int) -> Int\n",
            "  _port satisfy max => true\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_candidate_shadow"
            && diagnostic.kind == DiagnosticKind::Hole
            && diagnostic.message == "satisfy candidate `max` shadows a visible binding"
            && diagnostic.related.len() == 1
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_candidate_unused"
            && diagnostic.kind == DiagnosticKind::Hole
            && diagnostic.message == "satisfy predicate does not reference candidate `max`"
            && diagnostic.related.len() == 1
    }));
}

#[test]
fn satisfy_predicate_is_checked_with_candidate_expected_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(limit: Int) -> Int\n",
            "  _value satisfy candidate => candidate > 0 and candidate <= limit\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
}

#[test]
fn satisfy_predicate_reports_non_boolean_candidate_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose() -> Int\n",
            "  _value satisfy candidate => candidate\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_type_mismatch"
            && diagnostic.kind == DiagnosticKind::Hole
            && diagnostic.message == "satisfy predicate is not `Bool`"
            && diagnostic
                .details
                .to_json()
                .contains("\"actual_type\":\"Int\"")
    }));
}

#[test]
fn satisfy_predicate_reports_unresolved_names() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose() -> Int\n",
            "  _value satisfy candidate => candidate == missing\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.kind == DiagnosticKind::Name
            && diagnostic.message == "unresolved satisfy_predicate `missing`"
    }));
}

#[test]
fn propagates_try_expected_type_from_result_return() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Result<Int, AppError>\n  Ok(_?)\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"expected_type\":\"Result<Int, AppError>\"")
    );
}

#[test]
fn lowers_option_constructor_with_expected_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Option<String>\n",
            "  Some(\"ok\")\n",
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
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::option(CoreType::string()));
    let CoreExprKind::OptionSome(value) = &expr.kind else {
        panic!("Some call should lower to an option constructor");
    };
    assert_eq!(value.ty, CoreType::string());
    assert!(lowered.ir.is_some());
}

#[test]
fn lowers_none_constructor_with_expected_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!("pub fn main() -> Option<String>\n", "  None\n", "end\n",),
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
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(expr.ty, CoreType::option(CoreType::string()));
    assert!(matches!(expr.kind, CoreExprKind::OptionNone));
    assert!(lowered.ir.is_some());
}

#[test]
fn lowers_qualified_none_constructor_with_expected_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Option<String>\n",
            "  Option::None\n",
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
    assert_eq!(expr.ty, CoreType::option(CoreType::string()));
    assert!(matches!(expr.kind, CoreExprKind::OptionNone));
    assert!(lowered.ir.is_some());
}

#[test]
fn lowers_qualified_builtin_constructors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(use_result: Bool) -> Result<Option<String>, AppError>\n",
            "  if_missing(use_result)\n",
            "end\n",
            "fn if_missing(use_result: Bool) -> Result<Option<String>, AppError>\n",
            "  Result::Ok(Option::Some(\"ok\"))\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let helper = core
        .functions
        .iter()
        .find(|function| function.name == "if_missing")
        .expect("helper should be lowered");
    let CoreStmtKind::Return { expr } = &helper.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(
            CoreType::option(CoreType::string()),
            CoreType::named("AppError", Vec::new())
        )
    );
    let CoreExprKind::ResultOk(value) = &expr.kind else {
        panic!("Result::Ok call should lower to a result constructor");
    };
    assert!(matches!(value.kind, CoreExprKind::OptionSome(_)));
}

#[test]
fn infers_payload_constructor_type_arguments_without_expected_adt_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Box<A>\n",
            "  Box(value: A)\n",
            "end\n",
            "fn main() -> {option: Option<Int>, list: List<Int>, boxed: Box<String>}\n",
            "  let option = Some(1)\n",
            "  let list = Cons(1, Nil)\n",
            "  let boxed = Box(\"ok\")\n",
            "  {option: option, list: list, boxed: boxed}\n",
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
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("option binding should lower as let");
    };
    assert_eq!(expr.ty, CoreType::option(CoreType::int()));
    let CoreStmtKind::Let { expr, .. } = &main.body[1].kind else {
        panic!("list binding should lower as let");
    };
    assert_eq!(expr.ty, CoreType::named("List", vec![CoreType::int()]));
    let CoreStmtKind::Let { expr, .. } = &main.body[2].kind else {
        panic!("box binding should lower as let");
    };
    assert_eq!(expr.ty, CoreType::named("Box", vec![CoreType::string()]));
}

#[test]
fn unresolved_payload_constructor_type_arguments_are_ambiguous() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  let value = Ok(1)\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.inference_ambiguous");
    assert_eq!(
        diagnostics[0].message,
        "constructor `Ok` needs type context"
    );
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"inferred_type\":\"Result<Int, unknown>\"")
    );
}

#[test]
fn conflicting_payload_constructor_type_arguments_report_mismatch() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Both<A>\n",
            "  Both(left: A, right: A)\n",
            "end\n",
            "fn main() -> Int\n",
            "  let value = Both(1, \"bad\")\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
    assert_diagnostic_span(&diagnostics[0], 5, 23, 5, 28);
}

#[test]
fn non_constructor_expected_type_still_reports_outer_mismatch() {
    let source = SourceFile::new(
        "main.veln",
        concat!("fn main() -> Int\n", "  Some(1)\n", "end\n",),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `Int`, but found `Option<Int>`"
    );
}

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
