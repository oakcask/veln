use super::*;

#[test]
fn lower_complete_program_preserves_function_shape_and_calls() {
    let module = fixture_ids();
    let surface = main_function(&module);
    let mapper_type = mapper_type();
    let result_unit = CoreType::result(CoreType::unit(), CoreType::named("AppError", vec![]));
    let program = complete_program(vec![function_shape_fixture(
        surface,
        &mapper_type,
        &result_unit,
    )]);

    let ir = lower_checked_core(&program).expect("complete core should lower");

    assert_eq!(ir.functions.len(), 1);
    let function = &ir.functions[0];
    assert_function_shape(function, surface, &mapper_type, &result_unit);
    assert_function_shape_body(function);
}

fn mapper_type() -> CoreType {
    CoreType::Function {
        params: vec![CoreType::int()],
        variadic: None,
        return_type: Box::new(CoreType::int()),
        effects: Vec::new(),
    }
}

fn function_shape_fixture(
    surface: &Function,
    mapper_type: &CoreType,
    result_unit: &CoreType,
) -> CoreFunction {
    let input = &surface.params[0];
    let mapper = &surface.params[1];
    CoreFunction {
        node_id: surface.node_id,
        name: "main".to_string(),
        visibility: Visibility::Public,
        params: vec![
            CoreParam {
                node_id: input.node_id,
                name: "input".to_string(),
                ty: CoreType::int(),
                span: input.span.clone(),
            },
            CoreParam {
                node_id: mapper.node_id,
                name: "mapper".to_string(),
                ty: mapper_type.clone(),
                span: mapper.span.clone(),
            },
        ],
        return_binding: None,
        return_type: result_unit.clone(),
        effects: vec!["stdio".to_string()],
        callsite: false,
        contracts: Vec::new(),
        body: function_shape_body(surface, result_unit),
        span: surface.span.clone(),
    }
}

fn function_shape_body(surface: &Function, result_unit: &CoreType) -> CoreCleanupRegion {
    CoreCleanupRegion::new(vec![
        mapper_call_statement(&surface.body[0]),
        print_statement(&surface.body[1]),
        ok_return_statement(&surface.body[2], result_unit),
    ])
}

fn mapper_call_statement(line: &BodyLine) -> CoreStmt {
    let mapper_call = let_expr(line);
    let (_mapper_callee, mapper_args) = call_parts(mapper_call);
    core_stmt(
        line,
        CoreStmtKind::Let {
            name: "answer".to_string(),
            ty: CoreType::int(),
            expr: core_expr(
                mapper_call,
                CoreType::int(),
                CoreExprKind::Call {
                    target: CoreCallTarget::Value("mapper".to_string()),
                    args: vec![local(&mapper_args[0], "input", CoreType::int())],
                },
            ),
        },
    )
}

fn print_statement(line: &BodyLine) -> CoreStmt {
    let print_call = expr_line(line);
    let (_print_callee, print_args) = call_parts(print_call);
    core_stmt(
        line,
        CoreStmtKind::Expr {
            expr: core_expr(
                print_call,
                CoreType::unit(),
                CoreExprKind::Call {
                    target: CoreCallTarget::StdioBuiltin("stdio::println".to_string()),
                    args: vec![core_expr(
                        &print_args[0],
                        CoreType::string(),
                        CoreExprKind::StringLiteral("done".to_string()),
                    )],
                },
            ),
        },
    )
}

fn ok_return_statement(line: &BodyLine, result_unit: &CoreType) -> CoreStmt {
    let ok_call = expr_line(line);
    let (_ok_callee, ok_args) = call_parts(ok_call);
    core_stmt(
        line,
        CoreStmtKind::Return {
            expr: core_expr(
                ok_call,
                result_unit.clone(),
                CoreExprKind::ResultOk(Box::new(core_expr(
                    &ok_args[0],
                    CoreType::unit(),
                    CoreExprKind::Unit,
                ))),
            ),
        },
    )
}

fn assert_function_shape(
    function: &IrFunction,
    surface: &Function,
    mapper_type: &CoreType,
    result_unit: &CoreType,
) {
    assert_eq!(function.node_id, surface.node_id);
    assert_eq!(function.name, "main");
    assert_eq!(function.visibility, Visibility::Public);
    assert_eq!(&function.return_type, result_unit);
    assert_eq!(function.effects, vec!["stdio"]);
    assert_eq!(function.params.len(), 2);
    assert_eq!(function.params[0].name, "input");
    assert_eq!(function.params[0].ty, CoreType::int());
    assert_eq!(function.params[1].name, "mapper");
    assert_eq!(&function.params[1].ty, mapper_type);
}

fn assert_function_shape_body(function: &IrFunction) {
    let IrStmtKind::Let { name, ty, value } = &function.body[0].kind else {
        panic!("first statement should be let");
    };
    assert_eq!(name, "answer");
    assert_eq!(ty, &CoreType::int());
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::Value(name),
            args
        } if name == "mapper"
            && matches!(args.as_slice(), [IrExpr { kind: IrExprKind::Local(arg), .. }] if arg == "input")
    ));

    let IrStmtKind::Expr { value } = &function.body[1].kind else {
        panic!("second statement should be expression");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StdioBuiltin(name),
            args
        } if name == "stdio::println"
            && matches!(args.as_slice(), [IrExpr { kind: IrExprKind::StringLiteral(arg), .. }] if arg == "done")
    ));

    let IrStmtKind::Return { value } = &function.body[2].kind else {
        panic!("third statement should be return");
    };
    assert!(matches!(&value.kind, IrExprKind::ResultOk(inner) if inner.ty == CoreType::unit()));
}

#[test]
fn lower_preserves_contracts_result_binding_dict_match_and_builtin_targets() {
    let module = lower_source(concat!(
        "pub fn main(input: Option<Int>, receiver: Receiver<Int>, count: Int) -> result: () effects [concurrency, stdio]\n",
        "  ensure result == ()\n",
        "  let selected: Int = match input\n",
        "    Some(value) => value\n",
        "    None => 0\n",
        "  end\n",
        "  let table: Dict<String, Int> = {\"selected\": selected}\n",
        "  channel::recv(receiver)\n",
        "  stdio::println(list::len(table))\n",
        "  None\n",
        "end\n",
    ));
    let surface = main_function(&module);
    assert_surface_callee_ids(surface);
    let program = complete_program(vec![contracts_and_builtins_fixture(surface)]);

    let ir = lower_checked_core(&program).expect("complete core should lower");
    let function = &ir.functions[0];
    assert_contract_and_effects(function);
    assert_match_statement(function);
    assert_dict_statement(function);
    assert_builtin_call_statements(function);
}

fn contracts_and_builtins_fixture(surface: &Function) -> CoreFunction {
    CoreFunction {
        params: contracts_and_builtins_params(surface),
        return_binding: Some("result".to_string()),
        effects: vec!["concurrency".to_string(), "stdio".to_string()],
        contracts: vec![CoreContract {
            node_id: surface.contracts[0].node_id,
            kind: ContractKind::Ensure,
            predicate: "result == ()".to_string(),
            callsite_calls: Vec::new(),
            obligation_status: ContractObligationStatus::RuntimeRequired,
            span: surface.contracts[0].span.clone(),
        }],
        body: contracts_and_builtins_body(surface),
        ..function_shell(surface)
    }
}

fn contracts_and_builtins_params(surface: &Function) -> Vec<CoreParam> {
    vec![
        CoreParam {
            node_id: surface.params[0].node_id,
            name: "input".to_string(),
            ty: CoreType::option(CoreType::int()),
            span: surface.params[0].span.clone(),
        },
        CoreParam {
            node_id: surface.params[1].node_id,
            name: "receiver".to_string(),
            ty: CoreType::named("Receiver", vec![CoreType::int()]),
            span: surface.params[1].span.clone(),
        },
        CoreParam {
            node_id: surface.params[2].node_id,
            name: "count".to_string(),
            ty: CoreType::int(),
            span: surface.params[2].span.clone(),
        },
    ]
}

fn contracts_and_builtins_body(surface: &Function) -> CoreCleanupRegion {
    CoreCleanupRegion::new(vec![
        selected_statement(&surface.body[0]),
        table_statement(&surface.body[1]),
        recv_statement(&surface.body[2]),
        builtin_print_statement(&surface.body[3]),
        none_return_statement(&surface.body[4]),
    ])
}

fn selected_statement(line: &BodyLine) -> CoreStmt {
    let selected_match = let_expr(line);
    let (match_scrutinee, match_arms) = match_parts(selected_match);
    core_stmt(
        line,
        CoreStmtKind::Let {
            name: "selected".to_string(),
            ty: CoreType::int(),
            expr: core_expr(
                selected_match,
                CoreType::int(),
                CoreExprKind::Match {
                    scrutinee: Box::new(local(
                        match_scrutinee,
                        "input",
                        CoreType::option(CoreType::int()),
                    )),
                    arms: selected_match_arms(match_arms),
                },
            ),
        },
    )
}

fn selected_match_arms(match_arms: &[MatchArm]) -> Vec<CoreMatchArm> {
    vec![
        CoreMatchArm {
            node_id: match_arms[0].node_id,
            pattern: core_pattern(&match_arms[0].pattern),
            expr: local(&match_arms[0].expr, "value", CoreType::int()),
            span: match_arms[0].span.clone(),
        },
        CoreMatchArm {
            node_id: match_arms[1].node_id,
            pattern: core_pattern(&match_arms[1].pattern),
            expr: core_expr(
                &match_arms[1].expr,
                CoreType::int(),
                CoreExprKind::IntLiteral("0".to_string()),
            ),
            span: match_arms[1].span.clone(),
        },
    ]
}

fn table_statement(line: &BodyLine) -> CoreStmt {
    let table_dict = let_expr(line);
    let table_entries = dict_entries(table_dict);
    let entry = &table_entries[0];
    core_stmt(
        line,
        CoreStmtKind::Let {
            name: "table".to_string(),
            ty: CoreType::dict(CoreType::string(), CoreType::int()),
            expr: core_expr(
                table_dict,
                CoreType::dict(CoreType::string(), CoreType::int()),
                CoreExprKind::Dict(vec![CoreDictEntry {
                    node_id: entry.node_id,
                    key: core_expr(
                        &entry.key,
                        CoreType::string(),
                        CoreExprKind::StringLiteral("selected".to_string()),
                    ),
                    value: local(&entry.value, "selected", CoreType::int()),
                    span: entry.span.clone(),
                }]),
            ),
        },
    )
}

fn recv_statement(line: &BodyLine) -> CoreStmt {
    let recv_call = expr_line(line);
    let (_recv_callee, recv_args) = call_parts(recv_call);
    core_stmt(
        line,
        CoreStmtKind::Expr {
            expr: core_expr(
                recv_call,
                CoreType::option(CoreType::int()),
                CoreExprKind::Call {
                    target: CoreCallTarget::ConcurrencyBuiltin("channel::recv".to_string()),
                    args: vec![local(
                        &recv_args[0],
                        "receiver",
                        CoreType::named("Receiver", vec![CoreType::int()]),
                    )],
                },
            ),
        },
    )
}

fn builtin_print_statement(line: &BodyLine) -> CoreStmt {
    let print_call = expr_line(line);
    let (_print_callee, print_args) = call_parts(print_call);
    let len_call = &print_args[0];
    let (_len_callee, len_args) = call_parts(len_call);
    core_stmt(
        line,
        CoreStmtKind::Expr {
            expr: core_expr(
                print_call,
                CoreType::unit(),
                CoreExprKind::Call {
                    target: CoreCallTarget::StdioBuiltin("stdio::println".to_string()),
                    args: vec![core_expr(
                        len_call,
                        CoreType::int(),
                        CoreExprKind::Call {
                            target: CoreCallTarget::PreludeBuiltin("list::len".to_string()),
                            args: vec![local(
                                &len_args[0],
                                "table",
                                CoreType::dict(CoreType::string(), CoreType::int()),
                            )],
                        },
                    )],
                },
            ),
        },
    )
}

fn none_return_statement(line: &BodyLine) -> CoreStmt {
    core_stmt(
        line,
        CoreStmtKind::Return {
            expr: core_expr(
                expr_line(line),
                CoreType::option(CoreType::int()),
                CoreExprKind::OptionNone,
            ),
        },
    )
}

fn assert_surface_callee_ids(surface: &Function) {
    let recv_line = &surface.body[2];
    let print_line = &surface.body[3];
    let (recv_callee, _) = call_parts(expr_line(recv_line));
    let (print_callee, _) = call_parts(expr_line(print_line));
    assert_ne!(recv_callee.node_id, expr_line(recv_line).node_id);
    assert_ne!(print_callee.node_id, expr_line(print_line).node_id);
}

fn assert_contract_and_effects(function: &IrFunction) {
    assert_eq!(function.return_binding.as_deref(), Some("result"));
    assert_eq!(function.effects, vec!["concurrency", "stdio"]);
    assert_eq!(function.contracts.len(), 1);
    assert_eq!(function.contracts[0].kind, ContractKind::Ensure);
    assert_eq!(
        function.contracts[0].obligation_status,
        ContractObligationStatus::RuntimeRequired
    );
}

fn assert_match_statement(function: &IrFunction) {
    let IrStmtKind::Let { value, .. } = &function.body[0].kind else {
        panic!("selected should lower as let");
    };
    let IrExprKind::Match { scrutinee, arms } = &value.kind else {
        panic!("selected value should lower as match");
    };
    assert_eq!(scrutinee.kind, IrExprKind::Local("input".to_string()));
    assert!(matches!(
        &arms[0].pattern.kind,
        IrPatternKind::Constructor { name, args }
            if name == &vec!["Some".to_string()]
                && matches!(
                    args.as_slice(),
                    [IrPattern {
                        kind: IrPatternKind::Binding(binding),
                        ..
                    }] if binding == "value"
                )
    ));
    assert!(matches!(
        &arms[1].pattern.kind,
        IrPatternKind::Constructor { name, args }
            if name == &vec!["None".to_string()] && args.is_empty()
    ));
}

fn assert_dict_statement(function: &IrFunction) {
    let IrStmtKind::Let { value, .. } = &function.body[1].kind else {
        panic!("table should lower as let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Dict(entries)
            if matches!(
                entries.as_slice(),
                [IrDictEntry {
                    key: IrExpr {
                        kind: IrExprKind::StringLiteral(key),
                        ..
                    },
                    value: IrExpr {
                        kind: IrExprKind::Local(value),
                        ..
                    },
                    ..
                }] if key == "selected" && value == "selected"
            )
    ));
}

fn assert_builtin_call_statements(function: &IrFunction) {
    let IrStmtKind::Expr { value } = &function.body[2].kind else {
        panic!("recv should lower as expression");
    };
    assert!(matches!(
        value.kind,
        IrExprKind::Call {
            target: IrCallTarget::ConcurrencyBuiltin(_),
            ..
        }
    ));

    let IrStmtKind::Expr { value } = &function.body[3].kind else {
        panic!("print should lower as expression");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::StdioBuiltin(_),
            args
        } if matches!(
            args.as_slice(),
            [IrExpr {
                kind: IrExprKind::Call {
                    target: IrCallTarget::PreludeBuiltin(_),
                    ..
                },
                ..
            }]
        )
    ));

    let IrStmtKind::Return { value } = &function.body[4].kind else {
        panic!("none should lower as return");
    };
    assert_eq!(value.kind, IrExprKind::OptionNone);
}
