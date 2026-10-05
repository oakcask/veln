use super::*;

pub(crate) fn infer_private_signature_expr_type(
    expr: &Expr,
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &mut PrivateBindings,
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    let mut failures = 0;
    infer_private_signature_expr_type_with_failures(
        expr,
        expected,
        current_module,
        uses,
        bindings,
        None,
        returns_by_path,
        adts,
        &mut failures,
    )
    .ty
}

#[derive(Clone, Debug)]
pub(super) struct PrivateExprInference {
    pub(super) ty: Type,
    pub(super) may_contribute: bool,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn infer_private_signature_expr_type_with_failures(
    expr: &Expr,
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &mut PrivateBindings,
    signatures_by_path: Option<&FunctionSignatureMap>,
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
    failures: &mut usize,
) -> PrivateExprInference {
    let mut context = PrivateSignatureInferContext {
        current_module,
        uses,
        bindings,
        signatures_by_path,
        returns_by_path,
        adts,
        failures,
    };
    if let Some(ty) = infer_private_leaf_type(expr) {
        return PrivateExprInference::successful(ty);
    }
    if let Some(ty) = infer_private_branch_type(expr, expected, &mut context) {
        return PrivateExprInference::successful(ty);
    }
    if let Some(outcome) = infer_private_value_type(expr, expected, &mut context) {
        return outcome;
    }
    if let Some(ty) = infer_private_schema_type(expr, expected, &mut context) {
        return PrivateExprInference::successful(ty);
    }
    match &expr.kind {
        ExprKind::Prefix { expr, .. } => {
            context.infer(expr, expected);
            PrivateExprInference::successful(Type::Unknown)
        }
        ExprKind::Binary { op, left, right } => PrivateExprInference::successful(
            infer_private_binary_type(*op, left, right, expected, &mut context),
        ),
        _ => unreachable!("delegated expressions return before operator inference"),
    }
}

impl PrivateExprInference {
    pub(super) fn successful(ty: Type) -> Self {
        Self {
            ty,
            may_contribute: true,
        }
    }
}

fn infer_private_leaf_type(expr: &Expr) -> Option<Type> {
    match &expr.kind {
        ExprKind::Missing | ExprKind::Hole { .. } | ExprKind::TypeApply { .. } => {
            Some(Type::Unknown)
        }
        ExprKind::StringLiteral(_) => Some(Type::string()),
        ExprKind::IntLiteral(_) => Some(Type::int()),
        ExprKind::FloatLiteral(_) => Some(Type::float()),
        ExprKind::BoolLiteral(_) => Some(Type::bool()),
        ExprKind::Unit => Some(Type::unit()),
        _ => None,
    }
}

fn infer_private_branch_type(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Option<Type> {
    match &expr.kind {
        ExprKind::Match { scrutinee, arms } => {
            Some(infer_private_match_type(scrutinee, arms, expected, context))
        }
        ExprKind::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => Some(infer_private_if_result_type(
            then_branch,
            else_if_branches,
            else_branch,
            expected,
            context,
        )),
        ExprKind::Begin { body, .. } => Some(context.infer_body(body, expected)),
        _ => None,
    }
}

fn infer_private_value_type(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Option<PrivateExprInference> {
    match &expr.kind {
        ExprKind::NamePath { segments, .. } => Some(PrivateExprInference::successful(
            infer_private_signature_name_type(
                segments,
                expected,
                context.current_module,
                context.uses,
                context.bindings,
                context.returns_by_path,
                context.adts,
            ),
        )),
        ExprKind::List(items) => Some(infer_private_list_type(items, expected, context)),
        ExprKind::Dict(entries) => Some(infer_private_dict_type(entries, expected, context)),
        ExprKind::Record(fields) => Some(PrivateExprInference::successful(
            infer_private_record_type(fields, expected, context),
        )),
        ExprKind::Call { callee, args } => Some(PrivateExprInference::successful(
            infer_private_signature_call_type(callee, args, expected, context),
        )),
        ExprKind::Perform { args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            Some(PrivateExprInference::successful(Type::Unknown))
        }
        ExprKind::Handle { body, args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            Some(PrivateExprInference::successful(
                context.infer(body, expected),
            ))
        }
        _ => None,
    }
}

fn infer_private_schema_type(
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Option<Type> {
    match &expr.kind {
        ExprKind::SchemaDecode { input, base, .. } => {
            context.infer(input, Some(&Type::named("ByteView", Vec::new())));
            context.infer(base, Some(&Type::named("ByteOffset", Vec::new())));
            Some(Type::Unknown)
        }
        ExprKind::SchemaEncode { value, .. } => {
            context.infer(value, None);
            Some(Type::Unknown)
        }
        ExprKind::FieldAccess { base, field, .. } => Some(
            context
                .infer(base, None)
                .record_field(field)
                .cloned()
                .unwrap_or(Type::Unknown),
        ),
        ExprKind::Try { expr: inner, .. } => Some(expected.cloned().unwrap_or_else(|| {
            let inner_type = context.infer(inner, None);
            adt::result_parts(&inner_type).map_or(Type::Unknown, |(value, _)| value.clone())
        })),
        _ => None,
    }
}

pub(super) fn infer_private_list_type(
    items: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> PrivateExprInference {
    let failure_count = *context.failures;
    let mut item_type = expected
        .and_then(Type::vec_part)
        .cloned()
        .unwrap_or(Type::Unknown);
    let mut joined_items = None;
    for item in items {
        let has_context = expected.and_then(Type::vec_part).is_some();
        infer_private_aggregate_component(
            item,
            has_context,
            &mut item_type,
            &mut joined_items,
            context,
        );
    }
    if let Some(joined) = joined_items {
        item_type = joined.result_type();
    }
    PrivateExprInference {
        ty: Type::vec(item_type),
        may_contribute: *context.failures == failure_count,
    }
}

pub(super) fn infer_private_dict_type(
    entries: &[DictEntry],
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> PrivateExprInference {
    let failure_count = *context.failures;
    let (mut key_type, mut value_type) = expected
        .and_then(Type::dict_parts)
        .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
            (key.clone(), value.clone())
        });
    let mut joined_keys = None;
    let mut joined_values = None;
    for entry in entries {
        let has_key_context = expected.and_then(Type::dict_parts).is_some();
        infer_private_aggregate_component(
            &entry.key,
            has_key_context,
            &mut key_type,
            &mut joined_keys,
            context,
        );
        infer_private_aggregate_component(
            &entry.value,
            has_key_context,
            &mut value_type,
            &mut joined_values,
            context,
        );
    }
    if let Some(joined) = joined_keys {
        key_type = joined.result_type();
    }
    if let Some(joined) = joined_values {
        value_type = joined.result_type();
    }
    PrivateExprInference {
        ty: Type::dict(key_type, value_type),
        may_contribute: *context.failures == failure_count,
    }
}

fn infer_private_aggregate_component(
    expr: &Expr,
    has_context: bool,
    component_type: &mut Type,
    joined_components: &mut Option<crate::aggregate_type_join::AggregateTypeJoin>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) {
    let inferred_context = joined_components
        .as_ref()
        .map(crate::aggregate_type_join::AggregateTypeJoin::inference_type)
        .unwrap_or_else(|| component_type.clone());
    let actual = context.infer_outcome(expr, item_type_unknown_as_none(&inferred_context));
    if !actual.may_contribute {
        return;
    }
    let actual = actual.ty;
    let actual = if has_context {
        inferred_private_aggregate_member_type(actual, component_type)
    } else {
        actual
    };
    if !has_context && joined_components.is_none() && *component_type != Type::Unknown {
        *joined_components =
            crate::aggregate_type_join::AggregateTypeJoin::new(context.adts, component_type);
    }
    let joined = !has_context
        && joined_components
            .as_mut()
            .is_some_and(|joined| joined.try_join(&actual));
    let rejected_here = inferred_context != Type::Unknown
        && !crate::type_relations::is_assignable_nested(&inferred_context, &actual)
        && (has_context || !joined);
    if rejected_here {
        *context.failures += 1;
        return;
    }
    if !joined && *component_type == Type::Unknown {
        *component_type = actual;
        *joined_components = (!has_context)
            .then(|| {
                crate::aggregate_type_join::AggregateTypeJoin::new(context.adts, component_type)
            })
            .flatten();
    }
}

pub(crate) fn infer_private_record_type(
    fields: &[RecordField],
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    if fields.is_empty()
        && let Some(expected) = expected
        && expected.dict_parts().is_some()
    {
        return expected.clone();
    }
    Type::Record(
        fields
            .iter()
            .map(|field| {
                let field_expected =
                    expected.and_then(|expected| expected.record_field(&field.name));
                let actual = context.infer(&field.expr, field_expected);
                let actual = match field_expected {
                    Some(expected) => inferred_private_aggregate_member_type(actual, expected),
                    None => actual,
                };
                (field.name.clone(), actual)
            })
            .collect(),
    )
}

pub(super) fn inferred_private_aggregate_member_type(ty: Type, expected: &Type) -> Type {
    if expected != &Type::Unknown && crate::type_relations::is_assignable(expected, &ty) {
        expected.clone()
    } else {
        ty
    }
}

pub(crate) fn infer_private_match_type(
    scrutinee: &Expr,
    arms: &[MatchArm],
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    let scrutinee_expected = match infer_match_scrutinee_type_from_constructor_patterns(
        arms,
        context.current_module,
        context.uses,
        context.adts,
    ) {
        MatchScrutineePatternInference::Inferred(ty) => Some(ty),
        MatchScrutineePatternInference::Uninferred
        | MatchScrutineePatternInference::Ambiguous(_) => None,
    };
    context.infer(scrutinee, scrutinee_expected.as_ref());
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    let mut joined_result = None;
    let mut join_failed = false;
    for arm in arms {
        let recovery_expected = joined_result.is_none().then_some(&result);
        let actual = context.infer(
            &arm.expr,
            expected.or_else(|| recovery_expected.and_then(item_type_unknown_as_none)),
        );
        merge_private_control_flow_result(
            &mut result,
            &mut joined_result,
            &mut join_failed,
            actual,
            context.adts,
            expected.is_none(),
        );
    }
    materialize_private_control_flow_result(result, joined_result, join_failed)
}

pub(crate) fn infer_private_if_result_type(
    then_branch: &Expr,
    else_if_branches: &[IfBranch],
    else_branch: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    let mut joined_result = None;
    let mut join_failed = false;
    for branch_expr in std::iter::once(then_branch)
        .chain(else_if_branches.iter().map(|branch| &branch.expr))
        .chain(std::iter::once(else_branch))
    {
        let recovery_expected = joined_result.is_none().then_some(&result);
        let actual = context.infer(
            branch_expr,
            expected.or_else(|| recovery_expected.and_then(item_type_unknown_as_none)),
        );
        merge_private_control_flow_result(
            &mut result,
            &mut joined_result,
            &mut join_failed,
            actual,
            context.adts,
            expected.is_none(),
        );
    }
    materialize_private_control_flow_result(result, joined_result, join_failed)
}

fn merge_private_control_flow_result(
    result: &mut Type,
    joined_result: &mut Option<crate::aggregate_type_join::AggregateTypeJoin>,
    join_failed: &mut bool,
    actual: Type,
    adts: &AdtRegistry,
    allow_join: bool,
) {
    if *result == Type::Unknown {
        *joined_result = crate::aggregate_type_join::AggregateTypeJoin::new_resolved(adts, &actual);
        *result = private_control_flow_recovery_type(&actual);
        return;
    }
    if actual == Type::Unknown {
        return;
    }
    if !allow_join {
        merge_expected_private_control_flow_result(result, &actual);
        return;
    }
    if *join_failed {
        return;
    }
    if joined_result.is_none() && *result == actual {
        return;
    }
    if let Some(joined) = joined_result.as_mut()
        && joined.try_join_resolved(&actual)
    {
        return;
    }
    *join_failed = true;
    *joined_result = None;
}

fn private_control_flow_recovery_type(ty: &Type) -> Type {
    match ty {
        Type::VariantRefinement {
            name,
            identity,
            args,
            ..
        } => Type::resolved_named(name, identity, args.clone()),
        ty => ty.clone(),
    }
}

fn materialize_private_control_flow_result(
    recovery_type: Type,
    joined_result: Option<crate::aggregate_type_join::AggregateTypeJoin>,
    join_failed: bool,
) -> Type {
    if join_failed {
        recovery_type
    } else {
        joined_result
            .as_ref()
            .map(crate::aggregate_type_join::AggregateTypeJoin::result_type)
            .unwrap_or(recovery_type)
    }
}

fn merge_expected_private_control_flow_result(result: &mut Type, actual: &Type) {
    let common_base = match (&*result, actual) {
        (
            Type::VariantRefinement {
                name,
                identity,
                args,
                ..
            },
            Type::VariantRefinement {
                identity: actual_identity,
                args: actual_args,
                ..
            }
            | Type::Named {
                identity: actual_identity,
                args: actual_args,
                ..
            },
        ) if identity == actual_identity && args == actual_args => {
            Some(Type::resolved_named(name, identity, args.clone()))
        }
        (
            Type::Named {
                name,
                identity,
                args,
            },
            Type::VariantRefinement {
                identity: actual_identity,
                args: actual_args,
                ..
            },
        ) if identity == actual_identity && args == actual_args => {
            Some(Type::resolved_named(name, identity, args.clone()))
        }
        _ => None,
    };
    if let Some(common_base) = common_base {
        *result = common_base;
    }
}

pub(crate) fn infer_private_binary_type(
    op: veln_ast::BinaryOp,
    left: &Expr,
    right: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    match op {
        veln_ast::BinaryOp::Equal
        | veln_ast::BinaryOp::NotEqual
        | veln_ast::BinaryOp::Less
        | veln_ast::BinaryOp::LessEqual
        | veln_ast::BinaryOp::Greater
        | veln_ast::BinaryOp::GreaterEqual
        | veln_ast::BinaryOp::Or
        | veln_ast::BinaryOp::And => Type::bool(),
        veln_ast::BinaryOp::BitwiseOr
        | veln_ast::BinaryOp::BitwiseXor
        | veln_ast::BinaryOp::BitwiseAnd
        | veln_ast::BinaryOp::ShiftLeft
        | veln_ast::BinaryOp::ShiftRight
        | veln_ast::BinaryOp::ShiftRightLogical => Type::int(),
        veln_ast::BinaryOp::Add
        | veln_ast::BinaryOp::Subtract
        | veln_ast::BinaryOp::Multiply
        | veln_ast::BinaryOp::Divide => {
            let left = context.infer(left, expected);
            let right = context.infer(right, expected);
            if left == Type::float() || right == Type::float() {
                Type::float()
            } else {
                Type::int()
            }
        }
        veln_ast::BinaryOp::PipeGreater => Type::Unknown,
    }
}

pub(crate) fn item_type_unknown_as_none(ty: &Type) -> Option<&Type> {
    (ty != &Type::Unknown).then_some(ty)
}

pub(crate) fn infer_private_signature_name_type(
    segments: &[String],
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
    adts: &AdtRegistry,
) -> Type {
    if let Some(constructor) =
        private_nullary_constructor(segments, expected, current_module, uses, adts)
    {
        let args = expected
            .and_then(|expected| {
                unification::adt_args(expected, constructor.descriptor).map(<[Type]>::to_vec)
            })
            .unwrap_or_else(|| vec![Type::Unknown; constructor.descriptor.type_parameters.len()]);
        return adt::refined_constructed_type_from_args(constructor, &args);
    }
    private_name_value_type(segments, current_module, uses, bindings, returns_by_path)
}

fn private_nullary_constructor<'a>(
    segments: &[String],
    expected: Option<&Type>,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &'a AdtRegistry,
) -> Option<AdtConstructor<'a>> {
    let expected_constructor = (segments.len() == 1)
        .then_some(expected)
        .flatten()
        .and_then(|expected| adts.descriptor_for_type_prefer_module(expected, current_module))
        .and_then(|descriptor| {
            adts.constructor_for_descriptor(segments, descriptor, current_module, uses)
        })
        .filter(|constructor| constructor.variant.payload_fields.is_empty());
    expected_constructor.or_else(|| {
        match adts.nullary_constructor(segments, current_module, uses) {
            ConstructorLookup::Found(constructor) => Some(constructor),
            ConstructorLookup::Ambiguous => expected
                .and_then(|expected| {
                    adts.descriptor_for_type_prefer_module(expected, current_module)
                })
                .and_then(|descriptor| {
                    adts.constructor_for_descriptor(segments, descriptor, current_module, uses)
                })
                .filter(|constructor| constructor.variant.payload_fields.is_empty()),
            ConstructorLookup::Missing => None,
        }
    })
}

fn private_name_value_type(
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<(Option<String>, String), Type>,
) -> Type {
    match segments {
        [name] => bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
            .map(|binding| binding.ty.clone())
            .or_else(|| {
                returns_by_path
                    .get(&(current_module.map(str::to_string), name.clone()))
                    .cloned()
            })
            .unwrap_or(Type::Unknown),
        [_, .., name] => {
            imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                .and_then(|use_decl| {
                    returns_by_path
                        .get(&(Some(use_decl.name.clone()), name.clone()))
                        .cloned()
                })
                .unwrap_or(Type::Unknown)
        }
        _ => Type::Unknown,
    }
}

pub(crate) struct PrivateSignatureInferContext<'a, 'f> {
    pub(crate) current_module: Option<&'a str>,
    pub(crate) uses: &'a [UseDecl],
    pub(crate) bindings: &'a mut PrivateBindings,
    pub(crate) signatures_by_path: Option<&'a FunctionSignatureMap>,
    pub(crate) returns_by_path: &'a BTreeMap<(Option<String>, String), Type>,
    pub(crate) adts: &'a AdtRegistry,
    pub(crate) failures: &'f mut usize,
}

impl PrivateSignatureInferContext<'_, '_> {
    pub(crate) fn infer(&mut self, expr: &Expr, expected: Option<&Type>) -> Type {
        self.infer_outcome(expr, expected).ty
    }

    pub(super) fn infer_outcome(
        &mut self,
        expr: &Expr,
        expected: Option<&Type>,
    ) -> PrivateExprInference {
        let failure_count = *self.failures;
        let mut outcome = infer_private_signature_expr_type_with_failures(
            expr,
            expected,
            self.current_module,
            self.uses,
            self.bindings,
            self.signatures_by_path,
            self.returns_by_path,
            self.adts,
            self.failures,
        );
        outcome.may_contribute = *self.failures == failure_count;
        outcome
    }

    pub(super) fn infer_expected_outcome(
        &mut self,
        expr: &Expr,
        expected: &Type,
    ) -> PrivateExprInference {
        let mut outcome = self.infer_outcome(expr, Some(expected));
        let rejected_here = outcome.ty != Type::Unknown
            && !crate::type_relations::is_assignable(expected, &outcome.ty);
        if rejected_here && outcome.may_contribute {
            *self.failures += 1;
        }
        outcome.may_contribute &= !rejected_here;
        outcome
    }

    pub(super) fn infer_nested_expected_outcome(
        &mut self,
        expr: &Expr,
        expected: &Type,
    ) -> PrivateExprInference {
        let mut outcome = self.infer_outcome(expr, Some(expected));
        let rejected_here = outcome.ty != Type::Unknown
            && !crate::type_relations::is_assignable_nested(expected, &outcome.ty);
        if rejected_here && outcome.may_contribute {
            *self.failures += 1;
        }
        outcome.may_contribute &= !rejected_here;
        outcome
    }

    pub(super) fn infer_body(&mut self, body: &[BodyLine], expected: Option<&Type>) -> Type {
        let binding_count = self.bindings.len();
        record_scoped_binding_count(self.bindings);
        let outcome = infer_private_body_type(
            body,
            expected,
            self.current_module,
            self.uses,
            self.bindings,
            self.signatures_by_path,
            self.returns_by_path,
            self.adts,
            self.failures,
        );
        self.bindings.truncate(binding_count);
        outcome.ty
    }
}
