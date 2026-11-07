use super::*;

#[allow(clippy::too_many_arguments)]
pub(crate) fn infer_private_signature_expr_type(
    expr: &Expr,
    expected: Option<&Type>,
    current_module: Option<&str>,
    current_package: Option<&str>,
    uses: &[UseDecl],
    bindings: &mut PrivateBindings,
    returns_by_path: &BTreeMap<FunctionKey, Type>,
    adts: &AdtRegistry,
) -> Type {
    let mut failures = 0;
    infer_private_signature_expr_type_with_failures(
        expr,
        expected,
        current_module,
        current_package,
        uses,
        bindings,
        None,
        returns_by_path,
        adts,
        &mut failures,
    )
    .ty
}

pub(crate) struct CheckedEffectArgumentTypes {
    signatures: FunctionSignatureMap,
    returns: FunctionReturnMap,
}

impl CheckedEffectArgumentTypes {
    pub(crate) fn new(functions: &[FunctionSignature]) -> Self {
        #[cfg(test)]
        crate::types::effect_inference_counters::record_argument_signature_copies(functions.len());
        Self {
            signatures: signatures_by_path(functions),
            returns: returns_by_path(functions),
        }
    }

    pub(crate) fn update_effects(&mut self, key: &FunctionKey, effects: &[String]) {
        if let Some(signature) = self.signatures.get_mut(key) {
            signature.effects = effects.to_vec();
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn infer_checked_effect_argument_type(
    expr: &Expr,
    expected: &Type,
    current_module: Option<&str>,
    current_package: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    argument_types: &CheckedEffectArgumentTypes,
    adts: &AdtRegistry,
) -> Option<Type> {
    let mut bindings = PrivateBindings::without_builtin(bindings.to_vec());
    let mut failures = 0;
    let outcome = infer_private_signature_expr_type_with_failures(
        expr,
        Some(expected),
        current_module,
        current_package,
        uses,
        &mut bindings,
        Some(&argument_types.signatures),
        &argument_types.returns,
        adts,
        &mut failures,
    );
    (failures == 0 && outcome.may_contribute && outcome.ty != Type::Unknown).then_some(outcome.ty)
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
    current_package: Option<&str>,
    uses: &[UseDecl],
    bindings: &mut PrivateBindings,
    signatures_by_path: Option<&FunctionSignatureMap>,
    returns_by_path: &BTreeMap<FunctionKey, Type>,
    adts: &AdtRegistry,
    failures: &mut usize,
) -> PrivateExprInference {
    infer_private_signature_expr_type_with_expected_kind(
        expr,
        expected,
        current_module,
        current_package,
        uses,
        bindings,
        signatures_by_path,
        returns_by_path,
        adts,
        failures,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
fn infer_private_signature_expr_type_with_expected_kind(
    expr: &Expr,
    expected: Option<&Type>,
    current_module: Option<&str>,
    current_package: Option<&str>,
    uses: &[UseDecl],
    bindings: &mut PrivateBindings,
    signatures_by_path: Option<&FunctionSignatureMap>,
    returns_by_path: &BTreeMap<FunctionKey, Type>,
    adts: &AdtRegistry,
    failures: &mut usize,
    expected_is_admitted: bool,
) -> PrivateExprInference {
    let mut context = PrivateSignatureInferContext {
        current_module,
        current_package,
        uses,
        bindings,
        signatures_by_path,
        returns_by_path,
        adts,
        failures,
        expected_is_admitted,
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
        ExprKind::Prefix { op, expr } => PrivateExprInference::successful(
            infer_private_prefix_type(*op, expr, expected, &mut context),
        ),
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
            condition,
            then_branch,
            else_if_branches,
            else_branch,
        } => Some(infer_private_if_result_type(
            condition,
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
                context.current_package,
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
    _expected: Option<&Type>,
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
        ExprKind::SchemaValidate { value, base, .. } => {
            context.infer(value, None);
            context.infer(base, Some(&Type::named("ByteOffset", Vec::new())));
            Some(Type::Unknown)
        }
        ExprKind::FieldAccess { base, field, .. } => Some(
            context
                .infer(base, None)
                .record_field(field)
                .cloned()
                .unwrap_or(Type::Unknown),
        ),
        ExprKind::Try { expr: inner, .. } => Some({
            let inner_type = context.infer(inner, None);
            adt::result_parts(&inner_type).map_or(Type::Unknown, |(value, _)| value.clone())
        }),
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
    let has_context = expected.and_then(Type::vec_part).is_some();
    let mut joined_items = None;
    for item in items {
        infer_private_aggregate_component(
            item,
            has_context,
            &mut item_type,
            &mut joined_items,
            true,
            context,
        );
    }
    if let Some(joined) = joined_items {
        let Some(resolved) = joined.resolved_result_type() else {
            return PrivateExprInference {
                ty: Type::Unknown,
                may_contribute: false,
            };
        };
        item_type = resolved;
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
    let has_key_context = !type_has_unknown(&key_type);
    let has_value_context = !type_has_unknown(&value_type);
    for entry in entries {
        infer_private_aggregate_component(
            &entry.key,
            has_key_context,
            &mut key_type,
            &mut joined_keys,
            true,
            context,
        );
        infer_private_aggregate_component(
            &entry.value,
            has_value_context,
            &mut value_type,
            &mut joined_values,
            true,
            context,
        );
    }
    if let Some(joined) = joined_keys {
        let Some(resolved) = joined.resolved_result_type() else {
            return PrivateExprInference {
                ty: Type::Unknown,
                may_contribute: false,
            };
        };
        key_type = resolved;
    }
    if let Some(joined) = joined_values {
        let Some(resolved) = joined.resolved_result_type() else {
            return PrivateExprInference {
                ty: Type::Unknown,
                may_contribute: false,
            };
        };
        value_type = resolved;
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
    context_dependent_only: bool,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) {
    context.bindings.begin_inference_transaction();
    let inferred_context = private_aggregate_component_context(
        expr,
        component_type,
        joined_components.as_ref(),
        context_dependent_only,
        context,
    );
    let admitted_context = !has_context
        && context_dependent_only
        && joined_components.is_some()
        && inferred_context != Type::Unknown;
    let actual = if admitted_context {
        context.infer_admitted_outcome(expr, &inferred_context)
    } else {
        context.infer_outcome(expr, item_type_unknown_as_none(&inferred_context))
    };
    if !actual.may_contribute {
        context.bindings.rollback_inference_transaction();
        return;
    }
    let actual = actual.ty;
    let actual = if has_context {
        inferred_private_aggregate_member_type(actual, component_type, context.adts)
    } else {
        actual
    };
    initialize_private_aggregate_join(
        has_context,
        component_type,
        joined_components,
        context_dependent_only,
        context.adts,
    );
    let joined = !has_context
        && joined_components
            .as_mut()
            .is_some_and(|joined| joined.try_join(context.adts, &actual));
    if private_aggregate_component_rejected(
        has_context,
        joined_components.is_some(),
        joined,
        &inferred_context,
        &actual,
        context.adts,
    ) {
        *context.failures += 1;
        context.bindings.rollback_inference_transaction();
        return;
    }
    if !joined && *component_type == Type::Unknown {
        *component_type = actual;
        initialize_private_aggregate_join(
            has_context,
            component_type,
            joined_components,
            context_dependent_only,
            context.adts,
        );
    }
    context.bindings.commit_inference_transaction();
}

fn private_aggregate_component_context(
    expr: &Expr,
    component_type: &Type,
    joined_components: Option<&crate::aggregate_type_join::AggregateTypeJoin>,
    context_dependent_only: bool,
    context: &PrivateSignatureInferContext<'_, '_>,
) -> Type {
    match joined_components {
        Some(joined) if context_dependent_only => joined.expression_admitted_context(
            expr,
            context.adts,
            context.current_module,
            context.uses,
            &|name| context.projection_local_context(name),
        ),
        Some(joined) => joined.inference_type(),
        None if context_dependent_only && type_has_unknown(component_type) => Type::Unknown,
        None => component_type.clone(),
    }
}

fn initialize_private_aggregate_join(
    has_context: bool,
    component_type: &Type,
    joined_components: &mut Option<crate::aggregate_type_join::AggregateTypeJoin>,
    context_dependent_only: bool,
    adts: &AdtRegistry,
) {
    if !has_context
        && joined_components.is_none()
        && (!context_dependent_only || component_type != &Type::Unknown)
    {
        *joined_components =
            crate::aggregate_type_join::AggregateTypeJoin::new(adts, component_type);
    }
}

fn private_aggregate_component_rejected(
    has_context: bool,
    has_join: bool,
    joined: bool,
    inferred_context: &Type,
    actual: &Type,
    adts: &AdtRegistry,
) -> bool {
    if !has_context && has_join {
        return !joined;
    }
    inferred_context != &Type::Unknown
        && !crate::type_relations::is_assignable_nested_with_registry(
            inferred_context,
            actual,
            adts,
        )
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
                    Some(expected) => {
                        inferred_private_aggregate_member_type(actual, expected, context.adts)
                    }
                    None => actual,
                };
                (field.name.clone(), actual)
            })
            .collect(),
    )
}

pub(super) fn inferred_private_aggregate_member_type(
    ty: Type,
    expected: &Type,
    adts: &AdtRegistry,
) -> Type {
    if expected != &Type::Unknown
        && !type_has_unknown(expected)
        && crate::type_relations::is_assignable_with_registry(expected, &ty, adts)
    {
        expected.clone()
    } else {
        ty
    }
}

fn infer_private_prefix_type(
    op: veln_ast::PrefixOp,
    expr: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    let outcome = context.infer_outcome(expr, None);
    let operand_type = match op {
        veln_ast::PrefixOp::Not => Type::bool(),
        veln_ast::PrefixOp::Negate => {
            if expected.is_some_and(|expected| expected == &Type::float())
                || outcome.ty == Type::float()
            {
                Type::float()
            } else {
                Type::int()
            }
        }
        veln_ast::PrefixOp::BitwiseNot => Type::int(),
    };
    reject_private_operator_operand(&outcome, &operand_type, context);
    operand_type
}

pub(crate) fn infer_private_binary_type(
    op: veln_ast::BinaryOp,
    left: &Expr,
    right: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    if op == veln_ast::BinaryOp::PipeGreater {
        return infer_private_pipeline_type(left, right, expected, context);
    }

    let left = context.infer_outcome(left, None);
    let right = context.infer_outcome(right, None);
    let numeric_type = if expected.is_some_and(|expected| expected == &Type::float())
        || left.ty == Type::float()
        || right.ty == Type::float()
    {
        Type::float()
    } else {
        Type::int()
    };
    let (operand_type, result_type) = match op {
        veln_ast::BinaryOp::Equal | veln_ast::BinaryOp::NotEqual => (Type::Unknown, Type::bool()),
        veln_ast::BinaryOp::Less
        | veln_ast::BinaryOp::LessEqual
        | veln_ast::BinaryOp::Greater
        | veln_ast::BinaryOp::GreaterEqual => (numeric_type, Type::bool()),
        veln_ast::BinaryOp::Or | veln_ast::BinaryOp::And => (Type::bool(), Type::bool()),
        veln_ast::BinaryOp::BitwiseOr
        | veln_ast::BinaryOp::BitwiseXor
        | veln_ast::BinaryOp::BitwiseAnd
        | veln_ast::BinaryOp::ShiftLeft
        | veln_ast::BinaryOp::ShiftRight
        | veln_ast::BinaryOp::ShiftRightLogical => (Type::int(), Type::int()),
        veln_ast::BinaryOp::Add
        | veln_ast::BinaryOp::Subtract
        | veln_ast::BinaryOp::Multiply
        | veln_ast::BinaryOp::Divide => (numeric_type.clone(), numeric_type),
        veln_ast::BinaryOp::Remainder => (Type::int(), Type::int()),
        veln_ast::BinaryOp::PipeGreater => unreachable!("pipeline returned before operand checks"),
    };
    reject_private_operator_operand(&left, &operand_type, context);
    reject_private_operator_operand(&right, &operand_type, context);
    result_type
}

fn infer_private_pipeline_type(
    left: &Expr,
    right: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    let ExprKind::Call { callee, args } = &right.kind else {
        context.infer_outcome(left, None);
        context.infer_outcome(right, expected);
        *context.failures += 1;
        return Type::Unknown;
    };
    if !matches!(callee.kind, ExprKind::NamePath { .. }) {
        context.infer_outcome(left, None);
        context.infer_outcome(right, expected);
        *context.failures += 1;
        return Type::Unknown;
    }
    let mut piped_args = Vec::with_capacity(args.len() + 1);
    piped_args.push(left.clone());
    piped_args.extend(args.iter().cloned());
    infer_private_signature_call_type(callee, &piped_args, expected, context)
}

fn reject_private_operator_operand(
    outcome: &PrivateExprInference,
    expected: &Type,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) {
    if outcome.may_contribute
        && !crate::type_relations::is_assignable_with_registry(expected, &outcome.ty, context.adts)
    {
        *context.failures += 1;
    }
}

pub(crate) fn item_type_unknown_as_none(ty: &Type) -> Option<&Type> {
    (ty != &Type::Unknown).then_some(ty)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn infer_private_signature_name_type(
    segments: &[String],
    expected: Option<&Type>,
    current_module: Option<&str>,
    current_package: Option<&str>,
    uses: &[UseDecl],
    bindings: &mut PrivateBindings,
    returns_by_path: &BTreeMap<FunctionKey, Type>,
    adts: &AdtRegistry,
) -> Type {
    if let [name] = segments
        && let Some(index) = bindings.iter().rposition(|binding| binding.name == *name)
    {
        let current = bindings[index].ty.clone();
        if matches!(current, Type::Record(ref fields) if fields.is_empty())
            && let Some(expected) = expected
            && expected.dict_parts().is_some()
            && !type_has_unknown(expected)
        {
            bindings.set_inferred_type(index, expected.clone());
            return expected.clone();
        }
        if type_has_unknown(&current)
            && let Some(expected) = expected
            && !type_has_unknown(expected)
        {
            let mut constrained = current;
            adt::merge_type_holes(&mut constrained, expected);
            bindings.set_inferred_type(index, constrained.clone());
            return constrained;
        }
        return current;
    }
    if let Some(constructor) = private_nullary_constructor(
        segments,
        expected,
        current_module,
        current_package,
        uses,
        adts,
    ) {
        let args = expected
            .and_then(|expected| {
                unification::adt_args(expected, constructor.descriptor).map(<[Type]>::to_vec)
            })
            .unwrap_or_else(|| vec![Type::Unknown; constructor.descriptor.type_parameters.len()]);
        return adt::refined_constructed_type_from_args(constructor, &args);
    }
    private_name_value_type(
        segments,
        current_module,
        current_package,
        uses,
        bindings,
        returns_by_path,
    )
}

fn private_nullary_constructor<'a>(
    segments: &[String],
    expected: Option<&Type>,
    current_module: Option<&str>,
    _current_package: Option<&str>,
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
    current_package: Option<&str>,
    uses: &[UseDecl],
    bindings: &[Binding],
    returns_by_path: &BTreeMap<FunctionKey, Type>,
) -> Type {
    match segments {
        [name] => bindings
            .iter()
            .rev()
            .find(|binding| binding.name == *name)
            .map(|binding| binding.ty.clone())
            .or_else(|| {
                returns_by_path
                    .get(&(
                        current_package.map(str::to_string),
                        current_module.map(str::to_string),
                        name.clone(),
                    ))
                    .cloned()
            })
            .unwrap_or(Type::Unknown),
        [_, .., name] => imported_use_for_path_in_owner(
            uses,
            &segments[..segments.len() - 1],
            current_module,
            current_package,
        )
        .and_then(|use_decl| {
            returns_by_path
                .get(&(
                    use_decl.resolved_target().package,
                    Some(use_decl.name.clone()),
                    name.clone(),
                ))
                .cloned()
        })
        .unwrap_or(Type::Unknown),
        _ => Type::Unknown,
    }
}

pub(crate) struct PrivateSignatureInferContext<'a, 'f> {
    pub(crate) current_module: Option<&'a str>,
    pub(crate) current_package: Option<&'a str>,
    pub(crate) uses: &'a [UseDecl],
    pub(crate) bindings: &'a mut PrivateBindings,
    pub(crate) signatures_by_path: Option<&'a FunctionSignatureMap>,
    pub(crate) returns_by_path: &'a BTreeMap<FunctionKey, Type>,
    pub(crate) adts: &'a AdtRegistry,
    pub(crate) failures: &'f mut usize,
    pub(crate) expected_is_admitted: bool,
}

impl PrivateSignatureInferContext<'_, '_> {
    pub(crate) fn projection_local_context(
        &self,
        name: &str,
    ) -> Option<crate::aggregate_type_join::LocalContext> {
        let current = &self
            .bindings
            .iter()
            .rev()
            .find(|binding| binding.name == name)?
            .ty;
        Some(
            if matches!(current, Type::Record(fields) if fields.is_empty()) {
                crate::aggregate_type_join::LocalContext::EmptyRecord
            } else if type_has_unknown(current) {
                crate::aggregate_type_join::LocalContext::UnknownBearing
            } else {
                crate::aggregate_type_join::LocalContext::Complete
            },
        )
    }

    pub(crate) fn infer(&mut self, expr: &Expr, expected: Option<&Type>) -> Type {
        self.infer_outcome(expr, expected).ty
    }

    pub(super) fn infer_outcome(
        &mut self,
        expr: &Expr,
        expected: Option<&Type>,
    ) -> PrivateExprInference {
        let failure_count = *self.failures;
        let mut outcome = infer_private_signature_expr_type_with_expected_kind(
            expr,
            expected,
            self.current_module,
            self.current_package,
            self.uses,
            self.bindings,
            self.signatures_by_path,
            self.returns_by_path,
            self.adts,
            self.failures,
            false,
        );
        outcome.may_contribute &= *self.failures == failure_count;
        outcome
    }

    pub(super) fn infer_admitted_outcome(
        &mut self,
        expr: &Expr,
        expected: &Type,
    ) -> PrivateExprInference {
        let failure_count = *self.failures;
        let mut outcome = infer_private_signature_expr_type_with_expected_kind(
            expr,
            Some(expected),
            self.current_module,
            self.current_package,
            self.uses,
            self.bindings,
            self.signatures_by_path,
            self.returns_by_path,
            self.adts,
            self.failures,
            true,
        );
        outcome.may_contribute &= *self.failures == failure_count;
        outcome
    }

    pub(super) fn infer_expected_outcome(
        &mut self,
        expr: &Expr,
        expected: &Type,
    ) -> PrivateExprInference {
        let expected_is_admitted = self.expected_is_admitted;
        self.expected_is_admitted = false;
        let mut outcome = self.infer_outcome(expr, Some(expected));
        self.expected_is_admitted = expected_is_admitted;
        let rejected_here = outcome.ty != Type::Unknown
            && !crate::type_relations::is_assignable_with_registry(
                expected,
                &outcome.ty,
                self.adts,
            );
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
        let expected_is_admitted = self.expected_is_admitted;
        self.expected_is_admitted = false;
        let mut outcome = self.infer_outcome(expr, Some(expected));
        self.expected_is_admitted = expected_is_admitted;
        let rejected_here = outcome.ty != Type::Unknown
            && !crate::type_relations::is_assignable_nested_with_registry(
                expected,
                &outcome.ty,
                self.adts,
            );
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
            self.current_package,
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
