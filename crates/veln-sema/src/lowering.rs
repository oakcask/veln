use veln_ast::{
    BinaryOp, BodyLineKind, DictEntry, Expr, ExprKind, Function, MatchArm, Pattern, PatternKind,
    RecordField, SurfaceModule,
};
use veln_core::{
    CheckedProgram, ContractObligationStatus, CoreBlocker, CoreCallTarget, CoreContract,
    CoreDictEntry, CoreExpr, CoreExprKind, CoreFunction, CoreMatchArm, CoreParam, CorePattern,
    CorePatternField, CorePatternKind, CoreReadiness, CoreRecordField, CoreStmt, CoreStmtKind,
    CoreType,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};

use crate::contracts::contract_predicate_is_statically_true;
use crate::effects::{
    core_concurrency_signature, core_standard_library_signature, is_concurrency_call,
    standard_library_origin, stdio_signature,
};
use crate::prelude::{
    core_prelude_signature, float_arithmetic_prelude_name, float_comparison_prelude_name,
    float_prefix_prelude_name,
};
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
    diagnostics: Vec<Diagnostic>,
    generated_local_count: usize,
}

pub(crate) struct CoreLoweringOutput {
    pub(crate) program: CheckedProgram,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

pub(crate) fn lower_surface_module_to_core(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> CoreLoweringOutput {
    let mut blockers = Vec::new();
    let mut diagnostics = Vec::new();
    let functions = module
        .functions
        .iter()
        .map(|function| {
            let mut lowerer = CoreLowerer::new(function, environment);
            let lowered = lowerer.lower_function();
            blockers.extend(lowerer.blockers);
            diagnostics.extend(lowerer.diagnostics);
            lowered
        })
        .collect();
    CoreLoweringOutput {
        program: CheckedProgram {
            functions,
            readiness: if blockers.is_empty() {
                CoreReadiness::Complete
            } else {
                CoreReadiness::Blocked(blockers)
            },
        },
        diagnostics,
    }
}

impl<'a> CoreLowerer<'a> {
    fn new(function: &'a Function, environment: &'a TypeEnvironment) -> Self {
        Self {
            function,
            environment,
            bindings: Vec::new(),
            blockers: Vec::new(),
            diagnostics: Vec::new(),
            generated_local_count: 0,
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
                obligation_status: if contract_predicate_is_statically_true(&contract.text) {
                    ContractObligationStatus::StaticallyProven
                } else {
                    ContractObligationStatus::RuntimeRequired
                },
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
            return_binding: self
                .function
                .return_binding
                .as_ref()
                .map(|binding| binding.name.clone()),
            return_type,
            effects: self.function.effects.clone().unwrap_or_default(),
            contracts,
            body,
            span: self.function.span.clone(),
        }
    }

    fn unsupported_expression(
        &mut self,
        expr: &Expr,
        reason: &'static str,
        message: String,
        extra_details: Option<JsonValue>,
    ) {
        self.blockers.push(CoreBlocker::UnsupportedExpression {
            node_id: expr.node_id,
            reason: reason.to_string(),
        });
        let mut details = vec![
            ("phase", JsonValue::string("core_lowering")),
            ("node_id", JsonValue::string(expr.node_id.display("expr"))),
            ("reason", JsonValue::string(reason)),
        ];
        if let Some(extra_details) = extra_details {
            details.push(("facts", extra_details));
        }
        self.diagnostics.push(Diagnostic::new(
            format!("core.{reason}"),
            Severity::Error,
            DiagnosticKind::Type,
            message,
            Some(expr.span.clone()),
            JsonValue::object(details),
        ));
    }

    fn missing_expression(
        &mut self,
        expr: &Expr,
        expected: Option<&CoreType>,
        reason: &'static str,
    ) {
        self.blockers.push(CoreBlocker::MissingExpression {
            node_id: expr.node_id,
        });
        let mut details = vec![
            ("phase", JsonValue::string("core_lowering")),
            ("node_id", JsonValue::string(expr.node_id.display("expr"))),
            ("reason", JsonValue::string(reason)),
        ];
        if let Some(expected) = expected {
            details.push((
                "expected_type",
                JsonValue::string(render_core_type(expected)),
            ));
        }
        self.diagnostics.push(Diagnostic::new(
            "core.missing_expression",
            Severity::Error,
            DiagnosticKind::Type,
            "expression is missing",
            Some(expr.span.clone()),
            JsonValue::object(details),
        ));
    }

    fn lower_body(&mut self, return_type: &CoreType) -> Vec<CoreStmt> {
        let mut body = Vec::new();
        let mut has_tail_expression = false;
        for (index, line) in self.function.body.iter().enumerate() {
            match &line.kind {
                BodyLineKind::Let {
                    pattern,
                    annotation,
                    expr,
                } => {
                    let expected = annotation
                        .as_deref()
                        .map(|annotation| core_type(&parse_type_or_unknown(Some(annotation))));
                    let lowered = self.lower_expr(expr, expected.as_ref());
                    let ty = expected.unwrap_or_else(|| lowered.ty.clone());
                    self.lower_let_pattern(
                        line.node_id,
                        &line.span,
                        pattern,
                        lowered,
                        ty,
                        &mut body,
                    );
                }
                BodyLineKind::Expr { expr } => {
                    let is_tail = index + 1 == self.function.body.len();
                    has_tail_expression = is_tail;
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
        if !has_tail_expression {
            body.push(CoreStmt {
                node_id: self.function.node_id,
                kind: CoreStmtKind::Return {
                    expr: CoreExpr {
                        node_id: self.function.node_id,
                        ty: CoreType::unit(),
                        kind: CoreExprKind::Unit,
                        span: self.function.span.clone(),
                    },
                },
                span: self.function.span.clone(),
            });
        }
        body
    }

    fn lower_let_pattern(
        &mut self,
        node_id: veln_ast::NodeId,
        span: &veln_source::SourceSpan,
        pattern: &Pattern,
        expr: CoreExpr,
        ty: CoreType,
        body: &mut Vec<CoreStmt>,
    ) {
        match &pattern.kind {
            PatternKind::Binding(name) => {
                self.bindings.push(CoreBinding {
                    name: name.clone(),
                    ty: ty.clone(),
                });
                body.push(CoreStmt {
                    node_id,
                    kind: CoreStmtKind::Let {
                        name: name.clone(),
                        ty,
                        expr,
                    },
                    span: span.clone(),
                });
            }
            PatternKind::Wildcard => {
                body.push(CoreStmt {
                    node_id,
                    kind: CoreStmtKind::Expr { expr },
                    span: span.clone(),
                });
            }
            PatternKind::Record(_) => {
                let temp_name = self.generated_pattern_local();
                body.push(CoreStmt {
                    node_id,
                    kind: CoreStmtKind::Let {
                        name: temp_name.clone(),
                        ty: ty.clone(),
                        expr,
                    },
                    span: span.clone(),
                });
                let base = CoreExpr {
                    node_id,
                    ty: ty.clone(),
                    kind: CoreExprKind::Local(temp_name),
                    span: span.clone(),
                };
                self.lower_pattern_bindings(pattern, base, &ty, body);
            }
            PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit
            | PatternKind::Constructor { .. } => {
                body.push(CoreStmt {
                    node_id,
                    kind: CoreStmtKind::Expr { expr },
                    span: span.clone(),
                });
            }
        }
    }

    fn lower_pattern_bindings(
        &mut self,
        pattern: &Pattern,
        value: CoreExpr,
        ty: &CoreType,
        body: &mut Vec<CoreStmt>,
    ) {
        match &pattern.kind {
            PatternKind::Binding(name) => {
                self.bindings.push(CoreBinding {
                    name: name.clone(),
                    ty: ty.clone(),
                });
                body.push(CoreStmt {
                    node_id: pattern.node_id,
                    kind: CoreStmtKind::Let {
                        name: name.clone(),
                        ty: ty.clone(),
                        expr: value,
                    },
                    span: pattern.span.clone(),
                });
            }
            PatternKind::Record(fields) => {
                for field in fields {
                    let field_ty = ty
                        .record_field(&field.name)
                        .cloned()
                        .unwrap_or(CoreType::Unknown);
                    let field_value = CoreExpr {
                        node_id: field.node_id,
                        ty: field_ty.clone(),
                        kind: CoreExprKind::FieldAccess {
                            base: Box::new(value.clone()),
                            field: field.name.clone(),
                        },
                        span: field.span.clone(),
                    };
                    self.lower_pattern_bindings(&field.pattern, field_value, &field_ty, body);
                }
            }
            PatternKind::Wildcard
            | PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit
            | PatternKind::Constructor { .. } => {}
        }
    }

    fn generated_pattern_local(&mut self) -> String {
        let name = format!("$pattern{}", self.generated_local_count);
        self.generated_local_count += 1;
        name
    }

    fn lower_expr(&mut self, expr: &Expr, expected: Option<&CoreType>) -> CoreExpr {
        match &expr.kind {
            ExprKind::Missing => {
                self.missing_expression(expr, expected, "missing_expression");
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
            ExprKind::BoolLiteral(value) => {
                self.core_expr(expr, CoreType::bool(), CoreExprKind::BoolLiteral(*value))
            }
            ExprKind::Unit => self.core_expr(expr, CoreType::unit(), CoreExprKind::Unit),
            ExprKind::TypeApply { .. } => {
                self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing)
            }
            ExprKind::Call { callee, args } => self.lower_call(expr, callee, args, expected),
            ExprKind::FieldAccess { base, field, .. } => self.lower_field_access(expr, base, field),
            ExprKind::Try(inner) => self.lower_try(expr, inner, expected),
            ExprKind::Record(fields) => self.lower_record(expr, fields, expected),
            ExprKind::Dict(entries) => self.lower_dict(expr, entries, expected),
            ExprKind::List(items) => self.lower_list(expr, items, expected),
            ExprKind::Match { scrutinee, arms } => {
                self.lower_match(expr, scrutinee, arms, expected)
            }
            ExprKind::Prefix { op, expr: inner } => {
                let expected_operand = match op {
                    veln_ast::PrefixOp::Not => CoreType::bool(),
                    veln_ast::PrefixOp::Negate => self.numeric_operand_type(expected, &[inner]),
                };
                if expected_operand == CoreType::float()
                    && let Some(name) = float_prefix_prelude_name(*op)
                {
                    let arg = self.lower_expr(inner, Some(&CoreType::float()));
                    return self.core_expr(
                        expr,
                        CoreType::float(),
                        CoreExprKind::Call {
                            target: CoreCallTarget::PreludeBuiltin(name.to_string()),
                            args: vec![arg],
                        },
                    );
                }
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
                if *op == BinaryOp::PipeGreater {
                    return self.lower_pipeline(expr, left, right, expected);
                }

                let numeric_type = if is_ordering_op(*op) {
                    self.numeric_operand_type(None, &[left, right])
                } else {
                    self.numeric_operand_type(expected, &[left, right])
                };
                if numeric_type == CoreType::float() {
                    if let Some(name) = float_comparison_prelude_name(*op) {
                        let left = self.lower_expr(left, Some(&CoreType::float()));
                        let right = self.lower_expr(right, Some(&CoreType::float()));
                        return self.core_expr(
                            expr,
                            CoreType::bool(),
                            CoreExprKind::Call {
                                target: CoreCallTarget::PreludeBuiltin(name.to_string()),
                                args: vec![left, right],
                            },
                        );
                    }
                    if let Some(name) = float_arithmetic_prelude_name(*op) {
                        let left = self.lower_expr(left, Some(&CoreType::float()));
                        let right = self.lower_expr(right, Some(&CoreType::float()));
                        return self.core_expr(
                            expr,
                            CoreType::float(),
                            CoreExprKind::Call {
                                target: CoreCallTarget::PreludeBuiltin(name.to_string()),
                                args: vec![left, right],
                            },
                        );
                    }
                }
                let (operand, result) = match op {
                    BinaryOp::Or | BinaryOp::And => (CoreType::bool(), CoreType::bool()),
                    BinaryOp::Equal | BinaryOp::NotEqual => (CoreType::Unknown, CoreType::bool()),
                    BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => (numeric_type, CoreType::bool()),
                    BinaryOp::Add | BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide => {
                        (numeric_type.clone(), numeric_type)
                    }
                    BinaryOp::PipeGreater => {
                        unreachable!("pipeline handled before binary lowering")
                    }
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

    fn lower_pipeline(
        &mut self,
        expr: &Expr,
        left: &Expr,
        right: &Expr,
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let ExprKind::Call { callee, args } = &right.kind else {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: right.node_id,
                reason: "pipeline_target_not_call".to_string(),
            });
            self.lower_expr(left, None);
            self.lower_expr(right, None);
            return self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing);
        };
        if !matches!(callee.kind, ExprKind::NamePath(_)) {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: right.node_id,
                reason: "pipeline_target_not_named_call".to_string(),
            });
            self.lower_expr(left, None);
            self.lower_expr(right, expected);
            return self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing);
        }

        let mut piped_args = Vec::with_capacity(args.len() + 1);
        piped_args.push(left.clone());
        piped_args.extend(args.iter().cloned());
        self.lower_call(expr, callee, &piped_args, expected)
    }

    fn numeric_operand_type(&self, expected: Option<&CoreType>, operands: &[&Expr]) -> CoreType {
        if expected.is_some_and(|expected| expected == &CoreType::float()) {
            return CoreType::float();
        }
        if operands.iter().any(|expr| {
            self.shallow_expr_type(expr)
                .is_some_and(|ty| ty == CoreType::float())
        }) {
            return CoreType::float();
        }
        CoreType::int()
    }

    fn shallow_expr_type(&self, expr: &Expr) -> Option<CoreType> {
        match &expr.kind {
            ExprKind::IntLiteral(_) => Some(CoreType::int()),
            ExprKind::FloatLiteral(_) => Some(CoreType::float()),
            ExprKind::BoolLiteral(_) => Some(CoreType::bool()),
            ExprKind::NamePath(segments) => match segments.as_slice() {
                [name] => self
                    .bindings
                    .iter()
                    .rev()
                    .find(|binding| binding.name == *name)
                    .map(|binding| binding.ty.clone())
                    .or_else(|| {
                        self.environment
                            .function(name)
                            .map(|function| core_type(&function.ty()))
                    }),
                _ => self
                    .environment
                    .function_path(segments)
                    .map(|function| core_type(&function.ty())),
            },
            ExprKind::Call { callee, .. } => self
                .core_call_signature(callee, None)
                .map(|signature| signature.return_type),
            _ => None,
        }
    }

    fn lower_name_path(
        &self,
        expr: &Expr,
        segments: &[String],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        match segments {
            segments if is_option_none_constructor(segments) => self.core_expr(
                expr,
                expected
                    .filter(|expected| expected.option_part().is_some())
                    .cloned()
                    .unwrap_or_else(|| CoreType::option(CoreType::Unknown)),
                CoreExprKind::OptionNone,
            ),
            [name] => {
                if let Some(binding) = self
                    .bindings
                    .iter()
                    .rev()
                    .find(|binding| binding.name == *name)
                {
                    self.core_expr(expr, binding.ty.clone(), CoreExprKind::Local(name.clone()))
                } else if let Some(function) = self.environment.function(name) {
                    self.core_expr(
                        expr,
                        core_type(&function.ty()),
                        CoreExprKind::FunctionValue(name.clone()),
                    )
                } else {
                    self.core_expr(expr, CoreType::Unknown, CoreExprKind::Local(name.clone()))
                }
            }
            _ => {
                if let Some(function) = self.environment.function_path(segments) {
                    self.core_expr(
                        expr,
                        core_type(&function.ty()),
                        CoreExprKind::FunctionValue(function.name.clone()),
                    )
                } else {
                    self.core_expr(
                        expr,
                        CoreType::Unknown,
                        CoreExprKind::Local(segments.join("::")),
                    )
                }
            }
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
            if let Some(is_ok) = result_constructor_kind(segments) {
                return self.lower_result_constructor(expr, args, expected, is_ok);
            }
            if is_option_some_constructor(segments) {
                return self.lower_option_constructor(expr, args, expected);
            }
            if is_concurrency_call(segments) {
                let handle_type = args.first().and_then(|arg| self.shallow_expr_type(arg));
                let signature =
                    core_concurrency_signature(segments, expected, handle_type.as_ref(), None);
                if let Some((params, _)) = &signature
                    && args.len() != params.len()
                {
                    self.unsupported_expression(
                        expr,
                        "call_arity_mismatch",
                        format!(
                            "call expects {} argument(s), but got {}",
                            params.len(),
                            args.len()
                        ),
                        Some(JsonValue::object([
                            (
                                "expected_argument_count",
                                JsonValue::Number(params.len() as i64),
                            ),
                            (
                                "actual_argument_count",
                                JsonValue::Number(args.len() as i64),
                            ),
                        ])),
                    );
                }
                let lowered_args = args
                    .iter()
                    .enumerate()
                    .map(|(index, arg)| {
                        let expected = signature.as_ref().and_then(|(params, _)| params.get(index));
                        self.lower_expr(arg, expected)
                    })
                    .collect();
                return self.core_expr(
                    expr,
                    signature
                        .map(|(_, return_type)| return_type)
                        .unwrap_or(CoreType::Unknown),
                    CoreExprKind::Call {
                        target: CoreCallTarget::ConcurrencyBuiltin(segments.join("::")),
                        args: lowered_args,
                    },
                );
            }
        }
        if let Some((segments, type_args)) = callee_name_path_and_type_args(callee)
            && is_concurrency_call(segments)
            && matches!(callee.kind, ExprKind::TypeApply { .. })
        {
            let explicit_item = type_args
                .and_then(|type_args| type_args.first())
                .and_then(|type_arg| parse_type_annotation(type_arg).ok())
                .map(|ty| core_type(&ty));
            let signature =
                core_concurrency_signature(segments, expected, None, explicit_item.as_ref());
            if let Some((params, _)) = &signature
                && args.len() != params.len()
            {
                self.unsupported_expression(
                    expr,
                    "call_arity_mismatch",
                    format!(
                        "call expects {} argument(s), but got {}",
                        params.len(),
                        args.len()
                    ),
                    Some(JsonValue::object([
                        (
                            "expected_argument_count",
                            JsonValue::Number(params.len() as i64),
                        ),
                        (
                            "actual_argument_count",
                            JsonValue::Number(args.len() as i64),
                        ),
                    ])),
                );
            }
            let lowered_args = args
                .iter()
                .enumerate()
                .map(|(index, arg)| {
                    let expected = signature.as_ref().and_then(|(params, _)| params.get(index));
                    self.lower_expr(arg, expected)
                })
                .collect();
            return self.core_expr(
                expr,
                signature
                    .map(|(_, return_type)| return_type)
                    .unwrap_or(CoreType::Unknown),
                CoreExprKind::Call {
                    target: CoreCallTarget::ConcurrencyBuiltin(segments.join("::")),
                    args: lowered_args,
                },
            );
        }

        let signature = self.core_call_signature(callee, expected);
        if let Some(signature) = &signature
            && args.len() != signature.params.len()
        {
            self.unsupported_expression(
                expr,
                "call_arity_mismatch",
                format!(
                    "call expects {} argument(s), but got {}",
                    signature.params.len(),
                    args.len()
                ),
                Some(JsonValue::object([
                    (
                        "expected_argument_count",
                        JsonValue::Number(signature.params.len() as i64),
                    ),
                    (
                        "actual_argument_count",
                        JsonValue::Number(args.len() as i64),
                    ),
                ])),
            );
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
            self.unsupported_expression(
                expr,
                "result_constructor_arity_mismatch",
                format!(
                    "result constructor expects 1 argument, but got {}",
                    args.len()
                ),
                Some(JsonValue::object([
                    ("expected_argument_count", JsonValue::Number(1)),
                    (
                        "actual_argument_count",
                        JsonValue::Number(args.len() as i64),
                    ),
                ])),
            );
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
                self.missing_expression(expr, Some(arg_expected), "missing_constructor_argument");
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
            self.unsupported_expression(
                expr,
                "option_constructor_arity_mismatch",
                format!(
                    "option constructor expects 1 argument, but got {}",
                    args.len()
                ),
                Some(JsonValue::object([
                    ("expected_argument_count", JsonValue::Number(1)),
                    (
                        "actual_argument_count",
                        JsonValue::Number(args.len() as i64),
                    ),
                ])),
            );
        }
        let value_type = expected
            .and_then(CoreType::option_part)
            .cloned()
            .unwrap_or(CoreType::Unknown);
        let first = args
            .first()
            .map(|arg| self.lower_expr(arg, Some(&value_type)))
            .unwrap_or_else(|| {
                self.missing_expression(expr, Some(&value_type), "missing_constructor_argument");
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

    fn lower_dict(
        &mut self,
        expr: &Expr,
        entries: &[DictEntry],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let (key_expected, value_expected) = expected
            .and_then(CoreType::dict_parts)
            .map_or((None, None), |(key, value)| (Some(key), Some(value)));
        let entries = entries
            .iter()
            .map(|entry| CoreDictEntry {
                node_id: entry.node_id,
                key: self.lower_expr(&entry.key, key_expected),
                value: self.lower_expr(&entry.value, value_expected),
                span: entry.span.clone(),
            })
            .collect::<Vec<_>>();
        let ty = expected.cloned().unwrap_or_else(|| {
            let key_type = entries
                .first()
                .map_or(CoreType::Unknown, |entry| entry.key.ty.clone());
            let value_type = entries
                .first()
                .map_or(CoreType::Unknown, |entry| entry.value.ty.clone());
            CoreType::dict(key_type, value_type)
        });
        self.core_expr(expr, ty, CoreExprKind::Dict(entries))
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

    fn lower_match(
        &mut self,
        expr: &Expr,
        scrutinee: &Expr,
        arms: &[MatchArm],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let scrutinee = self.lower_expr(scrutinee, None);
        let mut result_type = expected.cloned().unwrap_or(CoreType::Unknown);
        let mut lowered_arms = Vec::new();
        if arms.is_empty() {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: expr.node_id,
                reason: "empty_match".to_string(),
            });
        }
        for arm in arms {
            let saved_bindings = self.bindings.len();
            for binding in Self::pattern_bindings(&arm.pattern, &scrutinee.ty) {
                self.bindings.push(binding);
            }
            let arm_expected = if result_type == CoreType::Unknown {
                None
            } else {
                Some(&result_type)
            };
            let lowered_expr = self.lower_expr(&arm.expr, arm_expected);
            if result_type == CoreType::Unknown {
                result_type = lowered_expr.ty.clone();
            }
            lowered_arms.push(CoreMatchArm {
                node_id: arm.node_id,
                pattern: Self::lower_pattern(&arm.pattern),
                expr: lowered_expr,
                span: arm.span.clone(),
            });
            self.bindings.truncate(saved_bindings);
        }
        self.core_expr(
            expr,
            result_type,
            CoreExprKind::Match {
                scrutinee: Box::new(scrutinee),
                arms: lowered_arms,
            },
        )
    }

    fn pattern_bindings(pattern: &Pattern, scrutinee_type: &CoreType) -> Vec<CoreBinding> {
        match &pattern.kind {
            PatternKind::Wildcard
            | PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => Vec::new(),
            PatternKind::Binding(name) => vec![CoreBinding {
                name: name.clone(),
                ty: scrutinee_type.clone(),
            }],
            PatternKind::Record(fields) => fields
                .iter()
                .flat_map(|field| {
                    let field_type = scrutinee_type
                        .record_field(&field.name)
                        .unwrap_or(&CoreType::Unknown);
                    Self::pattern_bindings(&field.pattern, field_type)
                })
                .collect(),
            PatternKind::Constructor { name, args } if is_option_some_constructor(name) => {
                scrutinee_type
                    .option_part()
                    .zip(args.first())
                    .map_or_else(Vec::new, |(inner, pattern)| {
                        Self::pattern_bindings(pattern, inner)
                    })
            }
            PatternKind::Constructor { name, args } => match result_constructor_kind(name) {
                Some(true) => scrutinee_type
                    .result_parts()
                    .zip(args.first())
                    .map_or_else(Vec::new, |((value, _), pattern)| {
                        Self::pattern_bindings(pattern, value)
                    }),
                Some(false) => scrutinee_type
                    .result_parts()
                    .zip(args.first())
                    .map_or_else(Vec::new, |((_, error), pattern)| {
                        Self::pattern_bindings(pattern, error)
                    }),
                None => Vec::new(),
            },
        }
    }

    fn lower_pattern(pattern: &Pattern) -> CorePattern {
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
                    fields
                        .iter()
                        .map(|field| CorePatternField {
                            node_id: field.node_id,
                            name: field.name.clone(),
                            pattern: Self::lower_pattern(&field.pattern),
                            span: field.span.clone(),
                        })
                        .collect(),
                ),
                PatternKind::Constructor { name, args } => CorePatternKind::Constructor {
                    name: name.clone(),
                    args: args.iter().map(Self::lower_pattern).collect(),
                },
            },
            span: pattern.span.clone(),
        }
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
        if is_concurrency_call(segments) {
            let (params, return_type) = core_concurrency_signature(segments, expected, None, None)?;
            return Some(CoreCallSignature {
                target: CoreCallTarget::ConcurrencyBuiltin(segments.join("::")),
                params,
                return_type,
            });
        }
        if standard_library_origin(segments, callee).is_some() {
            let (params, return_type) = core_standard_library_signature(segments)?;
            return Some(CoreCallSignature {
                target: CoreCallTarget::StandardLibraryBuiltin(segments.join("::")),
                params,
                return_type,
            });
        }
        if let [name] = segments.as_slice()
            && let Some(binding) = self
                .bindings
                .iter()
                .rev()
                .find(|binding| binding.name == *name)
        {
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
            return None;
        }
        if let Some(function) = self.environment.function_path(segments) {
            return Some(CoreCallSignature {
                target: CoreCallTarget::Function(function.name.clone()),
                params: function.params.iter().map(core_type).collect(),
                return_type: core_type(&function.return_type),
            });
        }
        if let [name] = segments.as_slice()
            && let Some((target, params, return_type)) = core_prelude_signature(name, expected)
        {
            return Some(CoreCallSignature {
                target,
                params,
                return_type,
            });
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
        ExprKind::TypeApply { callee, .. } => callee_symbol(callee),
        _ => None,
    }
}

fn callee_name_path_and_type_args(callee: &Expr) -> Option<(&[String], Option<&[String]>)> {
    match &callee.kind {
        ExprKind::NamePath(segments) => Some((segments, None)),
        ExprKind::TypeApply { callee, type_args } => {
            let ExprKind::NamePath(segments) = &callee.kind else {
                return None;
            };
            Some((segments, Some(type_args.as_slice())))
        }
        _ => None,
    }
}

fn render_core_type(ty: &CoreType) -> String {
    match ty {
        CoreType::Unknown => "unknown".to_string(),
        CoreType::Named { name, args } if name == "Unit" && args.is_empty() => "()".to_string(),
        CoreType::Named { name, args } if args.is_empty() => name.clone(),
        CoreType::Named { name, args } => {
            let args = args
                .iter()
                .map(render_core_type)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{name}({args})")
        }
        CoreType::Record(fields) => {
            let fields = fields
                .iter()
                .map(|(name, ty)| format!("{name}: {}", render_core_type(ty)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{fields}}}")
        }
        CoreType::Function {
            params,
            return_type,
            effects,
        } => {
            let params = params
                .iter()
                .map(render_core_type)
                .collect::<Vec<_>>()
                .join(", ");
            let effects = if effects.is_empty() {
                String::new()
            } else {
                format!(" effects [{}]", effects.join(", "))
            };
            format!("fn({params}) -> {}{effects}", render_core_type(return_type))
        }
    }
}

fn result_constructor_kind(segments: &[String]) -> Option<bool> {
    match segments {
        [name] if name == "Ok" => Some(true),
        [type_name, name] if type_name == "Result" && name == "Ok" => Some(true),
        [name] if name == "Err" => Some(false),
        [type_name, name] if type_name == "Result" && name == "Err" => Some(false),
        _ => None,
    }
}

fn is_option_some_constructor(segments: &[String]) -> bool {
    matches!(segments, [name] if name == "Some")
        || matches!(segments, [type_name, name] if type_name == "Option" && name == "Some")
}

fn is_option_none_constructor(segments: &[String]) -> bool {
    matches!(segments, [name] if name == "None")
        || matches!(segments, [type_name, name] if type_name == "Option" && name == "None")
}

fn is_ordering_op(op: BinaryOp) -> bool {
    matches!(
        op,
        BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual
    )
}
