use super::*;

impl<'a> CoreLowerer<'a> {
    pub(super) fn lower_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        self.lower_call_with_callsite(expr, expr, callee, args, expected)
    }

    pub(super) fn lower_call_with_callsite(
        &mut self,
        expr: &Expr,
        callsite_expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        if let Some(call) = self.lower_constructor_call(expr, callee, args, expected) {
            return call;
        }
        if let Some(call) = self.lower_name_concurrency_call(expr, callee, args, expected) {
            return call;
        }
        if let Some(call) = self.lower_type_applied_concurrency_call(expr, callee, args, expected) {
            return call;
        }
        self.lower_general_call(expr, callsite_expr, callee, args, expected)
    }

    pub(super) fn lower_perform(
        &mut self,
        expr: &Expr,
        effect_path: &[String],
        operation_name: &str,
        args: &[Expr],
    ) -> CoreExpr {
        let Some(effect) = self
            .environment
            .user_effect_path(effect_path, self.function.module_name.as_deref())
        else {
            return self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing);
        };
        let Some(operation) = effect
            .operations
            .iter()
            .find(|operation| operation.name == operation_name)
        else {
            return self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing);
        };
        let lowered_args = args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                self.lower_expr(arg, operation.params.get(index).map(core_type).as_ref())
            })
            .collect();
        self.core_expr(
            expr,
            core_type(&operation.return_type),
            CoreExprKind::Perform {
                effect: effect.qualified_name.clone(),
                operation: operation_name.to_string(),
                args: lowered_args,
            },
        )
    }

    pub(super) fn lower_handle(
        &mut self,
        expr: &Expr,
        body: &Expr,
        handler_path: &[String],
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let handler = match self
            .environment
            .handler_path(handler_path, self.function.module_name.as_deref())
        {
            HandlerPathResolution::Found(handler) => handler.clone(),
            HandlerPathResolution::PrivateCompanionTargetMismatch { .. }
            | HandlerPathResolution::QuarantinedImportTarget
            | HandlerPathResolution::Missing => {
                for arg in args {
                    self.lower_expr(arg, None);
                }
                return self.lower_expr(body, expected);
            }
        };
        let context_args = args
            .iter()
            .enumerate()
            .map(|(index, arg)| {
                self.lower_expr(arg, handler.params.get(index).map(core_type).as_ref())
            })
            .collect::<Vec<_>>();
        let operation_clauses = handler
            .operation_clauses
            .iter()
            .map(|clause| CoreHandlerProvider {
                operation: clause.operation.clone(),
                function: crate::standard_symbols::standard_function_link_name(
                    clause.module_name.as_deref(),
                    &clause.function,
                ),
            })
            .collect::<Vec<_>>();
        let lowered = self.lower_expr(body, expected);
        self.core_expr(
            expr,
            lowered.ty.clone(),
            CoreExprKind::Handle {
                effect: handler.effect,
                providers: operation_clauses,
                context_args,
                body: Box::new(lowered),
            },
        )
    }

    pub(super) fn lower_constructor_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> Option<CoreExpr> {
        if let ExprKind::NamePath { segments, .. } = &callee.kind
            && let ConstructorLookup::Found(constructor) =
                self.environment.adts.constructor_for_expected_type(
                    segments,
                    expected,
                    self.function.module_name.as_deref(),
                    &self.environment.uses,
                    ConstructorShape::Payload,
                )
        {
            return Some(self.lower_adt_constructor(expr, args, expected, constructor));
        }
        None
    }

    pub(super) fn lower_name_concurrency_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> Option<CoreExpr> {
        let ExprKind::NamePath { segments, .. } = &callee.kind else {
            return None;
        };
        if !is_concurrency_call(segments) {
            return None;
        }
        let handle_type = args.first().and_then(|arg| self.shallow_expr_type(arg));
        let signature =
            core_concurrency_signature(segments, expected, handle_type.as_ref(), None, None);
        Some(self.lower_concurrency_call_with_signature(expr, segments, args, signature))
    }

    pub(super) fn lower_type_applied_concurrency_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> Option<CoreExpr> {
        if let Some((segments, type_args)) = callee.callee_name_path_and_type_args()
            && is_concurrency_call(segments)
            && matches!(callee.kind, ExprKind::TypeApply { .. })
        {
            let type_args = type_args.unwrap_or(&[]);
            if let Some(expected) = expected_concurrency_type_arg_count(segments)
                && type_args.len() > expected
            {
                self.unsupported_expression(
                    callee,
                    "type_argument_count_mismatch",
                    format!(
                        "`{}` expects at most {expected} type argument(s), found {}",
                        segments.join("::"),
                        type_args.len()
                    ),
                    Some(JsonValue::object([
                        (
                            "expected_type_argument_count",
                            JsonValue::Number(expected as i64),
                        ),
                        (
                            "actual_type_argument_count",
                            JsonValue::Number(type_args.len() as i64),
                        ),
                    ])),
                );
            }
            let explicit_item = type_args
                .first()
                .and_then(|type_arg| parse_type_annotation(type_arg).ok())
                .map(|ty| self.parsed_core_type(ty));
            let explicit_context = type_args
                .get(1)
                .filter(|_| matches!(segments, [module, name] if module == "task" && name == "spawn_with"))
                .and_then(|type_arg| parse_type_annotation(type_arg).ok())
                .map(|ty| self.parsed_core_type(ty));
            let handle_type = args.first().and_then(|arg| self.shallow_expr_type(arg));
            let signature = core_concurrency_signature(
                segments,
                expected,
                handle_type.as_ref(),
                explicit_item.as_ref(),
                explicit_context.as_ref(),
            );
            return Some(
                self.lower_concurrency_call_with_signature(expr, segments, args, signature),
            );
        }
        None
    }

    pub(super) fn lower_concurrency_call_with_signature(
        &mut self,
        expr: &Expr,
        segments: &[String],
        args: &[Expr],
        signature: Option<(Vec<CoreType>, CoreType)>,
    ) -> CoreExpr {
        if let Some((params, _)) = &signature {
            self.validate_call_arity(expr, args.len(), params.len(), false);
        }
        let lowered_args = self.lower_args_with_params(
            args,
            signature.as_ref().map(|(params, _)| params.as_slice()),
        );
        let name = segments.join("::");
        let target = self.callback_boundary_target(
            CoreCallbackTarget::ConcurrencyBuiltin(name.clone()),
            CoreCallTarget::ConcurrencyBuiltin(name),
            &lowered_args,
            expr,
        );
        self.core_expr(
            expr,
            signature
                .map(|(_, return_type)| return_type)
                .unwrap_or(CoreType::Unknown),
            CoreExprKind::Call {
                target,
                args: lowered_args,
            },
        )
    }

    pub(super) fn lower_general_call(
        &mut self,
        expr: &Expr,
        callsite_expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        if let ExprKind::NamePath { segments, .. } = &callee.kind
            && let [name] = segments.as_slice()
            && let Some(index) = self
                .bindings
                .iter()
                .rposition(|binding| binding.name == *name)
        {
            self.record_defer_capture(index);
        }
        let signature = self.core_call_signature(callee, expected);
        if let Some(signature) = &signature {
            self.validate_call_arity(
                expr,
                args.len(),
                signature.params.len(),
                signature.variadic.is_some(),
            );
        }
        let mut lowered_args = match &signature {
            Some(signature) => self.lower_args_with_signature(args, signature),
            None => self.lower_args_with_params(args, None),
        };
        if signature
            .as_ref()
            .is_some_and(|signature| signature.callsite)
        {
            lowered_args.push(self.lower_direct_callsite(callsite_expr));
        }
        let (target, return_type) = signature.map_or_else(
            || {
                let symbol = callee_symbol(callee).unwrap_or_else(|| "<unknown>".to_string());
                (CoreCallTarget::Unresolved(symbol), CoreType::Unknown)
            },
            |signature| (signature.target, signature.return_type),
        );
        let target = match target {
            CoreCallTarget::Value(name) => CoreCallTarget::CallsiteValue {
                name,
                callsite: Box::new(self.lower_direct_callsite(callsite_expr)),
            },
            CoreCallTarget::Function(name) => self.callback_boundary_target(
                CoreCallbackTarget::Function(name.clone()),
                CoreCallTarget::Function(name),
                &lowered_args,
                callsite_expr,
            ),
            CoreCallTarget::StandardLibraryBuiltin(name) => self.callback_boundary_target(
                CoreCallbackTarget::StandardLibraryBuiltin(name.clone()),
                CoreCallTarget::StandardLibraryBuiltin(name),
                &lowered_args,
                callsite_expr,
            ),
            CoreCallTarget::PreludeBuiltin(name) => self.callback_boundary_target(
                CoreCallbackTarget::PreludeBuiltin(name.clone()),
                CoreCallTarget::PreludeBuiltin(name),
                &lowered_args,
                callsite_expr,
            ),
            target => target,
        };

        self.core_expr(
            expr,
            return_type,
            CoreExprKind::Call {
                target,
                args: lowered_args,
            },
        )
    }

    fn callback_boundary_target(
        &self,
        callback_target: CoreCallbackTarget,
        ordinary_target: CoreCallTarget,
        args: &[CoreExpr],
        callsite_expr: &Expr,
    ) -> CoreCallTarget {
        if args
            .iter()
            .any(|arg| matches!(arg.ty, CoreType::Function { .. }))
        {
            CoreCallTarget::CallbackBoundary {
                target: callback_target,
                callsite: Box::new(self.lower_direct_callsite(callsite_expr)),
            }
        } else {
            ordinary_target
        }
    }

    fn lower_direct_callsite(&self, expr: &Expr) -> CoreExpr {
        if self.function.callsite.is_some() {
            return self.core_expr(
                expr,
                self.parsed_core_type(Type::source_location()),
                CoreExprKind::Local("callsite".to_string()),
            );
        }

        let location = expr.span.resolved_or_generated();

        let string_field = |name: &str, value: String| CoreRecordField {
            node_id: expr.node_id,
            name: name.to_string(),
            expr: self.core_expr(expr, CoreType::string(), CoreExprKind::StringLiteral(value)),
            span: expr.span.clone(),
        };
        let int_field = |name: &str, value: usize| CoreRecordField {
            node_id: expr.node_id,
            name: name.to_string(),
            expr: self.core_expr(
                expr,
                CoreType::int(),
                CoreExprKind::IntLiteral(value.to_string()),
            ),
            span: expr.span.clone(),
        };
        let package = self.function.package_name.clone().unwrap_or_default();
        let module = self
            .function
            .module_name
            .as_deref()
            .map(|module_name| {
                self.function.package_name.as_deref().map_or_else(
                    || module_name.to_string(),
                    |package_name| {
                        module_name
                            .strip_prefix(package_name)
                            .and_then(|module_name| module_name.strip_prefix("::"))
                            .unwrap_or(module_name)
                            .to_string()
                    },
                )
            })
            .unwrap_or_default();
        let fields = vec![
            string_field("package", package),
            string_field("module", module),
            string_field("file", location.file.as_str().to_string()),
            int_field("start_line", location.start.line),
            int_field("start_column", location.start.column),
            int_field("start_offset", location.start.offset),
            int_field("end_line", location.end.line),
            int_field("end_column", location.end.column),
            int_field("end_offset", location.end.offset),
        ];
        self.core_expr(
            expr,
            self.parsed_core_type(Type::source_location()),
            CoreExprKind::Record(fields),
        )
    }

    pub(super) fn lower_schema_decode(
        &mut self,
        expr: &Expr,
        schema: &[String],
        input: &Expr,
        base: &Expr,
    ) -> CoreExpr {
        let signature = self
            .environment
            .schema_decode_step_signature(schema, self.function.module_name.as_deref())
            .cloned();
        let params = signature
            .as_ref()
            .map(|signature| signature.params.iter().map(core_type).collect::<Vec<_>>());
        let input = self.lower_expr(
            input,
            params
                .as_ref()
                .and_then(|params| params.first())
                .or(Some(&CoreType::named("ByteView", Vec::new()))),
        );
        let base = self.lower_expr(
            base,
            params
                .as_ref()
                .and_then(|params| params.get(1))
                .or(Some(&CoreType::named("ByteOffset", Vec::new()))),
        );
        let Some(signature) = signature else {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: expr.node_id,
                reason: "schema_decode_expression".to_string(),
            });
            return self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing);
        };
        let schema_name = signature
            .target_name
            .strip_prefix(SCHEMA_DECODE_STEP_TARGET_PREFIX)
            .unwrap_or_else(|| schema.last().map(String::as_str).unwrap_or("<missing>"))
            .to_string();
        self.core_expr(
            expr,
            core_type(&signature.return_type),
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaDecodeStep(schema_name),
                args: vec![input, base],
            },
        )
    }

    pub(super) fn lower_schema_encode(
        &mut self,
        expr: &Expr,
        schema: &[String],
        value: &Expr,
    ) -> CoreExpr {
        let signature = self
            .environment
            .schema_encode_signature(schema, self.function.module_name.as_deref())
            .cloned();
        let value = self.lower_expr(
            value,
            signature
                .as_ref()
                .and_then(|signature| signature.params.first())
                .map(core_type)
                .as_ref(),
        );
        let Some(signature) = signature else {
            self.blockers.push(CoreBlocker::UnsupportedExpression {
                node_id: expr.node_id,
                reason: "schema_encode_expression".to_string(),
            });
            return self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing);
        };
        let target = if let Some(schema_name) = signature
            .target_name
            .strip_prefix(SCHEMA_ENCODE_TARGET_PREFIX)
        {
            CoreCallTarget::SchemaEncode(schema_name.to_string())
        } else if let Some(schema_name) = signature
            .target_name
            .strip_prefix(SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX)
        {
            CoreCallTarget::SchemaNeutralEncode(schema_name.to_string())
        } else {
            CoreCallTarget::SchemaEncode(
                schema
                    .last()
                    .map(String::as_str)
                    .unwrap_or("<missing>")
                    .to_string(),
            )
        };
        self.core_expr(
            expr,
            core_type(&signature.return_type),
            CoreExprKind::Call {
                target,
                args: vec![value],
            },
        )
    }

    pub(super) fn validate_call_arity(
        &mut self,
        expr: &Expr,
        actual: usize,
        expected: usize,
        has_variadic: bool,
    ) {
        if (!has_variadic && actual == expected) || (has_variadic && actual >= expected) {
            return;
        }
        let message = if has_variadic {
            format!("call expects at least {expected} argument(s), but got {actual}")
        } else {
            format!("call expects {expected} argument(s), but got {actual}")
        };
        self.unsupported_expression(
            expr,
            "call_arity_mismatch",
            message,
            Some(JsonValue::object([
                (
                    "expected_argument_count",
                    JsonValue::Number(expected as i64),
                ),
                ("actual_argument_count", JsonValue::Number(actual as i64)),
            ])),
        );
    }

    pub(super) fn lower_args_with_params(
        &mut self,
        args: &[Expr],
        params: Option<&[CoreType]>,
    ) -> Vec<CoreExpr> {
        args.iter()
            .enumerate()
            .map(|(index, arg)| {
                let expected = params.and_then(|params| params.get(index));
                self.lower_expr(arg, expected)
            })
            .collect()
    }

    pub(super) fn lower_args_with_signature(
        &mut self,
        args: &[Expr],
        signature: &CoreCallSignature,
    ) -> Vec<CoreExpr> {
        let Some(variadic) = &signature.variadic else {
            return self.lower_args_with_params(args, Some(&signature.params));
        };
        let fixed_count = signature.params.len();
        let mut lowered = args
            .iter()
            .take(fixed_count)
            .enumerate()
            .map(|(index, arg)| self.lower_expr(arg, signature.params.get(index)))
            .collect::<Vec<_>>();
        let tail_items = args
            .iter()
            .skip(fixed_count)
            .map(|arg| self.lower_expr(arg, Some(variadic)))
            .collect::<Vec<_>>();
        let list_ty = CoreType::named("List", vec![variadic.clone()]);
        lowered.push(self.core_list_from_items(list_ty, tail_items, args.get(fixed_count)));
        lowered
    }

    pub(super) fn core_list_from_items(
        &self,
        list_ty: CoreType,
        items: Vec<CoreExpr>,
        first_tail_arg: Option<&Expr>,
    ) -> CoreExpr {
        let span =
            first_tail_arg.map_or_else(|| self.function.span.clone(), |arg| arg.span.clone());
        let mut list = CoreExpr {
            node_id: first_tail_arg.map_or(self.function.node_id, |arg| arg.node_id),
            ty: list_ty.clone(),
            kind: CoreExprKind::ListNil,
            span: span.clone(),
        };
        for item in items.into_iter().rev() {
            list = CoreExpr {
                node_id: item.node_id,
                ty: list_ty.clone(),
                kind: CoreExprKind::ListCons {
                    head: Box::new(item),
                    tail: Box::new(list),
                },
                span: span.clone(),
            };
        }
        list
    }

    pub(super) fn lower_adt_constructor(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
        constructor: AdtConstructor,
    ) -> CoreExpr {
        let expected_count = constructor.variant.payload_fields.len();
        if args.len() != expected_count {
            self.unsupported_expression(
                expr,
                constructor_arity_reason(constructor),
                format!(
                    "{} constructor expects {expected_count} argument, but got {}",
                    constructor.descriptor.diagnostic_name,
                    args.len()
                ),
                Some(JsonValue::object([
                    (
                        "expected_argument_count",
                        JsonValue::Number(expected_count as i64),
                    ),
                    (
                        "actual_argument_count",
                        JsonValue::Number(args.len() as i64),
                    ),
                ])),
            );
        }
        let expected_constructor_type = expected
            .and_then(|expected| unification::adt_args(expected, constructor.descriptor))
            .is_some();
        let mut inferred_type_args =
            vec![CoreType::Unknown; constructor.descriptor.type_parameters.len()];
        let mut lowered_args = Vec::new();
        for (index, _) in constructor.variant.payload_fields.iter().enumerate() {
            let payload_type = expected
                .filter(|_| expected_constructor_type)
                .and_then(|expected| adt::core_payload_type(expected, constructor, index))
                .or_else(|| {
                    adt::core_payload_type_with_args(constructor, &inferred_type_args, index)
                })
                .unwrap_or(CoreType::Unknown);
            let lowered = args
                .get(index)
                .map(|arg| self.lower_expr(arg, Some(&payload_type)))
                .unwrap_or_else(|| {
                    self.missing_expression(
                        expr,
                        Some(&payload_type),
                        "missing_constructor_argument",
                    );
                    self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing)
                });
            if !expected_constructor_type {
                adt::merge_core_type_args_from_payload(
                    &mut inferred_type_args,
                    constructor,
                    index,
                    &lowered.ty,
                );
            }
            lowered_args.push(lowered);
        }
        let ty = if expected_constructor_type {
            expected.cloned().unwrap_or(CoreType::Unknown)
        } else {
            adt::core_constructed_type_from_args(constructor, &inferred_type_args)
        };
        for arg in args.iter().skip(expected_count) {
            self.lower_expr(arg, None);
        }
        self.core_expr(
            expr,
            ty,
            core_payload_constructor_kind(constructor, lowered_args),
        )
    }
}
