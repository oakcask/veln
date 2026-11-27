use super::*;

pub(super) enum ConstructorTypeContext<'a> {
    Expected(&'a CoreType),
    Inferred(Vec<CoreType>),
}

impl<'a> ConstructorTypeContext<'a> {
    pub(super) fn new(expected: Option<&'a CoreType>, constructor: AdtConstructor<'_>) -> Self {
        match expected {
            Some(expected) if unification::adt_args(expected, constructor.descriptor).is_some() => {
                Self::Expected(expected)
            }
            _ => Self::Inferred(vec![
                CoreType::Unknown;
                constructor.descriptor.type_parameters.len()
            ]),
        }
    }

    pub(super) fn payload_type(&self, constructor: AdtConstructor<'_>, index: usize) -> CoreType {
        let resolved = match self {
            Self::Expected(expected) => adt::core_payload_type(expected, constructor, index),
            Self::Inferred(args) => adt::core_payload_type_with_args(constructor, args, index),
        };
        resolved.unwrap_or(CoreType::Unknown)
    }

    pub(super) fn observe_payload(
        &mut self,
        constructor: AdtConstructor<'_>,
        index: usize,
        actual: &CoreType,
    ) {
        if let Self::Inferred(args) = self {
            adt::merge_core_type_args_from_payload(args, constructor, index, actual);
        }
    }

    pub(super) fn into_constructed_type(self, constructor: AdtConstructor<'_>) -> CoreType {
        match self {
            Self::Expected(expected) => expected.clone(),
            Self::Inferred(args) => adt::core_constructed_type_from_args(constructor, &args),
        }
    }
}

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
        let Some(effect) = self.environment.user_effect_path_in_owner(
            effect_path,
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) else {
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
        entries: &[veln_ast::HandlerApplication],
        expected: Option<&CoreType>,
    ) -> CoreExpr {
        let mut lowered_entries = Vec::with_capacity(entries.len());
        for entry in entries {
            let handler = match self.environment.handler_path_in_owner(
                &entry.handler,
                self.function.module_name.as_deref(),
                self.function.package_name.as_deref(),
            ) {
                HandlerPathResolution::Found(handler) => handler.clone(),
                HandlerPathResolution::PrivateCompanionTargetMismatch { .. }
                | HandlerPathResolution::QuarantinedImportTarget
                | HandlerPathResolution::Missing => {
                    for arg in &entry.args {
                        self.lower_expr(arg, None);
                    }
                    continue;
                }
            };
            lowered_entries.push(CoreHandlerApplication {
                effect: handler.effect,
                providers: handler
                    .operation_clauses
                    .iter()
                    .map(|clause| CoreHandlerProvider {
                        operation: clause.operation.clone(),
                        function: clause.target_name.clone(),
                    })
                    .collect(),
                context_args: entry
                    .args
                    .iter()
                    .enumerate()
                    .map(|(index, arg)| {
                        self.lower_expr(arg, handler.params.get(index).map(core_type).as_ref())
                    })
                    .collect(),
            });
        }
        let lowered = self.lower_expr(body, expected);
        if lowered_entries.is_empty() {
            return lowered;
        }
        self.core_expr(
            expr,
            lowered.ty.clone(),
            CoreExprKind::Handle {
                entries: lowered_entries,
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
            CoreCallbackTarget::ConcurrencyBuiltin(name),
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
        self.record_call_defer_capture(callee);
        let (mut lowered_args, signature) =
            self.lower_general_call_args(expr, callee, args, expected);
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
        let target = self.call_target_with_context(target, &lowered_args, callsite_expr);

        self.core_expr(
            expr,
            return_type,
            CoreExprKind::Call {
                target,
                args: lowered_args,
            },
        )
    }

    fn lower_general_call_args(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&CoreType>,
    ) -> (Vec<CoreExpr>, Option<CoreCallSignature>) {
        let joined_state_initializer = self.joined_state_initializer_type(expr, callee);
        let preliminary_signature = self.core_call_signature(
            callee,
            expected,
            crate::call_resolution::CallArgumentEvidence {
                first: joined_state_initializer.as_ref(),
                second: None,
                third: None,
            },
        );
        let mut lowered_args = match &preliminary_signature {
            Some(signature) => self.lower_args_with_preliminary_signature(args, signature),
            None => self.lower_args_with_params(args, None),
        };
        let signature = self.core_call_signature(
            callee,
            expected,
            crate::call_resolution::CallArgumentEvidence {
                first: lowered_args.first().map(|arg| &arg.ty),
                second: lowered_args.get(1).map(|arg| &arg.ty),
                third: lowered_args.get(2).map(|arg| &arg.ty),
            },
        );
        if let Some(signature) = &signature {
            self.validate_call_arity(
                expr,
                args.len(),
                signature.params.len(),
                signature.variadic.is_some(),
            );
            if joined_state_initializer.is_some()
                && let (Some(initial), Some(expected)) =
                    (lowered_args.first_mut(), signature.params.first())
            {
                retag_state_initializer(initial, expected);
            }
        }
        (lowered_args, signature)
    }

    fn record_call_defer_capture(&mut self, callee: &Expr) {
        if let ExprKind::NamePath { segments, .. } = &callee.kind
            && let [name] = segments.as_slice()
            && let Some(index) = self
                .bindings
                .iter()
                .rposition(|binding| binding.name == *name)
        {
            self.record_defer_capture(index);
        }
    }

    fn joined_state_initializer_type(&self, expr: &Expr, callee: &Expr) -> Option<CoreType> {
        if self.declared_io_provider_state_operation(callee) != Some("state") {
            return None;
        }
        let Type::Named { args, .. } = self.state_call_types.get(&expr.node_id)? else {
            return None;
        };
        let item = args.first().map(core_type)?;
        #[cfg(test)]
        record_state_initializer_lowering_work(|work| {
            work.fact_lookups += 1;
            work.materialized_type_nodes += core_type_node_count(&item);
        });
        Some(item)
    }

    fn declared_io_provider_state_operation(&self, callee: &Expr) -> Option<&str> {
        let ExprKind::NamePath { segments, .. } = &callee.kind else {
            return None;
        };
        let function = match segments.as_slice() {
            [name] => self
                .environment
                .unqualified_function_in_owner(
                    name,
                    self.function.module_name.as_deref(),
                    self.function.package_name.as_deref(),
                )
                .found(),
            _ => self.environment.function_path_in_owner(
                segments,
                self.function.module_name.as_deref(),
                self.function.package_name.as_deref(),
            ),
        }?;
        (function.package_name.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
            && function.module_name.as_deref() == Some("std::io_provider")
            && matches!(function.name.as_str(), "state" | "update"))
        .then_some(function.name.as_str())
    }

    fn call_target_with_context(
        &self,
        target: CoreCallTarget,
        args: &[CoreExpr],
        callsite_expr: &Expr,
    ) -> CoreCallTarget {
        match target {
            CoreCallTarget::Value(name) => CoreCallTarget::CallsiteValue {
                name,
                callsite: Box::new(self.lower_direct_callsite(callsite_expr)),
            },
            CoreCallTarget::Function(name) => self.callback_boundary_target(
                CoreCallbackTarget::Function(name),
                args,
                callsite_expr,
            ),
            CoreCallTarget::StandardLibraryBuiltin(name) => self.callback_boundary_target(
                CoreCallbackTarget::StandardLibraryBuiltin(name),
                args,
                callsite_expr,
            ),
            CoreCallTarget::PreludeBuiltin(name) => self.callback_boundary_target(
                CoreCallbackTarget::PreludeBuiltin(name),
                args,
                callsite_expr,
            ),
            target => target,
        }
    }

    fn callback_boundary_target(
        &self,
        target: CoreCallbackTarget,
        args: &[CoreExpr],
        callsite_expr: &Expr,
    ) -> CoreCallTarget {
        if args
            .iter()
            .any(|arg| matches!(arg.ty, CoreType::Function { .. }))
        {
            CoreCallTarget::CallbackBoundary {
                target,
                callsite: Box::new(self.lower_direct_callsite(callsite_expr)),
            }
        } else {
            match target {
                CoreCallbackTarget::Function(name) => CoreCallTarget::Function(name),
                CoreCallbackTarget::ConcurrencyBuiltin(name) => {
                    CoreCallTarget::ConcurrencyBuiltin(name)
                }
                CoreCallbackTarget::StandardLibraryBuiltin(name) => {
                    CoreCallTarget::StandardLibraryBuiltin(name)
                }
                CoreCallbackTarget::PreludeBuiltin(name) => CoreCallTarget::PreludeBuiltin(name),
            }
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
            .schema_decode_step_signature_in_owner(
                schema,
                self.function.module_name.as_deref(),
                self.function.package_name.as_deref(),
            )
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
            .schema_encode_signature_in_owner(
                schema,
                self.function.module_name.as_deref(),
                self.function.package_name.as_deref(),
            )
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

    pub(super) fn lower_schema_validate(
        &mut self,
        expr: &Expr,
        schema: &[String],
        value: &Expr,
        base: &Expr,
    ) -> CoreExpr {
        let signature = self
            .environment
            .schema_runtime_validation_signature_in_owner(
                schema,
                self.function.module_name.as_deref(),
                self.function.package_name.as_deref(),
            )
            .cloned();
        let params = signature
            .as_ref()
            .map(|signature| signature.params.iter().map(core_type).collect::<Vec<_>>());
        let value = self.lower_expr(value, params.as_ref().and_then(|params| params.first()));
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
                reason: "schema_validate_expression".to_string(),
            });
            return self.core_expr(expr, CoreType::Unknown, CoreExprKind::Missing);
        };
        let schema_name = signature
            .target_name
            .strip_prefix(SCHEMA_REFINEMENT_VALIDATE_TARGET_PREFIX)
            .unwrap_or_else(|| schema.last().map(String::as_str).unwrap_or("<missing>"))
            .to_string();
        self.core_expr(
            expr,
            core_type(&signature.return_type),
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaRefinementValidate(schema_name),
                args: vec![value, base],
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

    fn lower_args_with_preliminary_signature(
        &mut self,
        args: &[Expr],
        signature: &CoreCallSignature,
    ) -> Vec<CoreExpr> {
        let Some(variadic) = &signature.variadic else {
            return args
                .iter()
                .enumerate()
                .map(|(index, arg)| {
                    let expected = signature
                        .params
                        .get(index)
                        .filter(|expected| !expected.contains_unknown());
                    self.lower_expr(arg, expected)
                })
                .collect();
        };
        let fixed_count = signature.params.len();
        let mut lowered = args
            .iter()
            .take(fixed_count)
            .enumerate()
            .map(|(index, arg)| {
                let expected = signature
                    .params
                    .get(index)
                    .filter(|expected| !expected.contains_unknown());
                self.lower_expr(arg, expected)
            })
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
        self.report_constructor_arity_mismatch(expr, args.len(), constructor);
        let mut type_context = ConstructorTypeContext::new(expected, constructor);
        let lowered_args =
            self.lower_constructor_payloads(expr, args, constructor, &mut type_context);
        let ty = type_context.into_constructed_type(constructor);
        let expected_count = constructor.variant.payload_fields.len();
        for arg in args.iter().skip(expected_count) {
            self.lower_expr(arg, None);
        }
        self.core_expr(
            expr,
            ty,
            core_payload_constructor_kind(constructor, lowered_args),
        )
    }

    fn report_constructor_arity_mismatch(
        &mut self,
        expr: &Expr,
        actual_count: usize,
        constructor: AdtConstructor<'_>,
    ) {
        let expected_count = constructor.variant.payload_fields.len();
        if actual_count != expected_count {
            self.unsupported_expression(
                expr,
                constructor_arity_reason(constructor),
                format!(
                    "{} constructor expects {expected_count} argument, but got {}",
                    constructor.descriptor.diagnostic_name, actual_count
                ),
                Some(JsonValue::object([
                    (
                        "expected_argument_count",
                        JsonValue::Number(expected_count as i64),
                    ),
                    (
                        "actual_argument_count",
                        JsonValue::Number(actual_count as i64),
                    ),
                ])),
            );
        }
    }

    fn lower_constructor_payloads(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        constructor: AdtConstructor<'_>,
        type_context: &mut ConstructorTypeContext<'_>,
    ) -> Vec<CoreExpr> {
        let mut lowered_args = Vec::with_capacity(constructor.variant.payload_fields.len());
        for (index, _) in constructor.variant.payload_fields.iter().enumerate() {
            let payload_type = type_context.payload_type(constructor, index);
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
            type_context.observe_payload(constructor, index, &lowered.ty);
            lowered_args.push(lowered);
        }
        lowered_args
    }
}

fn retag_state_initializer(expr: &mut CoreExpr, expected: &CoreType) {
    expr.ty = expected.clone();
    match (&mut expr.kind, expected) {
        (CoreExprKind::Record(fields), CoreType::Record(expected_fields)) => {
            let expected_by_name = expected_fields
                .iter()
                .map(|(name, ty)| (name.as_str(), ty))
                .collect::<HashMap<_, _>>();
            #[cfg(test)]
            record_state_initializer_lowering_work(|work| {
                work.retag_record_index_entries += expected_by_name.len();
                work.retag_record_lookups += fields.len();
            });
            for field in fields {
                if let Some(expected) = expected_by_name.get(field.name.as_str()) {
                    retag_state_initializer(&mut field.expr, expected);
                }
            }
        }
        (CoreExprKind::List(items), CoreType::Named { name, args })
            if name == "Vec" && args.len() == 1 =>
        {
            for item in items {
                retag_state_initializer(item, &args[0]);
            }
        }
        (CoreExprKind::Match { arms, .. }, _) => {
            for arm in arms {
                retag_state_initializer(&mut arm.expr, expected);
            }
        }
        (CoreExprKind::CleanupRegion { region }, _) => {
            if let Some(statement) = region.statements.last_mut() {
                match &mut statement.kind {
                    CoreStmtKind::Expr { expr } | CoreStmtKind::Return { expr } => {
                        retag_state_initializer(expr, expected);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}
