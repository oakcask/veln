use super::*;
use crate::adt::descriptors::AdtPayloadField;

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
    if let Some(ty) = infer_private_value_type(expr, expected, &mut context) {
        return PrivateExprInference::successful(ty);
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
) -> Option<Type> {
    match &expr.kind {
        ExprKind::NamePath { segments, .. } => Some(infer_private_signature_name_type(
            segments,
            expected,
            context.current_module,
            context.uses,
            context.bindings,
            context.returns_by_path,
            context.adts,
        )),
        ExprKind::List(items) => Some(infer_private_list_type(items, expected, context)),
        ExprKind::Dict(entries) => Some(infer_private_dict_type(entries, expected, context)),
        ExprKind::Record(fields) => Some(infer_private_record_type(fields, expected, context)),
        ExprKind::Call { callee, args } => Some(infer_private_signature_call_type(
            callee, args, expected, context,
        )),
        ExprKind::Perform { args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            Some(Type::Unknown)
        }
        ExprKind::Handle { body, args, .. } => {
            for arg in args {
                context.infer(arg, None);
            }
            Some(context.infer(body, expected))
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

pub(crate) fn infer_private_list_type(
    items: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    let mut item_type = expected
        .and_then(Type::vec_part)
        .cloned()
        .unwrap_or(Type::Unknown);
    let mut joined_items = None;
    for item in items {
        let has_context = expected.and_then(Type::vec_part).is_some();
        let inferred_context = joined_items
            .as_ref()
            .map(crate::aggregate_type_join::AggregateTypeJoin::inference_type)
            .unwrap_or_else(|| item_type.clone());
        let actual = context.infer_outcome(item, item_type_unknown_as_none(&inferred_context));
        if !actual.may_contribute {
            continue;
        }
        let actual = actual.ty;
        let actual = if has_context {
            inferred_private_aggregate_member_type(actual, &item_type)
        } else {
            actual
        };
        if !has_context && joined_items.is_none() && item_type != Type::Unknown {
            joined_items =
                crate::aggregate_type_join::AggregateTypeJoin::new(context.adts, &item_type);
        }
        let joined = !has_context
            && joined_items
                .as_mut()
                .is_some_and(|joined| joined.try_join(&actual));
        if !joined && item_type == Type::Unknown {
            item_type = actual;
            joined_items = (!has_context)
                .then(|| {
                    crate::aggregate_type_join::AggregateTypeJoin::new(context.adts, &item_type)
                })
                .flatten();
        }
    }
    if let Some(joined) = joined_items {
        item_type = joined.result_type();
    }
    Type::vec(item_type)
}

pub(crate) fn infer_private_dict_type(
    entries: &[DictEntry],
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    let (mut key_type, mut value_type) = expected
        .and_then(Type::dict_parts)
        .map_or((Type::Unknown, Type::Unknown), |(key, value)| {
            (key.clone(), value.clone())
        });
    let mut joined_keys = None;
    let mut joined_values = None;
    for entry in entries {
        let has_key_context = expected.and_then(Type::dict_parts).is_some();
        infer_private_dict_component(
            &entry.key,
            has_key_context,
            &mut key_type,
            &mut joined_keys,
            context,
        );
        infer_private_dict_component(
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
    Type::dict(key_type, value_type)
}

fn infer_private_dict_component(
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

fn inferred_private_aggregate_member_type(ty: Type, expected: &Type) -> Type {
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
    for arm in arms {
        let actual = context.infer(&arm.expr, item_type_unknown_as_none(&result));
        merge_private_control_flow_result(&mut result, actual);
    }
    result
}

pub(crate) fn infer_private_if_result_type(
    then_branch: &Expr,
    else_if_branches: &[IfBranch],
    else_branch: &Expr,
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    let mut result = expected.cloned().unwrap_or(Type::Unknown);
    for branch_expr in std::iter::once(then_branch)
        .chain(else_if_branches.iter().map(|branch| &branch.expr))
        .chain(std::iter::once(else_branch))
    {
        let actual = context.infer(branch_expr, item_type_unknown_as_none(&result));
        merge_private_control_flow_result(&mut result, actual);
    }
    result
}

fn merge_private_control_flow_result(result: &mut Type, actual: Type) {
    if *result == Type::Unknown {
        *result = actual;
        return;
    }
    if *result == actual || actual == Type::Unknown {
        return;
    }
    let common_base = match (&*result, &actual) {
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

fn private_type_contains_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } | Type::VariantRefinement { args, .. } => {
            args.iter().any(private_type_contains_unknown)
        }
        Type::Record(fields) => fields
            .iter()
            .any(|(_, field)| private_type_contains_unknown(field)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(private_type_contains_unknown)
                || variadic
                    .as_deref()
                    .is_some_and(private_type_contains_unknown)
                || private_type_contains_unknown(return_type)
        }
    }
}

pub(crate) fn infer_match_scrutinee_type_from_constructor_patterns(
    arms: &[MatchArm],
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> MatchScrutineePatternInference {
    let mut inferred: Option<(AdtConstructor<'_>, Vec<Type>)> = None;

    for arm in arms {
        let PatternKind::Constructor { name, args, .. } = &arm.pattern.kind else {
            continue;
        };
        if invalid_qualified_constructor_pattern(name) {
            continue;
        }
        let candidates = adts.constructor_candidates(name, current_module, uses);
        if candidates.is_empty() {
            continue;
        }
        let descriptor_names = unique_constructor_descriptor_names(&candidates);
        if descriptor_names.len() != 1 {
            return MatchScrutineePatternInference::Ambiguous(descriptor_names);
        }
        let constructor = candidates[0];
        if let Some((previous, _)) = &inferred {
            if !same_constructor_descriptor(previous, &constructor) {
                let mut names = unique_constructor_descriptor_names(&[*previous, constructor]);
                names.sort();
                return MatchScrutineePatternInference::Ambiguous(names);
            }
        } else {
            inferred = Some((
                constructor,
                vec![Type::Unknown; constructor.descriptor.type_parameters.len()],
            ));
        }
        let Some((_, type_args)) = &mut inferred else {
            continue;
        };
        for (index, pattern) in args.iter().enumerate() {
            let Some(pattern_type) =
                infer_pattern_type_from_constructor_patterns(pattern, current_module, uses, adts)
            else {
                continue;
            };
            adt::merge_type_args_from_payload(type_args, constructor, index, &pattern_type);
        }
    }

    match inferred {
        Some((constructor, type_args)) => MatchScrutineePatternInference::Inferred(
            adt::constructed_type_from_args(constructor, &type_args),
        ),
        None => MatchScrutineePatternInference::Uninferred,
    }
}

pub(crate) fn invalid_qualified_constructor_pattern(name: &[String]) -> bool {
    name.len() > 1
        && name
            .last()
            .and_then(|name| name.as_bytes().first())
            .is_some_and(u8::is_ascii_lowercase)
}

pub(crate) fn infer_pattern_type_from_constructor_patterns(
    pattern: &Pattern,
    current_module: Option<&str>,
    uses: &[UseDecl],
    adts: &AdtRegistry,
) -> Option<Type> {
    match &pattern.kind {
        PatternKind::StringLiteral(_) => Some(Type::string()),
        PatternKind::IntLiteral(_) => Some(Type::int()),
        PatternKind::FloatLiteral(_) => Some(Type::float()),
        PatternKind::BoolLiteral(_) => Some(Type::bool()),
        PatternKind::Unit => Some(Type::unit()),
        PatternKind::Record(fields) => Some(Type::Record(
            fields
                .iter()
                .map(|field| {
                    (
                        field.name.clone(),
                        infer_pattern_type_from_constructor_patterns(
                            &field.pattern,
                            current_module,
                            uses,
                            adts,
                        )
                        .unwrap_or(Type::Unknown),
                    )
                })
                .collect(),
        )),
        PatternKind::Constructor { name, args, .. } => {
            if invalid_qualified_constructor_pattern(name) {
                return None;
            }
            let candidates = adts.constructor_candidates(name, current_module, uses);
            let [constructor] = candidates.as_slice() else {
                return None;
            };
            let mut type_args = vec![Type::Unknown; constructor.descriptor.type_parameters.len()];
            for (index, pattern) in args.iter().enumerate() {
                let Some(pattern_type) = infer_pattern_type_from_constructor_patterns(
                    pattern,
                    current_module,
                    uses,
                    adts,
                ) else {
                    continue;
                };
                adt::merge_type_args_from_payload(
                    &mut type_args,
                    *constructor,
                    index,
                    &pattern_type,
                );
            }
            Some(adt::constructed_type_from_args(*constructor, &type_args))
        }
        PatternKind::Wildcard | PatternKind::Binding(_) => None,
    }
}

pub(crate) fn unique_constructor_descriptor_names(
    constructors: &[AdtConstructor<'_>],
) -> Vec<String> {
    let mut names = Vec::new();
    for constructor in constructors {
        let name = constructor.descriptor.diagnostic_name.clone();
        if !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

pub(crate) fn same_constructor_descriptor(
    left: &AdtConstructor<'_>,
    right: &AdtConstructor<'_>,
) -> bool {
    left.descriptor.identity() == right.descriptor.identity()
        && left.descriptor.type_parameters.len() == right.descriptor.type_parameters.len()
}

pub(crate) fn type_has_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } | Type::VariantRefinement { args, .. } => {
            args.iter().any(type_has_unknown)
        }
        Type::Record(fields) => fields.iter().any(|(_, ty)| type_has_unknown(ty)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(type_has_unknown)
                || variadic.as_deref().is_some_and(type_has_unknown)
                || type_has_unknown(return_type)
        }
    }
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

    fn infer_outcome(&mut self, expr: &Expr, expected: Option<&Type>) -> PrivateExprInference {
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

    fn infer_expected_outcome(&mut self, expr: &Expr, expected: &Type) -> PrivateExprInference {
        let mut outcome = self.infer_outcome(expr, Some(expected));
        let rejected_here = outcome.ty != Type::Unknown
            && !crate::type_relations::is_assignable(expected, &outcome.ty);
        if rejected_here && outcome.may_contribute {
            *self.failures += 1;
        }
        outcome.may_contribute &= !rejected_here;
        outcome
    }

    fn infer_nested_expected_outcome(
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

    fn infer_body(&mut self, body: &[BodyLine], expected: Option<&Type>) -> Type {
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

pub(crate) fn infer_private_signature_call_type(
    callee: &Expr,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    let ExprKind::NamePath { segments, .. } = &callee.kind else {
        return Type::Unknown;
    };
    if let Some(constructor) = private_payload_constructor(segments, expected, context) {
        return infer_private_constructor_call(constructor, args, expected, context);
    }
    if let Some(signature) = private_declared_call_signature(segments, context).cloned() {
        let arity_matches = args.len() >= signature.params.len()
            && (signature.variadic.is_some() || args.len() == signature.params.len());
        if !arity_matches {
            *context.failures += 1;
        }
        for (arg, param) in args.iter().zip(&signature.params) {
            context.infer_expected_outcome(arg, param);
        }
        if let Some(variadic) = &signature.variadic {
            for arg in args.iter().skip(signature.params.len()) {
                context.infer_expected_outcome(arg, variadic);
            }
        } else {
            for arg in args.iter().skip(signature.params.len()) {
                context.infer_outcome(arg, None);
            }
        }
        return signature.return_type;
    }
    if let Some(return_type) = private_declared_call_return(segments, context) {
        return return_type.clone();
    }
    if let Some(name) = segments.last()
        && let Some((params, return_type)) = crate::prelude::prelude_signature(name, expected)
    {
        if args.len() != params.len() {
            *context.failures += 1;
        }
        for (arg, param) in args.iter().zip(params.iter()) {
            context.infer_expected_outcome(arg, param);
        }
        for arg in args.iter().skip(params.len()) {
            context.infer_outcome(arg, None);
        }
        return return_type;
    }
    Type::Unknown
}

fn private_declared_call_signature<'a>(
    segments: &[String],
    context: &'a PrivateSignatureInferContext<'_, '_>,
) -> Option<&'a FunctionSignature> {
    let signatures = context.signatures_by_path?;
    match segments {
        [name] => signatures.get(&(context.current_module.map(str::to_string), name.clone())),
        [_, .., name] => imported_use_for_path(
            context.uses,
            &segments[..segments.len() - 1],
            context.current_module,
        )
        .and_then(|use_decl| signatures.get(&(Some(use_decl.name.clone()), name.clone()))),
        _ => None,
    }
}

fn private_payload_constructor<'a>(
    segments: &[String],
    expected: Option<&Type>,
    context: &PrivateSignatureInferContext<'a, '_>,
) -> Option<AdtConstructor<'a>> {
    let expected_constructor = (segments.len() == 1)
        .then_some(expected)
        .flatten()
        .and_then(|expected| {
            context
                .adts
                .descriptor_for_type_prefer_module(expected, context.current_module)
        })
        .and_then(|descriptor| {
            context.adts.constructor_for_descriptor(
                segments,
                descriptor,
                context.current_module,
                context.uses,
            )
        })
        .filter(|constructor| !constructor.variant.payload_fields.is_empty());
    let ordinary_constructor =
        match context
            .adts
            .constructor(segments, context.current_module, context.uses)
        {
            ConstructorLookup::Found(constructor) => Some(constructor),
            ConstructorLookup::Ambiguous | ConstructorLookup::Missing => None,
        };
    expected_constructor.or(ordinary_constructor)
}

fn infer_private_constructor_call(
    constructor: AdtConstructor<'_>,
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Type {
    if args.len() != constructor.variant.payload_fields.len() {
        *context.failures += 1;
    }
    let mut type_args = PrivateConstructorTypeArgInference::new(constructor);
    for (index, arg) in args.iter().enumerate() {
        let field = constructor.variant.payload_fields.get(index);
        let payload_expected = type_args.payload_expected(expected, constructor, index, field);
        let widens_aggregate_member = expected.is_some()
            || (field.is_some_and(|field| !matches!(field.ty, AdtPayloadType::TypeParameter(_)))
                && !private_type_contains_unknown(&payload_expected));
        let direct_joining_parameter =
            field.is_some_and(|field| matches!(field.ty, AdtPayloadType::TypeParameter(_)));
        let actual = if direct_joining_parameter || payload_expected == Type::Unknown {
            context.infer_outcome(arg, item_type_unknown_as_none(&payload_expected))
        } else {
            context.infer_nested_expected_outcome(arg, &payload_expected)
        };
        let payload_succeeded = actual.may_contribute;
        let actual = actual.ty;
        let inferred_actual = if widens_aggregate_member {
            inferred_private_aggregate_member_type(actual, &payload_expected)
        } else {
            actual
        };
        if !payload_succeeded {
            continue;
        }
        let joined = type_args.try_join(field, &inferred_actual, context.adts);
        let merge_invariant =
            crate::type_relations::is_assignable_nested(&payload_expected, &inferred_actual);
        type_args.commit(
            field,
            constructor,
            index,
            &inferred_actual,
            joined,
            merge_invariant,
            context.adts,
        );
    }
    let inferred_type_args = type_args.finish(expected, constructor);
    adt::refined_constructed_type_from_args(constructor, &inferred_type_args)
}

struct PrivateConstructorTypeArgInference {
    inferred: Vec<Type>,
    joined: Vec<Option<crate::aggregate_type_join::AggregateTypeJoin>>,
    invariant: Vec<bool>,
}

impl PrivateConstructorTypeArgInference {
    fn new(constructor: AdtConstructor<'_>) -> Self {
        let parameter_count = constructor.descriptor.type_parameters.len();
        crate::aggregate_type_join::record_work(parameter_count);
        Self {
            inferred: vec![Type::Unknown; parameter_count],
            joined: (0..parameter_count).map(|_| None).collect(),
            invariant: vec![false; parameter_count],
        }
    }

    fn payload_expected(
        &self,
        expected: Option<&Type>,
        constructor: AdtConstructor<'_>,
        index: usize,
        field: Option<&AdtPayloadField>,
    ) -> Type {
        expected
            .and_then(|expected| {
                let expected_args = unification::adt_args(expected, constructor.descriptor)?;
                adt::payload_type_with_resolved_args(constructor, index, |type_index| {
                    crate::aggregate_type_join::record_work(1);
                    expected_args[type_index].clone()
                })
            })
            .or_else(|| field.and_then(|field| self.direct_payload_context(field)))
            .or_else(|| {
                adt::payload_type_with_resolved_args(constructor, index, |type_index| {
                    crate::aggregate_type_join::record_work(1);
                    self.joined[type_index]
                        .as_ref()
                        .map(crate::aggregate_type_join::AggregateTypeJoin::result_type)
                        .unwrap_or_else(|| self.inferred[type_index].clone())
                })
            })
            .unwrap_or(Type::Unknown)
    }

    fn direct_payload_context(&self, field: &AdtPayloadField) -> Option<Type> {
        let AdtPayloadType::TypeParameter(type_index) = field.ty else {
            return None;
        };
        (!self.invariant[type_index])
            .then(|| self.joined[type_index].as_ref())
            .flatten()
            .map(crate::aggregate_type_join::AggregateTypeJoin::inference_type)
    }

    fn try_join(
        &mut self,
        field: Option<&AdtPayloadField>,
        actual: &Type,
        adts: &AdtRegistry,
    ) -> bool {
        let Some(AdtPayloadField {
            ty: AdtPayloadType::TypeParameter(type_index),
            ..
        }) = field
        else {
            return false;
        };
        if self.invariant[*type_index] {
            return false;
        }
        if self.joined[*type_index].is_none() && self.inferred[*type_index] != Type::Unknown {
            self.joined[*type_index] = crate::aggregate_type_join::AggregateTypeJoin::new(
                adts,
                &self.inferred[*type_index],
            );
        }
        self.joined[*type_index]
            .as_mut()
            .is_some_and(|joined| joined.try_join(actual))
    }

    #[allow(clippy::too_many_arguments)]
    fn commit(
        &mut self,
        field: Option<&AdtPayloadField>,
        constructor: AdtConstructor<'_>,
        index: usize,
        actual: &Type,
        joined: bool,
        merge_invariant: bool,
        adts: &AdtRegistry,
    ) {
        let Some(field) = field else {
            return;
        };
        if let AdtPayloadType::TypeParameter(type_index) = field.ty
            && joined
        {
            self.inferred[type_index] = self.joined[type_index]
                .as_ref()
                .expect("joined private aggregate type argument")
                .inference_type();
            return;
        }
        if !matches!(field.ty, AdtPayloadType::TypeParameter(_)) {
            if merge_invariant {
                self.commit_invariant_payload(constructor, index, actual);
            }
            return;
        }
        crate::aggregate_type_join::record_work(1);
        adt::merge_type_args_from_payload(&mut self.inferred, constructor, index, actual);
        let AdtPayloadType::TypeParameter(type_index) = field.ty else {
            return;
        };
        if self.joined[type_index].is_none() {
            self.joined[type_index] = crate::aggregate_type_join::AggregateTypeJoin::new(
                adts,
                &self.inferred[type_index],
            );
            if let Some(joined) = &self.joined[type_index] {
                self.inferred[type_index] = joined.inference_type();
            }
        }
    }

    fn commit_invariant_payload(
        &mut self,
        constructor: AdtConstructor<'_>,
        index: usize,
        actual: &Type,
    ) {
        adt::visit_type_arg_contributions_from_payload(
            constructor,
            index,
            actual,
            |type_index, contribution| {
                crate::aggregate_type_join::record_work(1);
                let mut constraint = self.joined[type_index]
                    .as_ref()
                    .map(crate::aggregate_type_join::AggregateTypeJoin::result_type)
                    .unwrap_or_else(|| self.inferred[type_index].clone());
                unification::merge_type_slot(&mut constraint, contribution);
                self.inferred[type_index] = constraint;
                self.joined[type_index] = None;
                self.invariant[type_index] = true;
            },
        );
    }

    fn finish(mut self, expected: Option<&Type>, constructor: AdtConstructor<'_>) -> Vec<Type> {
        crate::aggregate_type_join::record_work(self.inferred.len());
        for (inferred, joined) in self.inferred.iter_mut().zip(self.joined) {
            if let Some(joined) = joined {
                *inferred = joined.result_type();
            }
        }
        if let Some(expected_args) =
            expected.and_then(|expected| unification::adt_args(expected, constructor.descriptor))
        {
            for (inferred, expected) in self.inferred.iter_mut().zip(expected_args) {
                if *inferred == Type::Unknown {
                    *inferred = expected.clone();
                }
            }
        }
        self.inferred
    }
}

fn private_declared_call_return<'a>(
    segments: &[String],
    context: &'a PrivateSignatureInferContext<'_, '_>,
) -> Option<&'a Type> {
    match segments {
        [name] => context
            .returns_by_path
            .get(&(context.current_module.map(str::to_string), name.clone())),
        [_, .., name] => imported_use_for_path(
            context.uses,
            &segments[..segments.len() - 1],
            context.current_module,
        )
        .and_then(|use_decl| {
            context
                .returns_by_path
                .get(&(Some(use_decl.name.clone()), name.clone()))
        }),
        _ => None,
    }
}
