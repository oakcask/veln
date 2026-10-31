use super::*;
use crate::effect_rows::{
    call_bound_effect_row, effect_row_in_type, instantiate_effect_rows_in_type,
    solve_call_bound_effect_row,
};
use crate::types::effect_inference::user_effect_signature_for_path;

pub(super) fn type_for_effect_call_argument(
    expr: &Expr,
    expected: &Type,
    context: &ExprEffectContext<'_>,
) -> Option<Type> {
    match &expr.kind {
        ExprKind::SchemaDecode {
            schema,
            input,
            base,
        } => type_for_effect_schema_call(
            schema,
            schema_decode_step_function_name,
            &[input, base],
            context,
        ),
        ExprKind::SchemaEncode { schema, value } => {
            type_for_effect_schema_call(schema, schema_encode_function_name, &[value], context)
        }
        ExprKind::SchemaValidate {
            schema,
            value,
            base,
            ..
        } => type_for_effect_schema_call(
            schema,
            schema_runtime_validation_function_name,
            &[value, base],
            context,
        ),
        ExprKind::StringLiteral(_) => Some(Type::string()),
        ExprKind::IntLiteral(_) => Some(Type::int()),
        ExprKind::FloatLiteral(_) => Some(Type::float()),
        ExprKind::BoolLiteral(_) => Some(Type::bool()),
        ExprKind::Unit => Some(Type::unit()),
        ExprKind::Perform {
            effect,
            operation,
            args,
            ..
        } => type_for_effect_perform(effect, operation, args, context),
        ExprKind::Record(fields) => type_for_effect_record(fields, expected, context),
        ExprKind::Call { callee, args } => type_for_effect_call(callee, args, expected, context),
        ExprKind::Binary {
            op: veln_ast::BinaryOp::PipeGreater,
            left,
            right,
        } => type_for_effect_pipeline(left, right, expected, context),
        _ => function_type_for_expr(expr, context).or_else(|| {
            infer_checked_effect_argument_type(
                expr,
                expected,
                context.current_module,
                context.current_package,
                context.uses,
                context.bindings,
                context.functions,
                context.adts,
            )
        }),
    }
}

fn type_for_effect_perform(
    effect: &[String],
    operation: &str,
    args: &[Expr],
    context: &ExprEffectContext<'_>,
) -> Option<Type> {
    let effect = user_effect_signature_for_path(
        effect,
        context.uses,
        context.current_module,
        context.current_package,
        context.user_effects,
        context.companion_effect_access_targets,
    )?;
    let operation = effect
        .operations
        .iter()
        .find(|candidate| candidate.name == operation)?;
    if args.len() != operation.params.len() {
        return None;
    }
    for (arg, param) in args.iter().zip(&operation.params) {
        let actual = type_for_effect_call_argument(arg, param, context)?;
        if !crate::type_relations::is_assignable(param, &actual) {
            return None;
        }
    }
    Some(operation.return_type.clone())
}

fn type_for_effect_record(
    fields: &[veln_ast::RecordField],
    expected: &Type,
    context: &ExprEffectContext<'_>,
) -> Option<Type> {
    let Type::Record(expected_fields) = expected else {
        return None;
    };
    fields
        .iter()
        .map(|field| {
            let expected_field = expected_fields
                .iter()
                .find(|(name, _)| name == &field.name)
                .map(|(_, ty)| ty)?;
            Some((
                field.name.clone(),
                type_for_effect_call_argument(&field.expr, expected_field, context)?,
            ))
        })
        .collect::<Option<Vec<_>>>()
        .map(Type::Record)
}

fn type_for_effect_pipeline(
    left: &Expr,
    right: &Expr,
    expected: &Type,
    context: &ExprEffectContext<'_>,
) -> Option<Type> {
    let ExprKind::Call { callee, args } = &right.kind else {
        return None;
    };
    if !matches!(callee.kind, ExprKind::NamePath { .. }) {
        return None;
    }
    let mut piped_args = Vec::with_capacity(args.len() + 1);
    piped_args.push(left.clone());
    piped_args.extend(args.iter().cloned());
    type_for_effect_call(callee, &piped_args, expected, context)
}

fn type_for_effect_call(
    callee: &Expr,
    args: &[Expr],
    expected: &Type,
    context: &ExprEffectContext<'_>,
) -> Option<Type> {
    if let Some(constructed) = type_for_effect_constructor_call(callee, args, expected, context) {
        return Some(constructed);
    }
    let Type::Function {
        params,
        variadic,
        return_type,
        effects,
    } = function_type_for_expr(callee, context)?
    else {
        return None;
    };
    let arity_matches =
        args.len() >= params.len() && (variadic.is_some() || args.len() == params.len());
    if !arity_matches {
        return None;
    }
    let comparisons =
        effect_call_argument_comparisons(args, &params, variadic.as_deref(), context)?;
    let row = call_bound_effect_row_for_expr(callee, context)
        .or_else(|| call_bound_effect_row(&params, variadic.as_deref(), &return_type, &effects));
    let substitutions = solve_call_bound_effect_row(
        row.as_deref(),
        comparisons
            .iter()
            .map(|(expected, actual)| (*expected, actual)),
    );
    if comparisons.iter().any(|(expected, actual)| {
        !crate::type_relations::is_assignable(
            &instantiate_effect_rows_in_type(expected, &substitutions),
            actual,
        )
    }) {
        return None;
    }
    Some(instantiate_effect_rows_in_type(
        &return_type,
        &substitutions,
    ))
}

fn effect_call_argument_comparisons<'a>(
    args: &[Expr],
    params: &'a [Type],
    variadic: Option<&'a Type>,
    context: &ExprEffectContext<'_>,
) -> Option<Vec<(&'a Type, Type)>> {
    args.iter()
        .enumerate()
        .map(|(index, arg)| {
            let expected = params.get(index).or(variadic)?;
            Some((
                expected,
                type_for_effect_call_argument(arg, expected, context)?,
            ))
        })
        .collect()
}

fn type_for_effect_schema_call(
    schema: &[String],
    helper_name_for: fn(&str) -> String,
    args: &[&Expr],
    context: &ExprEffectContext<'_>,
) -> Option<Type> {
    let (schema_name, qualifier) = schema.split_last()?;
    let mut helper_path = qualifier.to_vec();
    helper_path.push(helper_name_for(schema_name));
    let signature = effect_function_signature_path(
        &helper_path,
        context.uses,
        context.functions,
        context.current_module,
        context.current_package,
        context.companion_access_targets,
    )?;
    if args.len() != signature.params.len() {
        return None;
    }
    for (arg, param) in args.iter().zip(&signature.params) {
        let actual = type_for_effect_call_argument(arg, param, context)?;
        if !crate::type_relations::is_assignable(param, &actual) {
            return None;
        }
    }
    Some(signature.return_type.clone())
}

fn type_for_effect_constructor_call(
    callee: &Expr,
    args: &[Expr],
    expected: &Type,
    context: &ExprEffectContext<'_>,
) -> Option<Type> {
    let segments = callee.callee_name_path()?;
    let descriptor = context
        .adts
        .descriptor_for_type_prefer_module(expected, context.current_module)?;
    let constructor = context.adts.constructor_for_descriptor(
        segments,
        descriptor,
        context.current_module,
        context.uses,
    )?;
    if args.len() != constructor.variant.payload_fields.len() {
        return None;
    }
    let expected_args = crate::adt::unification::adt_args(expected, descriptor)?;
    let EffectConstructorPayloads {
        mut actual_args,
        comparisons: payload_comparisons,
    } = effect_constructor_payloads(constructor, args, expected_args, context)?;
    let call_bound_row = effect_row_in_type(expected);
    let row_substitutions = solve_call_bound_effect_row(
        call_bound_row.as_deref(),
        payload_comparisons
            .iter()
            .map(|(expected, actual)| (expected, actual)),
    );
    if payload_comparisons.iter().any(|(expected, actual)| {
        !crate::type_relations::is_assignable(
            &instantiate_effect_rows_in_type(expected, &row_substitutions),
            actual,
        )
    }) {
        return None;
    }
    complete_constructor_type_args(&mut actual_args, expected_args);
    Some(crate::adt::type_operations::refined_constructed_type_from_args(constructor, &actual_args))
}

struct EffectConstructorPayloads {
    actual_args: Vec<Type>,
    comparisons: Vec<(Type, Type)>,
}

fn effect_constructor_payloads(
    constructor: crate::adt::descriptors::AdtConstructor<'_>,
    args: &[Expr],
    expected_args: &[Type],
    context: &ExprEffectContext<'_>,
) -> Option<EffectConstructorPayloads> {
    let mut actual_args = vec![Type::Unknown; expected_args.len()];
    let mut payload_comparisons = Vec::with_capacity(args.len());
    for (index, arg) in args.iter().enumerate() {
        let payload_expected = crate::adt::type_operations::payload_type_with_resolved_args(
            constructor,
            index,
            |type_index| expected_args[type_index].clone(),
        )?;
        let actual = type_for_effect_call_argument(arg, &payload_expected, context)?;
        crate::adt::type_operations::merge_type_args_from_payload(
            &mut actual_args,
            constructor,
            index,
            &actual,
        );
        payload_comparisons.push((payload_expected, actual));
    }
    Some(EffectConstructorPayloads {
        actual_args,
        comparisons: payload_comparisons,
    })
}

fn complete_constructor_type_args(actual_args: &mut [Type], expected_args: &[Type]) {
    for (actual, expected) in actual_args.iter_mut().zip(expected_args) {
        if *actual == Type::Unknown {
            *actual = expected.clone();
        }
    }
}
