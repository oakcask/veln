use super::*;

impl<'a> FunctionChecker<'a> {
    pub(in crate::analysis) fn infer_expr(
        &mut self,
        expr: &Expr,
        expected: Option<&ExpectedType>,
    ) -> Type {
        match &expr.kind {
            ExprKind::Missing => Type::Unknown,
            ExprKind::Hole { name, satisfy } => {
                if let Some(satisfy) = satisfy {
                    self.check_satisfy_clause(expr, satisfy, expected);
                }
                self.push_hole_diagnostic(expr, name.as_deref(), satisfy.as_ref(), expected);
                expected
                    .map(|expected| expected.ty.clone())
                    .unwrap_or(Type::Unknown)
            }
            ExprKind::NamePath(segments) => self.infer_name_path(segments, expr, expected),
            ExprKind::StringLiteral(_) => Type::string(),
            ExprKind::IntLiteral(_) => Type::int(),
            ExprKind::FloatLiteral(_) => Type::float(),
            ExprKind::BoolLiteral(_) => Type::bool(),
            ExprKind::Unit => Type::unit(),
            ExprKind::TypeApply { .. } => Type::Unknown,
            ExprKind::Call { callee, args } => self.infer_call(expr, callee, args, expected),
            ExprKind::Perform {
                effect,
                effect_span,
                operation,
                operation_span,
                args,
            } => self.infer_perform(expr, effect, effect_span, operation, operation_span, args),
            ExprKind::Handle {
                body,
                handler,
                handler_span,
                args,
            } => self.infer_handle(expr, body, handler, handler_span, args, expected),
            ExprKind::SchemaDecode {
                schema,
                input,
                base,
            } => self.infer_schema_decode(expr, schema, input, base),
            ExprKind::SchemaEncode { schema, value } => {
                self.infer_schema_encode(expr, schema, value)
            }
            ExprKind::FieldAccess {
                base,
                field,
                field_span,
            } => self.infer_field_access(expr, base, field, field_span),
            ExprKind::Try(inner) => self.infer_try(expr, inner, expected),
            ExprKind::Record(fields) => self.infer_record(expr, fields, expected),
            ExprKind::Dict(entries) => self.infer_dict(expr, entries, expected),
            ExprKind::List(items) => self.infer_list(expr, items, expected),
            ExprKind::Match { scrutinee, arms } => {
                self.infer_match(expr, scrutinee, arms, expected)
            }
            ExprKind::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
            } => self.infer_if(
                expr,
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                expected,
            ),
            ExprKind::Prefix { op, expr } => self.infer_prefix(*op, expr, expected),
            ExprKind::Binary { op, left, right } => self.infer_binary(*op, left, right, expected),
        }
    }

    pub(super) fn infer_perform(
        &mut self,
        expr: &Expr,
        effect_path: &[String],
        effect_span: &SourceSpan,
        operation_name: &str,
        operation_span: &SourceSpan,
        args: &[Expr],
    ) -> Type {
        let effect = match self
            .environment
            .resolve_user_effect_path(effect_path, self.function.module_name.as_deref())
        {
            UserEffectPathResolution::Found(effect) => effect,
            UserEffectPathResolution::PrivateCompanionTargetMismatch { effect, access } => {
                for arg in args {
                    self.infer_expr(arg, None);
                }
                self.diagnostics
                    .push(private_companion_effect_target_diagnostic(
                        expr.node_id.display("expr"),
                        "perform_expression",
                        &effect_path.join("::"),
                        effect,
                        access,
                        effect_span.clone(),
                    ));
                return Type::Unknown;
            }
            UserEffectPathResolution::QuarantinedImportTarget => {
                for arg in args {
                    self.infer_expr(arg, None);
                }
                return Type::Unknown;
            }
            UserEffectPathResolution::Missing => {
                for arg in args {
                    self.infer_expr(arg, None);
                }
                self.diagnostics.push(Diagnostic::new(
                    "effect.unknown",
                    Severity::Error,
                    DiagnosticKind::Effect,
                    format!("performed effect `{}` is not known", effect_path.join("::")),
                    Some(effect_span.clone()),
                    effect_details(expr.node_id.display("expr"), "perform_expression"),
                ));
                return Type::Unknown;
            }
        };
        let Some(operation) = effect
            .operations
            .iter()
            .find(|operation| operation.name == operation_name)
        else {
            for arg in args {
                self.infer_expr(arg, None);
            }
            self.diagnostics.push(Diagnostic::new(
                "effect.unknown_operation",
                Severity::Error,
                DiagnosticKind::Effect,
                format!(
                    "effect `{}` has no operation `{operation_name}`",
                    effect.qualified_name
                ),
                Some(operation_span.clone()),
                effect_details(expr.node_id.display("expr"), "perform_expression"),
            ));
            return Type::Unknown;
        };

        let origin = CallOrigin {
            node_id: operation.node_id,
            span: operation.name_span.clone(),
            symbol: format!("{}::{operation_name}", effect.qualified_name),
            effects: vec![effect.qualified_name.clone()],
        };
        self.check_call_arguments(args, &operation.params, None, &origin);
        self.inferred_effects.push(EffectUse {
            effect: effect.qualified_name.clone(),
            node_id: expr.node_id,
            span: expr.span.clone(),
            kind: "perform_expression",
            symbol: origin.symbol,
        });
        operation.return_type.clone()
    }

    pub(super) fn infer_handle(
        &mut self,
        expr: &Expr,
        body: &Expr,
        handler_path: &[String],
        handler_span: &SourceSpan,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Type {
        let handler = match self
            .environment
            .handler_path(handler_path, self.function.module_name.as_deref())
        {
            HandlerPathResolution::Found(handler) => handler.clone(),
            HandlerPathResolution::PrivateCompanionTargetMismatch { handler, access } => {
                for arg in args {
                    self.infer_expr(arg, None);
                }
                let body_ty = self.infer_expr(body, expected);
                self.diagnostics
                    .push(private_companion_handler_target_diagnostic(
                        expr.node_id.display("expr"),
                        "handle_expression",
                        &handler_path.join("::"),
                        handler,
                        access,
                        handler_span.clone(),
                    ));
                return body_ty;
            }
            HandlerPathResolution::QuarantinedImportTarget => {
                for arg in args {
                    self.infer_expr(arg, None);
                }
                return self.infer_expr(body, expected);
            }
            HandlerPathResolution::Missing => {
                for arg in args {
                    self.infer_expr(arg, None);
                }
                let body_ty = self.infer_expr(body, expected);
                self.diagnostics.push(Diagnostic::new(
                    "handler.unknown",
                    Severity::Error,
                    DiagnosticKind::Effect,
                    format!("handler `{}` is not known", handler_path.join("::")),
                    Some(handler_span.clone()),
                    effect_details(expr.node_id.display("expr"), "handle_expression"),
                ));
                return body_ty;
            }
        };

        self.check_call_arguments(
            args,
            &handler.params,
            None,
            &CallOrigin {
                node_id: expr.node_id,
                span: handler_span.clone(),
                symbol: handler.qualified_name.clone(),
                effects: handler.effects.clone(),
            },
        );
        let before_body = self.inferred_effects.len();
        let body_ty = self.infer_expr(body, expected);
        let mut retained = self.inferred_effects[..before_body].to_vec();
        retained.extend(
            self.inferred_effects[before_body..]
                .iter()
                .filter(|effect_use| effect_use.effect != handler.effect)
                .cloned(),
        );
        self.inferred_effects = retained;
        for effect in &handler.effects {
            self.inferred_effects.push(EffectUse {
                effect: effect.clone(),
                node_id: expr.node_id,
                span: expr.span.clone(),
                kind: "handle_expression",
                symbol: handler.qualified_name.clone(),
            });
        }
        body_ty
    }

    pub(super) fn infer_schema_decode(
        &mut self,
        expr: &Expr,
        schema: &[String],
        input: &Expr,
        base: &Expr,
    ) -> Type {
        let input_expected = ExpectedType {
            ty: Type::named("ByteView", Vec::new()),
            source: ExpectedTypeSource::DeclaredParameter,
            origin_node_id: expr.node_id,
            origin_span: Some(expr.span.clone()),
            origin_message: "Schema decode input must be a ByteView.",
        };
        let input_actual = self.infer_expr(input, Some(&input_expected));
        self.check_assignable(
            input,
            &input_expected.ty,
            &input_actual,
            &input_expected,
            "schema_decode_input",
        );

        let base_expected = ExpectedType {
            ty: Type::named("ByteOffset", Vec::new()),
            source: ExpectedTypeSource::DeclaredParameter,
            origin_node_id: expr.node_id,
            origin_span: Some(expr.span.clone()),
            origin_message: "Schema decode base offset must be a ByteOffset.",
        };
        let base_actual = self.infer_expr(base, Some(&base_expected));
        self.check_assignable(
            base,
            &base_expected.ty,
            &base_actual,
            &base_expected,
            "schema_decode_base_offset",
        );

        let Some(signature) = self
            .environment
            .schema_decode_step_signature(schema, self.function.module_name.as_deref())
        else {
            self.push_schema_decode_expression_diagnostic(expr, schema);
            return Type::Unknown;
        };
        signature.return_type.clone()
    }

    pub(super) fn push_schema_decode_expression_diagnostic(
        &mut self,
        expr: &Expr,
        schema: &[String],
    ) {
        self.push_schema_operation_expression_diagnostic(expr, schema, "decode", "decode_step");
    }

    pub(super) fn infer_schema_encode(
        &mut self,
        expr: &Expr,
        schema: &[String],
        value: &Expr,
    ) -> Type {
        let Some(signature) = self
            .environment
            .schema_encode_signature(schema, self.function.module_name.as_deref())
            .cloned()
        else {
            self.infer_expr(value, None);
            self.push_schema_encode_expression_diagnostic(expr, schema);
            return Type::Unknown;
        };
        let Some(value_type) = signature.params.first().cloned() else {
            self.infer_expr(value, None);
            self.push_schema_encode_expression_diagnostic(expr, schema);
            return Type::Unknown;
        };
        let value_expected = ExpectedType {
            ty: value_type,
            source: ExpectedTypeSource::DeclaredParameter,
            origin_node_id: expr.node_id,
            origin_span: Some(expr.span.clone()),
            origin_message: "Schema encode value must match the schema-local visible record.",
        };
        let value_actual = self.infer_expr(value, Some(&value_expected));
        self.check_assignable(
            value,
            &value_expected.ty,
            &value_actual,
            &value_expected,
            "schema_encode_value",
        );
        signature.return_type
    }

    pub(super) fn push_schema_encode_expression_diagnostic(
        &mut self,
        expr: &Expr,
        schema: &[String],
    ) {
        if let Some(unsupported) = self
            .environment
            .unsupported_schema_encode_field(schema, self.function.module_name.as_deref())
        {
            self.diagnostics
                .push(format_neutral_schema_encode_helper_diagnostic(
                    &unsupported.schema_name,
                    &unsupported.schema_span,
                    &unsupported.field,
                ));
        }
        self.push_schema_operation_expression_diagnostic(expr, schema, "encode", "encode");
    }

    pub(super) fn push_schema_operation_expression_diagnostic(
        &mut self,
        expr: &Expr,
        schema: &[String],
        operation: &str,
        operation_detail: &str,
    ) {
        let symbol = if schema.is_empty() {
            "<missing>".to_string()
        } else {
            schema.join("::")
        };
        let current_module = self.function.module_name.as_deref();
        let error = self
            .environment
            .schema_reference_error(schema, current_module);
        let reason = match error.kind {
            SchemaReferenceErrorKind::Unresolved => "unresolved_schema",
            SchemaReferenceErrorKind::Private => "private_schema",
            SchemaReferenceErrorKind::WrongKind => "wrong_kind",
        };
        let message = match (error.kind, error.resolved_kind) {
            (SchemaReferenceErrorKind::Private, _) => {
                format!("schema {operation} expression schema `{symbol}` is private")
            }
            (SchemaReferenceErrorKind::WrongKind, Some(kind)) => {
                format!("schema {operation} expression target `{symbol}` is a {kind}, not a schema")
            }
            _ => {
                let eligibility = if operation == "encode" {
                    "eligible schema encode helper"
                } else {
                    "eligible binary schema"
                };
                format!(
                    "schema {operation} expression cannot resolve `{symbol}` as an {eligibility}"
                )
            }
        };
        let mut details = vec![
            ("phase", JsonValue::string("body_analysis")),
            ("node_id", JsonValue::string(expr.node_id.display("expr"))),
            ("schema_path", JsonValue::string(symbol)),
            ("operation", JsonValue::string(operation_detail)),
            ("reason", JsonValue::string(reason)),
        ];
        if let Some(kind) = error.resolved_kind {
            details.push(("resolved_kind", JsonValue::string(kind)));
        }
        if error.kind == SchemaReferenceErrorKind::Private
            && let Some(target_module) = self
                .environment
                .companion_schema_access_target(current_module)
        {
            if let Some(current_module) = current_module {
                details.push(("companion_module", JsonValue::string(current_module)));
            }
            details.push(("companion_target_module", JsonValue::string(target_module)));
        }
        let mut diagnostic = Diagnostic::new(
            format!("schema.{operation}_expression"),
            Severity::Error,
            DiagnosticKind::Type,
            message,
            Some(expr.span.clone()),
            JsonValue::object(details),
        );
        if error.kind == SchemaReferenceErrorKind::Private
            && let Some(target_module) = self
                .environment
                .companion_schema_access_target(current_module)
        {
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("companion_target")),
                (
                    "message",
                    JsonValue::string(format!(
                        "This test companion may access private schemas only from target module `{target_module}`."
                    )),
                ),
                ("target_module", JsonValue::string(target_module)),
            ]));
        }
        self.diagnostics.push(diagnostic);
    }
}
