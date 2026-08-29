use super::*;

#[test]
fn blocked_readiness_returns_first_blocker_before_lowering_body() {
    let module = fixture_ids();
    let surface = main_function(&module);
    let invalid_expr = expr_line(&surface.body[2]);
    let first = CoreBlocker::UnsupportedExpression {
        node_id: surface.node_id,
        reason: "contract".to_string(),
    };
    let second = CoreBlocker::Hole {
        node_id: invalid_expr.node_id,
    };
    let program = CheckedProgram {
        functions: vec![CoreFunction {
            body: vec![core_stmt(
                &surface.body[2],
                CoreStmtKind::Return {
                    expr: core_expr(
                        invalid_expr,
                        CoreType::Unknown,
                        CoreExprKind::Call {
                            target: CoreCallTarget::Unresolved("late_error".to_string()),
                            args: Vec::new(),
                        },
                    ),
                },
            )],
            ..function_shell(surface)
        }],
        effects: Vec::new(),
        readiness: CoreReadiness::Blocked(vec![first.clone(), second]),
    };

    assert_eq!(
        lower_checked_core(&program),
        Err(IrLowerError::Blocked(first))
    );
}

#[test]
fn complete_program_reports_unresolved_call_target_with_call_node() {
    let module = lower_source(concat!("fn main() -> ()\n", "  missing()\n", "end\n",));
    let surface = main_function(&module);
    let call = expr_line(&surface.body[0]);
    let (callee, _args) = call_parts(call);
    let program = complete_program(vec![CoreFunction {
        body: vec![core_stmt(
            &surface.body[0],
            CoreStmtKind::Return {
                expr: core_expr(
                    call,
                    CoreType::Unknown,
                    CoreExprKind::Call {
                        target: CoreCallTarget::Unresolved("missing".to_string()),
                        args: Vec::new(),
                    },
                ),
            },
        )],
        ..function_shell(surface)
    }]);

    assert_ne!(call.node_id, callee.node_id);
    assert_eq!(
        lower_checked_core(&program),
        Err(IrLowerError::UnresolvedCallTarget {
            node_id: call.node_id,
            symbol: "missing".to_string()
        })
    );
}

#[test]
fn complete_program_rejects_missing_and_hole_expressions() {
    let module = lower_source(concat!("fn main() -> ()\n", "  _\n", "end\n",));
    let surface = main_function(&module);
    let expr = expr_line(&surface.body[0]);

    for (kind, expected) in [
        (
            CoreExprKind::Missing,
            IrLowerError::MissingExpression {
                node_id: expr.node_id,
            },
        ),
        (
            CoreExprKind::Hole { label: None },
            IrLowerError::Hole {
                node_id: expr.node_id,
            },
        ),
    ] {
        let program = complete_program(vec![CoreFunction {
            body: vec![core_stmt(
                &surface.body[0],
                CoreStmtKind::Return {
                    expr: core_expr(expr, CoreType::Unknown, kind),
                },
            )],
            ..function_shell(surface)
        }]);

        assert_eq!(lower_checked_core(&program), Err(expected));
    }
}

#[test]
fn complete_program_reports_missing_expression_from_nested_record_field() {
    let module = lower_source(concat!(
        "fn main() -> ()\n",
        "  { ready: true, value: 1 }\n",
        "end\n",
    ));
    let surface = main_function(&module);
    let return_line = &surface.body[0];
    let record = expr_line(return_line);
    let fields = record_fields(record);
    let ready = named_field(fields, "ready");
    let value = named_field(fields, "value");
    let program = complete_program(vec![CoreFunction {
        body: vec![core_stmt(
            return_line,
            CoreStmtKind::Return {
                expr: core_expr(
                    record,
                    CoreType::Record(vec![
                        ("ready".to_string(), CoreType::bool()),
                        ("value".to_string(), CoreType::int()),
                    ]),
                    CoreExprKind::Record(vec![
                        CoreRecordField {
                            node_id: ready.node_id,
                            name: "ready".to_string(),
                            expr: core_expr(
                                &ready.expr,
                                CoreType::bool(),
                                CoreExprKind::BoolLiteral(true),
                            ),
                            span: ready.span.clone(),
                        },
                        CoreRecordField {
                            node_id: value.node_id,
                            name: "value".to_string(),
                            expr: core_expr(&value.expr, CoreType::Unknown, CoreExprKind::Missing),
                            span: value.span.clone(),
                        },
                    ]),
                ),
            },
        )],
        ..function_shell(surface)
    }]);

    assert_eq!(
        lower_checked_core(&program),
        Err(IrLowerError::MissingExpression {
            node_id: value.expr.node_id
        })
    );
}

#[test]
fn complete_program_reports_unresolved_call_target_from_nested_call_argument() {
    let module = lower_source(concat!("fn main() -> ()\n", "  wrap(missing())\n", "end\n",));
    let surface = main_function(&module);
    let return_line = &surface.body[0];
    let outer_call = expr_line(return_line);
    let (_outer_callee, outer_args) = call_parts(outer_call);
    let inner_call = &outer_args[0];
    let program = complete_program(vec![CoreFunction {
        body: vec![core_stmt(
            return_line,
            CoreStmtKind::Return {
                expr: core_expr(
                    outer_call,
                    CoreType::Unknown,
                    CoreExprKind::Call {
                        target: CoreCallTarget::Function("wrap".to_string()),
                        args: vec![core_expr(
                            inner_call,
                            CoreType::Unknown,
                            CoreExprKind::Call {
                                target: CoreCallTarget::Unresolved("missing".to_string()),
                                args: Vec::new(),
                            },
                        )],
                    },
                ),
            },
        )],
        ..function_shell(surface)
    }]);

    assert_eq!(
        lower_checked_core(&program),
        Err(IrLowerError::UnresolvedCallTarget {
            node_id: inner_call.node_id,
            symbol: "missing".to_string()
        })
    );
}
