use super::*;

#[test]
fn match_exhaustiveness_reports_missing_result_case() {
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
    assert_eq!(diagnostic.message, "match is missing case Ok(_)");
}

#[test]
fn accepts_float_numeric_operators() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Float, right: Float) -> {sum: Float, negated: Float, ordered: Bool}\n",
            "  {sum: left + right, negated: -left, ordered: left < right}\n",
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
    let CoreExprKind::Record(fields) = &expr.kind else {
        panic!("tail expression should lower as record");
    };
    assert_eq!(fields[0].expr.ty, CoreType::float());
    assert_eq!(fields[1].expr.ty, CoreType::float());
    assert_eq!(fields[2].expr.ty, CoreType::bool());
    assert!(matches!(
        &fields[0].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
    assert!(matches!(
        &fields[1].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_negate"
    ));
    assert!(matches!(
        &fields[2].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_less"
    ));
    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Record(fields) = &value.kind else {
        panic!("tail expression should lower as IR record");
    };
    assert!(matches!(
        &fields[0].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
    assert!(matches!(
        &fields[1].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_negate"
    ));
    assert!(matches!(
        &fields[2].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_less"
    ));
}

#[test]
fn variadic_parameter_lowers_body_binding_to_list() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn collect(values: ...Int) -> List<Int>\n",
            "  values\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let collect = core
        .functions
        .iter()
        .find(|function| function.name == "collect")
        .expect("collect should be lowered");
    assert_eq!(
        collect.params[0].ty,
        CoreType::named("List", vec![CoreType::int()])
    );
}

#[test]
fn function_header_lowering_preserves_resolved_core_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn summarize(prefix: String, values: ...Int) -> total: Int effects [stdio]\n",
            "require prefix == prefix\n",
            "ensure total >= 0\n",
            "  stdio::println(prefix)\n",
            "  0\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let summarize = core
        .functions
        .iter()
        .find(|function| function.name == "summarize")
        .expect("summarize should be lowered");
    assert_eq!(summarize.visibility, veln_ast::Visibility::Public);
    assert_eq!(summarize.params[0].ty, CoreType::string());
    assert_eq!(
        summarize.params[1].ty,
        CoreType::named("List", vec![CoreType::int()])
    );
    assert_eq!(summarize.return_binding.as_deref(), Some("total"));
    assert_eq!(summarize.return_type, CoreType::int());
    assert_eq!(summarize.effects, ["stdio"]);
    assert_eq!(summarize.contracts.len(), 2);
    assert_eq!(
        summarize.contracts[0].obligation_status,
        ContractObligationStatus::StaticallyProven
    );
    assert_eq!(
        summarize.contracts[1].obligation_status,
        ContractObligationStatus::RuntimeRequired
    );
}

#[test]
fn variadic_call_lowers_tail_arguments_to_list() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn collect(prefix: String, values: ...Int) -> List<Int>\n",
            "  values\n",
            "end\n",
            "pub fn main() -> List<Int>\n",
            "  collect(\"ok\", 1, 2)\n",
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
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert_eq!(args.len(), 2);
    assert_eq!(core_list_len(&args[1]), 2);
}

#[test]
fn variadic_pipeline_lowers_shifted_tail_arguments_to_list() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn collect(values: ...String) -> List<String>\n",
            "  values\n",
            "end\n",
            "pub fn main() -> List<String>\n",
            "  \"red\" |> collect(\"green\")\n",
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
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert_eq!(args.len(), 1);
    assert_eq!(core_list_len(&args[0]), 2);
}

fn core_list_len(expr: &CoreExpr) -> usize {
    match &expr.kind {
        CoreExprKind::ListNil => 0,
        CoreExprKind::ListCons { tail, .. } => 1 + core_list_len(tail),
        other => panic!("expected list expression, got {other:?}"),
    }
}

#[test]
fn lowers_boolean_literals_through_core_and_ir() {
    let source = SourceFile::new("main.veln", "fn main() -> Bool\n  true\nend\n");
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
    assert_eq!(expr.ty, CoreType::bool());
    assert!(matches!(expr.kind, CoreExprKind::BoolLiteral(true)));

    let ir = lowered.ir.expect("checked core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert_eq!(value.ty, CoreType::bool());
    assert!(matches!(value.kind, IrExprKind::BoolLiteral(true)));
}

#[test]
fn infers_float_numeric_operators_from_call_results() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn value() -> Float\n",
            "  1.0\n",
            "end\n",
            "fn main()\n",
            "  value() + value()\n",
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
    assert_eq!(expr.ty, CoreType::float());
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
}

#[test]
fn accepts_int_operands_in_float_operator_contexts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Float, count: Int) -> {sum: Float, ordered: Bool, expected: Float}\n",
            "  {sum: left + count, ordered: count < left, expected: 1 + 2}\n",
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
    let CoreExprKind::Record(fields) = &expr.kind else {
        panic!("tail expression should lower as record");
    };
    assert_eq!(fields[0].expr.ty, CoreType::float());
    assert_eq!(fields[1].expr.ty, CoreType::bool());
    assert_eq!(fields[2].expr.ty, CoreType::float());
    assert!(matches!(
        &fields[0].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
    assert!(matches!(
        &fields[1].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_less"
    ));
    assert!(matches!(
        &fields[2].expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "float_add"
    ));
}

#[test]
fn rejects_int_values_in_float_assignment_contexts() {
    let source = SourceFile::new(
        "main.veln",
        concat!("pub fn main() -> Float\n", "  1\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Float`, but found `Int`");
}

#[test]
fn reports_float_operator_operand_mismatch() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Float) -> Float\n",
            "  left + \"bad\"\n",
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
        "expected `Float`, but found `String`"
    );
}

#[test]
fn comparison_does_not_select_float_from_expected_result() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Int, right: Int) -> Float\n",
            "  left < right\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Float`, but found `Bool`");
}

#[test]
fn reports_invalid_type_annotations() {
    let source = SourceFile::new(
        "main.veln",
        "fn bad(value: Result<Int>) -> Option<>\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.invalid_annotation")
    );
}
