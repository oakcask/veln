use veln_ast::NodeId;
use veln_core::{
    CheckedProgram, CoreBlocker, CoreCallTarget, CoreExpr, CoreExprKind, CoreReadiness, CoreStmt,
    CoreStmtKind,
};

use crate::{
    IrCallTarget, IrExpr, IrExprKind, IrFunction, IrParam, IrRecordField, IrStmt, IrStmtKind,
    TypedProgram,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrLowerError {
    Blocked(CoreBlocker),
    UnresolvedCallTarget { node_id: NodeId, symbol: String },
    MissingExpression { node_id: NodeId },
    Hole { node_id: NodeId },
}

pub fn lower_checked_core(program: &CheckedProgram) -> Result<TypedProgram, IrLowerError> {
    if let CoreReadiness::Blocked(blockers) = &program.readiness {
        if let Some(blocker) = blockers.first() {
            return Err(IrLowerError::Blocked(blocker.clone()));
        }
    }

    let functions = program
        .functions
        .iter()
        .map(lower_function)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(TypedProgram { functions })
}

fn lower_function(function: &veln_core::CoreFunction) -> Result<IrFunction, IrLowerError> {
    Ok(IrFunction {
        node_id: function.node_id,
        name: function.name.clone(),
        visibility: function.visibility,
        params: function
            .params
            .iter()
            .map(|param| IrParam {
                node_id: param.node_id,
                name: param.name.clone(),
                ty: param.ty.clone(),
            })
            .collect(),
        return_type: function.return_type.clone(),
        effects: function.effects.clone(),
        body: function
            .body
            .iter()
            .map(lower_stmt)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn lower_stmt(stmt: &CoreStmt) -> Result<IrStmt, IrLowerError> {
    Ok(IrStmt {
        node_id: stmt.node_id,
        kind: match &stmt.kind {
            CoreStmtKind::Let { name, ty, expr } => IrStmtKind::Let {
                name: name.clone(),
                ty: ty.clone(),
                value: lower_expr(expr)?,
            },
            CoreStmtKind::Expr { expr } => IrStmtKind::Expr {
                value: lower_expr(expr)?,
            },
            CoreStmtKind::Return { expr } => IrStmtKind::Return {
                value: lower_expr(expr)?,
            },
        },
    })
}

fn lower_expr(expr: &CoreExpr) -> Result<IrExpr, IrLowerError> {
    Ok(IrExpr {
        node_id: expr.node_id,
        ty: expr.ty.clone(),
        span: expr.span.clone(),
        kind: match &expr.kind {
            CoreExprKind::Missing => {
                return Err(IrLowerError::MissingExpression {
                    node_id: expr.node_id,
                });
            }
            CoreExprKind::Hole { .. } => {
                return Err(IrLowerError::Hole {
                    node_id: expr.node_id,
                });
            }
            CoreExprKind::Local(name) => IrExprKind::Local(name.clone()),
            CoreExprKind::BoolLiteral(value) => IrExprKind::BoolLiteral(*value),
            CoreExprKind::StringLiteral(value) => IrExprKind::StringLiteral(value.clone()),
            CoreExprKind::IntLiteral(value) => IrExprKind::IntLiteral(value.clone()),
            CoreExprKind::FloatLiteral(value) => IrExprKind::FloatLiteral(value.clone()),
            CoreExprKind::Unit => IrExprKind::Unit,
            CoreExprKind::ResultOk(value) => IrExprKind::ResultOk(Box::new(lower_expr(value)?)),
            CoreExprKind::ResultErr(value) => IrExprKind::ResultErr(Box::new(lower_expr(value)?)),
            CoreExprKind::OptionSome(value) => IrExprKind::OptionSome(Box::new(lower_expr(value)?)),
            CoreExprKind::OptionNone => IrExprKind::OptionNone,
            CoreExprKind::Call { target, args } => IrExprKind::Call {
                target: lower_call_target(expr.node_id, target)?,
                args: args.iter().map(lower_expr).collect::<Result<Vec<_>, _>>()?,
            },
            CoreExprKind::Try(value) => IrExprKind::Try(Box::new(lower_expr(value)?)),
            CoreExprKind::Record(fields) => IrExprKind::Record(
                fields
                    .iter()
                    .map(|field| {
                        Ok(IrRecordField {
                            node_id: field.node_id,
                            name: field.name.clone(),
                            value: lower_expr(&field.expr)?,
                        })
                    })
                    .collect::<Result<Vec<_>, IrLowerError>>()?,
            ),
            CoreExprKind::List(items) => IrExprKind::List(
                items
                    .iter()
                    .map(lower_expr)
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            CoreExprKind::Prefix { op, expr } => IrExprKind::Prefix {
                op: *op,
                expr: Box::new(lower_expr(expr)?),
            },
            CoreExprKind::Binary { op, left, right } => IrExprKind::Binary {
                op: *op,
                left: Box::new(lower_expr(left)?),
                right: Box::new(lower_expr(right)?),
            },
        },
    })
}

fn lower_call_target(
    node_id: NodeId,
    target: &CoreCallTarget,
) -> Result<IrCallTarget, IrLowerError> {
    match target {
        CoreCallTarget::Function(name) => Ok(IrCallTarget::Function(name.clone())),
        CoreCallTarget::StdioBuiltin(name) => Ok(IrCallTarget::StdioBuiltin(name.clone())),
        CoreCallTarget::PreludeBuiltin(name) => Ok(IrCallTarget::PreludeBuiltin(name.clone())),
        CoreCallTarget::Value(name) => Ok(IrCallTarget::Value(name.clone())),
        CoreCallTarget::Unresolved(symbol) => Err(IrLowerError::UnresolvedCallTarget {
            node_id,
            symbol: symbol.clone(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veln_ast::{
        BinaryOp, BodyLine, BodyLineKind, Expr, ExprKind, Function, PrefixOp, RecordField,
        SurfaceModule, Visibility, lower_surface_ast,
    };
    use veln_core::{
        CoreFunction, CoreParam, CoreReadiness, CoreRecordField, CoreStmtKind, CoreType,
    };
    use veln_source::SourceFile;
    use veln_syntax::parse;

    fn lower_source(text: &str) -> SurfaceModule {
        let source = SourceFile::new("main.veln", text);
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics: {:?}",
            parsed.diagnostics
        );
        lower_surface_ast(&parsed.tree)
    }

    fn core_expr(expr: &Expr, ty: CoreType, kind: CoreExprKind) -> CoreExpr {
        CoreExpr {
            node_id: expr.node_id,
            ty,
            kind,
            span: expr.span.clone(),
        }
    }

    fn core_stmt(line: &BodyLine, kind: CoreStmtKind) -> CoreStmt {
        CoreStmt {
            node_id: line.node_id,
            kind,
            span: line.span.clone(),
        }
    }

    fn local(expr: &Expr, name: &str, ty: CoreType) -> CoreExpr {
        core_expr(expr, ty, CoreExprKind::Local(name.to_string()))
    }

    fn function_shell(function: &Function) -> CoreFunction {
        CoreFunction {
            node_id: function.node_id,
            name: function.name.clone().expect("function should be named"),
            visibility: function.visibility,
            params: Vec::new(),
            return_type: CoreType::unit(),
            effects: Vec::new(),
            contracts: Vec::new(),
            body: Vec::new(),
            span: function.span.clone(),
        }
    }

    fn complete_program(functions: Vec<CoreFunction>) -> CheckedProgram {
        CheckedProgram {
            functions,
            readiness: CoreReadiness::Complete,
        }
    }

    fn let_expr(line: &BodyLine) -> &Expr {
        let BodyLineKind::Let { expr, .. } = &line.kind else {
            panic!("expected let line");
        };
        expr
    }

    fn expr_line(line: &BodyLine) -> &Expr {
        let BodyLineKind::Expr { expr } = &line.kind else {
            panic!("expected expression line");
        };
        expr
    }

    fn call_parts(expr: &Expr) -> (&Expr, &[Expr]) {
        let ExprKind::Call { callee, args } = &expr.kind else {
            panic!("expected call expression");
        };
        (callee, args)
    }

    fn try_inner(expr: &Expr) -> &Expr {
        let ExprKind::Try(inner) = &expr.kind else {
            panic!("expected try expression");
        };
        inner
    }

    fn list_items(expr: &Expr) -> &[Expr] {
        let ExprKind::List(items) = &expr.kind else {
            panic!("expected list expression");
        };
        items
    }

    fn prefix_inner(expr: &Expr) -> &Expr {
        let ExprKind::Prefix { expr, .. } = &expr.kind else {
            panic!("expected prefix expression");
        };
        expr
    }

    fn binary_parts(expr: &Expr) -> (&Expr, &Expr) {
        let ExprKind::Binary { left, right, .. } = &expr.kind else {
            panic!("expected binary expression");
        };
        (left, right)
    }

    fn record_fields(expr: &Expr) -> &[RecordField] {
        let ExprKind::Record(fields) = &expr.kind else {
            panic!("expected record expression");
        };
        fields
    }

    fn named_field<'a>(fields: &'a [RecordField], name: &str) -> &'a RecordField {
        fields
            .iter()
            .find(|field| field.name == name)
            .expect("record field should exist")
    }

    fn main_function(module: &SurfaceModule) -> &Function {
        module
            .functions
            .iter()
            .find(|function| function.name.as_deref() == Some("main"))
            .expect("main should exist")
    }

    fn fixture_ids() -> SurfaceModule {
        lower_source(concat!(
            "pub fn main(input: Int, mapper: Mapper) -> Result((), AppError) effects [stdio]\n",
            "  let answer: Int = mapper(input)\n",
            "  stdio::println(\"done\")\n",
            "  Ok(())\n",
            "end\n",
        ))
    }

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
                    CoreType::list(CoreType::int()),
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
}
