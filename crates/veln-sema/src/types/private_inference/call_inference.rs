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
    let field = constructor.variant.payload_fields.get(index);
    let payload_expected = type_args.payload_expected(expected, constructor, index, field);
    let plan = PrivatePayloadInferencePlan::new(expected.is_some(), field, &payload_expected);
    let Some(inferred_actual) = plan.infer(arg, &payload_expected, context) else {
        return;
    };
    let joined = type_args.try_join(field, &inferred_actual, context.adts);
    let merge_invariant =
        crate::type_relations::is_assignable_nested(&payload_expected, &inferred_actual)
            || unification::is_direct_variant_carrier(&payload_expected, &inferred_actual);
    if plan.unresolved_concrete_carrier && !merge_invariant {
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
                inferred_private_aggregate_member_type(outcome.ty, payload_expected)
            } else {
                outcome.ty
            }
        })
    }
}

struct PrivateConstructorTypeArgInference {
    inferred: Vec<Type>,
    joined: Vec<Option<crate::aggregate_type_join::AggregateTypeJoin>>,
    invariant: Vec<bool>,
    presentations: Vec<crate::type_relations::TypePresentationJoin>,
}

impl PrivateConstructorTypeArgInference {
    fn new(constructor: AdtConstructor<'_>) -> Self {
        let parameter_count = constructor.descriptor.type_parameters.len();
        crate::aggregate_type_join::record_work(parameter_count);
        Self {
            inferred: vec![Type::Unknown; parameter_count],
            joined: (0..parameter_count).map(|_| None).collect(),
            invariant: vec![false; parameter_count],
            presentations: (0..parameter_count).map(|_| Default::default()).collect(),
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
        let joined = self.joined[*type_index]
            .as_mut()
            .is_some_and(|joined| joined.try_join(actual));
        if joined {
            self.presentations[*type_index].merge(&mut self.inferred[*type_index], actual);
        }
        joined
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
    ) -> Result<(), unification::TypeParameterContributionConflict> {
        let Some(field) = field else {
            return Ok(());
        };
        if let AdtPayloadType::TypeParameter(type_index) = field.ty
            && joined
        {
            self.inferred[type_index] = self.joined[type_index]
                .as_ref()
                .expect("joined private aggregate type argument")
                .inference_type();
            return Ok(());
        }
        if !matches!(field.ty, AdtPayloadType::TypeParameter(_)) {
            if merge_invariant {
                return crate::aggregate_type_join::merge_invariant_payload_type_args(
                    &mut self.inferred,
                    &mut self.joined,
                    &mut self.invariant,
                    &mut self.presentations,
                    constructor,
                    index,
                    actual,
                );
            }
            return Ok(());
        }
        crate::aggregate_type_join::record_work(1);
        adt::merge_type_args_from_payload(&mut self.inferred, constructor, index, actual);
        let AdtPayloadType::TypeParameter(type_index) = field.ty else {
            return Ok(());
        };
        self.presentations[type_index].merge(&mut self.inferred[type_index], actual);
        if self.joined[type_index].is_none() {
            self.joined[type_index] = crate::aggregate_type_join::AggregateTypeJoin::new(
                adts,
                &self.inferred[type_index],
            );
            if let Some(joined) = &self.joined[type_index] {
                self.inferred[type_index] = joined.inference_type();
            }
        }
        Ok(())
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
