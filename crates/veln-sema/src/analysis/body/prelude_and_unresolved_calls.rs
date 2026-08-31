use super::*;

impl<'a> FunctionChecker<'a> {
    pub(super) fn infer_prelude_call(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        let ExprKind::NamePath { segments, .. } = &callee.kind else {
            return None;
        };
        let (name, params, return_type) = if let [name] = segments.as_slice() {
            if self.bare_name_is_shadowed(name) || self.bare_prelude_import_is_ambiguous(name) {
                return None;
            }
            let input_type =
                prelude_input_arg(args, name).and_then(|arg| self.shallow_expr_type(arg));
            let (params, return_type) = prelude_signature_with_input(
                name,
                expected.map(|expected| &expected.ty),
                input_type.as_ref(),
            )?;
            (name.clone(), params, return_type)
        } else if let Some((name, params, return_type)) = segments.last().and_then(|name| {
            let input_type =
                prelude_input_arg(args, name).and_then(|arg| self.shallow_expr_type(arg));
            qualified_prelude_signature_with_input(
                segments,
                expected.map(|expected| &expected.ty),
                input_type.as_ref(),
            )
        }) {
            (name, params, return_type)
        } else {
            segments.last().and_then(|name| {
                let input_type =
                    prelude_input_arg(args, name).and_then(|arg| self.shallow_expr_type(arg));
                qualified_prelude_builtin_signature_with_input(
                    segments,
                    expected.map(|expected| &expected.ty),
                    input_type.as_ref(),
                )
            })?
        };

        if let Some(origin) = prelude_effect_origin(segments, callee) {
            for effect in &origin.effects {
                self.inferred_effects.push(EffectUse {
                    effect: effect.clone(),
                    node_id: callee.node_id,
                    span: callee.span.clone(),
                    kind: "direct_call",
                    symbol: origin.symbol.clone(),
                });
            }
        }

        for (index, arg) in args.iter().enumerate() {
            let Some(param_type) = params.get(index) else {
                self.infer_expr(arg, None);
                continue;
            };
            let expected = ExpectedType {
                ty: param_type.clone(),
                source: ExpectedTypeSource::Inferred,
                origin_node_id: callee.node_id,
                origin_span: Some(callee.span.clone()),
                origin_message: "Prelude helper parameter type inferred here.",
            };
            let actual = self.infer_expr(arg, Some(&expected));
            self.check_prelude_argument_assignable(&name, index, arg, &expected, &actual);
        }
        Some(return_type)
    }

    pub(super) fn diagnose_method_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
    ) -> Option<Type> {
        if let ExprKind::FieldAccess {
            base,
            field,
            field_span,
        } = &callee.kind
        {
            self.infer_expr(base, None);
            for arg in args {
                self.infer_expr(arg, None);
            }
            let mut diagnostic = Diagnostic::new(
                "type.method_call",
                Severity::Error,
                DiagnosticKind::Type,
                "method call syntax is not supported",
                Some(field_span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("type")),
                    ("node_id", JsonValue::string(expr.node_id.display("expr"))),
                    ("expected", JsonValue::string("function_call")),
                    ("actual", JsonValue::string("method_call")),
                    ("constraint", JsonValue::string("call_target")),
                    ("method", JsonValue::string(field.clone())),
                ]),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("call_style")),
                (
                    "message",
                    JsonValue::string(
                        "Use a named function call with the receiver as an explicit argument.",
                    ),
                ),
                ("span", span_json(&callee.span)),
            ]));
            self.diagnostics.push(diagnostic);
            return Some(Type::Unknown);
        }
        None
    }

    pub(super) fn infer_unresolved_call(&mut self, callee: &Expr, args: &[Expr]) -> Type {
        if let Some((segments, type_args)) = callee.callee_name_path_and_type_args()
            && !known_concurrency_type_arg_overflow(segments, type_args)
        {
            let recovered = matches!(segments, [name] if self
            .environment
                .local_call_recovery_candidate_count(
                    name,
                    self.function.module_name.as_deref(),
                    args.len(),
                )
                + self.invalid_local_callable_recovery_count(name)
                == 1);
            let recovered = recovered
                || self
                    .environment
                    .quarantined_import_call_recovery_candidate_count(
                        segments,
                        self.function.module_name.as_deref(),
                        args.len(),
                    )
                    == 1;
            let recovered = recovered
                || self
                    .environment
                    .quarantined_import_constructor_recovery_candidate_count(
                        segments,
                        self.function.module_name.as_deref(),
                        Some(args.len()),
                    )
                    == 1;
            let recovered = recovered
                || self
                    .environment
                    .has_invalid_path_segment_in_span(&callee.span);
            if !recovered {
                let symbol = segments.join("::");
                self.push_unresolved_name(
                    callee.node_id,
                    callee.span.clone(),
                    &symbol,
                    "call_target",
                );
            }
        }
        for arg in args {
            self.infer_expr(arg, None);
        }
        Type::Unknown
    }

    pub(super) fn infer_field_access(
        &mut self,
        expr: &Expr,
        base: &Expr,
        field: &str,
        field_span: &SourceSpan,
    ) -> Type {
        let base_type = self.infer_expr(base, None);
        if let Some(field_type) = base_type.record_field(field) {
            return field_type.clone();
        }
        if base_type == Type::Unknown {
            return Type::Unknown;
        }
        let mut diagnostic = Diagnostic::new(
            "type.field_missing",
            Severity::Error,
            DiagnosticKind::Type,
            format!("type `{}` has no field `{field}`", base_type.render()),
            Some(field_span.clone()),
            type_details(
                expr.node_id.display("expr"),
                format!("record field `{field}`"),
                base_type.render(),
                "field_access",
                "inferred_expression",
                "field_access",
                [
                    self.function.node_id.display("fn"),
                    base.node_id.display("expr"),
                ],
            ),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("field_base")),
            (
                "message",
                JsonValue::string(format!(
                    "Field access base has type `{}`.",
                    base_type.render()
                )),
            ),
            ("span", span_json(&base.span)),
        ]));
        self.diagnostics.push(diagnostic);
        Type::Unknown
    }

    pub(super) fn call_signature(
        &self,
        callee: &Expr,
        expected: Option<&Type>,
        handle_type: Option<&Type>,
        arg_count: Option<usize>,
    ) -> Option<(Vec<Type>, Option<Type>, Type, CallOrigin)> {
        let bindings = self
            .bindings
            .iter()
            .map(|binding| crate::call_resolution::TypeBinding {
                name: &binding.name,
                ty: &binding.ty,
            })
            .collect::<Vec<_>>();
        let signature = crate::call_resolution::type_call_signature(
            callee,
            expected,
            handle_type,
            arg_count,
            &bindings,
            self.environment,
            self.function.module_name.as_deref(),
        )?;
        Some((
            signature.params,
            signature.variadic,
            signature.return_type,
            signature.origin,
        ))
    }

    pub(super) fn bare_call_is_ambiguous(&self, callee: &Expr) -> bool {
        let ExprKind::NamePath { segments, .. } = &callee.kind else {
            return false;
        };
        let [name] = segments.as_slice() else {
            return false;
        };
        if self.bare_name_is_shadowed(name) {
            return false;
        }
        if self.bare_prelude_import_is_ambiguous(name) {
            return true;
        }
        matches!(
            self.environment
                .unqualified_function(name, self.function.module_name.as_deref()),
            FunctionLookup::Ambiguous
        )
    }

    pub(super) fn bare_name_is_shadowed(&self, name: &str) -> bool {
        self.bindings
            .iter()
            .rev()
            .any(|binding| binding.name == name)
            || matches!(
                self.environment
                    .unqualified_function(name, self.function.module_name.as_deref()),
                FunctionLookup::Found(function)
                    if function.module_name.as_deref() == self.function.module_name.as_deref()
            )
    }

    pub(super) fn invalid_local_binding_recovery_count(&self, name: &str) -> usize {
        self.invalid_binding_recoveries
            .iter()
            .filter(|recovery| recovery.name == name)
            .count()
    }

    pub(super) fn invalid_local_callable_recovery_count(&self, name: &str) -> usize {
        self.invalid_binding_recoveries
            .iter()
            .filter(|recovery| {
                recovery.name == name && matches!(recovery.ty, Type::Function { .. })
            })
            .count()
    }

    pub(super) fn push_invalid_binding_recovery(&mut self, binding: PatternBinding) {
        self.invalid_binding_recoveries
            .push(InvalidBindingRecovery {
                name: binding.name,
                ty: binding.ty,
            });
    }

    pub(super) fn bare_prelude_import_is_ambiguous(&self, name: &str) -> bool {
        let candidates = self
            .environment
            .unqualified_function_import_candidates(name, self.function.module_name.as_deref());
        let has_source_prelude = candidates
            .iter()
            .any(|candidate| candidate.module_name.as_deref() == Some("std::prelude"));
        if has_source_prelude {
            candidates
                .iter()
                .any(|candidate| candidate.module_name.as_deref() != Some("std::prelude"))
        } else {
            prelude_symbol(name).is_some() && !candidates.is_empty()
        }
    }

    pub(super) fn push_ambiguous_unqualified_function_import(
        &mut self,
        node_id: NodeId,
        span: SourceSpan,
        name: &str,
        namespace: &'static str,
    ) {
        let mut diagnostic = Diagnostic::new(
            "name.ambiguous",
            Severity::Error,
            DiagnosticKind::Name,
            format!("ambiguous {namespace} `{name}`"),
            Some(span),
            JsonValue::object([
                ("phase", JsonValue::string("name")),
                ("node_id", JsonValue::string(node_id.display("name"))),
                ("symbol", JsonValue::string(name)),
                ("namespace", JsonValue::string(namespace)),
                ("resolution_status", JsonValue::string("ambiguous")),
            ]),
        );
        for candidate in self
            .environment
            .unqualified_function_import_candidates(name, self.function.module_name.as_deref())
        {
            let Some(module_name) = candidate.module_name.as_deref() else {
                continue;
            };
            let Some(use_decl) = self.environment.uses.iter().find(|use_decl| {
                use_decl.name == module_name && use_decl.module_name == self.function.module_name
            }) else {
                continue;
            };
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("import_candidate")),
                (
                    "message",
                    JsonValue::string(format!(
                        "Imported module `{module_name}` exports `{name}`; use `{}::{name}` to select it.",
                        use_decl.alias
                    )),
                ),
                ("span", span_json(&use_decl.span)),
            ]));
        }
        let source_prelude_is_listed = self
            .environment
            .unqualified_function_import_candidates(name, self.function.module_name.as_deref())
            .iter()
            .any(|candidate| candidate.module_name.as_deref() == Some("std::prelude"));
        if prelude_symbol(name).is_some() && !source_prelude_is_listed {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("import_candidate")),
                (
                    "message",
                    JsonValue::string(format!(
                        "The standard prelude exports `{name}`; use `prelude::{name}` to select it.",
                    )),
                ),
            ]));
        }
        self.diagnostics.push(diagnostic);
    }
}
