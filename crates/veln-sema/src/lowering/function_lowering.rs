use super::*;

impl<'a> CoreLowerer<'a> {
    pub(super) fn new(function: &'a Function, environment: &'a TypeEnvironment) -> Self {
        Self {
            function,
            environment,
            bindings: Vec::new(),
            blockers: Vec::new(),
            diagnostics: Vec::new(),
            generated_local_count: 0,
        }
    }

    pub(super) fn lower_function(&mut self) -> CoreFunction {
        let params = self.lower_params();
        let return_type = self.lower_return_type();
        let contracts = self.lower_contracts();
        let body = self.lower_body(&return_type);

        CoreFunction {
            node_id: self.function.node_id,
            name: self.lowered_function_name(),
            visibility: self.function.visibility,
            params,
            return_binding: self
                .function
                .return_binding
                .as_ref()
                .map(|binding| binding.name.clone()),
            return_type,
            effects: self.lower_effects(),
            contracts,
            body,
            span: self.function.span.clone(),
        }
    }

    pub(super) fn lower_params(&mut self) -> Vec<CoreParam> {
        let signature = self.environment.function_for(self.function);
        self.function
            .params
            .iter()
            .enumerate()
            .map(|(index, param)| {
                let mut ty = signature
                    .and_then(|function| function.params.get(index))
                    .map(core_type)
                    .unwrap_or_else(|| core_type(&parse_type_or_unknown(param.ty.as_deref())));
                if param.is_variadic {
                    ty = signature
                        .and_then(|function| function.variadic.as_ref())
                        .map(core_type)
                        .map(|ty| CoreType::named("List", vec![ty]))
                        .unwrap_or_else(|| CoreType::named("List", vec![ty]));
                }
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
            .collect()
    }

    pub(super) fn lower_return_type(&self) -> CoreType {
        self.environment
            .function_for(self.function)
            .map(|function| core_type(&function.return_type))
            .unwrap_or_else(|| {
                core_type(&parse_type_or_unknown(self.function.return_type.as_deref()))
            })
    }

    pub(super) fn lower_contracts(&self) -> Vec<CoreContract> {
        self.function
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
            .collect()
    }

    pub(super) fn lowered_function_name(&self) -> String {
        self.function.name.as_deref().map_or_else(
            || "<missing>".to_string(),
            |name| {
                if self.function.kind == veln_ast::FunctionKind::Test {
                    name.to_string()
                } else {
                    crate::standard_symbols::standard_function_link_name(
                        self.function.module_name.as_deref(),
                        name,
                    )
                }
            },
        )
    }

    pub(super) fn lower_effects(&self) -> Vec<String> {
        self.environment
            .function_for(self.function)
            .map(|function| function.effects.clone())
            .unwrap_or_else(|| self.function.effects.clone().unwrap_or_default())
    }

    pub(super) fn unsupported_expression(
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

    pub(super) fn missing_expression(
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

    pub(super) fn lower_body(&mut self, return_type: &CoreType) -> Vec<CoreStmt> {
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

    pub(super) fn lower_let_pattern(
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
                self.bind_pattern_value(node_id, span, name, ty, expr, body);
            }
            PatternKind::Wildcard => {
                body.push(CoreStmt {
                    node_id,
                    kind: CoreStmtKind::Expr { expr },
                    span: span.clone(),
                });
            }
            PatternKind::Record(_) | PatternKind::Constructor { .. } => {
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
            | PatternKind::Unit => {
                body.push(CoreStmt {
                    node_id,
                    kind: CoreStmtKind::Expr { expr },
                    span: span.clone(),
                });
            }
        }
    }

    pub(super) fn lower_pattern_bindings(
        &mut self,
        pattern: &Pattern,
        value: CoreExpr,
        ty: &CoreType,
        body: &mut Vec<CoreStmt>,
    ) {
        match &pattern.kind {
            PatternKind::Binding(name) => {
                self.bind_pattern_value(
                    pattern.node_id,
                    &pattern.span,
                    name,
                    ty.clone(),
                    value,
                    body,
                );
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
            PatternKind::Constructor { .. } => {
                for binding in self.pattern_bindings(pattern, ty) {
                    self.bindings.push(binding.clone());
                    body.push(CoreStmt {
                        node_id: pattern.node_id,
                        kind: CoreStmtKind::Let {
                            name: binding.name.clone(),
                            ty: binding.ty.clone(),
                            expr: self.lower_constructor_pattern_binding(
                                pattern,
                                value.clone(),
                                ty,
                                &binding,
                            ),
                        },
                        span: pattern.span.clone(),
                    });
                }
            }
            PatternKind::Wildcard
            | PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => {}
        }
    }

    fn bind_pattern_value(
        &mut self,
        node_id: veln_ast::NodeId,
        span: &veln_source::SourceSpan,
        name: &str,
        ty: CoreType,
        expr: CoreExpr,
        body: &mut Vec<CoreStmt>,
    ) {
        self.bindings.push(CoreBinding {
            name: name.to_string(),
            ty: ty.clone(),
        });
        body.push(CoreStmt {
            node_id,
            kind: CoreStmtKind::Let {
                name: name.to_string(),
                ty,
                expr,
            },
            span: span.clone(),
        });
    }

    pub(super) fn lower_constructor_pattern_binding(
        &self,
        pattern: &Pattern,
        value: CoreExpr,
        ty: &CoreType,
        binding: &CoreBinding,
    ) -> CoreExpr {
        CoreExpr {
            node_id: pattern.node_id,
            ty: binding.ty.clone(),
            kind: CoreExprKind::Match {
                scrutinee: Box::new(value),
                arms: vec![CoreMatchArm {
                    node_id: pattern.node_id,
                    pattern: self.lower_pattern(pattern, Some(ty)),
                    expr: CoreExpr {
                        node_id: pattern.node_id,
                        ty: binding.ty.clone(),
                        kind: CoreExprKind::Local(binding.name.clone()),
                        span: pattern.span.clone(),
                    },
                    span: pattern.span.clone(),
                }],
            },
            span: pattern.span.clone(),
        }
    }

    pub(super) fn generated_pattern_local(&mut self) -> String {
        let name = format!("$pattern{}", self.generated_local_count);
        self.generated_local_count += 1;
        name
    }
}
