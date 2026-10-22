use super::*;

struct LetBindingContext {
    type_origin: Option<TypeOrigin>,
    omitted_annotation: bool,
    initializer_failed: bool,
    diagnosed_unknown: bool,
    deferred_diagnostic: Option<usize>,
    pattern_failed: bool,
    alias_group: Option<usize>,
    record_root: Option<usize>,
    stable_place: Option<usize>,
    schema_encode_callable: bool,
}

type LetInitializer = (Type, Option<TypeOrigin>, bool, bool, Option<usize>);

fn repair(message: &str, span: &SourceSpan) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        ("message", JsonValue::string(message)),
        ("span", span_json(span)),
    ])
}

impl<'a> FunctionChecker<'a> {
    pub(super) fn check_body_line(&mut self, index: usize, line: &BodyLine) {
        if let Some(expr) = self.check_statement_line(line) {
            self.check_expr_line(index, line, expr);
        }
    }

    fn check_statement_line<'line>(&mut self, line: &'line BodyLine) -> Option<&'line Expr> {
        match &line.kind {
            BodyLineKind::Let {
                pattern,
                annotation,
                annotation_structure,
                expr,
            } => {
                self.check_let_line(
                    line,
                    pattern,
                    annotation.as_deref(),
                    &annotation_structure.variant_refinements,
                    &annotation_structure.paths,
                    expr,
                );
                None
            }
            BodyLineKind::Expr { expr } => Some(expr),
            BodyLineKind::Defer {
                body,
                keyword_span,
                block_span,
            } => {
                self.check_defer_line(line, body, keyword_span, block_span);
                None
            }
        }
    }

    fn check_defer_line(
        &mut self,
        line: &BodyLine,
        body: &[BodyLine],
        keyword_span: &SourceSpan,
        block_span: &SourceSpan,
    ) {
        if let Some(containing_block_span) = self.defer_blocks.last().cloned() {
            self.push_defer_restriction_diagnostic(DeferRestrictionDiagnostic {
                id: "defer.nested",
                message: "deferred block cannot register another deferred block".to_string(),
                node_id: line.node_id.display("defer"),
                span: keyword_span.clone(),
                reason: "nested_defer",
                repair: "Move the nested `defer` to a cleanup-region body.",
                repair_span: containing_block_span,
            });
        }

        self.defer_blocks.push(block_span.clone());
        self.defer_capture_boundaries.push(self.bindings.len());
        let actual = self.infer_scoped_body(body, None);
        self.defer_capture_boundaries
            .pop()
            .expect("defer capture boundary");
        self.defer_blocks.pop();

        if actual != Type::Unknown && !is_assignable(&Type::unit(), &actual) {
            let result_span = body
                .last()
                .and_then(|line| match &line.kind {
                    BodyLineKind::Expr { expr } => Some(expr.span.clone()),
                    BodyLineKind::Let { .. } | BodyLineKind::Defer { .. } => None,
                })
                .unwrap_or_else(|| block_span.clone());
            self.push_defer_restriction_diagnostic(DeferRestrictionDiagnostic {
                id: "defer.non_unit",
                message: format!(
                    "deferred block must have type `()`, but found `{}`",
                    actual.render()
                ),
                node_id: line.node_id.display("defer"),
                span: result_span,
                reason: "non_unit_result",
                repair: "End the deferred block with `()` so cleanup cannot replace the region value.",
                repair_span: block_span.clone(),
            });
        }
    }

    pub(super) fn infer_begin_body(
        &mut self,
        body: &[BodyLine],
        expected: Option<&ExpectedType>,
    ) -> Type {
        self.infer_scoped_body(body, expected)
    }

    fn infer_scoped_body(&mut self, body: &[BodyLine], expected: Option<&ExpectedType>) -> Type {
        let saved_bindings = self.bindings.len();
        let saved_invalid_binding_recoveries = self.invalid_binding_recoveries.len();
        let saved_omitted_bindings = self.omitted_local_bindings.len();
        self.local_name_scopes.push(Vec::new());

        let mut result = Type::unit();
        for (index, line) in body.iter().enumerate() {
            if let Some(expr) = self.check_statement_line(line) {
                let tail_expected = (index + 1 == body.len()).then_some(expected).flatten();
                let actual = self.infer_expr(expr, tail_expected);
                if index + 1 == body.len() {
                    result = actual;
                }
            }
        }

        self.check_omitted_local_inference_from(saved_omitted_bindings);
        self.omitted_local_bindings.truncate(saved_omitted_bindings);
        self.truncate_bindings(saved_bindings);
        self.invalid_binding_recoveries
            .truncate(saved_invalid_binding_recoveries);
        for (name, previous) in self
            .local_name_scopes
            .pop()
            .expect("scoped body name frame")
            .into_iter()
            .rev()
        {
            if let Some(previous) = previous {
                self.local_names.insert(name, previous);
            } else {
                self.local_names.remove(&name);
            }
        }
        result
    }

    pub(super) fn push_defer_restriction_diagnostic(&mut self, input: DeferRestrictionDiagnostic) {
        let mut diagnostic = Diagnostic::new(
            input.id,
            Severity::Error,
            DiagnosticKind::Type,
            input.message,
            Some(input.span),
            JsonValue::object([
                ("phase", JsonValue::string("type_check")),
                ("node_id", JsonValue::string(input.node_id)),
                ("boundary", JsonValue::string("deferred_block")),
                ("reason", JsonValue::string(input.reason)),
            ]),
        );
        let hint = repair(input.repair, &input.repair_span);
        diagnostic.related.push(hint);
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn check_let_line(
        &mut self,
        line: &BodyLine,
        pattern: &Pattern,
        annotation: Option<&str>,
        annotation_refinements: &[veln_ast::VariantRefinementType],
        annotation_paths: &[veln_ast::TypePathSegments],
        expr: &Expr,
    ) {
        let (transparent_alias_group, stable_record_root, stable_place) =
            self.inherited_let_binding_metadata(pattern, expr);
        let (binding_type, type_origin, initializer_failed, diagnosed_unknown, deferred_diagnostic) =
            self.infer_let_initializer(
                line,
                annotation,
                annotation_refinements,
                annotation_paths,
                expr,
            );
        let pattern_diagnostic_count = self.diagnostics.len();
        self.check_let_pattern_supported(pattern);
        let pattern_bindings = self.let_pattern_bindings(pattern, &binding_type);
        let binding_context = LetBindingContext {
            type_origin,
            omitted_annotation: annotation.is_none(),
            initializer_failed,
            diagnosed_unknown,
            deferred_diagnostic,
            pattern_failed: self.diagnostics.len() != pattern_diagnostic_count,
            alias_group: transparent_alias_group,
            record_root: stable_record_root,
            stable_place,
            schema_encode_callable: self.expr_is_schema_encode_callable(expr),
        };
        for binding in pattern_bindings {
            self.bind_let_pattern(binding, &binding_context);
        }
    }

    fn infer_let_initializer(
        &mut self,
        line: &BodyLine,
        annotation: Option<&str>,
        annotation_refinements: &[veln_ast::VariantRefinementType],
        annotation_paths: &[veln_ast::TypePathSegments],
        expr: &Expr,
    ) -> LetInitializer {
        let expected = annotation.and_then(|annotation| {
            self.parse_annotation(
                annotation,
                (annotation_refinements, annotation_paths),
                line.node_id,
                &line.span,
                ExpectedTypeSource::LocalAnnotation,
                "Type annotation declared here.",
            )
        });
        let initializer_diagnostic_count = self.diagnostics.len();
        let actual = self.infer_expr(expr, expected.as_ref());
        let diagnosed_unknown = type_contains_unknown(&actual)
            && (self.diagnostics[initializer_diagnostic_count..]
                .iter()
                .any(|diagnostic| diagnostic.id == "name.callsite_requires_modifier")
                || self.expr_is_diagnosed_unknown_reference(expr));
        let deferred_diagnostic = annotation
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
        let initializer_failed = self.diagnostics.len() != initializer_diagnostic_count;
        let binding_type = expected
            .as_ref()
            .map_or_else(|| actual.clone(), |expected| expected.ty.clone());
        (
            binding_type,
            expected.as_ref().and_then(|expected| {
                expected.origin_span.as_ref().map(|span| TypeOrigin {
                    node_id: expected.origin_node_id,
                    span: span.clone(),
                    source: expected.source,
                    message: expected.origin_message,
                })
            }),
            initializer_failed,
            diagnosed_unknown,
            deferred_diagnostic,
        )
    }

    fn bind_let_pattern(&mut self, binding: PatternBinding, context: &LetBindingContext) {
        if !valid_value_binding_name(&binding.name) {
            self.push_invalid_binding_recovery(binding);
            return;
        }
        if !self.declare_local_name(
            &binding.name,
            binding.node_id.display("pattern"),
            binding.span.clone(),
            "local binding",
            true,
        ) {
            return;
        }
        let admitted = self.admitted_let_binding(&binding, context);
        self.push_binding(admitted);
        if context.omitted_annotation
            && (!context.initializer_failed || context.deferred_diagnostic.is_some())
            && !context.pattern_failed
            && type_contains_unknown(&binding.ty)
        {
            self.omitted_local_bindings.push(OmittedLocalBinding {
                name: binding.name,
                node_id: binding.node_id,
                span: binding.span,
                deferred_initializer_diagnostic: context.deferred_diagnostic,
            });
        }
    }

    fn admitted_let_binding(
        &mut self,
        binding: &PatternBinding,
        context: &LetBindingContext,
    ) -> Binding {
        let initializer_is_reliable = !context.initializer_failed && !context.pattern_failed;
        let mut admitted = if context.diagnosed_unknown {
            Binding::diagnosed_unknown(binding.name.clone(), binding.ty.clone())
        } else {
            Binding::new(binding.name.clone(), binding.ty.clone())
        };
        admitted.stable_record_root = context.record_root.filter(|_| initializer_is_reliable);
        admitted.stable_place = context.stable_place.filter(|_| initializer_is_reliable);
        admitted.transparent_alias_group = if admitted.stable_place.is_some() {
            None
        } else {
            context
                .alias_group
                .filter(|_| initializer_is_reliable)
                .or_else(|| self.fresh_transparent_alias_group(binding.ty.clone()))
        };
        if matches!(binding.ty, Type::Function { .. })
            && let Some(type_origin) = &context.type_origin
        {
            admitted.type_origin = Some(type_origin.clone());
        }
        admitted.schema_encode_callable = matches!(binding.ty, Type::Function { .. })
            && context.schema_encode_callable
            && initializer_is_reliable;
        admitted
    }

    fn expr_is_schema_encode_callable(&self, expr: &Expr) -> bool {
        let ExprKind::NamePath { segments, .. } = &expr.kind else {
            return false;
        };
        if let [name] = segments.as_slice()
            && let Some(index) = self.visible_binding_index(name)
        {
            return self.bindings[index].schema_encode_callable;
        }
        let function = match segments.as_slice() {
            [name] => self
                .environment
                .unqualified_function(name, self.function.module_name.as_deref())
                .found(),
            _ => self
                .environment
                .function_path(segments, self.function.module_name.as_deref()),
        };
        function.is_some_and(|function| {
            function
                .target_name
                .starts_with(SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX)
        })
    }

    fn inherited_let_binding_metadata(
        &self,
        pattern: &Pattern,
        expr: &Expr,
    ) -> (Option<usize>, Option<usize>, Option<usize>) {
        if !matches!(pattern.kind, PatternKind::Binding(_)) {
            return (None, None, None);
        }
        let ExprKind::NamePath { segments, .. } = &expr.kind else {
            return (None, None, None);
        };
        let [name] = segments.as_slice() else {
            return (None, None, None);
        };
        self.visible_binding_index(name)
            .map_or((None, None, None), |index| {
                let binding = &self.bindings[index];
                (
                    binding.transparent_alias_group,
                    binding.stable_record_root,
                    binding.stable_place,
                )
            })
    }

    pub(super) fn check_expr_line(&mut self, index: usize, line: &BodyLine, expr: &Expr) {
        let expected = self.return_expected(line.node_id);
        let diagnostic_count = self.diagnostics.len();
        let actual = self.infer_expr(expr, expected.as_ref());
        if index + 1 != self.function.body.len() {
            return;
        }
        self.inferred_return_type = Some(actual.clone());
        self.inferred_return_unknown_is_diagnosed = type_contains_unknown(&actual)
            && (self.diagnostics[diagnostic_count..]
                .iter()
                .any(|diagnostic| diagnostic.id == "name.callsite_requires_modifier")
                || self.expr_is_diagnosed_unknown_reference(expr));
        if let Some(expected) = &expected {
            self.check_assignable(expr, &expected.ty, &actual, expected, "return_value");
        }
    }

    fn expr_is_diagnosed_unknown_reference(&self, expr: &Expr) -> bool {
        let ExprKind::NamePath { segments, .. } = &expr.kind else {
            return false;
        };
        let [name] = segments.as_slice() else {
            return false;
        };
        self.bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
            .is_some_and(|binding| binding.is_diagnosed_unknown)
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
            .enumerate()
            .rev()
            .find(|(_, binding)| binding.name == param.name)
            .map(|(index, _)| self.binding_type(index))
            .unwrap_or(Type::Unknown);
        if !type_contains_unknown(&inferred) {
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
        let hint = repair("Add a parameter type annotation.", &param.span);
        diagnostic.related.push(hint);
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn check_private_return_inference(&mut self) {
        let inferred = self.inferred_return_type.as_ref().unwrap_or(&Type::Unknown);
        if !type_contains_unknown(inferred) {
            return;
        }
        if self.inferred_return_unknown_is_diagnosed {
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
        let hint = repair("Add a return type annotation.", &self.function.span);
        diagnostic.related.push(hint);
        self.diagnostics.push(diagnostic);
    }

    pub(super) fn check_omitted_local_inference_complete(&mut self) {
        self.check_omitted_local_inference_from(0);
    }

    fn check_omitted_local_inference_from(&mut self, start: usize) {
        for omitted in &self.omitted_local_bindings[start..] {
            let inferred = self
                .bindings
                .iter()
                .enumerate()
                .rev()
                .find(|(_, binding)| binding.name == omitted.name)
                .map(|(index, _)| self.binding_type(index))
                .unwrap_or(Type::Unknown);
            if !type_contains_unknown(&inferred) {
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
            diagnostic.related.push(repair(
                "Add a type annotation or a later same-function use that fixes the type.",
                &omitted.span,
            ));
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
        if let Some(span) = &function.callsite {
            self.local_names.insert(
                "callsite".to_string(),
                (function.node_id.display("callsite"), span.clone()),
            );
            self.push_binding(Binding::new(
                "callsite".to_string(),
                Type::source_location(),
            ));
        }
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
