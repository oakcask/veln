use super::*;
use crate::adt::descriptors::AdtPayloadField;

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
    if let Some(return_type) = infer_private_declared_signature_call(segments, args, context) {
        return return_type;
    }
    if let Some(return_type) = private_declared_call_return(segments, context) {
        return return_type.clone();
    }
    infer_private_prelude_call(segments, args, expected, context).unwrap_or(Type::Unknown)
}

fn infer_private_declared_signature_call(
    segments: &[String],
    args: &[Expr],
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Option<Type> {
    let signature = private_declared_call_signature(segments, context)?.clone();
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
    Some(signature.return_type)
}

fn infer_private_prelude_call(
    segments: &[String],
    args: &[Expr],
    expected: Option<&Type>,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) -> Option<Type> {
    let name = segments.last()?;
    let (params, return_type) = crate::prelude::prelude_signature(name, expected)?;
    if args.len() != params.len() {
        *context.failures += 1;
    }
    for (arg, param) in args.iter().zip(params.iter()) {
        context.infer_expected_outcome(arg, param);
    }
    for arg in args.iter().skip(params.len()) {
        context.infer_outcome(arg, None);
    }
    Some(return_type)
}

fn private_declared_call_signature<'a>(
    segments: &[String],
    context: &'a PrivateSignatureInferContext<'_, '_>,
) -> Option<&'a FunctionSignature> {
    let signatures = context.signatures_by_path?;
    match segments {
        [name] => signatures.get(&(
            context.current_package.map(str::to_string),
            context.current_module.map(str::to_string),
            name.clone(),
        )),
        [_, .., name] => imported_use_for_path_in_owner(
            context.uses,
            &segments[..segments.len() - 1],
            context.current_module,
            context.current_package,
        )
        .and_then(|use_decl| {
            signatures.get(&(
                use_decl.resolved_target().package,
                Some(use_decl.name.clone()),
                name.clone(),
            ))
        }),
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
    let failure_count = *context.failures;
    context.bindings.begin_inference_transaction();
    if args.len() != constructor.variant.payload_fields.len() {
        *context.failures += 1;
    }
    let mut type_args = PrivateConstructorTypeArgInference::new(constructor);
    for (index, arg) in args.iter().enumerate() {
        infer_private_constructor_payload(
            constructor,
            index,
            arg,
            expected,
            &mut type_args,
            context,
        );
    }
    let inferred_type_args = type_args.finish(expected, constructor);
    if *context.failures == failure_count {
        context.bindings.commit_inference_transaction();
    } else {
        context.bindings.rollback_inference_transaction();
    }
    adt::refined_constructed_type_from_args(constructor, &inferred_type_args)
}

fn infer_private_constructor_payload(
    constructor: AdtConstructor<'_>,
    index: usize,
    arg: &Expr,
    expected: Option<&Type>,
    type_args: &mut PrivateConstructorTypeArgInference,
    context: &mut PrivateSignatureInferContext<'_, '_>,
) {
    let payload_context =
        private_constructor_payload_context(constructor, index, arg, expected, type_args, context);
    let field = payload_context.field;
    let plan = PrivatePayloadInferencePlan::new(
        expected.is_some() && !context.expected_is_admitted,
        field,
        &payload_context.expected,
    );
    let Some(inferred_actual) = plan.infer(arg, &payload_context.expected, context) else {
        return;
    };
    if !admitted_payload_accepts_inferred(
        payload_context.admitted_direct.as_ref(),
        &inferred_actual,
        context.adts,
    ) {
        *context.failures += 1;
        return;
    }
    let admission = type_args.lower_bound_admission(field, &inferred_actual, context.adts);
    let joined = matches!(
        admission,
        crate::aggregate_type_join::ConstructorLowerBoundAdmission::Admitted
    );
    if expected.is_some()
        && !context.expected_is_admitted
        && !crate::type_relations::is_assignable_nested_with_registry(
            &payload_context.expected,
            &inferred_actual,
            context.adts,
        )
        && !unification::is_direct_variant_carrier(&payload_context.expected, &inferred_actual)
    {
        *context.failures += 1;
        return;
    }
    if expected.is_none()
        && matches!(
            admission,
            crate::aggregate_type_join::ConstructorLowerBoundAdmission::Rejected
        )
    {
        *context.failures += 1;
        return;
    }
    let merge_invariant =
        crate::type_relations::is_assignable_nested_with_registry(
            &payload_context.expected,
            &inferred_actual,
            context.adts,
        ) || unification::is_direct_variant_carrier(&payload_context.expected, &inferred_actual);
    if plan.unresolved_concrete_carrier
        && !merge_invariant
        && !type_args.payload_can_infer_without_context(constructor, index, context.adts)
    {
        *context.failures += 1;
        return;
    }
    if type_args
        .commit(
            field,
            constructor,
            index,
            &inferred_actual,
            joined,
            merge_invariant,
            context.adts,
        )
        .is_err()
    {
        *context.failures += 1;
    }
}

struct PrivatePayloadContext<'a> {
    field: Option<&'a AdtPayloadField>,
    expected: Type,
    admitted_direct: Option<Type>,
}

fn private_constructor_payload_context<'a>(
    constructor: AdtConstructor<'a>,
    index: usize,
    arg: &Expr,
    expected: Option<&Type>,
    type_args: &PrivateConstructorTypeArgInference,
    context: &PrivateSignatureInferContext<'_, '_>,
) -> PrivatePayloadContext<'a> {
    let field = constructor.variant.payload_fields.get(index);
    let payload_infers_without_context =
        type_args.payload_can_infer_without_context(constructor, index, context.adts);
    let immediately_independent = (expected.is_none() || context.expected_is_admitted)
        && field.is_some()
        && payload_infers_without_context
        && crate::aggregate_type_join::expression_is_immediately_context_free(
            arg,
            context.adts,
            context.current_module,
            context.uses,
            &|name| context.projection_local_context(name),
        );
    let admitted_payload_context = if immediately_independent {
        Type::Unknown
    } else if context.expected_is_admitted {
        let mut local = type_args.payload_expected(None, constructor, index, false);
        if type_has_unknown(&local) {
            let admitted = type_args.payload_expected(expected, constructor, index, false);
            adt::merge_type_holes(&mut local, &admitted);
        }
        local
    } else {
        type_args.payload_expected(expected, constructor, index, false)
    };
    let sole_direct_payload = index == 0
        && constructor.variant.payload_fields.len() == 1
        && field.is_some_and(|field| matches!(field.ty, AdtPayloadType::TypeParameter(_)));
    let projected_payload_context =
        if expected.is_some() && (!context.expected_is_admitted || sole_direct_payload) {
            admitted_payload_context.clone()
        } else {
            crate::aggregate_type_join::expression_admitted_context(
                arg,
                &admitted_payload_context,
                context.adts,
                context.current_module,
                context.uses,
                &|name| context.projection_local_context(name),
            )
        };
    let direct_payload_joins_independently = immediately_independent
        || (field.is_some()
            && payload_infers_without_context
            && projected_payload_context == Type::Unknown);
    let admitted_direct = (context.expected_is_admitted
        && matches!(
            field.map(|field| &field.ty),
            Some(AdtPayloadType::TypeParameter(_))
        )
        && projected_payload_context == Type::Unknown
        && admitted_payload_context != Type::Unknown)
        .then(|| admitted_payload_context.clone());
    let expected = if direct_payload_joins_independently {
        Type::Unknown
    } else if projected_payload_context == Type::Unknown {
        admitted_payload_context
    } else {
        projected_payload_context
    };
    PrivatePayloadContext {
        field,
        expected,
        admitted_direct,
    }
}

fn admitted_payload_accepts_inferred(
    admitted: Option<&Type>,
    inferred: &Type,
    adts: &AdtRegistry,
) -> bool {
    admitted.is_none_or(|admitted| {
        inferred == admitted
            || crate::aggregate_type_join::AggregateTypeJoin::new(adts, admitted).is_some_and(
                |mut joined| joined.try_join(adts, inferred) && joined.has_resolved_result(),
            )
    })
}

struct PrivatePayloadInferencePlan {
    widens_aggregate_member: bool,
    direct_joining_parameter: bool,
    unresolved_concrete_carrier: bool,
}

impl PrivatePayloadInferencePlan {
    fn new(
        has_expected_type: bool,
        field: Option<&AdtPayloadField>,
        payload_expected: &Type,
    ) -> Self {
        let direct_joining_parameter =
            field.is_some_and(|field| matches!(field.ty, AdtPayloadType::TypeParameter(_)));
        let concrete_payload =
            field.is_some_and(|field| matches!(field.ty, AdtPayloadType::Concrete(_)));
        let payload_has_unknown = private_type_contains_unknown(payload_expected);
        Self {
            widens_aggregate_member: has_expected_type
                || (field.is_some() && !direct_joining_parameter && !payload_has_unknown),
            direct_joining_parameter,
            unresolved_concrete_carrier: concrete_payload && payload_has_unknown,
        }
    }

    fn infer(
        &self,
        arg: &Expr,
        payload_expected: &Type,
        context: &mut PrivateSignatureInferContext<'_, '_>,
    ) -> Option<Type> {
        let outcome = if self.direct_joining_parameter
            || payload_expected == &Type::Unknown
            || self.unresolved_concrete_carrier
        {
            context.infer_outcome(arg, item_type_unknown_as_none(payload_expected))
        } else {
            context.infer_nested_expected_outcome(arg, payload_expected)
        };
        outcome.may_contribute.then(|| {
            if self.widens_aggregate_member {
                inferred_private_aggregate_member_type(outcome.ty, payload_expected, context.adts)
            } else {
                outcome.ty
            }
        })
    }
}

struct PrivateConstructorTypeArgInference {
    constraints: crate::aggregate_type_join::ConstructorTypeArgConstraints,
}

impl PrivateConstructorTypeArgInference {
    fn new(constructor: AdtConstructor<'_>) -> Self {
        Self {
            constraints: crate::aggregate_type_join::ConstructorTypeArgConstraints::new(
                constructor,
            ),
        }
    }

    fn payload_expected(
        &self,
        expected: Option<&Type>,
        constructor: AdtConstructor<'_>,
        index: usize,
        _direct_payload_joins_independently: bool,
    ) -> Type {
        self.constraints
            .payload_expected(expected, constructor, index)
    }

    fn payload_can_infer_without_context(
        &self,
        constructor: AdtConstructor<'_>,
        index: usize,
        adts: &AdtRegistry,
    ) -> bool {
        self.constraints
            .payload_can_infer_without_context(constructor, index, adts)
    }

    fn lower_bound_admission(
        &self,
        field: Option<&AdtPayloadField>,
        actual: &Type,
        adts: &AdtRegistry,
    ) -> crate::aggregate_type_join::ConstructorLowerBoundAdmission {
        let Some(AdtPayloadField {
            ty: AdtPayloadType::TypeParameter(type_index),
            ..
        }) = field
        else {
            return crate::aggregate_type_join::ConstructorLowerBoundAdmission::NoPrior;
        };
        self.constraints
            .lower_bound_admission(*type_index, actual, adts)
    }

    #[allow(clippy::too_many_arguments)]
    fn commit(
        &mut self,
        field: Option<&AdtPayloadField>,
        constructor: AdtConstructor<'_>,
        index: usize,
        actual: &Type,
        _joined: bool,
        _merge_invariant: bool,
        adts: &AdtRegistry,
    ) -> Result<(), unification::TypeParameterContributionConflict> {
        let Some(field) = field else {
            return Ok(());
        };
        let _ = field;
        self.constraints
            .contribute_payload(constructor, index, actual, adts)
    }

    fn finish(self, expected: Option<&Type>, constructor: AdtConstructor<'_>) -> Vec<Type> {
        self.constraints.finish(expected, constructor)
    }
}

fn private_declared_call_return<'a>(
    segments: &[String],
    context: &'a PrivateSignatureInferContext<'_, '_>,
) -> Option<&'a Type> {
    match segments {
        [name] => context.returns_by_path.get(&(
            context.current_package.map(str::to_string),
            context.current_module.map(str::to_string),
            name.clone(),
        )),
        [_, .., name] => imported_use_for_path_in_owner(
            context.uses,
            &segments[..segments.len() - 1],
            context.current_module,
            context.current_package,
        )
        .and_then(|use_decl| {
            context.returns_by_path.get(&(
                use_decl.resolved_target().package,
                Some(use_decl.name.clone()),
                name.clone(),
            ))
        }),
        _ => None,
    }
}
