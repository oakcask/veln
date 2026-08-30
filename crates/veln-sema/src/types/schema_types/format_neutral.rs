use super::*;

pub(super) fn format_neutral_schema_decode_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    let decoded_type = Type::Record(format_neutral_schema_decode_record_fields(module, schema)?);
    Some(FunctionSignature {
        name: schema_decode_function_name(schema_name),
        target_name: format!("{SCHEMA_NEUTRAL_DECODE_TARGET_PREFIX}{schema_name}"),
        module_name: schema.module_name.clone(),
        visibility: schema.visibility,
        params: vec![decoded_type.clone()],
        variadic: None,
        return_type: Type::named("Result", vec![decoded_type, Type::string()]),
        effects: Vec::new(),
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

pub(crate) fn format_neutral_schema_decode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<(String, Type)>> {
    let adts = AdtRegistry::from_module(module);
    schema
        .fields
        .iter()
        .map(|field| {
            Some((
                field.name.clone(),
                format_neutral_schema_field_type_for_schema(module, schema, &adts, &field.ty)?,
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
    format_neutral_schema_field_type_for_traversal(
        module,
        schema,
        adts,
        text,
        FormatNeutralSchemaTraversal::Decode,
    )
}

fn format_neutral_schema_field_type_for_traversal(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    let ty = parse_type_annotation(text).ok()?;
    if let Some(ty) = format_neutral_schema_visible_shape_type_for_schema(
        module,
        schema.module_name.as_deref(),
        adts,
        &ty,
        &mut FormatNeutralSchemaTraversalState::default(),
        traversal,
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
            &mut Vec::new(),
        );
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
        Type::Named { name, args }
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
        Type::Named { name, args }
            if format_neutral_schema_scalar_type_is_supported(name, args)
    )
}

pub(crate) fn format_neutral_schema_encode_field_type_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    adts: &AdtRegistry,
    text: &str,
) -> Option<Type> {
    format_neutral_schema_field_type_for_traversal(
        module,
        schema,
        adts,
        text,
        FormatNeutralSchemaTraversal::Encode,
    )
}

pub(crate) fn format_neutral_schema_encode_field_is_source_adt_candidate(text: &str) -> bool {
    parse_type_annotation(text)
        .ok()
        .is_some_and(|ty| format_neutral_schema_encode_type_is_source_adt_candidate(&ty))
}

fn format_neutral_schema_encode_type_is_source_adt_candidate(ty: &Type) -> bool {
    matches!(ty, Type::Named { name, .. } if !matches!(
        name.as_str(),
        "Int" | "Bool" | "Float" | "String" | "Option" | "List" | "Vec" | "Dict" | "Result"
    ))
}

pub(super) fn format_neutral_schema_encode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<(String, Type)>> {
    let adts = AdtRegistry::from_module(module);
    schema
        .fields
        .iter()
        .map(|field| {
            let ty = format_neutral_schema_encode_field_type_for_schema(
                module, schema, &adts, &field.ty,
            )?;
            Some((field.name.clone(), ty))
        })
        .collect()
}

pub(crate) fn format_neutral_schema_first_unsupported_encode_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<SchemaField> {
    if schema.format.is_some() {
        return None;
    }
    let adts = AdtRegistry::from_module(module);
    schema
        .fields
        .iter()
        .find(|field| {
            let declaration_diagnostic_exists =
                format_neutral_schema_field_type_for_schema(module, schema, &adts, &field.ty)
                    .is_none()
                    && format_neutral_schema_encode_field_is_source_adt_candidate(&field.ty);
            !declaration_diagnostic_exists
                && format_neutral_schema_encode_field_type_for_schema(
                    module, schema, &adts, &field.ty,
                )
                .is_none()
        })
        .cloned()
}

#[derive(Clone, PartialEq, Eq)]
struct FormatNeutralSchemaAdtFrame {
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
    Decode,
    Encode,
}

fn format_neutral_schema_composition_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    traversal: FormatNeutralSchemaTraversal,
    stack: &mut Vec<(Option<String>, String)>,
) -> Option<Type> {
    let key = (schema.module_name.clone(), schema.name.clone()?);
    if stack.contains(&key) {
        return None;
    }
    stack.push(key);
    let adts = AdtRegistry::from_module(module);
    let mut fields = Vec::new();
    for field in &schema.fields {
        let parsed = parse_type_annotation(&field.ty).ok()?;
        let ty = if let Some(ty) = format_neutral_schema_visible_shape_type_for_schema(
            module,
            schema.module_name.as_deref(),
            &adts,
            &parsed,
            &mut FormatNeutralSchemaTraversalState::default(),
            traversal,
        ) {
            ty
        } else if let Some(target) = schema_field_target(module, schema, &field.ty)
            && target.format.is_none()
        {
            format_neutral_schema_composition_value_type(module, target, traversal, stack)?
        } else {
            return None;
        };
        fields.push((field.name.clone(), ty));
    }
    stack.pop();
    Some(Type::Record(fields))
}

fn format_neutral_schema_visible_shape_type_for_schema(
    module: &SurfaceModule,
    current_module: Option<&str>,
    adts: &AdtRegistry,
    ty: &Type,
    state: &mut FormatNeutralSchemaTraversalState,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    match ty {
        Type::Named { name, args }
            if matches!(name.as_str(), "List" | "Vec") && args.len() == 1 =>
        {
            Some(Type::named(
                name.clone(),
                vec![format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[0],
                    state,
                    traversal,
                )?],
            ))
        }
        Type::Named { name, args } if name == "Option" && args.len() == 1 => Some(Type::named(
            "Option",
            vec![format_neutral_schema_visible_shape_type_for_schema(
                module,
                current_module,
                adts,
                &args[0],
                state,
                traversal,
            )?],
        )),
        Type::Named { name, args } if name == "Dict" && args.len() == 2 => {
            if !matches!(&args[0], Type::Named { name, args } if name == "String" && args.is_empty())
            {
                return None;
            }
            Some(Type::dict(
                Type::string(),
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[1],
                    state,
                    traversal,
                )?,
            ))
        }
        Type::Named { name, args } if name == "Result" && args.len() == 2 => Some(Type::named(
            "Result",
            vec![
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[0],
                    state,
                    traversal,
                )?,
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    current_module,
                    adts,
                    &args[1],
                    state,
                    traversal,
                )?,
            ],
        )),
        Type::Named { .. } if format_neutral_schema_scalar_type(ty) => Some(ty.clone()),
        Type::Named { .. } => format_neutral_schema_source_adt_type(
            module,
            current_module,
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

fn format_neutral_schema_source_adt_type(
    module: &SurfaceModule,
    current_module: Option<&str>,
    adts: &AdtRegistry,
    ty: &Type,
    state: &mut FormatNeutralSchemaTraversalState,
    traversal: FormatNeutralSchemaTraversal,
) -> Option<Type> {
    let descriptor = format_neutral_schema_source_adt_descriptor(module, current_module, adts, ty)?;
    let descriptor_ty = format_neutral_schema_descriptor_type(ty, descriptor);
    let Type::Named {
        args: type_arguments,
        ..
    } = &descriptor_ty
    else {
        return None;
    };
    let frame = FormatNeutralSchemaAdtFrame {
        module_name: descriptor.module_name.clone(),
        type_name: descriptor.type_name.clone(),
        type_arguments: type_arguments.clone(),
    };
    if let Some((_, supported)) = state.completed.iter().find(|(key, _)| key == &frame) {
        return (*supported).then_some(descriptor_ty);
    }
    if let Some(index) = state.stack.iter().position(|active| active == &frame) {
        state.stack_cacheable[index + 1..].fill(false);
        return Some(descriptor_ty);
    }
    if let Some(index) = state.stack.iter().position(|active| {
        active.module_name == frame.module_name && active.type_name == frame.type_name
    }) {
        state.stack_cacheable[index + 1..].fill(false);
        if traversal == FormatNeutralSchemaTraversal::Decode {
            return Some(descriptor_ty);
        }
        return type_arguments
            .iter()
            .all(|arg| {
                format_neutral_schema_visible_shape_type_for_schema(
                    module,
                    descriptor.module_name.as_deref(),
                    adts,
                    arg,
                    state,
                    traversal,
                )
                .is_some()
            })
            .then_some(descriptor_ty);
    }
    state.stack.push(frame.clone());
    state.stack_cacheable.push(true);
    let supported = descriptor.variants.iter().all(|variant| {
        variant
            .payload_fields
            .iter()
            .enumerate()
            .all(|(index, _field)| {
                let Some(payload_ty) = adt::payload_type(
                    &descriptor_ty,
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
                    adts,
                    &payload_ty,
                    state,
                    traversal,
                )
                .is_some()
            })
    });
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

fn format_neutral_schema_descriptor_type(ty: &Type, descriptor: &AdtDescriptor) -> Type {
    let Type::Named { args, .. } = ty else {
        return ty.clone();
    };
    Type::named(descriptor.type_name.clone(), args.clone())
}

fn format_neutral_schema_source_adt_descriptor<'a>(
    module: &SurfaceModule,
    current_module: Option<&str>,
    adts: &'a AdtRegistry,
    ty: &Type,
) -> Option<&'a AdtDescriptor> {
    let Type::Named { name, args } = ty else {
        return None;
    };
    let segments = name.split("::").collect::<Vec<_>>();
    match segments.as_slice() {
        [local_name] => adts
            .descriptor_for_type_in_module(&Type::named(*local_name, args.clone()), current_module),
        [_, .., type_name] => {
            let import_path = segments[..segments.len() - 1]
                .iter()
                .map(|segment| (*segment).to_string())
                .collect::<Vec<_>>();
            let use_decl = normal_imported_use_for_path(module, &import_path, current_module)?;
            adts.descriptors().iter().rev().find(|descriptor| {
                descriptor.type_name == *type_name
                    && descriptor.module_name.as_deref() == Some(use_decl.name.as_str())
                    && descriptor.type_parameters.len() == args.len()
                    && descriptor.visibility == Visibility::Public
            })
        }
        _ => None,
    }
}
