use super::*;

#[derive(Clone, Copy)]
enum SchemaOperation {
    Decode,
    Encode,
    Validate,
}

struct SchemaOperationMetadata {
    name: &'static str,
    detail: &'static str,
    unresolved_eligibility: &'static str,
}

impl SchemaOperation {
    fn metadata(self) -> SchemaOperationMetadata {
        match self {
            Self::Decode => SchemaOperationMetadata {
                name: "decode",
                detail: "decode_step",
                unresolved_eligibility: "eligible binary schema",
            },
            Self::Encode => SchemaOperationMetadata {
                name: "encode",
                detail: "encode",
                unresolved_eligibility: "eligible schema encode helper",
            },
            Self::Validate => SchemaOperationMetadata {
                name: "validate",
                detail: "runtime_validate",
                unresolved_eligibility: "eligible format-neutral schema",
            },
        }
    }
}

fn schema_operation_failure(
    operation: SchemaOperation,
    symbol: &str,
    kind: SchemaReferenceErrorKind,
    resolved_kind: Option<&str>,
) -> (&'static str, String) {
    let reason = match kind {
        SchemaReferenceErrorKind::Unresolved => "unresolved_schema",
        SchemaReferenceErrorKind::Private => "private_schema",
        SchemaReferenceErrorKind::WrongKind => "wrong_kind",
        SchemaReferenceErrorKind::Ambiguous => "ambiguous_target",
    };
    let metadata = operation.metadata();
    let operation_name = metadata.name;
    let message = match (kind, resolved_kind) {
        (SchemaReferenceErrorKind::Private, _) => {
            format!("schema {operation_name} expression schema `{symbol}` is private")
        }
        (SchemaReferenceErrorKind::WrongKind, Some(kind)) => {
            format!(
                "schema {operation_name} expression target `{symbol}` is a {kind}, not a schema"
            )
        }
        (SchemaReferenceErrorKind::Ambiguous, Some(kind)) => format!(
            "schema {operation_name} expression target `{symbol}` is ambiguous between a schema and a {kind}"
        ),
        (SchemaReferenceErrorKind::Ambiguous, None) => format!(
            "schema {operation_name} expression target `{symbol}` is ambiguous in the schema namespace"
        ),
        _ => format!(
            "schema {operation_name} expression cannot resolve `{symbol}` as an {}",
            metadata.unresolved_eligibility,
        ),
    };
    (reason, message)
}

fn schema_operation_details(
    expr: &Expr,
    operation: SchemaOperation,
    symbol: &str,
    reason: &str,
    resolved_kind: Option<&str>,
    current_module: Option<&str>,
    companion_target_module: Option<&str>,
) -> JsonValue {
    let mut details = vec![
        ("phase", JsonValue::string("body_analysis")),
        ("node_id", JsonValue::string(expr.node_id.display("expr"))),
        ("schema_path", JsonValue::string(symbol)),
        ("operation", JsonValue::string(operation.metadata().detail)),
        ("reason", JsonValue::string(reason)),
    ];
    if let Some(kind) = resolved_kind {
        details.push(("resolved_kind", JsonValue::string(kind)));
    }
    if let Some(target_module) = companion_target_module {
        if let Some(current_module) = current_module {
            details.push(("companion_module", JsonValue::string(current_module)));
        }
        details.push(("companion_target_module", JsonValue::string(target_module)));
    }
    JsonValue::object(details)
}

fn companion_schema_target_related(target_module: &str) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("companion_target")),
        (
            "message",
            JsonValue::string(format!(
                "This test companion may access private schemas only from target module `{target_module}`."
            )),
        ),
        ("target_module", JsonValue::string(target_module)),
    ])
}

impl<'a> FunctionChecker<'a> {
    pub(in crate::analysis) fn infer_expr(
        &mut self,
        expr: &Expr,
        expected: Option<&ExpectedType<'_>>,
    ) -> Type {
        if let Some(inferred) = self.infer_atomic_expr(expr, expected) {
            return inferred;
        }
        match &expr.kind {
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
            ExprKind::SchemaValidate {
                schema,
                schema_spans,
                value,
                base,
            } => self.infer_schema_validate(expr, schema, schema_spans, value, base),
            ExprKind::FieldAccess {
                base,
                field,
                field_span,
            } => self.infer_field_access(expr, base, field, field_span),
            ExprKind::Try {
                expr: inner,
                question_span,
            } => self.infer_try_with_defer_restriction(expr, inner, question_span, expected),
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
            ExprKind::Begin { body, .. } => self.infer_begin_body(body, expected),
            ExprKind::Prefix { op, expr } => self.infer_prefix(*op, expr, expected),
            ExprKind::Binary { op, left, right } => self.infer_binary(*op, left, right, expected),
            ExprKind::Missing
            | ExprKind::Hole { .. }
            | ExprKind::NamePath { .. }
            | ExprKind::StringLiteral(_)
            | ExprKind::IntLiteral(_)
            | ExprKind::FloatLiteral(_)
            | ExprKind::BoolLiteral(_)
            | ExprKind::Unit
            | ExprKind::TypeApply { .. } => unreachable!("atomic expression handled above"),
        }
    }

    fn infer_atomic_expr(
        &mut self,
        expr: &Expr,
        expected: Option<&ExpectedType<'_>>,
    ) -> Option<Type> {
        match &expr.kind {
            ExprKind::Missing => Some(Type::Unknown),
            ExprKind::Hole { name, satisfy } => {
                if let Some(satisfy) = satisfy {
                    self.check_satisfy_clause(expr, satisfy, expected);
                }
                self.push_hole_diagnostic(expr, name.as_deref(), satisfy.as_ref(), expected);
                Some(
                    expected
                        .map(|expected| expected.ty.as_ref().clone())
                        .unwrap_or(Type::Unknown),
                )
            }
            ExprKind::NamePath { segments, .. } => {
                Some(self.infer_name_path(segments, expr, expected))
            }
            ExprKind::StringLiteral(_) => Some(Type::string()),
            ExprKind::IntLiteral(_) => Some(Type::int()),
            ExprKind::FloatLiteral(_) => Some(Type::float()),
            ExprKind::BoolLiteral(_) => Some(Type::bool()),
            ExprKind::Unit => Some(Type::unit()),
            ExprKind::TypeApply { .. } => Some(Type::Unknown),
            _ => None,
        }
    }

    fn infer_try_with_defer_restriction(
        &mut self,
        expr: &Expr,
        inner: &Expr,
        question_span: &SourceSpan,
        expected: Option<&ExpectedType<'_>>,
    ) -> Type {
        if let Some(block_span) = self.defer_blocks.last().cloned() {
            self.push_defer_restriction_diagnostic(DeferRestrictionDiagnostic {
                id: "defer.propagation",
                message: "deferred block cannot use `?`".to_string(),
                node_id: expr.node_id.display("expr"),
                span: question_span.clone(),
                reason: "result_propagation",
                repair: "Handle the `Result` inside the deferred block instead of propagating it with `?`.",
                repair_span: block_span,
            });
        }
        self.infer_try(expr, inner, expected)
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
        let effect = match self.environment.resolve_user_effect_path_in_owner(
            effect_path,
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) {
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
            schema_encode: false,
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
        expected: Option<&ExpectedType<'_>>,
    ) -> Type {
        let handler = match self.environment.handler_path_in_owner(
            handler_path,
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) {
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
                schema_encode: false,
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
            ty: std::borrow::Cow::Owned(Type::named("ByteView", Vec::new())),
            source: ExpectedTypeSource::DeclaredParameter,
            origin_node_id: expr.node_id,
            origin_span: Some(expr.span.clone()),
            origin_message: "Schema decode input must be a ByteView.",
        };
        let input_actual = self.infer_expr(input, Some(&input_expected));
        self.check_assignable(
            input,
            input_expected.ty.as_ref(),
            &input_actual,
            &input_expected,
            "schema_decode_input",
        );

        let base_expected = ExpectedType {
            ty: std::borrow::Cow::Owned(Type::named("ByteOffset", Vec::new())),
            source: ExpectedTypeSource::DeclaredParameter,
            origin_node_id: expr.node_id,
            origin_span: Some(expr.span.clone()),
            origin_message: "Schema decode base offset must be a ByteOffset.",
        };
        let base_actual = self.infer_expr(base, Some(&base_expected));
        self.check_assignable(
            base,
            base_expected.ty.as_ref(),
            &base_actual,
            &base_expected,
            "schema_decode_base_offset",
        );

        let Some(signature) = self.environment.schema_decode_step_signature_in_owner(
            schema,
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) else {
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
        self.push_schema_operation_expression_diagnostic(
            expr,
            schema,
            SchemaOperation::Decode,
            None,
        );
    }

    pub(super) fn infer_schema_encode(
        &mut self,
        expr: &Expr,
        schema: &[String],
        value: &Expr,
    ) -> Type {
        let Some(signature) = self
            .environment
            .schema_encode_signature_in_owner(
                schema,
                self.function.module_name.as_deref(),
                self.function.package_name.as_deref(),
            )
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
            ty: std::borrow::Cow::Owned(value_type),
            source: ExpectedTypeSource::SchemaEncodeParameter,
            origin_node_id: expr.node_id,
            origin_span: Some(expr.span.clone()),
            origin_message: "Schema encode value must match the schema-local visible record.",
        };
        let value_actual = self.infer_expr(value, Some(&value_expected));
        self.check_schema_encode_assignable(
            value,
            value_expected.ty.as_ref(),
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
        if let Some(unsupported) = self.environment.unsupported_schema_encode_field(
            schema,
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) {
            self.diagnostics
                .push(format_neutral_schema_encode_helper_diagnostic(
                    &unsupported.schema_name,
                    &unsupported.schema_span,
                    &unsupported.field,
                ));
        }
        self.push_schema_operation_expression_diagnostic(
            expr,
            schema,
            SchemaOperation::Encode,
            None,
        );
    }

    pub(super) fn infer_schema_validate(
        &mut self,
        expr: &Expr,
        schema: &[String],
        schema_spans: &[SourceSpan],
        value: &Expr,
        base: &Expr,
    ) -> Type {
        let signature = self
            .environment
            .schema_runtime_validation_signature_in_owner(
                schema,
                self.function.module_name.as_deref(),
                self.function.package_name.as_deref(),
            )
            .cloned();
        let value_expected = signature
            .as_ref()
            .and_then(|signature| signature.params.first())
            .cloned()
            .map(|ty| ExpectedType {
                ty: std::borrow::Cow::Owned(ty),
                source: ExpectedTypeSource::DeclaredParameter,
                origin_node_id: expr.node_id,
                origin_span: Some(expr.span.clone()),
                origin_message: "Schema validation value must match the erased schema record.",
            });
        let value_actual = self.infer_expr(value, value_expected.as_ref());
        if let Some(expected) = &value_expected {
            self.check_assignable(
                value,
                expected.ty.as_ref(),
                &value_actual,
                expected,
                "schema_validation_value",
            );
        }
        let offset_expected = ExpectedType {
            ty: std::borrow::Cow::Owned(Type::named("ByteOffset", Vec::new())),
            source: ExpectedTypeSource::DeclaredParameter,
            origin_node_id: expr.node_id,
            origin_span: Some(expr.span.clone()),
            origin_message: "Schema validation offset must be a ByteOffset.",
        };
        let offset_actual = self.infer_expr(base, Some(&offset_expected));
        self.check_assignable(
            base,
            offset_expected.ty.as_ref(),
            &offset_actual,
            &offset_expected,
            "schema_validation_offset",
        );
        let Some(signature) = signature else {
            let mut schema_span = schema_spans.first().cloned();
            if let (Some(span), Some(last)) = (&mut schema_span, schema_spans.last()) {
                span.end = last.end;
            }
            self.push_schema_operation_expression_diagnostic(
                expr,
                schema,
                SchemaOperation::Validate,
                schema_span.as_ref(),
            );
            return Type::Unknown;
        };
        signature.return_type
    }

    fn push_schema_operation_expression_diagnostic(
        &mut self,
        expr: &Expr,
        schema: &[String],
        operation: SchemaOperation,
        primary_span: Option<&SourceSpan>,
    ) {
        let symbol = if schema.is_empty() {
            "<missing>".to_string()
        } else {
            schema.join("::")
        };
        let current_module = self.function.module_name.as_deref();
        let error = self.environment.schema_reference_error_in_owner(
            schema,
            current_module,
            self.function.package_name.as_deref(),
        );
        let (reason, message) =
            schema_operation_failure(operation, &symbol, error.kind, error.resolved_kind);
        let companion_target_module = if error.kind == SchemaReferenceErrorKind::Private {
            self.environment
                .companion_schema_access_target(current_module)
        } else {
            None
        };
        let mut diagnostic = Diagnostic::new(
            format!("schema.{}_expression", operation.metadata().name),
            Severity::Error,
            DiagnosticKind::Type,
            message,
            Some(primary_span.unwrap_or(&expr.span).clone()),
            schema_operation_details(
                expr,
                operation,
                &symbol,
                reason,
                error.resolved_kind,
                current_module,
                companion_target_module,
            ),
        );
        if let Some(target_module) = companion_target_module {
            diagnostic
                .related
                .push(companion_schema_target_related(target_module));
        }
        self.diagnostics.push(diagnostic);
    }
}
