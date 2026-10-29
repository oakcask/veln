use super::*;

use crate::adt::descriptors::{AdtConstructor, AdtDescriptor};
use crate::adt::registry::AdtRegistry;
use crate::adt::type_operations as adt;
use crate::name_recovery::{normal_use_decls, use_decl_is_recovery_candidate};
use crate::type_syntax::parse_type_annotation;

mod refinement;

use refinement::format_neutral_schema_written_constructors_are_visible;
pub(crate) use refinement::{
    format_neutral_schema_encode_field_contains_refinement,
    format_neutral_schema_encode_field_is_source_adt_candidate,
};

pub(super) fn format_neutral_schema_decode_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    let decoded_type = Type::Record(format_neutral_schema_decode_record_fields_with_adts(
        module, schema, adts,
    )?);
    Some(FunctionSignature {
        name: schema_decode_function_name(schema_name),
        target_name: format!(
            "{SCHEMA_NEUTRAL_DECODE_TARGET_PREFIX}{}",
            crate::schema::schema_link_name(module, schema)
        ),
        module_name: schema.module_name.clone(),
        package_name: schema.package_name.clone(),
        visibility: schema.visibility,
        params: vec![decoded_type.clone()],
        variadic: None,
        return_type: Type::named("Result", vec![decoded_type, Type::string()]),
        effects: Vec::new(),
        callsite: false,
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

pub(crate) fn format_neutral_schema_runtime_validation_types(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
) -> Option<(Type, Type)> {
    let (erased, refined, has_refinement) = format_neutral_schema_runtime_validation_types_inner(
        module,
        schema,
        adts,
        &mut Vec::new(),
    )?;
    has_refinement.then_some((erased, refined))
}

fn format_neutral_schema_runtime_validation_types_inner(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    composition_stack: &mut Vec<(Option<String>, Option<String>, String)>,
) -> Option<(Type, Type, bool)> {
    if schema.format.is_some()
        || !schema.validations.is_empty()
        || schema
            .fields
            .iter()
            .any(|field| field.where_clause.is_some())
    {
        return None;
    }
    let key = (
        schema.package_name.clone(),
        schema.module_name.clone(),
        schema.name.clone()?,
    );
    if composition_stack.contains(&key) {
        return None;
    }
    composition_stack.push(key);
    let mut erased_fields = Vec::with_capacity(schema.fields.len());
    let mut refined_fields = Vec::with_capacity(schema.fields.len());
    let mut has_refinement = false;
    for field in &schema.fields {
        let refined =
            format_neutral_schema_encode_field_type_for_schema(module, schema, adts, &field.ty)?;
        let traversal = if schema.visibility == Visibility::Public {
            FormatNeutralSchemaTraversal::EncodePublic
        } else {
            FormatNeutralSchemaTraversal::EncodePrivate
        };
        let erased = if let Some(child) =
            format_neutral_schema_composed_field_target(module, schema, adts, &field.ty, traversal)
        {
            let (child_erased, child_refined, child_has_refinement) =
                format_neutral_schema_runtime_validation_types_inner(
                    module,
                    child,
                    adts,
                    composition_stack,
                )?;
            if child_refined != refined {
                return None;
            }
            has_refinement |= child_has_refinement;
            child_erased
        } else {
            match &refined {
                Type::VariantRefinement {
                    name,
                    identity,
                    args,
                    unresolved_alternatives,
                    ..
                } if unresolved_alternatives.is_empty()
                    && args
                        .iter()
                        .all(|arg| !format_neutral_type_contains_refinement(arg)) =>
                {
                    has_refinement = true;
                    Type::resolved_named(name.clone(), identity.clone(), args.clone())
                }
                _ if !format_neutral_type_contains_refinement(&refined) => refined.clone(),
                _ => return None,
            }
        };
        erased_fields.push((field.name.clone(), erased));
        refined_fields.push((field.name.clone(), refined));
    }
    composition_stack.pop();
    Some((
        Type::Record(erased_fields),
        Type::Record(refined_fields),
        has_refinement,
    ))
}

fn format_neutral_type_contains_refinement(ty: &Type) -> bool {
    match ty {
        Type::VariantRefinement { .. } => true,
        Type::Named { args, .. } => args.iter().any(format_neutral_type_contains_refinement),
        Type::Record(fields) => fields
            .iter()
            .any(|(_, field_ty)| format_neutral_type_contains_refinement(field_ty)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(format_neutral_type_contains_refinement)
                || variadic
                    .as_deref()
                    .is_some_and(format_neutral_type_contains_refinement)
                || format_neutral_type_contains_refinement(return_type)
        }
        Type::Unknown => false,
    }
}

pub(crate) fn format_neutral_schema_decode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<(String, Type)>> {
    let adts = AdtRegistry::from_module(module);
    format_neutral_schema_decode_record_fields_with_adts(module, schema, &adts)
}

fn format_neutral_schema_decode_record_fields_with_adts(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
) -> Option<Vec<(String, Type)>> {
    schema
        .fields
        .iter()
        .map(|field| {
            Some((
                field.name.clone(),
                format_neutral_schema_field_type_for_schema(module, schema, adts, &field.ty)?,
            ))
        })
        .collect()
}

pub(crate) fn format_neutral_schema_field_type_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
) -> Option<Type> {
    let traversal = if schema.visibility == Visibility::Public {
        FormatNeutralSchemaTraversal::DecodePublic
    } else {
        FormatNeutralSchemaTraversal::DecodePrivate
    };
    format_neutral_schema_field_type_for_traversal(module, schema, adts, text, traversal)
}

fn format_neutral_schema_field_type_for_traversal(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    format_neutral_schema_resolved_field_type(
        module,
        schema,
        adts,
        text,
        traversal,
        &mut Vec::new(),
    )
}

fn format_neutral_schema_resolved_field_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
    traversal: FormatNeutralSchemaTraversal,
    composition_stack: &mut Vec<(Option<String>, Option<String>, String)>,
) -> Option<Type> {
    if let Some(ty) = format_neutral_schema_non_composed_field_type_for_traversal(
        module, schema, adts, text, traversal,
    ) {
        return Some(ty);
    }
    if let Some(target) = schema_field_target(module, schema, text)
        && target.format.is_none()
    {
        return format_neutral_schema_composition_value_type(
            module,
            target,
            traversal,
            composition_stack,
        );
    }
    None
}

fn format_neutral_schema_non_composed_field_type_for_traversal(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    let parsed = parse_type_annotation(text).ok()?;
    if !format_neutral_schema_written_constructors_are_visible(
        module,
        schema.module_name.as_deref(),
        schema.package_name.as_deref(),
        adts,
        &parsed,
        traversal,
    ) {
        return None;
    }
    let ty = crate::types::canonicalize_type_effects_in_owner(
        parsed,
        &normal_use_decls(module),
        &module
            .uses
            .iter()
            .filter(|use_decl| use_decl_is_recovery_candidate(module, use_decl))
            .cloned()
            .collect::<Vec<_>>(),
        schema.module_name.as_deref(),
        schema.package_name.as_deref(),
        &[],
        adts,
        &BTreeMap::new(),
    );
    if let Some(ty) = format_neutral_schema_visible_shape_type_for_schema(
        module,
        schema.module_name.as_deref(),
        schema.package_name.as_deref(),
        adts,
        &ty,
        &mut FormatNeutralSchemaTraversalState::default(),
        traversal,
    ) {
        return Some(ty);
    }
    None
}

pub(crate) fn binary_schema_anonymous_record_decode_type(text: &str) -> Option<Type> {
    let Type::Record(fields) = parse_type_annotation(text).ok()? else {
        return None;
    };
    binary_schema_anonymous_record_type(fields).map(Type::Record)
}

fn binary_schema_anonymous_record_type(fields: Vec<(String, Type)>) -> Option<Vec<(String, Type)>> {
    fields
        .into_iter()
        .map(|(name, ty)| {
            if binary_schema_anonymous_record_leaf_type(&ty).is_some() {
                return Some((name, Type::int()));
            }
            let Type::Record(fields) = ty else {
                return None;
            };
            Some((
                name,
                Type::Record(binary_schema_anonymous_record_type(fields)?),
            ))
        })
        .collect()
}

fn binary_schema_anonymous_record_leaf_type(ty: &Type) -> Option<()> {
    match ty {
        Type::Named { name, args, .. }
            if args.is_empty() && exact_width_schema_primitive(name).is_some() =>
        {
            Some(())
        }
        _ => None,
    }
}

fn format_neutral_schema_scalar_type_is_supported(name: &str, args: &[Type]) -> bool {
    args.is_empty() && matches!(name, "Int" | "Bool" | "Float" | "String")
}

fn format_neutral_schema_scalar_type(ty: &Type) -> bool {
    matches!(
        ty,
        Type::Named { name, args, .. }
            if format_neutral_schema_scalar_type_is_supported(name, args)
    )
}

pub(crate) fn format_neutral_schema_encode_field_type_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
) -> Option<Type> {
    let traversal = if schema.visibility == Visibility::Public {
        FormatNeutralSchemaTraversal::EncodePublic
    } else {
        FormatNeutralSchemaTraversal::EncodePrivate
    };
    format_neutral_schema_field_type_for_traversal(module, schema, adts, text, traversal)
}

pub(super) fn format_neutral_schema_encode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
) -> Option<Vec<(String, Type)>> {
    schema
        .fields
        .iter()
        .map(|field| {
            let ty = format_neutral_schema_encode_field_type_for_schema(
                module, schema, adts, &field.ty,
            )?;
            Some((field.name.clone(), ty))
        })
        .collect()
}

pub(crate) fn format_neutral_schema_first_unsupported_encode_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
) -> Option<SchemaField> {
    if schema.format.is_some() {
        return None;
    }
    schema
        .fields
        .iter()
        .find(|field| {
            let declaration_diagnostic_exists =
                format_neutral_schema_field_type_for_schema(module, schema, adts, &field.ty)
                    .is_none()
                    && format_neutral_schema_encode_field_is_source_adt_candidate(
                        module, schema, adts, &field.ty,
                    );
            !declaration_diagnostic_exists
                && format_neutral_schema_encode_field_type_for_schema(
                    module, schema, adts, &field.ty,
                )
                .is_none()
        })
        .cloned()
}

#[derive(Clone, PartialEq, Eq)]
struct FormatNeutralSchemaAdtFrame {
    package_name: Option<String>,
    module_name: Option<String>,
    type_name: String,
    type_arguments: Vec<Type>,
}

#[derive(Default)]
struct FormatNeutralSchemaTraversalState {
    stack: Vec<FormatNeutralSchemaAdtFrame>,
    stack_cacheable: Vec<bool>,
    completed: Vec<(FormatNeutralSchemaAdtFrame, bool)>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FormatNeutralSchemaTraversal {
    DecodePrivate,
    DecodePublic,
    EncodePrivate,
    EncodePublic,
}

impl FormatNeutralSchemaTraversal {
    fn permits_private_variants(self) -> bool {
        matches!(self, Self::DecodePrivate | Self::EncodePrivate)
    }

    fn is_decode(self) -> bool {
        matches!(self, Self::DecodePrivate | Self::DecodePublic)
    }
}

fn format_neutral_schema_composed_field_target<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<&'a SchemaDecl> {
    if format_neutral_schema_non_composed_field_type_for_traversal(
        module, schema, adts, text, traversal,
    )
    .is_some()
    {
        return None;
    }
    schema_field_target(module, schema, text)
}

fn format_neutral_schema_composition_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    traversal: FormatNeutralSchemaTraversal,
    stack: &mut Vec<(Option<String>, Option<String>, String)>,
) -> Option<Type> {
    let key = (
        schema.package_name.clone(),
        schema.module_name.clone(),
        schema.name.clone()?,
    );
    if stack.contains(&key) {
        return None;
    }
    stack.push(key);
    let adts = AdtRegistry::from_module(module);
    let mut fields = Vec::new();
    for field in &schema.fields {
        let ty = format_neutral_schema_resolved_field_type(
            module, schema, &adts, &field.ty, traversal, stack,
        )?;
        fields.push((field.name.clone(), ty));
    }
    stack.pop();
    Some(Type::Record(fields))
}

fn format_neutral_schema_visible_shape_type_for_schema(
    module: &SurfaceModule,
    current_module: Option<&str>,
    current_package: Option<&str>,
    adts: &AdtRegistry,
    ty: &Type,
    state: &mut FormatNeutralSchemaTraversalState,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    match ty {
        Type::Named { name, args, .. } => format_neutral_schema_named_visible_shape_type(
            (module, current_module, current_package, adts, traversal),
            ty,
            name,
            args,
            state,
        ),
        Type::VariantRefinement { .. } => format_neutral_schema_source_adt_type(
            module,
            current_module,
            current_package,
            adts,
            ty,
            state,
            traversal,
        ),
        Type::Record(fields) => Some(Type::Record(
            fields
                .iter()
                .map(|(name, field_ty)| {
                    Some((
                        name.clone(),
                        format_neutral_schema_visible_shape_type_for_schema(
                            module,
                            current_module,
                            current_package,
                            adts,
                            field_ty,
                            state,
                            traversal,
                        )?,
                    ))
                })
                .collect::<Option<Vec<_>>>()?,
        )),
        _ => None,
    }
}

type FormatNeutralSchemaContext<'a> = (
    &'a SurfaceModule,
    Option<&'a str>,
    Option<&'a str>,
    &'a AdtRegistry,
    FormatNeutralSchemaTraversal,
);

fn format_neutral_schema_named_visible_shape_type(
    context: FormatNeutralSchemaContext<'_>,
    ty: &Type,
    name: &str,
    args: &[Type],
    state: &mut FormatNeutralSchemaTraversalState,
) -> Option<Type> {
    let (module, current_module, current_package, adts, traversal) = context;
    match (name, args) {
        ("List" | "Vec", [item]) | ("Option", [item]) => Some(Type::named(
            name,
            vec![format_neutral_schema_visible_shape_type_for_schema(
                module,
                current_module,
                current_package,
                adts,
                item,
                state,
                traversal,
            )?],
        )),
        ("Dict", [key, value]) if *key == Type::string() => Some(Type::dict(
            Type::string(),
            format_neutral_schema_visible_shape_type_for_schema(
                module,
                current_module,
                current_package,
                adts,
                value,
                state,
                traversal,
            )?,
        )),
        ("Result", [ok, error]) => Some(Type::named(
            "Result",
            vec![
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    current_package,
                    adts,
                    ok,
                    state,
                    traversal,
                )?,
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    current_package,
                    adts,
                    error,
                    state,
                    traversal,
                )?,
            ],
        )),
        _ if format_neutral_schema_scalar_type(ty) => Some(ty.clone()),
        _ => format_neutral_schema_source_adt_type(
            module,
            current_module,
            current_package,
            adts,
            ty,
            state,
            traversal,
        ),
    }
}

fn format_neutral_schema_source_adt_type(
    module: &SurfaceModule,
    current_module: Option<&str>,
    current_package: Option<&str>,
    adts: &AdtRegistry,
    ty: &Type,
    state: &mut FormatNeutralSchemaTraversalState,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    let descriptor = format_neutral_schema_source_adt_descriptor(
        module,
        current_module,
        current_package,
        adts,
        ty,
    )?;
    if !format_neutral_schema_refinement_variants_are_visible(
        ty,
        descriptor,
        current_module,
        current_package,
        traversal,
    ) {
        return None;
    }
    let descriptor_ty = format_neutral_schema_descriptor_type(ty, descriptor);
    let type_arguments = match &descriptor_ty {
        Type::Named { args, .. } | Type::VariantRefinement { args, .. } => args,
        _ => return None,
    };
    let frame = FormatNeutralSchemaAdtFrame {
        package_name: descriptor.package_name.clone(),
        module_name: descriptor.module_name.clone(),
        type_name: descriptor.type_name.clone(),
        type_arguments: type_arguments.clone(),
    };
    if let Some(result) = format_neutral_schema_prior_adt_traversal_result(
        (module, current_module, current_package, adts, traversal),
        descriptor,
        &descriptor_ty,
        type_arguments,
        &frame,
        state,
    ) {
        return result;
    }
    state.stack.push(frame.clone());
    state.stack_cacheable.push(true);
    let supported = format_neutral_schema_adt_payloads_are_supported(
        module,
        adts,
        descriptor,
        &descriptor_ty,
        state,
        traversal,
    );
    state.stack.pop();
    if state
        .stack_cacheable
        .pop()
        .expect("ADT traversal cacheability should match the active stack")
    {
        state.completed.push((frame, supported));
    }
    supported.then_some(descriptor_ty)
}

fn format_neutral_schema_refinement_variants_are_visible(
    ty: &Type,
    descriptor: &AdtDescriptor,
    current_module: Option<&str>,
    current_package: Option<&str>,
    traversal: FormatNeutralSchemaTraversal,
) -> bool {
    let Type::VariantRefinement { variants, .. } = ty else {
        return true;
    };
    variants.iter().all(|selected| {
        descriptor.variants.iter().any(|variant| {
            variant.name == *selected
                && (variant.visibility == Visibility::Public
                    || traversal.permits_private_variants()
                        && descriptor.module_name.as_deref() == current_module
                        && descriptor.package_name.as_deref() == current_package)
        })
    })
}

fn format_neutral_schema_prior_adt_traversal_result(
    context: FormatNeutralSchemaContext<'_>,
    descriptor: &AdtDescriptor,
    descriptor_ty: &Type,
    type_arguments: &[Type],
    frame: &FormatNeutralSchemaAdtFrame,
    state: &mut FormatNeutralSchemaTraversalState,
) -> Option<Option<Type>> {
    let (module, _, _, adts, traversal) = context;
    if let Some((_, supported)) = state.completed.iter().find(|(key, _)| key == frame) {
        return Some((*supported).then(|| descriptor_ty.clone()));
    }
    if let Some(index) = state.stack.iter().position(|active| active == frame) {
        state.stack_cacheable[index + 1..].fill(false);
        return Some(Some(descriptor_ty.clone()));
    }
    let recursive_index = state.stack.iter().position(|active| {
        active.module_name == frame.module_name
            && active.type_name == frame.type_name
            && active.package_name == frame.package_name
    })?;
    state.stack_cacheable[recursive_index + 1..].fill(false);
    if traversal.is_decode() {
        return Some(Some(descriptor_ty.clone()));
    }
    Some(
        type_arguments
            .iter()
            .all(|arg| {
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    descriptor.module_name.as_deref(),
                    descriptor.package_name.as_deref(),
                    adts,
                    arg,
                    state,
                    traversal,
                )
                .is_some()
            })
            .then(|| descriptor_ty.clone()),
    )
}

fn format_neutral_schema_adt_payloads_are_supported(
    module: &SurfaceModule,
    adts: &AdtRegistry,
    descriptor: &AdtDescriptor,
    descriptor_ty: &Type,
    state: &mut FormatNeutralSchemaTraversalState,
    traversal: FormatNeutralSchemaTraversal,
) -> bool {
    descriptor.variants.iter().all(|variant| {
        variant
            .payload_fields
            .iter()
            .enumerate()
            .all(|(index, _field)| {
                let Some(payload_ty) = adt::payload_type(
                    descriptor_ty,
                    AdtConstructor {
                        descriptor,
                        variant,
                    },
                    index,
                ) else {
                    return false;
                };
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    descriptor.module_name.as_deref(),
                    descriptor.package_name.as_deref(),
                    adts,
                    &payload_ty,
                    state,
                    traversal,
                )
                .is_some()
            })
    })
}

fn format_neutral_schema_descriptor_type(ty: &Type, descriptor: &AdtDescriptor) -> Type {
    match ty {
        Type::Named { args, .. } => Type::resolved_named(
            descriptor.type_name.clone(),
            descriptor.identity(),
            args.clone(),
        ),
        Type::VariantRefinement {
            args,
            variants,
            unresolved_alternatives,
            ..
        } if unresolved_alternatives.is_empty() => Type::resolved_variant_refinement_shared(
            descriptor.type_name.clone(),
            descriptor.identity(),
            args.clone(),
            Arc::clone(variants),
        ),
        _ => ty.clone(),
    }
}

fn format_neutral_schema_source_adt_descriptor<'a>(
    module: &SurfaceModule,
    current_module: Option<&str>,
    current_package: Option<&str>,
    adts: &'a AdtRegistry,
    ty: &Type,
) -> Option<&'a AdtDescriptor> {
    let has_resolved_identity = matches!(
        ty,
        Type::Named { name, identity, .. } | Type::VariantRefinement { name, identity, .. }
            if identity != name
    );
    if let Some(descriptor) = adts.descriptor_for_type(ty)
        && ((descriptor.module_name.is_none() && descriptor.package_name.is_none())
            || (descriptor.module_name.as_deref() == current_module
                && descriptor.package_name.as_deref() == current_package)
            || (has_resolved_identity && descriptor.visibility == Visibility::Public))
        && format_neutral_schema_descriptor_supports_type(adts, descriptor, ty)
    {
        return Some(descriptor);
    }
    let (name, args) = match ty {
        Type::Named { name, args, .. } | Type::VariantRefinement { name, args, .. } => (name, args),
        _ => return None,
    };
    let segments = name.split("::").collect::<Vec<_>>();
    match segments.as_slice() {
        [local_name] => format_neutral_schema_local_adt_descriptor(
            current_module,
            current_package,
            adts,
            ty,
            local_name,
            args,
        ),
        [_, .., type_name] => format_neutral_schema_imported_adt_descriptor(
            module,
            (current_module, current_package),
            adts,
            ty,
            &segments[..segments.len() - 1],
            type_name,
            args.len(),
        ),
        _ => None,
    }
}

fn format_neutral_schema_descriptor_supports_type(
    adts: &AdtRegistry,
    descriptor: &AdtDescriptor,
    ty: &Type,
) -> bool {
    !matches!(ty, Type::VariantRefinement { .. })
        || descriptor_has_source_declaration(adts, descriptor)
}

fn format_neutral_schema_local_adt_descriptor<'a>(
    current_module: Option<&str>,
    current_package: Option<&str>,
    adts: &'a AdtRegistry,
    ty: &Type,
    local_name: &str,
    args: &[Type],
) -> Option<&'a AdtDescriptor> {
    adts.descriptors().iter().rev().find(|descriptor| {
        descriptor.type_name == local_name
            && descriptor.module_name.as_deref() == current_module
            && descriptor.package_name.as_deref() == current_package
            && descriptor.type_parameters.len() == args.len()
            && format_neutral_schema_descriptor_supports_type(adts, descriptor, ty)
    })
}

fn format_neutral_schema_imported_adt_descriptor<'a>(
    module: &SurfaceModule,
    owner: (Option<&str>, Option<&str>),
    adts: &'a AdtRegistry,
    ty: &Type,
    import_segments: &[&str],
    type_name: &str,
    argument_count: usize,
) -> Option<&'a AdtDescriptor> {
    let import_path = import_segments
        .iter()
        .map(|segment| (*segment).to_string())
        .collect::<Vec<_>>();
    let use_decl = normal_imported_use_for_path_in_owner(module, &import_path, owner.0, owner.1)?;
    let target = use_decl.resolved_target();
    adts.descriptors().iter().rev().find(|descriptor| {
        descriptor.type_name == type_name
            && descriptor.module_name.as_deref() == Some(target.module.as_str())
            && descriptor.package_name == target.package
            && descriptor.type_parameters.len() == argument_count
            && descriptor.visibility == Visibility::Public
            && format_neutral_schema_descriptor_supports_type(adts, descriptor, ty)
    })
}

fn descriptor_has_source_declaration(adts: &AdtRegistry, descriptor: &AdtDescriptor) -> bool {
    adts.declaration_span(descriptor).is_some()
        || adts
            .terminal_descriptor_for_alias(descriptor)
            .is_some_and(|terminal| adts.declaration_span(terminal).is_some())
}
