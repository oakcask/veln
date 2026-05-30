use veln_ast::NodeId;
use veln_core::{
    CheckedProgram, CoreBlocker, CoreCallTarget, CoreExpr, CoreExprKind, CoreReadiness, CoreStmt,
    CoreStmtKind,
};

use crate::{
    IrCallTarget, IrContract, IrDictEntry, IrExpr, IrExprKind, IrFunction, IrMatchArm, IrParam,
    IrPattern, IrPatternField, IrPatternKind, IrRecordField, IrStmt, IrStmtKind, TypedProgram,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IrLowerError {
    Blocked(CoreBlocker),
    UnresolvedCallTarget { node_id: NodeId, symbol: String },
    MissingExpression { node_id: NodeId },
    Hole { node_id: NodeId },
}

pub fn lower_checked_core(program: &CheckedProgram) -> Result<TypedProgram, IrLowerError> {
    if let CoreReadiness::Blocked(blockers) = &program.readiness
        && let Some(blocker) = blockers.first()
    {
        return Err(IrLowerError::Blocked(blocker.clone()));
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
        return_binding: function.return_binding.clone(),
        return_type: function.return_type.clone(),
        effects: function.effects.clone(),
        contracts: function
            .contracts
            .iter()
            .map(|contract| IrContract {
                node_id: contract.node_id,
                kind: contract.kind,
                predicate: contract.predicate.clone(),
                obligation_status: contract.obligation_status,
                span: contract.span.clone(),
            })
            .collect(),
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
        kind: lower_expr_kind(expr)?,
    })
}

fn lower_expr_kind(expr: &CoreExpr) -> Result<IrExprKind, IrLowerError> {
    if let Some(kind) = lower_blocked_expr(expr)? {
        return Ok(kind);
    }
    if let Some(kind) = lower_scalar_expr(expr) {
        return Ok(kind);
    }
    if let Some(kind) = lower_wrapped_expr(expr)? {
        return Ok(kind);
    }
    if let Some(kind) = lower_collection_expr(expr)? {
        return Ok(kind);
    }
    lower_operator_expr(expr)
}

fn lower_blocked_expr(expr: &CoreExpr) -> Result<Option<IrExprKind>, IrLowerError> {
    match &expr.kind {
        CoreExprKind::Missing => Err(IrLowerError::MissingExpression {
            node_id: expr.node_id,
        }),
        CoreExprKind::Hole { .. } => Err(IrLowerError::Hole {
            node_id: expr.node_id,
        }),
        _ => Ok(None),
    }
}

fn lower_scalar_expr(expr: &CoreExpr) -> Option<IrExprKind> {
    match &expr.kind {
        CoreExprKind::Local(name) => Some(IrExprKind::Local(name.clone())),
        CoreExprKind::BoolLiteral(value) => Some(IrExprKind::BoolLiteral(*value)),
        CoreExprKind::StringLiteral(value) => Some(IrExprKind::StringLiteral(value.clone())),
        CoreExprKind::IntLiteral(value) => Some(IrExprKind::IntLiteral(value.clone())),
        CoreExprKind::FloatLiteral(value) => Some(IrExprKind::FloatLiteral(value.clone())),
        CoreExprKind::Unit => Some(IrExprKind::Unit),
        CoreExprKind::FunctionValue(name) => Some(IrExprKind::FunctionValue(name.clone())),
        CoreExprKind::OptionNone => Some(IrExprKind::OptionNone),
        CoreExprKind::ListNil => Some(IrExprKind::ListNil),
        _ => None,
    }
}

fn lower_wrapped_expr(expr: &CoreExpr) -> Result<Option<IrExprKind>, IrLowerError> {
    match &expr.kind {
        CoreExprKind::ResultOk(value) => lower_unary_expr(value, IrExprKind::ResultOk).map(Some),
        CoreExprKind::ResultErr(value) => lower_unary_expr(value, IrExprKind::ResultErr).map(Some),
        CoreExprKind::OptionSome(value) => {
            lower_unary_expr(value, IrExprKind::OptionSome).map(Some)
        }
        CoreExprKind::ListCons { head, tail } => Ok(Some(IrExprKind::ListCons {
            head: Box::new(lower_expr(head)?),
            tail: Box::new(lower_expr(tail)?),
        })),
        CoreExprKind::Call { target, args } => {
            lower_call_expr(expr.node_id, target, args).map(Some)
        }
        CoreExprKind::FieldAccess { base, field } => Ok(Some(IrExprKind::FieldAccess {
            base: Box::new(lower_expr(base)?),
            field: field.clone(),
        })),
        CoreExprKind::Try(value) => lower_unary_expr(value, IrExprKind::Try).map(Some),
        _ => Ok(None),
    }
}

fn lower_collection_expr(expr: &CoreExpr) -> Result<Option<IrExprKind>, IrLowerError> {
    match &expr.kind {
        CoreExprKind::Record(fields) => lower_record_expr(fields).map(Some),
        CoreExprKind::Dict(entries) => lower_dict_expr(entries).map(Some),
        CoreExprKind::List(items) => Ok(Some(IrExprKind::List(lower_exprs(items)?))),
        CoreExprKind::Match { scrutinee, arms } => lower_match_expr(scrutinee, arms).map(Some),
        _ => Ok(None),
    }
}

fn lower_operator_expr(expr: &CoreExpr) -> Result<IrExprKind, IrLowerError> {
    match &expr.kind {
        CoreExprKind::Prefix { op, expr } => Ok(IrExprKind::Prefix {
            op: *op,
            expr: Box::new(lower_expr(expr)?),
        }),
        CoreExprKind::Binary { op, left, right } => Ok(IrExprKind::Binary {
            op: *op,
            left: Box::new(lower_expr(left)?),
            right: Box::new(lower_expr(right)?),
        }),
        _ => unreachable!("core expression variant should be handled before operator lowering"),
    }
}

fn lower_unary_expr(
    expr: &CoreExpr,
    build: impl FnOnce(Box<IrExpr>) -> IrExprKind,
) -> Result<IrExprKind, IrLowerError> {
    Ok(build(Box::new(lower_expr(expr)?)))
}

fn lower_call_expr(
    node_id: NodeId,
    target: &veln_core::CoreCallTarget,
    args: &[CoreExpr],
) -> Result<IrExprKind, IrLowerError> {
    Ok(IrExprKind::Call {
        target: lower_call_target(node_id, target)?,
        args: lower_exprs(args)?,
    })
}

fn lower_exprs(exprs: &[CoreExpr]) -> Result<Vec<IrExpr>, IrLowerError> {
    exprs.iter().map(lower_expr).collect()
}

fn lower_record_expr(fields: &[veln_core::CoreRecordField]) -> Result<IrExprKind, IrLowerError> {
    Ok(IrExprKind::Record(
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
    ))
}

fn lower_dict_expr(entries: &[veln_core::CoreDictEntry]) -> Result<IrExprKind, IrLowerError> {
    Ok(IrExprKind::Dict(
        entries
            .iter()
            .map(|entry| {
                Ok(IrDictEntry {
                    node_id: entry.node_id,
                    key: lower_expr(&entry.key)?,
                    value: lower_expr(&entry.value)?,
                })
            })
            .collect::<Result<Vec<_>, IrLowerError>>()?,
    ))
}

fn lower_match_expr(
    scrutinee: &CoreExpr,
    arms: &[veln_core::CoreMatchArm],
) -> Result<IrExprKind, IrLowerError> {
    Ok(IrExprKind::Match {
        scrutinee: Box::new(lower_expr(scrutinee)?),
        arms: arms
            .iter()
            .map(|arm| {
                Ok(IrMatchArm {
                    node_id: arm.node_id,
                    pattern: lower_pattern(&arm.pattern),
                    value: lower_expr(&arm.expr)?,
                })
            })
            .collect::<Result<Vec<_>, IrLowerError>>()?,
    })
}

fn lower_pattern(pattern: &veln_core::CorePattern) -> IrPattern {
    IrPattern {
        node_id: pattern.node_id,
        kind: match &pattern.kind {
            veln_core::CorePatternKind::Wildcard => IrPatternKind::Wildcard,
            veln_core::CorePatternKind::Binding(name) => IrPatternKind::Binding(name.clone()),
            veln_core::CorePatternKind::StringLiteral(value) => {
                IrPatternKind::StringLiteral(value.clone())
            }
            veln_core::CorePatternKind::IntLiteral(value) => {
                IrPatternKind::IntLiteral(value.clone())
            }
            veln_core::CorePatternKind::FloatLiteral(value) => {
                IrPatternKind::FloatLiteral(value.clone())
            }
            veln_core::CorePatternKind::BoolLiteral(value) => IrPatternKind::BoolLiteral(*value),
            veln_core::CorePatternKind::Unit => IrPatternKind::Unit,
            veln_core::CorePatternKind::Record(fields) => IrPatternKind::Record(
                fields
                    .iter()
                    .map(|field| IrPatternField {
                        node_id: field.node_id,
                        name: field.name.clone(),
                        pattern: lower_pattern(&field.pattern),
                    })
                    .collect(),
            ),
            veln_core::CorePatternKind::Constructor { name, args } => IrPatternKind::Constructor {
                name: name.clone(),
                args: args.iter().map(lower_pattern).collect(),
            },
        },
    }
}

fn lower_call_target(
    node_id: NodeId,
    target: &CoreCallTarget,
) -> Result<IrCallTarget, IrLowerError> {
    match target {
        CoreCallTarget::Function(name) => Ok(IrCallTarget::Function(name.clone())),
        CoreCallTarget::StdioBuiltin(name) => Ok(IrCallTarget::StdioBuiltin(name.clone())),
        CoreCallTarget::ConcurrencyBuiltin(name) => {
            Ok(IrCallTarget::ConcurrencyBuiltin(name.clone()))
        }
        CoreCallTarget::StandardLibraryBuiltin(name) => {
            Ok(IrCallTarget::StandardLibraryBuiltin(name.clone()))
        }
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
        BinaryOp, BodyLine, BodyLineKind, ContractKind, DictEntry, Expr, ExprKind, Function,
        MatchArm, Pattern, PatternField, PatternKind, PrefixOp, RecordField, SurfaceModule,
        Visibility, lower_surface_ast,
    };
    use veln_core::{
        ContractObligationStatus, CoreContract, CoreDictEntry, CoreFunction, CoreMatchArm,
        CoreParam, CorePattern, CorePatternField, CorePatternKind, CoreReadiness, CoreRecordField,
        CoreStmtKind, CoreType,
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
            return_binding: None,
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

    fn dict_entries(expr: &Expr) -> &[DictEntry] {
        let ExprKind::Dict(entries) = &expr.kind else {
            panic!("expected dictionary expression");
        };
        entries
    }

    fn match_parts(expr: &Expr) -> (&Expr, &[MatchArm]) {
        let ExprKind::Match { scrutinee, arms } = &expr.kind else {
            panic!("expected match expression");
        };
        (scrutinee, arms)
    }

    fn core_pattern(pattern: &Pattern) -> CorePattern {
        CorePattern {
            node_id: pattern.node_id,
            kind: match &pattern.kind {
                PatternKind::Wildcard => CorePatternKind::Wildcard,
                PatternKind::Binding(name) => CorePatternKind::Binding(name.clone()),
                PatternKind::StringLiteral(value) => CorePatternKind::StringLiteral(value.clone()),
                PatternKind::IntLiteral(value) => CorePatternKind::IntLiteral(value.clone()),
                PatternKind::FloatLiteral(value) => CorePatternKind::FloatLiteral(value.clone()),
                PatternKind::BoolLiteral(value) => CorePatternKind::BoolLiteral(*value),
                PatternKind::Unit => CorePatternKind::Unit,
                PatternKind::Record(fields) => CorePatternKind::Record(
                    fields.iter().map(core_pattern_field).collect::<Vec<_>>(),
                ),
                PatternKind::Constructor { name, args } => CorePatternKind::Constructor {
                    name: name.clone(),
                    args: args.iter().map(core_pattern).collect(),
                },
            },
            span: pattern.span.clone(),
        }
    }

    fn core_pattern_field(field: &PatternField) -> CorePatternField {
        CorePatternField {
            node_id: field.node_id,
            name: field.name.clone(),
            pattern: core_pattern(&field.pattern),
            span: field.span.clone(),
        }
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
            "pub fn main(input: Option(Int), receiver: Receiver(Int), count: Int) -> result: () effects [concurrency, stdio]\n",
            "  ensure result == ()\n",
            "  let selected: Int = match input\n",
            "    Some(value) => value\n",
            "    None => 0\n",
            "  end\n",
            "  let table: Dict(String, Int) = {\"selected\": selected}\n",
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
                                target: CoreCallTarget::ConcurrencyBuiltin(
                                    "channel::recv".to_string(),
                                ),
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
                                        target: CoreCallTarget::PreludeBuiltin(
                                            "list::len".to_string(),
                                        ),
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
                                expr: core_expr(
                                    &value.expr,
                                    CoreType::Unknown,
                                    CoreExprKind::Missing,
                                ),
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
}
