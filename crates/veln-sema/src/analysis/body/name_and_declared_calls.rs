use super::*;

impl<'a> FunctionChecker<'a> {
    pub(super) fn infer_name_path(
        &mut self,
        segments: &[String],
        expr: &Expr,
        expected: Option<&ExpectedType>,
    ) -> Type {
        match self.environment.adts.nullary_constructor(
            segments,
            self.function.module_name.as_deref(),
            &self.environment.uses,
        ) {
            ConstructorLookup::Found(constructor) => {
                let inferred = expected
                    .and_then(|expected| {
                        adt::adt_args(&expected.ty, constructor.descriptor)
                            .map(|_| expected.ty.clone())
                    })
                    .unwrap_or_else(|| adt::constructed_type(constructor, &[]));
                if type_contains_unknown(&inferred)
                    && ((expected.is_none() && constructor.variant.kind != AdtVariantKind::ListNil)
                        || (expected.is_some()
                            && constructor.variant.kind == AdtVariantKind::ListNil))
                {
                    self.push_ambiguous_constructor_type(
                        expr.node_id,
                        expr.span.clone(),
                        &segments.join("::"),
                        &inferred,
                    );
                }
                inferred
            }
            ConstructorLookup::Ambiguous => {
                self.push_ambiguous_name(
                    expr.node_id,
                    expr.span.clone(),
                    &segments.join("::"),
                    "value",
                );
                Type::Unknown
            }
            ConstructorLookup::Missing => match segments {
                [name] => {
                    if let Some(ty) = self.infer_local_binding_name(name, expected) {
                        ty
                    } else if self.bare_prelude_import_is_ambiguous(name) {
                        self.push_ambiguous_unqualified_function_import(
                            expr.node_id,
                            expr.span.clone(),
                            name,
                            "value",
                        );
                        Type::Unknown
                    } else {
                        match self
                            .environment
                            .unqualified_function(name, self.function.module_name.as_deref())
                        {
                            FunctionLookup::Found(function) => function.ty(),
                            FunctionLookup::Ambiguous => {
                                self.push_ambiguous_unqualified_function_import(
                                    expr.node_id,
                                    expr.span.clone(),
                                    name,
                                    "value",
                                );
                                Type::Unknown
                            }
                            FunctionLookup::Missing => {
                                if let Some(function) =
                                    self.environment.local_function_value_recovery(
                                        name,
                                        self.function.module_name.as_deref(),
                                    )
                                {
                                    function.ty()
                                } else if self.environment.local_value_recovery_candidate_count(
                                    name,
                                    self.function.module_name.as_deref(),
                                ) + self.invalid_local_binding_recovery_count(name)
                                    == 1
                                {
                                    Type::Unknown
                                } else {
                                    self.push_unresolved_name(
                                        expr.node_id,
                                        expr.span.clone(),
                                        name,
                                        "value",
                                    );
                                    Type::Unknown
                                }
                            }
                        }
                    }
                }
                _ => {
                    if let Some(function) = self
                        .environment
                        .function_path_for_value(segments, self.function.module_name.as_deref())
                    {
                        return function.ty();
                    }
                    if self
                        .environment
                        .quarantined_import_value_recovery_candidate_count(
                            segments,
                            self.function.module_name.as_deref(),
                        )
                        == 1
                    {
                        return Type::Unknown;
                    }
                    let symbol = segments.join("::");
                    self.push_unresolved_name(expr.node_id, expr.span.clone(), &symbol, "value");
                    Type::Unknown
                }
            },
        }
    }

    pub(super) fn infer_local_binding_name(
        &mut self,
        name: &str,
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        let index = self
            .bindings
            .iter()
            .rposition(|binding| binding.name == name)?;
        let current = self.bindings[index].ty.clone();
        if matches!(current, Type::Record(ref fields) if fields.is_empty())
            && let Some(expected) = expected
            && expected.ty.dict_parts().is_some()
            && !type_contains_unknown(&expected.ty)
        {
            self.bindings[index].ty = expected.ty.clone();
            return Some(expected.ty.clone());
        }
        if type_contains_unknown(&current)
            && let Some(expected) = expected
            && !type_contains_unknown(&expected.ty)
        {
            self.bindings[index].ty = expected.ty.clone();
            return Some(expected.ty.clone());
        }
        Some(current)
    }

    pub(super) fn infer_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Type {
        if let Some(ty) = self.infer_local_callable_call(expr, callee, args) {
            return ty;
        }
        if let Some(ty) = self.infer_constructor_call(expr, callee, args, expected) {
            return ty;
        }
        if let Some(ty) = self.infer_declared_call(expr, callee, args, expected) {
            return ty;
        }
        if let Some(ty) = self.infer_prelude_call(callee, args, expected) {
            return ty;
        }
        if let Some(ty) = self.diagnose_method_call(expr, callee, args) {
            return ty;
        }
        self.infer_unresolved_call(callee, args)
    }

    pub(super) fn infer_local_callable_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
    ) -> Option<Type> {
        let ExprKind::NamePath(segments) = &callee.kind else {
            return None;
        };
        let [name] = segments.as_slice() else {
            return None;
        };
        let binding = self
            .bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)?;
        let Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } = binding.ty.clone()
        else {
            return None;
        };
        let origin = CallOrigin {
            node_id: callee.node_id,
            span: callee.span.clone(),
            symbol: name.clone(),
            effects,
        };
        let instantiated_effects =
            self.check_call_arguments(args, &params, variadic.as_deref(), &origin);
        for effect in &instantiate_effects(&origin.effects, &instantiated_effects) {
            self.inferred_effects.push(EffectUse {
                effect: effect.clone(),
                node_id: expr.node_id,
                span: expr.span.clone(),
                kind: "direct_call",
                symbol: origin.symbol.clone(),
            });
        }
        Some(*return_type)
    }

    pub(super) fn infer_constructor_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        if let ExprKind::NamePath(segments) = &callee.kind {
            match self.environment.adts.constructor(
                segments,
                self.function.module_name.as_deref(),
                &self.environment.uses,
            ) {
                ConstructorLookup::Found(constructor)
                    if !constructor.variant.payload_fields.is_empty() =>
                {
                    return Some(self.infer_adt_constructor(expr, args, expected, constructor));
                }
                ConstructorLookup::Found(_) => {}
                ConstructorLookup::Ambiguous => {
                    if let Some(constructor) = expected
                        .and_then(|expected| {
                            self.environment.adts.descriptor_for_type(&expected.ty)
                        })
                        .and_then(|descriptor| {
                            self.environment.adts.constructor_for_descriptor(
                                segments,
                                descriptor,
                                self.function.module_name.as_deref(),
                                &self.environment.uses,
                            )
                        })
                        .filter(|constructor| !constructor.variant.payload_fields.is_empty())
                    {
                        return Some(self.infer_adt_constructor(expr, args, expected, constructor));
                    }
                    self.push_ambiguous_name(
                        callee.node_id,
                        callee.span.clone(),
                        &segments.join("::"),
                        "call_target",
                    );
                    return Some(Type::Unknown);
                }
                ConstructorLookup::Missing => {
                    if self
                        .environment
                        .quarantined_import_constructor_recovery_candidate_count(
                            segments,
                            self.function.module_name.as_deref(),
                            Some(args.len()),
                        )
                        == 1
                    {
                        for arg in args {
                            self.infer_expr(arg, None);
                        }
                        return Some(Type::Unknown);
                    }
                }
            }
        }
        None
    }

    pub(super) fn infer_declared_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        if self.bare_call_is_ambiguous(callee) {
            if let ExprKind::NamePath(segments) = &callee.kind
                && let [name] = segments.as_slice()
            {
                self.push_ambiguous_unqualified_function_import(
                    callee.node_id,
                    callee.span.clone(),
                    name,
                    "call_target",
                );
            }
            for arg in args {
                self.infer_expr(arg, None);
            }
            return Some(Type::Unknown);
        }
        if self.declared_call_is_standard_prelude(callee) {
            return None;
        }

        let (params, variadic, return_type, origin) = self.call_signature(
            callee,
            expected.map(|expected| &expected.ty),
            args.first()
                .and_then(|arg| self.shallow_expr_type(arg))
                .as_ref(),
            Some(args.len()),
        )?;

        let instantiated_effects =
            self.check_call_arguments(args, &params, variadic.as_ref(), &origin);

        for effect in &instantiate_effects(&origin.effects, &instantiated_effects) {
            self.inferred_effects.push(EffectUse {
                effect: effect.clone(),
                node_id: expr.node_id,
                span: expr.span.clone(),
                kind: "direct_call",
                symbol: origin.symbol.clone(),
            });
        }
        Some(return_type)
    }

    pub(super) fn declared_call_is_standard_prelude(&self, callee: &Expr) -> bool {
        if self.function.module_name.as_deref() == Some("std::prelude") {
            return false;
        }
        let ExprKind::NamePath(segments) = &callee.kind else {
            return false;
        };
        let function = match segments.as_slice() {
            [name] => self
                .environment
                .unqualified_function(name, self.function.module_name.as_deref())
                .found(),
            _ => self
                .environment
                .function_path(segments, self.function.module_name.as_deref()),
        };
        function.is_some_and(|function| function.module_name.as_deref() == Some("std::prelude"))
    }

    pub(super) fn check_call_arguments(
        &mut self,
        args: &[Expr],
        params: &[Type],
        variadic: Option<&Type>,
        origin: &CallOrigin,
    ) -> Vec<(String, Vec<String>)> {
        let mut row_substitutions = Vec::<(String, Vec<String>)>::new();
        for (index, arg) in args.iter().enumerate() {
            let param_type = params.get(index).or(variadic);
            let Some(param_type) = param_type else {
                self.infer_expr(arg, None);
                continue;
            };
            let expected = ExpectedType {
                ty: param_type.clone(),
                source: ExpectedTypeSource::DeclaredParameter,
                origin_node_id: origin.node_id,
                origin_span: Some(origin.span.clone()),
                origin_message: "Callee parameter type declared here.",
            };
            let actual = self.infer_expr(arg, Some(&expected));
            collect_effect_row_substitution(param_type, &actual, &mut row_substitutions);
            self.check_assignable(arg, &expected.ty, &actual, &expected, "call_argument");
        }
        row_substitutions
    }
}
