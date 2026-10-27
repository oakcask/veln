use super::*;
use crate::effect_rows::EffectRowSubstitutions;
use crate::types::ModuleValueLookup;

impl<'a> FunctionChecker<'a> {
    pub(super) fn infer_name_path(
        &mut self,
        segments: &[String],
        expr: &Expr,
        expected: Option<&ExpectedType>,
    ) -> Type {
        if let [name] = segments
            && let Some(ty) = self.infer_local_binding_name(name, expected)
        {
            return ty;
        }
        match self
            .environment
            .module_value_path(segments, self.function.module_name.as_deref())
        {
            ModuleValueLookup::Available(value) => return value.ty.clone(),
            ModuleValueLookup::Unavailable => return Type::Unknown,
            ModuleValueLookup::Missing => {}
        }
        if segments.len() == 1
            && let Some(ty) = self.infer_expected_nullary_constructor(segments, expr, expected)
        {
            return ty;
        }
        match self.environment.adts.nullary_constructor(
            segments,
            self.function.module_name.as_deref(),
            &self.environment.uses,
        ) {
            ConstructorLookup::Found(constructor) => {
                self.infer_nullary_constructor_name(segments, expr, expected, constructor)
            }
            ConstructorLookup::Ambiguous => {
                if let Some(ty) = self.infer_expected_nullary_constructor(segments, expr, expected)
                {
                    return ty;
                }
                self.push_ambiguous_name(
                    expr.node_id,
                    expr.span.clone(),
                    &segments.join("::"),
                    "value",
                );
                Type::Unknown
            }
            ConstructorLookup::Missing => self.infer_non_constructor_name(segments, expr, expected),
        }
    }

    fn infer_expected_nullary_constructor(
        &mut self,
        segments: &[String],
        expr: &Expr,
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        let constructor = expected
            .and_then(|expected| {
                self.environment.adts.descriptor_for_type_prefer_module(
                    &expected.ty,
                    self.function.module_name.as_deref(),
                )
            })
            .and_then(|descriptor| {
                self.environment.adts.constructor_for_descriptor(
                    segments,
                    descriptor,
                    self.function.module_name.as_deref(),
                    &self.environment.uses,
                )
            })
            .filter(|constructor| constructor.variant.payload_fields.is_empty())?;
        Some(self.infer_nullary_constructor_name(segments, expr, expected, constructor))
    }

    fn infer_nullary_constructor_name(
        &mut self,
        segments: &[String],
        expr: &Expr,
        expected: Option<&ExpectedType>,
        constructor: AdtConstructor<'_>,
    ) -> Type {
        let matching_expected = expected
            .filter(|expected| adt::type_matches_descriptor(&expected.ty, constructor.descriptor));
        let inferred_args = matching_expected
            .and_then(|expected| {
                unification::adt_args(&expected.ty, constructor.descriptor).map(<[Type]>::to_vec)
            })
            .unwrap_or_else(|| vec![Type::Unknown; constructor.descriptor.type_parameters.len()]);
        let inferred = adt::refined_constructed_type_from_args(constructor, &inferred_args);
        if type_contains_unknown(&inferred) {
            let inferred_base = adt::constructed_type_from_args(constructor, &inferred_args);
            if (expected.is_none() && constructor.variant.kind != AdtVariantKind::ListNil)
                || (expected.is_some() && constructor.variant.kind == AdtVariantKind::ListNil)
            {
                self.push_ambiguous_constructor_type(
                    expr.node_id,
                    expr.span.clone(),
                    &segments.join("::"),
                    &inferred_base,
                );
            }
            return inferred;
        }
        inferred
    }

    fn infer_non_constructor_name(
        &mut self,
        segments: &[String],
        expr: &Expr,
        expected: Option<&ExpectedType>,
    ) -> Type {
        match segments {
            [name] => self.infer_bare_name(name, expr, expected),
            _ => self.infer_qualified_function_value(segments, expr),
        }
    }

    fn infer_bare_name(
        &mut self,
        name: &str,
        expr: &Expr,
        _expected: Option<&ExpectedType>,
    ) -> Type {
        debug_assert!(self.visible_binding_index(name).is_none());
        if self.bare_prelude_import_is_ambiguous(name) {
            return self.ambiguous_function_value(name, expr);
        }
        match self.environment.unqualified_function_in_owner(
            name,
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) {
            FunctionLookup::Found(function) => function.ty(),
            FunctionLookup::Ambiguous => self.ambiguous_function_value(name, expr),
            FunctionLookup::Missing => self.infer_recovered_bare_name(name, expr),
        }
    }

    fn ambiguous_function_value(&mut self, name: &str, expr: &Expr) -> Type {
        self.push_ambiguous_unqualified_function_import(
            expr.node_id,
            expr.span.clone(),
            name,
            "value",
        );
        Type::Unknown
    }

    fn infer_recovered_bare_name(&mut self, name: &str, expr: &Expr) -> Type {
        if let Some(function) = self.environment.local_function_value_recovery_in_owner(
            name,
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) {
            return function.ty();
        }
        if self
            .environment
            .local_value_recovery_candidate_count_in_owner(
                name,
                self.function.module_name.as_deref(),
                self.function.package_name.as_deref(),
            )
            + self.invalid_local_binding_recovery_count(name)
            == 1
        {
            return Type::Unknown;
        }
        self.push_unresolved_name(expr.node_id, expr.span.clone(), name, "value");
        Type::Unknown
    }

    fn infer_qualified_function_value(&mut self, segments: &[String], expr: &Expr) -> Type {
        if let Some(function) = self.environment.function_path_for_value_in_owner(
            segments,
            self.function.module_name.as_deref(),
            self.function.package_name.as_deref(),
        ) {
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

    pub(super) fn infer_local_binding_name(
        &mut self,
        name: &str,
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        let index = self.visible_binding_index(name)?;
        self.record_defer_capture(index, name);
        let current = self.binding_type(index);
        if matches!(current, Type::Record(ref fields) if fields.is_empty())
            && let Some(expected) = expected
            && expected.ty.dict_parts().is_some()
            && !type_contains_unknown(&expected.ty)
        {
            self.set_binding_type(index, expected.ty.clone());
            return Some(expected.ty.clone());
        }
        if type_contains_unknown(&current)
            && let Some(expected) = expected
            && !type_contains_unknown(&expected.ty)
        {
            if matches!(current, Type::VariantRefinement { .. }) {
                let mut constrained = current.clone();
                adt::merge_type_holes(&mut constrained, &expected.ty);
                self.set_binding_type(index, constrained.clone());
                return Some(constrained);
            }
            self.set_binding_type(index, expected.ty.clone());
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
        if let Some(ty) = self.infer_module_value_call(callee, args) {
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

    fn infer_module_value_call(&mut self, callee: &Expr, args: &[Expr]) -> Option<Type> {
        if let Some(segments) = callee.callee_name_path()
            && !matches!(segments, [name] if self.visible_binding_index(name).is_some())
        {
            let value = match self
                .environment
                .module_value_path(segments, self.function.module_name.as_deref())
            {
                ModuleValueLookup::Available(value) => value,
                ModuleValueLookup::Unavailable => {
                    for arg in args {
                        self.infer_expr(arg, None);
                    }
                    return Some(Type::Unknown);
                }
                ModuleValueLookup::Missing => return None,
            };
            let name = value.declaration.name.as_deref().unwrap_or("<missing>");
            let mut diagnostic = Diagnostic::new(
                "type.not_callable",
                Severity::Error,
                DiagnosticKind::Type,
                format!("module value `{name}` is not callable"),
                Some(callee.span.clone()),
                JsonValue::object([
                    ("phase", JsonValue::string("type_check")),
                    ("node_id", JsonValue::string(callee.node_id.display("name"))),
                    ("constraint", JsonValue::string("call_target")),
                    ("actual", JsonValue::string("module_value")),
                    ("name", JsonValue::string(name)),
                ]),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("module_value_declaration")),
                (
                    "message",
                    JsonValue::string(format!("Module value `{name}` is declared here.")),
                ),
                ("span", span_json(&value.declaration.span)),
            ]));
            self.diagnostics.push(diagnostic);
            for arg in args {
                self.infer_expr(arg, None);
            }
            return Some(Type::Unknown);
        }
        None
    }

    pub(super) fn infer_local_callable_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
    ) -> Option<Type> {
        let ExprKind::NamePath { segments, .. } = &callee.kind else {
            return None;
        };
        let [name] = segments.as_slice() else {
            return None;
        };
        let binding_index = self.visible_binding_index(name)?;
        self.record_defer_capture(binding_index, name);
        let binding = &self.bindings[binding_index];
        let type_origin = binding.type_origin.clone();
        let Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } = self.binding_type(binding_index)
        else {
            return None;
        };
        let origin = CallOrigin {
            node_id: callee.node_id,
            span: callee.span.clone(),
            symbol: name.clone(),
            effects,
            schema_encode: binding.schema_encode_callable,
        };
        let instantiated_effects = self.check_call_arguments_with_origin(
            args,
            &params,
            variadic.as_deref(),
            &origin,
            type_origin.as_ref(),
        );
        self.record_call_effects(expr, &origin, &instantiated_effects);
        Some(*return_type)
    }

    pub(super) fn infer_constructor_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        if let ExprKind::NamePath { segments, .. } = &callee.kind {
            if segments.len() == 1
                && let Some(ty) =
                    self.infer_expected_payload_constructor_call(expr, args, expected, segments)
            {
                return Some(ty);
            }
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
                    if let Some(ty) =
                        self.infer_expected_payload_constructor_call(expr, args, expected, segments)
                    {
                        return Some(ty);
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
                    return self.infer_quarantined_constructor_call(expr, args, expected, segments);
                }
            }
        }
        None
    }

    fn infer_expected_payload_constructor_call(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
        segments: &[String],
    ) -> Option<Type> {
        let constructor = expected
            .and_then(|expected| {
                self.environment.adts.descriptor_for_type_prefer_module(
                    &expected.ty,
                    self.function.module_name.as_deref(),
                )
            })
            .and_then(|descriptor| {
                self.environment.adts.constructor_for_descriptor(
                    segments,
                    descriptor,
                    self.function.module_name.as_deref(),
                    &self.environment.uses,
                )
            })
            .filter(|constructor| !constructor.variant.payload_fields.is_empty())?;
        Some(self.infer_adt_constructor(expr, args, expected, constructor))
    }

    fn infer_quarantined_constructor_call(
        &mut self,
        expr: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
        segments: &[String],
    ) -> Option<Type> {
        let constructor = self.environment.quarantined_explicit_import_constructor(
            segments,
            self.function.module_name.as_deref(),
        )?;
        if args.len() != constructor.variant.payload_fields.len() {
            self.diagnostics
                .push(quarantined_constructor_arity_diagnostic(
                    expr,
                    constructor,
                    args.len(),
                ));
        }
        Some(self.infer_adt_constructor(expr, args, expected, constructor))
    }

    pub(super) fn infer_declared_call(
        &mut self,
        expr: &Expr,
        callee: &Expr,
        args: &[Expr],
        expected: Option<&ExpectedType>,
    ) -> Option<Type> {
        if self.bare_call_is_ambiguous(callee) {
            if let ExprKind::NamePath { segments, .. } = &callee.kind
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
        )?;

        let instantiated_effects =
            self.check_call_arguments(args, &params, variadic.as_ref(), &origin);

        self.record_call_effects(expr, &origin, &instantiated_effects);
        Some(return_type)
    }

    fn record_call_effects(
        &mut self,
        expr: &Expr,
        origin: &CallOrigin,
        instantiated_effects: &EffectRowSubstitutions,
    ) {
        for effect in &instantiate_effect_rows(&origin.effects, instantiated_effects) {
            self.inferred_effects.push(EffectUse {
                effect: effect.clone(),
                node_id: expr.node_id,
                span: expr.span.clone(),
                kind: "direct_call",
                symbol: origin.symbol.clone(),
            });
        }
    }

    pub(super) fn declared_call_is_standard_prelude(&self, callee: &Expr) -> bool {
        if self.function.module_name.as_deref() == Some("std::prelude") {
            return false;
        }
        let ExprKind::NamePath { segments, .. } = &callee.kind else {
            return false;
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
        };
        function.is_some_and(|function| {
            function.module_name.as_deref() == Some("std::prelude")
                && crate::prelude::prelude_signature(&function.name, None).is_some()
        })
    }

    pub(super) fn check_call_arguments(
        &mut self,
        args: &[Expr],
        params: &[Type],
        variadic: Option<&Type>,
        origin: &CallOrigin,
    ) -> Vec<(String, Vec<String>)> {
        self.check_call_arguments_with_origin(args, params, variadic, origin, None)
    }

    fn check_call_arguments_with_origin(
        &mut self,
        args: &[Expr],
        params: &[Type],
        variadic: Option<&Type>,
        origin: &CallOrigin,
        type_origin: Option<&TypeOrigin>,
    ) -> Vec<(String, Vec<String>)> {
        let mut row_substitutions = Vec::<(String, Vec<String>)>::new();
        for (index, arg) in args.iter().enumerate() {
            let param_type = params.get(index).or(variadic);
            let Some(param_type) = param_type else {
                self.infer_expr(arg, None);
                continue;
            };
            let expected = type_origin.map_or_else(
                || ExpectedType {
                    ty: param_type.clone(),
                    source: if origin.schema_encode {
                        ExpectedTypeSource::SchemaEncodeParameter
                    } else {
                        ExpectedTypeSource::DeclaredParameter
                    },
                    origin_node_id: origin.node_id,
                    origin_span: Some(origin.span.clone()),
                    origin_message: "Callee parameter type declared here.",
                },
                |type_origin| ExpectedType {
                    ty: param_type.clone(),
                    source: type_origin.source,
                    origin_node_id: type_origin.node_id,
                    origin_span: Some(type_origin.span.clone()),
                    origin_message: type_origin.message,
                },
            );
            let actual = self.infer_expr(arg, Some(&expected));
            collect_effect_row_substitution(param_type, &actual, &mut row_substitutions);
            if origin.schema_encode {
                self.check_schema_encode_assignable(
                    arg,
                    &expected.ty,
                    &actual,
                    &expected,
                    "call_argument",
                );
            } else {
                self.check_assignable(arg, &expected.ty, &actual, &expected, "call_argument");
            }
        }
        row_substitutions
    }
}
