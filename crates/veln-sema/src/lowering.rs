use veln_ast::{BinaryOp, BodyLineKind, Expr, ExprKind, Function, RecordField, SurfaceModule};
use veln_core::{
    CheckedProgram, CoreBlocker, CoreCallTarget, CoreContract, CoreExpr, CoreExprKind,
    CoreFunction, CoreParam, CoreReadiness, CoreRecordField, CoreStmt, CoreStmtKind, CoreType,
};

use crate::effects::stdio_signature;
use crate::prelude::core_prelude_signature;
use crate::types::{TypeEnvironment, core_type, parse_type_annotation, parse_type_or_unknown};

struct CoreBinding {
    name: String,
    ty: CoreType,
}

struct CoreLowerer<'a> {
    function: &'a Function,
    environment: &'a TypeEnvironment,
    bindings: Vec<CoreBinding>,
    blockers: Vec<CoreBlocker>,
}

pub(crate) fn lower_surface_module_to_core(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> CheckedProgram {
    let mut blockers = Vec::new();
    let functions = module
        .functions
        .iter()
        .map(|function| {
            let mut lowerer = CoreLowerer::new(function, environment);
            let lowered = lowerer.lower_function();
            blockers.extend(lowerer.blockers);
            lowered
        })
        .collect();
    CheckedProgram {
        functions,
        readiness: if blockers.is_empty() {
            CoreReadiness::Complete
        } else {
            CoreReadiness::Blocked(blockers)
        },
    }
}

impl<'a> CoreLowerer<'a> {
    fn new(function: &'a Function, environment: &'a TypeEnvironment) -> Self {
        Self {
            function,
            environment,
            bindings: Vec::new(),
            blockers: Vec::new(),
        }
    }

    fn lower_function(&mut self) -> CoreFunction {
        let params = self
            .function
            .params
            .iter()
            .map(|param| {
                let ty = core_type(&parse_type_or_unknown(param.ty.as_deref()));
                self.bindings.push(CoreBinding {
                    name: param.name.clone(),
                    ty: ty.clone(),
                });
                CoreParam {
                    node_id: param.node_id,
                    name: param.name.clone(),
                    ty,
                    span: param.span.clone(),
                }
            })
            .collect();
        let return_type = core_type(&parse_type_or_unknown(self.function.return_type.as_deref()));
        let contracts = self
            .function
            .contracts
            .iter()
            .map(|contract| CoreContract {
                node_id: contract.node_id,
                kind: contract.kind,
                predicate: contract.text.clone(),
                span: contract.span.clone(),
            })
            .collect();
        let body = self.lower_body(&return_type);

        CoreFunction {
            node_id: self.function.node_id,
            name: self
                .function
                .name
                .clone()
                .unwrap_or_else(|| "<missing>".to_string()),
            visibility: self.function.visibility,
            params,
            return_type,
            effects: self.function.effects.clone().unwrap_or_default(),
            contracts,
            body,
            span: self.function.span.clone(),
        }
    }

    fn lower_body(&mut self, return_type: &CoreType) -> Vec<CoreStmt> {
        let mut body = Vec::new();
        for (index, line) in self.function.body.iter().enumerate() {
            match &line.kind {
                BodyLineKind::Let {
                    name,
                    annotation,
                    expr,
                } => {
                    let expected = annotation
                        .as_deref()
                        .map(|annotation| core_type(&parse_type_or_unknown(Some(annotation))));
                    let lowered = self.lower_expr(expr, expected.as_ref());
                    let ty = expected.unwrap_or_else(|| lowered.ty.clone());
                    let name = name.clone().unwrap_or_else(|| "<missing>".to_string());
                    self.bindings.push(CoreBinding {
                        name: name.clone(),
                        ty: ty.clone(),
                    });
                    body.push(CoreStmt {
                        node_id: line.node_id,
                        kind: CoreStmtKind::Let {
                            name,
                            ty,
                            expr: lowered,
                        },
                        span: line.span.clone(),
                    });
                }
                BodyLineKind::Expr { expr } => {
                    let is_tail = index + 1 == self.function.body.len();
                    let expected = is_tail.then_some(return_type);
                    let lowered = self.lower_expr(expr, expected);
                    body.push(CoreStmt {
                        node_id: line.node_id,
                        kind: if is_tail {
                            CoreStmtKind::Return { expr: lowered }
                        } else {
                            CoreStmtKind::Expr { expr: lowered }
                        },
                        span: line.span.clone(),
                    });
                }
            }
        }
        body
    }

    fn lower_expr(&mut self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        match &expr.kind {
            ExprKind::Missing => {
                self.blockers.push(CoreBlocker::MissingExpression {
                    node_id: expr.node_id,
                });
                self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing)
            }
            ExprKind::Hole { name, .. } => {
                self.blockers.push(CoreBlocker::Hole {
                    node_id: expr.node_id,
                });
                self.core_expr(
                    expr,
                    expected.cloned().unwrap_or(CoreType::Unknown),
                    CoreExprKind::Hole {
                        label: name.clone(),
                    },
                )
            }
            ExprKind::NamePath(segments) => self.lower_name_path(expr, segments, expected),
            ExprKind::StringLiteral(value) => self.core_expr(
                expr,
                CoreType::string(),
                CoreExprKind::StringLiteral(value.clone()),
            ),
            ExprKind::IntLiteral(value) => self.core_expr(
                expr,
                CoreType::int(),
                CoreExprKind::IntLiteral(value.clone()),
            ),
            ExprKind::FloatLiteral(value) => self.core_expr(
                expr,
                CoreType::float(),
                CoreExprKind::FloatLiteral(value.clone()),
            ),
            ExprKind::Unit => self.core_expr(expr, CoreType::unit(), CoreExprKind::Unit),
            ExprKind::Call { callee, args } => self.lower_call(expr, callee, args, expected),
            ExprKind::FieldAccess { base, field, .. } => self.lower_field_access(expr, base, field),
            ExprKind::Try(inner) => self.lower_try(expr, inner, expected),
            ExprKind::Record(fields) => self.lower_record(expr, fields, expected),
            ExprKind::List(items) => self.lower_list(expr, items, expected),
            ExprKind::Prefix { op, expr: inner } => {
                let expected_operand = match op {
                    veln_ast::PrefixOp::Not => CoreType::bool(),
                    veln_ast::PrefixOp::Negate => CoreType::int(),
                };
                let lowered = self.lower_expr(inner, Some(&expected_operand));
                self.core_expr(
                    expr,
                    expected_operand,
                    CoreExprKind::Prefix {
                        op: *op,
                        expr: Box::new(lowered),
                    },
                )
            }
            ExprKind::Binary { op, left, right } => {
                let (operand, result) = match op {
                    BinaryOp::Or | BinaryOp::And => (CoreType::bool(), CoreType::bool()),
                    BinaryOp::Equal | BinaryOp::NotEqual => (CoreType::Unknown, CoreType::bool()),
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => (CoreType::int(), CoreType::bool()),
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                        (CoreType::int(), CoreType::int())
                    }
                    BinaryOp::PipeGreater => (CoreType::Unknown, CoreType::Unknown),
                };
                let left = self.lower_expr(left, Some(&operand));
                let right = self.lower_expr(right, Some(&operand));
                self.core_expr(
                    expr,
                    result,
                    CoreExprKind::Binary {
                        op: *op,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                )
            }
        }
    }

    fn lower_name_path(
        &self,
        expr: &Expr,
        segments: &[String],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        match segments {
            [name] if name == "true" => {
                self.core_expr(expr, CoreType::bool(), CoreExprKind::BoolLiteral(true))
            }
            [name] if name == "false" => {
                self.core_expr(expr, CoreType::bool(), CoreExprKind::BoolLiteral(false))
            }
            [name] if name == "None" => self.core_expr(
                expr,
                expected
                    .filter(|expected| expected.option_part().is_some())
                    .cloned()
                    .unwrap_or_else(|| CoreType::option(CoreType::Unknown)),
                CoreExprKind::OptionNone,
            ),
            [name] => {
                let ty = self
                    .bindings
                    .iter()
                    .rev()
                    .find(|binding| binding.name == *name)
                    .map_or(CoreType::Unknown, |binding| binding.ty.clone());
                self.core_expr(expr, ty, CoreExprKind::Local(name.clone()))
            }
            _ => self.core_expr(
                expr,
                CoreType::Unknown,
                CoreExprKind::Local(segments.join("::")),
            ),
        }
    }

    fn lower_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        if let ExprKind::NamePath(segments) = &callee.kind {
            if matches!(segments.as_slice(), [name] if name == "Ok") {
                return self.lower_result_constructor(expr, args, expected, true);
            }
            if matches!(segments.as_slice(), [name] if name == "Err") {
                return self.lower_result_constructor(expr, args, expected, false);
            }
            if matches!(segments.as_slice(), [name] if name == "Some") {
                return self.lower_option_constructor(expr, args, expected);
            }
        }

        let signature = self.core_call_signature(callee, expected);
        if let Some(signature) = &signature {
            if args.len() != signature.params.len() {
                self.blockers.push(CoreBlocker::UnsupportedExpression {
                    node_id: expr.node_id,
                    reason: "call_arity_mismatch".to_string(),
                });
            }
        }
        let lowered_args = args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                let expected = signature
                    .as_ref()
                    .and_then(|signature| signature.params.get(index));
                self.lower_expr(arg, expected)
            })
            .collect();
        let (target, return_type) = signature.map_or_else(
            || {
                let symbol = callee_symbol(callee).unwrap_or_else(|| "<unknown>".to_string());
                (CoreCallTarget::Unresolved(symbol), CoreType::Unknown)
            },
            |signature| (signature.target, signature.return_type),
        );

        self.core_expr(
            expr,
            return_type,
            CoreExprKind::Call {
                target,
                args: lowered_args,
            },
        )
    }

    fn lower_result_constructor(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
        is_ok: bool,
    ) -> CoreExpr {
        if args.len() != 1 {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: expr.node_id,
                reason: "result_constructor_arity_mismatch".to_string(),
            });
        }
        let (value_type, error_type) = expected
            .and_then(CoreType::result_parts)
            .map_or((CoreType::Unknown, CoreType::Unknown), |(value, error)| {
                (value.clone(), error.clone())
            });
        let arg_expected = if is_ok { &value_type } else { &error_type };
        let first = args
            .first()
            .map(|arg| self.lower_expr(arg, Some(arg_expected)))
            .unwrap_or_else(|| {
                self.blockers.push(CoreBlocker::MissingExpression {
                    node_id: expr.node_id,
                });
                self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing)
            });
        let ty = if expected.and_then(CoreType::result_parts).is_some() {
            CoreType::result(value_type, error_type)
        } else if is_ok {
            CoreType::result(first.ty.clone(), CoreType::Unknown)
        } else {
            CoreType::result(CoreType::Unknown, first.ty.clone())
        };
        for arg in args.iter().skip(1) {
            self.lower_expr(arg, None);
        }
        self.core_expr(
            expr,
            ty,
            if is_ok {
                CoreExprKind::ResultOk(Box::new(first))
            } else {
                CoreExprKind::ResultErr(Box::new(first))
            },
        )
    }

    fn lower_option_constructor(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        if args.len() != 1 {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: expr.node_id,
                reason: "option_constructor_arity_mismatch".to_string(),
            });
        }
        let value_type = expected
            .and_then(CoreType::option_part)
            .cloned()
            .unwrap_or(CoreType::Unknown);
        let first = args
            .first()
            .map(|arg| self.lower_expr(arg, Some(&value_type)))
            .unwrap_or_else(|| {
                self.blockers.push(CoreBlocker::MissingExpression {
                    node_id: expr.node_id,
                });
                self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing)
            });
        for arg in args.iter().skip(1) {
            self.lower_expr(arg, None);
        }
        let ty = if value_type == CoreType::Unknown {
            CoreType::option(first.ty.clone())
        } else {
            CoreType::option(value_type)
        };
        self.core_expr(expr, ty, CoreExprKind::OptionSome(Box::new(first)))
    }

    fn lower_field_access(&mut self, expr: &Expr, base: &Expr, field: &str) -> CoreExpr {
        let base = self.lower_expr(base, None);
        let ty = base
            .ty
            .record_field(field)
            .cloned()
            .unwrap_or(CoreType::Unknown);
        self.core_expr(
            expr,
            ty,
            CoreExprKind::FieldAccess {
                base: Box::new(base),
                field: field.to_string(),
            },
        )
    }

    fn lower_try(&mut self, expr: &Expr, inner: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        let return_result = self
            .function
            .return_type
            .as_deref()
            .and_then(|return_type| parse_type_annotation(return_type).ok())
            .map(|ty| core_type(&ty))
            .and_then(|ty| {
                ty.result_parts()
                    .map(|(value, error)| (value.clone(), error.clone()))
            });
        let (value_type, error_type) = match (expected, return_result) {
            (Some(expected), Some((_, error))) => (expected.clone(), error),
            (Some(expected), None) => (expected.clone(), CoreType::Unknown),
            (None, Some((value, error))) => (value, error),
            (None, None) => (CoreType::Unknown, CoreType::Unknown),
        };
        let inner_expected = CoreType::result(value_type.clone(), error_type);
        let inner = self.lower_expr(inner, Some(&inner_expected));
        self.core_expr(expr, value_type, CoreExprKind::Try(Box::new(inner)))
    }

    fn lower_record(
        &mut self,
        expr: &Expr,
        fields: &[RecordField],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let fields = fields
            .iter()
            .map(|field| {
                let expected = expected.and_then(|expected| expected.record_field(&field.name));
                let expr = self.lower_expr(&field.expr, expected);
                CoreRecordField {
                    node_id: field.node_id,
                    name: field.name.clone(),
                    span: field.span.clone(),
                    expr,
                }
            })
            .collect::<Vec<_>>();
        let ty = expected.cloned().unwrap_or_else(|| {
            CoreType::Record(
                fields
                    .iter()
                    .map(|field| (field.name.clone(), field.expr.ty.clone()))
                    .collect(),
            )
        });
        self.core_expr(expr, ty, CoreExprKind::Record(fields))
    }

    fn lower_list(&mut self, expr: &Expr, items: &[Expr], expected: Option<&CoreType>) -> CoreExpr {
        let item_expected = expected.and_then(CoreType::list_part).cloned();
        let items = items
            .iter()
            .map(|item| self.lower_expr(item, item_expected.as_ref()))
            .collect::<Vec<_>>();
        let item_type = item_expected.unwrap_or_else(|| {
            items
                .first()
                .map_or(CoreType::Unknown, |item| item.ty.clone())
        });
        self.core_expr(expr, CoreType::list(item_type), CoreExprKind::List(items))
    }

    fn core_call_signature(
        &self,
        callee: &Expr,
        expected: Option<&CoreType>,
    ) -> Option<CoreCallSignature> {
        let ExprKind::NamePath(segments) = &callee.kind else {
            return None;
        };
        if stdio_signature(segments, callee).is_some() {
            return Some(CoreCallSignature {
                target: CoreCallTarget::StdioBuiltin(segments.join("::")),
                params: vec![CoreType::string()],
                return_type: CoreType::unit(),
            });
        }
        let name = segments.last()?;
        if let Some(function) = self.environment.function(name) {
            return Some(CoreCallSignature {
                target: CoreCallTarget::Function(function.name.clone()),
                params: function.params.iter().map(core_type).collect(),
                return_type: core_type(&function.return_type),
            });
        }
        let binding = self
            .bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name);
        if let Some(binding) = binding {
            if let CoreType::Function {
                params,
                return_type,
                ..
            } = &binding.ty
            {
                return Some(CoreCallSignature {
                    target: CoreCallTarget::Value(name.clone()),
                    params: params.clone(),
                    return_type: return_type.as_ref().clone(),
                });
            }
        }
        if let [name] = segments.as_slice() {
            if let Some((target, params, return_type)) = core_prelude_signature(name, expected) {
                return Some(CoreCallSignature {
                    target,
                    params,
                    return_type,
                });
            }
        }
        None
    }

    fn core_expr(&self, expr: &Expr, ty: CoreType, kind: CoreExprKind) -> CoreExpr {
        CoreExpr {
            node_id: expr.node_id,
            ty,
            kind,
            span: expr.span.clone(),
        }
    }
}

struct CoreCallSignature {
    target: CoreCallTarget,
    params: Vec<CoreType>,
    return_type: CoreType,
}

fn callee_symbol(callee: &Expr) -> Option<String> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some(segments.join("::")),
        _ => None,
    }
}
