use super::*;

#[test]
fn lower_complete_program_preserves_function_shape_and_calls() {
    let module = fixture_ids();
    let surface = main_function(&module);
    let input = &surface.params[0];
    let mapper = &surface.params[1];
    let let_line = &surface.body[0];
    let print_line = &surface.body[1];
    let return_line = &surface.body[2];

    let mapper_call = let_expr(let_line);
    let (_mapper_callee, mapper_args) = call_parts(mapper_call);
    let input_arg = &mapper_args[0];
    let mapper_type = CoreType::Function {
        params: vec![CoreType::int()],
        variadic: None,
        return_type: Box::new(CoreType::int()),
        effects: Vec::new(),
    };

    let print_call = expr_line(print_line);
    let (_print_callee, print_args) = call_parts(print_call);
    let print_arg = &print_args[0];

    let ok_call = expr_line(return_line);
    let (_ok_callee, ok_args) = call_parts(ok_call);
    let ok_arg = &ok_args[0];
    let result_unit = CoreType::result(CoreType::unit(), CoreType::named("AppError", vec![]));

    let program = complete_program(vec![CoreFunction {
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
        contracts: Vec::new(),
        body: vec![
            core_stmt(
                let_line,
                CoreStmtKind::Let {
                    name: "answer".to_string(),
                    ty: CoreType::int(),
                    expr: core_expr(
                        mapper_call,
                        CoreType::int(),
                        CoreExprKind::Call {
                            target: CoreCallTarget::Value("mapper".to_string()),
                            args: vec![local(input_arg, "input", CoreType::int())],
                        },
                    ),
                },
            ),
            core_stmt(
                print_line,
                CoreStmtKind::Expr {
                    expr: core_expr(
                        print_call,
                        CoreType::unit(),
                        CoreExprKind::Call {
                            target: CoreCallTarget::StdioBuiltin("stdio::println".to_string()),
                            args: vec![core_expr(
                                print_arg,
                                CoreType::string(),
                                CoreExprKind::StringLiteral("done".to_string()),
                            )],
                        },
                    ),
                },
            ),
            core_stmt(
                return_line,
                CoreStmtKind::Return {
                    expr: core_expr(
                        ok_call,
                        result_unit.clone(),
                        CoreExprKind::ResultOk(Box::new(core_expr(
                            ok_arg,
                            CoreType::unit(),
                            CoreExprKind::Unit,
                        ))),
                    ),
                },
            ),
        ],
        span: surface.span.clone(),
    }]);

    let ir = lower_checked_core(&program).expect("complete core should lower");

    assert_eq!(ir.functions.len(), 1);
    let function = &ir.functions[0];
    assert_eq!(function.node_id, surface.node_id);
    assert_eq!(function.name, "main");
    assert_eq!(function.visibility, Visibility::Public);
    assert_eq!(function.return_type, result_unit);
    assert_eq!(function.effects, vec!["stdio"]);
    assert_eq!(function.params.len(), 2);
    assert_eq!(function.params[0].name, "input");
    assert_eq!(function.params[0].ty, CoreType::int());
    assert_eq!(function.params[1].name, "mapper");
    assert_eq!(function.params[1].ty, mapper_type);

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
fn lower_nested_expression_variants_preserves_structure_and_types() {
    let module = lower_source(concat!(
        "fn main(flag: Bool) -> ()\n",
        "  { some: Some(flag), err: Err(\"bad\"), items: [1, 2], tried: parse(\"1\")?, ",
        "negated: -1, checked: not false, combined: 1 + 2, ratio: 1.5 }\n",
        "end\n",
    ));
    let surface = main_function(&module);
    let return_line = &surface.body[0];
    let record = expr_line(return_line);
    let fields = record_fields(record);

    let some = named_field(fields, "some");
    let (some_call, some_args) = call_parts(&some.expr);
    let some_value = &some_args[0];
    let err = named_field(fields, "err");
    let (err_call, err_args) = call_parts(&err.expr);
    let err_value = &err_args[0];
    let items = named_field(fields, "items");
    let item_exprs = list_items(&items.expr);
    let tried = named_field(fields, "tried");
    let try_call = try_inner(&tried.expr);
    let (_parse_callee, parse_args) = call_parts(try_call);
    let negated = named_field(fields, "negated");
    let negated_inner = prefix_inner(&negated.expr);
    let checked = named_field(fields, "checked");
    let checked_inner = prefix_inner(&checked.expr);
    let combined = named_field(fields, "combined");
    let (left, right) = binary_parts(&combined.expr);
    let ratio = named_field(fields, "ratio");

    let app_error = CoreType::named("AppError", vec![]);
    let lowered_fields = vec![
        CoreRecordField {
            node_id: some.node_id,
            name: "some".to_string(),
            expr: core_expr(
                some_call,
                CoreType::option(CoreType::bool()),
                CoreExprKind::OptionSome(Box::new(local(some_value, "flag", CoreType::bool()))),
            ),
            span: some.span.clone(),
        },
        CoreRecordField {
            node_id: err.node_id,
            name: "err".to_string(),
            expr: core_expr(
                err_call,
                CoreType::result(CoreType::Unknown, CoreType::string()),
                CoreExprKind::ResultErr(Box::new(core_expr(
                    err_value,
                    CoreType::string(),
                    CoreExprKind::StringLiteral("bad".to_string()),
                ))),
            ),
            span: err.span.clone(),
        },
        CoreRecordField {
            node_id: items.node_id,
            name: "items".to_string(),
            expr: core_expr(
                &items.expr,
                CoreType::vec(CoreType::int()),
                CoreExprKind::List(vec![
                    core_expr(
                        &item_exprs[0],
                        CoreType::int(),
                        CoreExprKind::IntLiteral("1".to_string()),
                    ),
                    core_expr(
                        &item_exprs[1],
                        CoreType::int(),
                        CoreExprKind::IntLiteral("2".to_string()),
                    ),
                ]),
            ),
            span: items.span.clone(),
        },
        CoreRecordField {
            node_id: tried.node_id,
            name: "tried".to_string(),
            expr: core_expr(
                &tried.expr,
                CoreType::int(),
                CoreExprKind::Try(Box::new(core_expr(
                    try_call,
                    CoreType::result(CoreType::int(), app_error.clone()),
                    CoreExprKind::Call {
                        target: CoreCallTarget::Function("parse".to_string()),
                        args: vec![core_expr(
                            &parse_args[0],
                            CoreType::string(),
                            CoreExprKind::StringLiteral("1".to_string()),
                        )],
                    },
                ))),
            ),
            span: tried.span.clone(),
        },
        CoreRecordField {
            node_id: negated.node_id,
            name: "negated".to_string(),
            expr: core_expr(
                &negated.expr,
                CoreType::int(),
                CoreExprKind::Prefix {
                    op: PrefixOp::Negate,
                    expr: Box::new(core_expr(
                        negated_inner,
                        CoreType::int(),
                        CoreExprKind::IntLiteral("1".to_string()),
                    )),
                },
            ),
            span: negated.span.clone(),
        },
        CoreRecordField {
            node_id: checked.node_id,
            name: "checked".to_string(),
            expr: core_expr(
                &checked.expr,
                CoreType::bool(),
                CoreExprKind::Prefix {
                    op: PrefixOp::Not,
                    expr: Box::new(core_expr(
                        checked_inner,
                        CoreType::bool(),
                        CoreExprKind::BoolLiteral(false),
                    )),
                },
            ),
            span: checked.span.clone(),
        },
        CoreRecordField {
            node_id: combined.node_id,
            name: "combined".to_string(),
            expr: core_expr(
                &combined.expr,
                CoreType::int(),
                CoreExprKind::Binary {
                    op: BinaryOp::Add,
                    left: Box::new(core_expr(
                        left,
                        CoreType::int(),
                        CoreExprKind::IntLiteral("1".to_string()),
                    )),
                    right: Box::new(core_expr(
                        right,
                        CoreType::int(),
                        CoreExprKind::IntLiteral("2".to_string()),
                    )),
                },
            ),
            span: combined.span.clone(),
        },
        CoreRecordField {
            node_id: ratio.node_id,
            name: "ratio".to_string(),
            expr: core_expr(
                &ratio.expr,
                CoreType::float(),
                CoreExprKind::FloatLiteral("1.5".to_string()),
            ),
            span: ratio.span.clone(),
        },
    ];
    let record_type = CoreType::Record(
        lowered_fields
            .iter()
            .map(|field| (field.name.clone(), field.expr.ty.clone()))
            .collect(),
    );

    let program = complete_program(vec![CoreFunction {
        params: vec![CoreParam {
            node_id: surface.params[0].node_id,
            name: "flag".to_string(),
            ty: CoreType::bool(),
            span: surface.params[0].span.clone(),
        }],
        return_type: record_type.clone(),
        body: vec![core_stmt(
            return_line,
            CoreStmtKind::Return {
                expr: core_expr(
                    record,
                    record_type,
                    CoreExprKind::Record(lowered_fields.clone()),
                ),
            },
        )],
        ..function_shell(surface)
    }]);

    let ir = lower_checked_core(&program).expect("record variants should lower");
    let IrStmtKind::Return { value } = &ir.functions[0].body[0].kind else {
        panic!("record expression should be returned");
    };
    let IrExprKind::Record(fields) = &value.kind else {
        panic!("return value should be a record");
    };

    assert_eq!(
        fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "some", "err", "items", "tried", "negated", "checked", "combined", "ratio"
        ]
    );
    assert!(matches!(fields[0].value.kind, IrExprKind::OptionSome(_)));
    assert!(matches!(fields[1].value.kind, IrExprKind::ResultErr(_)));
    assert!(matches!(fields[2].value.kind, IrExprKind::List(ref items) if items.len() == 2));
    assert!(matches!(fields[3].value.kind, IrExprKind::Try(_)));
    assert!(matches!(
        fields[4].value.kind,
        IrExprKind::Prefix {
            op: PrefixOp::Negate,
            ..
        }
    ));
    assert!(matches!(
        fields[5].value.kind,
        IrExprKind::Prefix {
            op: PrefixOp::Not,
            ..
        }
    ));
    assert!(matches!(
        fields[6].value.kind,
        IrExprKind::Binary {
            op: BinaryOp::Add,
            ..
        }
    ));
    assert_eq!(
        fields[7].value.kind,
        IrExprKind::FloatLiteral("1.5".to_string())
    );
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
    let selected_line = &surface.body[0];
    let table_line = &surface.body[1];
    let recv_line = &surface.body[2];
    let print_line = &surface.body[3];
    let none_line = &surface.body[4];
    let selected_match = let_expr(selected_line);
    let (match_scrutinee, match_arms) = match_parts(selected_match);
    let table_dict = let_expr(table_line);
    let table_entries = dict_entries(table_dict);
    let (recv_callee, recv_args) = call_parts(expr_line(recv_line));
    let (print_callee, print_args) = call_parts(expr_line(print_line));
    let len_call = &print_args[0];
    let (_len_callee, len_args) = call_parts(len_call);

    let program = complete_program(vec![CoreFunction {
        params: vec![
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
        ],
        return_binding: Some("result".to_string()),
        effects: vec!["concurrency".to_string(), "stdio".to_string()],
        contracts: vec![CoreContract {
            node_id: surface.contracts[0].node_id,
            kind: ContractKind::Ensure,
            predicate: "result == ()".to_string(),
            obligation_status: ContractObligationStatus::RuntimeRequired,
            span: surface.contracts[0].span.clone(),
        }],
        body: vec![
            core_stmt(
                selected_line,
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
                            arms: vec![
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
                            ],
                        },
                    ),
                },
            ),
            core_stmt(
                table_line,
                CoreStmtKind::Let {
                    name: "table".to_string(),
                    ty: CoreType::dict(CoreType::string(), CoreType::int()),
                    expr: core_expr(
                        table_dict,
                        CoreType::dict(CoreType::string(), CoreType::int()),
                        CoreExprKind::Dict(vec![CoreDictEntry {
                            node_id: table_entries[0].node_id,
                            key: core_expr(
                                &table_entries[0].key,
                                CoreType::string(),
                                CoreExprKind::StringLiteral("selected".to_string()),
                            ),
                            value: local(&table_entries[0].value, "selected", CoreType::int()),
                            span: table_entries[0].span.clone(),
                        }]),
                    ),
                },
            ),
            core_stmt(
                recv_line,
                CoreStmtKind::Expr {
                    expr: core_expr(
                        expr_line(recv_line),
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
            ),
            core_stmt(
                print_line,
                CoreStmtKind::Expr {
                    expr: core_expr(
                        expr_line(print_line),
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
            ),
            core_stmt(
                none_line,
                CoreStmtKind::Return {
                    expr: core_expr(
                        expr_line(none_line),
                        CoreType::option(CoreType::int()),
                        CoreExprKind::OptionNone,
                    ),
                },
            ),
        ],
        ..function_shell(surface)
    }]);

    assert_ne!(recv_callee.node_id, expr_line(recv_line).node_id);
    assert_ne!(print_callee.node_id, expr_line(print_line).node_id);

    let ir = lower_checked_core(&program).expect("complete core should lower");
    let function = &ir.functions[0];

    assert_eq!(function.return_binding.as_deref(), Some("result"));
    assert_eq!(function.effects, vec!["concurrency", "stdio"]);
    assert_eq!(function.contracts.len(), 1);
    assert_eq!(function.contracts[0].kind, ContractKind::Ensure);
    assert_eq!(
        function.contracts[0].obligation_status,
        ContractObligationStatus::RuntimeRequired
    );

    let IrStmtKind::Let { value, .. } = &function.body[0].kind else {
        panic!("selected should lower as let");
    };
    assert!(matches!(&value.kind, IrExprKind::Match { arms, .. } if arms.len() == 2));
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
