use super::*;

impl<'a> FunctionChecker<'a> {
    pub(super) fn check_body_line(&mut self, index: usize, line: &BodyLine) {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                expr,
            } => self.check_let_line(line, pattern, annotation.as_deref(), expr),
            BodyLineKind::Expr { expr } => self.check_expr_line(index, line, expr),
        }
    }

    pub(super) fn check_let_line(
        &mut self,
        line: &BodyLine,
        pattern: &Pattern,
        annotation: Option<&str>,
        expr: &Expr,
    ) {
        let expected = annotation.and_then(|annotation| {
            self.parse_annotation(
                annotation,
                line.node_id,
                &line.span,
                ExpectedTypeSource::LocalAnnotation,
                "Type annotation declared here.",
            )
        });
        let initializer_diagnostic_count = self.diagnostics.len();
        let actual = self.infer_expr(expr, expected.as_ref());
        let initializer_has_diagnostic = self.diagnostics.len() != initializer_diagnostic_count;
        let deferred_initializer_diagnostic = annotation
            .is_none()
            .then(|| {
                self.deferred_ambiguous_initializer_diagnostic(
                    initializer_diagnostic_count,
                    expr,
                    &actual,
                )
            })
            .flatten();
        if let Some(expected) = &expected {
            self.check_assignable(expr, &expected.ty, &actual, expected, "assignable");
        }

        let pattern_diagnostic_count = self.diagnostics.len();
        self.check_let_pattern_supported(pattern);
        let binding_type = expected
            .as_ref()
            .map_or_else(|| actual.clone(), |expected| expected.ty.clone());
        let pattern_bindings = self.let_pattern_bindings(pattern, &binding_type);
        let pattern_has_diagnostic = self.diagnostics.len() != pattern_diagnostic_count;
        for binding in pattern_bindings {
            self.bind_let_pattern(
                binding,
                annotation.is_none(),
                initializer_has_diagnostic,
                deferred_initializer_diagnostic,
                pattern_has_diagnostic,
            );
        }
    }

    pub(super) fn bind_let_pattern(
        &mut self,
        binding: PatternBinding,
        annotation_is_omitted: bool,
        initializer_has_diagnostic: bool,
        deferred_initializer_diagnostic: Option<usize>,
        pattern_has_diagnostic: bool,
    ) {
        if !valid_value_binding_name(&binding.name) {
            self.push_invalid_binding_recovery(binding);
            return;
        }
        if !self.declare_local_name(
            &binding.name,
            binding.node_id.display("pattern"),
            binding.span.clone(),
            "local binding",
        ) {
            return;
        }
        self.bindings
            .push(Binding::new(binding.name.clone(), binding.ty.clone()));
        if annotation_is_omitted
            && (!initializer_has_diagnostic || deferred_initializer_diagnostic.is_some())
            && !pattern_has_diagnostic
            && type_contains_unknown(&binding.ty)
        {
            self.omitted_local_bindings.push(OmittedLocalBinding {
                name: binding.name,
                node_id: binding.node_id,
                span: binding.span,
                deferred_initializer_diagnostic,
            });
        }
    }

    pub(super) fn check_expr_line(&mut self, index: usize, line: &BodyLine, expr: &Expr) {
        let expected = self.return_expected(line.node_id);
        let actual = self.infer_expr(expr, expected.as_ref());
        if index + 1 != self.function.body.len() {
            return;
        }
        self.inferred_return_type = Some(actual.clone());
        if let Some(expected) = &expected {
            self.check_assignable(expr, &expected.ty, &actual, expected, "return_value");
        }
    }

    pub(super) fn deferred_ambiguous_initializer_diagnostic(
        &self,
        start_index: usize,
        expr: &Expr,
        actual: &Type,
    ) -> Option<usize> {
        if !type_contains_unknown(actual) || self.diagnostics.len() != start_index + 1 {
            return None;
        }
        let diagnostic = self.diagnostics.get(start_index)?;
        if diagnostic.id == "type.inference_ambiguous"
            && diagnostic.span.as_ref() == Some(&expr.span)
            && json_string_field_is(&diagnostic.details, "slot_kind", "constructor_type")
        {
            Some(start_index)
        } else {
            None
        }
    }

    pub(super) fn remove_suppressed_diagnostics(&mut self) {
        if self.suppressed_diagnostic_indices.is_empty() {
            return;
        }
        self.diagnostics = std::mem::take(&mut self.diagnostics)
            .into_iter()
            .enumerate()
            .filter_map(|(index, diagnostic)| {
                (!self.suppressed_diagnostic_indices.contains(&index)).then_some(diagnostic)
            })
            .collect();
    }

    pub(super) fn check_implicit_unit_return(&mut self) {
        if matches!(
            self.function.body.last().map(|line| &line.kind),
            Some(BodyLineKind::Expr { .. })
        ) {
            return;
        }
        let Some(expected) = self.return_expected(self.function.node_id) else {
            self.inferred_return_type = Some(Type::unit());
            return;
        };
        let actual = Type::unit();
        if is_assignable(&expected.ty, &actual) {
            return;
        }
        self.diagnostics.push(Diagnostic::new(
            "type.mismatch",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "expected `{}`, but found `{}`",
                expected.ty.render(),
                actual.render()
            ),
            Some(self.function.span.clone()),
            type_details(
                self.function.node_id.display("fn"),
                expected.ty.render(),
                actual.render(),
                expected.source.as_type_source(),
                "implicit_unit",
                "return_value",
                [
                    self.function.node_id.display("fn"),
                    expected.origin_node_id.display("fn"),
                ],
            ),
        ));
    }

    pub(super) fn check_private_inference_complete(&mut self) {
        if self.function.visibility == Visibility::Public
            || self.function.kind != FunctionKind::Function
        {
            return;
        }
        let function = self.function;
        for param in &function.params {
            self.check_private_parameter_inference(param);
        }
        if self.function.return_type.is_some() {
            return;
        }
        self.check_private_return_inference();
    }

    pub(super) fn check_private_parameter_inference(&mut self, param: &Param) {
        if !parameter_annotation_is_omitted(param) {
            return;
        }
        let inferred = self
            .bindings
            .iter()
            .rev()
            .find(|binding| binding.name == param.name)
            .map(|binding| &binding.ty)
            .unwrap_or(&Type::Unknown);
        if !type_contains_unknown(inferred) {
            return;
        }
        let mut diagnostic = Diagnostic::new(
            "type.private_inference_incomplete",
            Severity::Error,
            DiagnosticKind::Type,
            format!("private parameter `{}` has no inferred type", param.name),
            Some(param.span.clone()),
            JsonValue::object([
                ("phase", JsonValue::string("type_check")),
                ("node_id", JsonValue::string(param.node_id.display("param"))),
                ("boundary", JsonValue::string("private_function")),
                ("slot_kind", JsonValue::string("private_parameter")),
                ("parameter", JsonValue::string(param.name.clone())),
                ("missing_fact", JsonValue::string("parameter_type")),
                ("inferred_type", JsonValue::string(inferred.render())),
            ]),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("repair_hint")),
            (
                "message",
                JsonValue::string("Add a parameter type annotation."),
            ),
            ("span", span_json(&param.span)),
        ]));
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn check_private_return_inference(&mut self) {
        let inferred = self.inferred_return_type.as_ref().unwrap_or(&Type::Unknown);
        if !type_contains_unknown(inferred) {
            return;
        }
        let mut diagnostic = Diagnostic::new(
            "type.private_inference_incomplete",
            Severity::Error,
            DiagnosticKind::Type,
            "private function has no inferred return type",
            Some(self.function.span.clone()),
            JsonValue::object([
                ("phase", JsonValue::string("type_check")),
                (
                    "node_id",
                    JsonValue::string(self.function.node_id.display("fn")),
                ),
                ("boundary", JsonValue::string("private_function")),
                ("slot_kind", JsonValue::string("private_return")),
                ("missing_fact", JsonValue::string("return_type")),
                ("inferred_type", JsonValue::string(inferred.render())),
            ]),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("repair_hint")),
            (
                "message",
                JsonValue::string("Add a return type annotation."),
            ),
            ("span", span_json(&self.function.span)),
        ]));
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn check_omitted_local_inference_complete(&mut self) {
        for omitted in &self.omitted_local_bindings {
            let inferred = self
                .bindings
                .iter()
                .rev()
                .find(|binding| binding.name == omitted.name)
                .map(|binding| &binding.ty)
                .unwrap_or(&Type::Unknown);
            if !type_contains_unknown(inferred) {
                if let Some(index) = omitted.deferred_initializer_diagnostic {
                    self.suppressed_diagnostic_indices.insert(index);
                }
                continue;
            }
            if omitted.deferred_initializer_diagnostic.is_some() {
                continue;
            }
            let mut diagnostic = Diagnostic::new(
                "type.local_inference_incomplete",
                Severity::Error,
                DiagnosticKind::Type,
                format!(
                    "omitted local binding `{}` has no concrete inferred type",
                    omitted.name
                ),
                Some(omitted.span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("type_check")),
                    (
                        "node_id",
                        JsonValue::string(omitted.node_id.display("pattern")),
                    ),
                    ("slot_kind", JsonValue::string("local_binding")),
                    ("binding", JsonValue::string(omitted.name.clone())),
                    ("inferred_type", JsonValue::string(inferred.render())),
                ]),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("repair_hint")),
                (
                    "message",
                    JsonValue::string(
                        "Add a type annotation or a later same-function use that fixes the type.",
                    ),
                ),
                ("span", span_json(&omitted.span)),
            ]));
            self.diagnostics.push(diagnostic);
        }
    }

    pub(super) fn check_let_pattern_supported(&mut self, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Wildcard | PatternKind::Binding(_) => {}
            PatternKind::Record(fields) => {
                for field in fields {
                    self.check_let_pattern_supported(&field.pattern);
                }
            }
            PatternKind::Constructor { args, .. } => {
                for arg in args {
                    self.check_let_pattern_supported(arg);
                }
            }
            PatternKind::StringLiteral(_)
            | PatternKind::IntLiteral(_)
            | PatternKind::FloatLiteral(_)
            | PatternKind::BoolLiteral(_)
            | PatternKind::Unit => {
                let mut diagnostic = Diagnostic::new(
                    "pattern.refutable_let",
                    Severity::Error,
                    DiagnosticKind::Type,
                    "refutable let pattern is not supported",
                    Some(pattern.span.clone()),
                    JsonValue::object([
                        ("phase", JsonValue::string("type_check")),
                        (
                            "node_id",
                            JsonValue::string(pattern.node_id.display("pattern")),
                        ),
                    ]),
                );
                diagnostic.related.push(JsonValue::object([
                    ("kind", JsonValue::string("let_pattern")),
                    (
                        "message",
                        JsonValue::string(
                            "Use a binding, wildcard, record pattern, or constructor pattern in a let statement.",
                        ),
                    ),
                    ("span", span_json(&pattern.span)),
                ]));
                self.diagnostics.push(diagnostic);
            }
        }
    }

    pub(super) fn check_function_annotations(&mut self) {
        let function = self.function;
        let variadic_count = self
            .function
            .params
            .iter()
            .filter(|param| param.is_variadic)
            .count();
        let signature = self.environment.function_for(function);
        for (index, param) in function.params.iter().enumerate() {
            self.check_parameter_annotation(param, index, variadic_count, signature);
        }

        self.check_return_annotation();
        self.check_result_binding_name();
    }
}
