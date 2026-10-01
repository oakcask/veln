use super::*;

impl<'a> CoreLowerer<'a> {
    pub(super) fn new(
        function: &'a Function,
        environment: &'a TypeEnvironment,
        block_unsupported_callsite_runtime: bool,
    ) -> Self {
        Self {
            function,
            environment,
            block_unsupported_callsite_runtime,
            bindings: Vec::new(),
            blockers: Vec::new(),
            diagnostics: Vec::new(),
            generated_local_count: 0,
            defer_capture_boundaries: Vec::new(),
            defer_captures: Vec::new(),
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
            body: CoreCleanupRegion::new(body),
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
                    .unwrap_or_else(|| {
                        param.ty.as_deref().map_or(CoreType::Unknown, |annotation| {
                            self.core_type_annotation(annotation)
                        })
                    });
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
                self.function
                    .return_type
                    .as_deref()
                    .map_or(CoreType::Unknown, |annotation| {
                        self.core_type_annotation(annotation)
                    })
            })
    }

    pub(super) fn lower_contracts(&mut self) -> Vec<CoreContract> {
        let contracts = self.function.contracts.clone();
        contracts
            .iter()
            .map(|contract| {
                if self.block_unsupported_callsite_runtime {
                    for span in &contract.callsite_reference_spans {
                        self.unsupported_callsite_reference(contract.node_id, span);
                    }
                }
                CoreContract {
                    node_id: contract.node_id,
                    kind: contract.kind,
                    predicate: contract.text.clone(),
                    obligation_status: if contract_predicate_is_statically_true(&contract.text) {
                        ContractObligationStatus::StaticallyProven
                    } else {
                        ContractObligationStatus::RuntimeRequired
                    },
                    span: contract.span.clone(),
                }
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

    pub(super) fn unsupported_callsite_reference(
        &mut self,
        node_id: veln_ast::NodeId,
        span: &veln_source::SourceSpan,
    ) {
        const REASON: &str = "callsite_runtime_unsupported";
        self.blockers.push(CoreBlocker::UnsupportedExpression {
            node_id,
            reason: REASON.to_string(),
        });
        let mut diagnostic = Diagnostic::new(
            "core.callsite_runtime_unsupported",
            Severity::Error,
            DiagnosticKind::Type,
            "`callsite` is not available during execution",
            Some(span.clone()),
            JsonValue::object([
                ("phase", JsonValue::string("core_lowering")),
                ("node_id", JsonValue::string(node_id.display("expr"))),
                ("reason", JsonValue::string(REASON)),
            ]),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("runtime_support")),
            (
                "message",
                JsonValue::string(
                    "Runtime support for call-site locations and their hidden call ABI is not implemented.",
                ),
            ),
        ]));
        self.diagnostics.push(diagnostic);
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
        let source_body = self.function.body.clone();
        self.lower_body_lines(&source_body, Some(return_type), false)
            .0
    }

    pub(super) fn lower_scoped_body(
        &mut self,
        source_body: &[BodyLine],
        expected: Option<&CoreType>,
    ) -> (Vec<CoreStmt>, CoreType) {
        self.lower_body_lines(source_body, expected, true)
    }

    fn lower_body_lines(
        &mut self,
        source_body: &[BodyLine],
        expected: Option<&CoreType>,
        scoped: bool,
    ) -> (Vec<CoreStmt>, CoreType) {
        let saved_bindings = self.bindings.len();
        let mut body = Vec::new();
        let mut has_tail_expression = false;
        let mut result_type = expected.cloned().unwrap_or_else(CoreType::unit);
        for (index, line) in source_body.iter().enumerate() {
            let is_tail = index + 1 == source_body.len();
            if let Some(tail_type) = self.lower_body_line(line, is_tail, expected, &mut body) {
                has_tail_expression = true;
                result_type = tail_type;
            }
        }
        if !has_tail_expression {
            body.push(CoreStmt {
                node_id: self.function.node_id,
                kind: CoreStmtKind::Return {
                    expr: CoreExpr {
                        node_id: self.function.node_id,
                        ty: result_type.clone(),
                        kind: CoreExprKind::Unit,
                        span: self.function.span.clone(),
                    },
                },
                span: self.function.span.clone(),
            });
        }
        if scoped {
            self.bindings.truncate(saved_bindings);
        }
        (body, result_type)
    }

    fn lower_body_line(
        &mut self,
        line: &BodyLine,
        is_tail: bool,
        expected: Option<&CoreType>,
        body: &mut Vec<CoreStmt>,
    ) -> Option<CoreType> {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
                ..
            } => {
                self.lower_body_let(line, pattern, annotation.as_deref(), expr, body);
                None
            }
            BodyLineKind::Expr { expr } => {
                self.lower_body_expr(line, expr, is_tail, expected, body)
            }
            BodyLineKind::Defer {
                body: deferred_body,
                ..
            } => {
                self.lower_body_defer(line, deferred_body, body);
                None
            }
        }
    }

    fn lower_body_let(
        &mut self,
        line: &BodyLine,
        pattern: &Pattern,
        annotation: Option<&str>,
        expr: &Expr,
        body: &mut Vec<CoreStmt>,
    ) {
        let expected = annotation.map(|annotation| self.core_type_annotation(annotation));
        let lowered = self.lower_expr(expr, expected.as_ref());
        let ty = expected.unwrap_or_else(|| lowered.ty.clone());
        self.lower_let_pattern(line.node_id, &line.span, pattern, lowered, ty, body);
    }

    fn lower_body_expr(
        &mut self,
        line: &BodyLine,
        expr: &Expr,
        is_tail: bool,
        expected: Option<&CoreType>,
        body: &mut Vec<CoreStmt>,
    ) -> Option<CoreType> {
        let tail_expected = is_tail.then_some(expected).flatten();
        let lowered = self.lower_expr(expr, tail_expected);
        let tail_type = is_tail.then(|| lowered.ty.clone());
        let kind = if is_tail {
            CoreStmtKind::Return { expr: lowered }
        } else {
            CoreStmtKind::Expr { expr: lowered }
        };
        body.push(CoreStmt {
            node_id: line.node_id,
            kind,
            span: line.span.clone(),
        });
        tail_type
    }

    fn lower_body_defer(
        &mut self,
        line: &BodyLine,
        deferred_body: &[BodyLine],
        body: &mut Vec<CoreStmt>,
    ) {
        self.defer_capture_boundaries.push(self.bindings.len());
        self.defer_captures.push(Vec::new());
        let (deferred_body, _) = self.lower_scoped_body(deferred_body, Some(&CoreType::unit()));
        let captures = self.defer_captures.pop().expect("defer capture frame");
        self.defer_capture_boundaries
            .pop()
            .expect("defer capture boundary");
        body.push(CoreStmt {
            node_id: line.node_id,
            kind: CoreStmtKind::Defer(CoreDeferredBlock {
                captures,
                body: deferred_body,
            }),
            span: line.span.clone(),
        });
    }

    pub(super) fn record_defer_capture(&mut self, index: usize) {
        let Some(boundary) = self.defer_capture_boundaries.last().copied() else {
            return;
        };
        if index >= boundary {
            return;
        }
        let binding = self.bindings[index].clone();
        let captures = self.defer_captures.last_mut().expect("defer capture frame");
        if let Some(capture) = captures
            .iter_mut()
            .find(|capture| capture.name == binding.name)
        {
            capture.ty = binding.ty;
        } else {
            captures.push(CoreDeferredCapture {
                name: binding.name,
                ty: binding.ty,
            });
        }
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
