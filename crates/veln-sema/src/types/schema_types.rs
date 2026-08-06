use super::*;

mod schema_encode;

pub(super) fn codec_call_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
) -> Vec<CodecCallSignature> {
    module
        .codecs
        .iter()
        .flat_map(|codec| {
            let name = codec.name.clone()?;
            Some(
                codec
                    .implementations
                    .iter()
                    .flat_map(move |implementation| {
                        match (&implementation.direction, &implementation.kind) {
                            (
                                CodecDirection::Decode,
                                CodecImplementationKind::With {
                                    function: Some(function_name),
                                },
                            ) => codec_with_signature(
                                codec,
                                functions,
                                name.clone(),
                                function_name,
                                CodecCallBoundary::HandWrittenDecode,
                            )
                            .into_iter()
                            .collect(),
                            (
                                CodecDirection::Encode,
                                CodecImplementationKind::With {
                                    function: Some(function_name),
                                },
                            ) => codec_with_signature(
                                codec,
                                functions,
                                name.clone(),
                                function_name,
                                CodecCallBoundary::Direct,
                            )
                            .into_iter()
                            .collect(),
                            (CodecDirection::Decode, CodecImplementationKind::Derive) => {
                                codec_derive_decode_signature(
                                    module,
                                    functions,
                                    codec,
                                    name.clone(),
                                )
                                .into_iter()
                                .collect()
                            }
                            (CodecDirection::Encode, CodecImplementationKind::Derive) => {
                                codec_derive_encode_signatures(
                                    module,
                                    functions,
                                    codec,
                                    name.clone(),
                                )
                            }
                            (_, CodecImplementationKind::With { function: None }) => Vec::new(),
                        }
                    }),
            )
        })
        .flatten()
        .collect()
}

fn codec_with_signature(
    codec: &CodecDecl,
    functions: &[FunctionSignature],
    name: String,
    function_name: &str,
    boundary: CodecCallBoundary,
) -> Option<CodecCallSignature> {
    let function = functions.iter().find(|function| {
        function.name == function_name && function.module_name == codec.module_name
    })?;
    Some(CodecCallSignature {
        name,
        target_name: function.target_name.clone(),
        boundary,
        module_name: codec.module_name.clone(),
        visibility: codec.visibility,
        params: function.params.clone(),
        return_type: function.return_type.clone(),
        effects: function.effects.clone(),
        node_id: codec.node_id,
        span: codec.span.clone(),
    })
}

fn codec_derive_decode_signature(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    codec: &CodecDecl,
    name: String,
) -> Option<CodecCallSignature> {
    let schema = codec_referenced_schema(module, codec)?;
    let schema_name = schema.name.as_ref()?;
    let step_name = schema_decode_step_function_name(schema_name);
    let function = functions.iter().find(|function| {
        function.name == step_name && function.module_name == schema.module_name
    })?;
    Some(CodecCallSignature {
        name,
        target_name: function.target_name.clone(),
        boundary: CodecCallBoundary::Direct,
        module_name: codec.module_name.clone(),
        visibility: codec.visibility,
        params: function.params.clone(),
        return_type: function.return_type.clone(),
        effects: function.effects.clone(),
        node_id: codec.node_id,
        span: codec.span.clone(),
    })
}

fn codec_derive_encode_signatures(
    module: &SurfaceModule,
    functions: &[FunctionSignature],
    codec: &CodecDecl,
    name: String,
) -> Vec<CodecCallSignature> {
    let Some(schema) = codec_referenced_schema(module, codec) else {
        return Vec::new();
    };
    let Some(schema_name) = schema.name.as_ref() else {
        return Vec::new();
    };
    let encode_name = schema_encode_function_name(schema_name);
    let Some(function) = functions.iter().find(|function| {
        function.name == encode_name && function.module_name == schema.module_name
    }) else {
        return Vec::new();
    };
    let unbounded = CodecCallSignature {
        name,
        target_name: format!("{SCHEMA_ENCODE_STEP_TARGET_PREFIX}{schema_name}"),
        boundary: CodecCallBoundary::Direct,
        module_name: codec.module_name.clone(),
        visibility: codec.visibility,
        params: function.params.clone(),
        return_type: Type::named("EncodeStep", vec![Type::unit()]),
        effects: function.effects.clone(),
        node_id: codec.node_id,
        span: codec.span.clone(),
    };
    let Some(value_type) = function.params.first().cloned() else {
        return vec![unbounded];
    };
    let mut state_fields = match &value_type {
        Type::Record(fields) => fields.clone(),
        _ => Vec::new(),
    };
    state_fields.push((
        "encoded_offset".to_string(),
        Type::named("ByteCount", Vec::new()),
    ));
    let budgeted = CodecCallSignature {
        name: unbounded.name.clone(),
        target_name: unbounded.target_name.clone(),
        boundary: unbounded.boundary,
        module_name: unbounded.module_name.clone(),
        visibility: unbounded.visibility,
        params: vec![value_type, Type::named("ByteCount", Vec::new())],
        return_type: Type::named("EncodeStep", vec![Type::Record(state_fields)]),
        effects: unbounded.effects.clone(),
        node_id: unbounded.node_id,
        span: unbounded.span.clone(),
    };
    vec![unbounded, budgeted]
}

fn codec_referenced_schema<'a>(
    module: &'a SurfaceModule,
    codec: &CodecDecl,
) -> Option<&'a SchemaDecl> {
    let schema_name = codec.schema.as_ref()?;
    let segments = schema_name
        .split("::")
        .map(str::to_string)
        .collect::<Vec<_>>();
    schema_reference(
        module,
        &segments,
        codec.module_name.as_deref(),
        true,
        &mut Vec::new(),
    )
}

fn schema_reference<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> Option<&'a SchemaDecl> {
    let companion_access_targets = companion_access_targets(module);
    match segments {
        [name] => schema_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
        ),
        [_, .., name] => {
            let use_decl = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            schema_in_module(
                module,
                Some(&use_decl.name),
                name,
                companion_private_schema_access_allowed(
                    use_decl,
                    current_module,
                    &companion_access_targets,
                ),
                visited_aliases,
            )
        }
        _ => None,
    }
}

pub(crate) fn schema_field_target<'a>(
    module: &'a SurfaceModule,
    containing_schema: &SchemaDecl,
    text: &str,
) -> Option<&'a SchemaDecl> {
    if schema_field_uses_existing_grammar(containing_schema, text) {
        return None;
    }
    let segments = schema_payload_name_path(text)?;
    schema_reference(
        module,
        &segments,
        containing_schema.module_name.as_deref(),
        true,
        &mut Vec::new(),
    )
}

pub(crate) fn schema_field_uses_existing_grammar(schema: &SchemaDecl, text: &str) -> bool {
    match schema.format.as_ref().map(|format| format.name.as_str()) {
        None => matches!(text, "Int" | "Bool" | "Float" | "String"),
        Some("binary") => {
            exact_width_schema_primitive(text).is_some()
                || lowercase_reserved_bits_schema_primitive(text).is_some()
                || lowercase_schema_primitive(text).is_some()
                || !lowercase_schema_primitive_nested_payloads(text).is_empty()
                || byte_view_schema_primitive(text).is_some()
                || repeat_schema_primitive(text).is_some()
                || binary_schema_anonymous_record_decode_type(text).is_some()
                || closed_dispatch_schema_primitive(text).is_some()
                || extension_dispatch_schema_primitive(text).is_some()
                || reserved_bits_schema_primitive(text).is_some()
        }
        Some(_) => false,
    }
}

fn schema_in_module<'a>(
    module: &'a SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
) -> Option<&'a SchemaDecl> {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return (allow_private_schema || schema.visibility == Visibility::Public).then_some(schema);
    }
    let alias = module.aliases.iter().find(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    })?;
    let alias_name = alias.name.as_ref()?;
    let key = (alias.module_name.clone(), alias_name.clone());
    if visited_aliases.contains(&key) {
        return None;
    }
    visited_aliases.push(key);
    let schema = schema_reference(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
    );
    visited_aliases.pop();
    schema
}

pub(super) fn schema_decode_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .flat_map(|schema| schema_decode_function_signatures_for_schema(module, schema))
        .collect()
}

fn schema_decode_function_signatures_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Vec<FunctionSignature> {
    let Some(schema_name) = schema.name.as_ref() else {
        return Vec::new();
    };
    if schema.format.is_none() {
        return format_neutral_schema_decode_function_signature_for_schema(module, schema)
            .into_iter()
            .collect();
    }
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return Vec::new();
    }
    let Some(fields) = schema_decode_record_fields(module, schema) else {
        return Vec::new();
    };
    let byte_view = Type::named("ByteView", Vec::new());
    let byte_offset = Type::named("ByteOffset", Vec::new());
    let decoded_type = Type::Record(fields.into_iter().map(|(name, ty, _)| (name, ty)).collect());
    let result = Type::named("Result", vec![decoded_type.clone(), Type::string()]);
    let step = Type::named("DecodeStep", vec![decoded_type]);
    vec![
        FunctionSignature {
            name: schema_decode_function_name(schema_name),
            target_name: format!("{SCHEMA_DECODE_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![byte_view.clone()],
            variadic: None,
            return_type: result,
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        },
        FunctionSignature {
            name: schema_decode_step_function_name(schema_name),
            target_name: format!("{SCHEMA_DECODE_STEP_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![byte_view, byte_offset],
            variadic: None,
            return_type: step,
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        },
    ]
}

fn format_neutral_schema_decode_function_signature_for_schema(
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
    let ty = parse_type_annotation(text).ok()?;
    if let Some(ty) = format_neutral_schema_visible_shape_type_for_schema(
        module,
        schema.module_name.as_deref(),
        adts,
        &ty,
        &mut FormatNeutralSchemaTraversalState::default(),
        FormatNeutralSchemaTraversal::Decode,
    ) {
        return Some(ty);
    }
    if let Some(target) = schema_field_target(module, schema, text)
        && target.format.is_none()
    {
        return format_neutral_schema_composition_value_type(
            module,
            target,
            FormatNeutralSchemaTraversal::Decode,
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
    let ty = parse_type_annotation(text).ok()?;
    if let Some(ty) = format_neutral_schema_visible_shape_type_for_schema(
        module,
        schema.module_name.as_deref(),
        adts,
        &ty,
        &mut FormatNeutralSchemaTraversalState::default(),
        FormatNeutralSchemaTraversal::Encode,
    ) {
        return Some(ty);
    }
    if let Some(target) = schema_field_target(module, schema, text)
        && target.format.is_none()
    {
        return format_neutral_schema_composition_value_type(
            module,
            target,
            FormatNeutralSchemaTraversal::Encode,
            &mut Vec::new(),
        );
    }
    None
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

fn format_neutral_schema_encode_record_fields(
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

pub(super) fn format_neutral_schema_first_unsupported_encode_field(
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
                    adt::AdtConstructor {
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

fn format_neutral_schema_descriptor_type(ty: &Type, descriptor: &adt::AdtDescriptor) -> Type {
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
) -> Option<&'a adt::AdtDescriptor> {
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
            let use_decl = imported_use_for_path(&module.uses, &import_path, current_module)?;
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

pub(crate) fn schema_decode_record_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Vec<(String, Type, u8)>> {
    schema_decode_record_fields_inner(module, schema, &mut Vec::new())
}

fn schema_decode_record_fields_inner(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<Vec<(String, Type, u8)>> {
    let schema_name = schema.name.as_ref()?;
    if stack.iter().any(|name| name == schema_name) {
        return None;
    }
    stack.push(schema_name.clone());
    let fields = schema_decode_record_fields_inner_after_push(module, schema, stack);
    stack.pop();
    fields
}

fn schema_decode_record_fields_inner_after_push(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<Vec<(String, Type, u8)>> {
    if schema.format.is_none() {
        return format_neutral_schema_decode_record_fields(module, schema)
            .map(|fields| fields.into_iter().map(|(name, ty)| (name, ty, 0)).collect());
    }
    let mut decoded_fields = BTreeMap::<String, Type>::new();
    let mut fields = Vec::new();
    for (index, field) in schema.fields.iter().enumerate() {
        let decoded = schema_decode_binary_record_field(
            module,
            schema,
            &decoded_fields,
            index,
            field,
            stack,
        )?;
        let SchemaDecodedRecordField::Visible { ty, width } = decoded else {
            continue;
        };
        decoded_fields.insert(field.name.clone(), ty.clone());
        fields.push((field.name.clone(), ty, width));
    }
    Some(fields)
}

enum SchemaDecodedRecordField {
    Omitted,
    Visible { ty: Type, width: u8 },
}

fn schema_decode_binary_record_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &BTreeMap<String, Type>,
    index: usize,
    field: &SchemaField,
    stack: &mut Vec<String>,
) -> Option<SchemaDecodedRecordField> {
    if let Some(reserved) = reserved_bits_schema_primitive(&field.ty) {
        supported_encode_reserved_bits(&schema.fields, index, reserved)?;
        return Some(SchemaDecodedRecordField::Omitted);
    }
    if let Some(width) = exact_width_schema_primitive(&field.ty) {
        return Some(SchemaDecodedRecordField::Visible {
            ty: Type::int(),
            width,
        });
    }
    if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
        return schema_references_are_decoded_ints(decoded_fields, length_expr.references())
            .then_some(SchemaDecodedRecordField::Visible {
                ty: Type::named("ByteView", Vec::new()),
                width: 0,
            });
    }
    if let Some(repeat) = repeat_schema_primitive(&field.ty) {
        return schema_decode_repeat_record_field(module, schema, decoded_fields, &repeat, stack);
    }
    if let Some(nested) = schema_field_target(module, schema, &field.ty)
        && nested.format.as_ref().map(|format| format.name.as_str()) == Some("binary")
    {
        return Some(SchemaDecodedRecordField::Visible {
            ty: schema_decode_value_type_inner(module, nested, stack)?,
            width: 0,
        });
    }
    if let Some(ty) = binary_schema_anonymous_record_decode_type(&field.ty) {
        return Some(SchemaDecodedRecordField::Visible { ty, width: 0 });
    }
    schema_decode_dispatch_record_field(module, schema, decoded_fields, field, stack)
}

fn schema_decode_repeat_record_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &BTreeMap<String, Type>,
    repeat: &SchemaRepeatSpec,
    stack: &mut Vec<String>,
) -> Option<SchemaDecodedRecordField> {
    let count_references = schema_length_expression_references(&repeat.count_field)?;
    if !schema_references_are_decoded_ints(decoded_fields, count_references) {
        return None;
    }
    if let SchemaRepeatPayload::ByteView { length_field } = &repeat.payload {
        let length_references = schema_length_expression_references(length_field)?;
        if !schema_references_are_decoded_ints(decoded_fields, length_references) {
            return None;
        }
    }
    if let SchemaRepeatPayload::ReservedBits { .. } = repeat.payload {
        return Some(SchemaDecodedRecordField::Omitted);
    }
    let element_ty = schema_repeat_payload_type(module, schema, repeat, stack)?;
    Some(SchemaDecodedRecordField::Visible {
        ty: Type::named("List", vec![element_ty]),
        width: 0,
    })
}

fn schema_decode_dispatch_record_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    decoded_fields: &BTreeMap<String, Type>,
    field: &SchemaField,
    stack: &mut Vec<String>,
) -> Option<SchemaDecodedRecordField> {
    let dispatch = closed_dispatch_schema_primitive(&field.ty)
        .or_else(|| extension_dispatch_schema_primitive(&field.ty))?;
    let references =
        std::iter::once(dispatch.tag_field.as_str()).chain(dispatch.length_field.as_deref());
    if !schema_references_are_decoded_ints(decoded_fields, references) {
        return None;
    }
    let payload_types = schema_dispatch_case_types(module, schema, &dispatch, stack)?;
    let payload_ty = schema_dispatch_payload_type(module, schema, &dispatch, &payload_types)?;
    let ty = if dispatch.preserves_unknown {
        Type::named("SchemaDispatchPayload", vec![payload_ty])
    } else {
        payload_ty
    };
    Some(SchemaDecodedRecordField::Visible { ty, width: 0 })
}

fn schema_references_are_decoded_ints<'a>(
    decoded_fields: &BTreeMap<String, Type>,
    references: impl IntoIterator<Item = &'a str>,
) -> bool {
    references.into_iter().all(|reference| {
        schema_field_reference_type(decoded_fields, reference) == Some(&Type::int())
    })
}

fn schema_dispatch_case_types(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    stack: &mut Vec<String>,
) -> Option<Vec<(i64, Type)>> {
    dispatch
        .cases
        .iter()
        .map(|case| {
            let ty = schema_dispatch_case_type(module, schema, case, stack)?;
            Some((case.tag, ty))
        })
        .collect()
}

fn schema_dispatch_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    payload_types: &[(i64, Type)],
) -> Option<Type> {
    let first = payload_types.first()?.1.clone();
    if payload_types.iter().all(|(_, ty)| ty == &first) {
        Some(first)
    } else if dispatch.length_field.is_some()
        && dispatch.cases.iter().any(|case| {
            matches!(
            &case.payload,
            SchemaDispatchCasePayload::Schema { schema_name }
                if recursive_dispatch_decode_only_payload_case_is_eligible(
                    module,
                    schema,
                    dispatch,
                    schema_name,
                )
            )
        })
    {
        schema_recursive_dispatch_helper_payload_type(module, schema, dispatch)
    } else {
        None
    }
}

pub(crate) fn schema_dispatch_case_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    case: &SchemaDispatchCase,
    stack: &mut Vec<String>,
) -> Option<Type> {
    match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
        SchemaDispatchCasePayload::ReservedBits { .. } => Some(Type::unit()),
        SchemaDispatchCasePayload::Schema { schema_name } => {
            if schema.name.as_deref() == Some(schema_name.as_str()) {
                return schema_recursive_dispatch_payload_type(module, schema);
            }
            let nested = schema_dispatch_payload_schema(module, schema, schema_name)?;
            if schema_payload_has_generalized_reserved_byte_prefix(nested) {
                return None;
            }
            schema_decode_value_type_inner(module, nested, stack)
        }
    }
}

fn schema_encode_dispatch_case_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    case: &SchemaDispatchCase,
) -> Option<Type> {
    match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
        SchemaDispatchCasePayload::ReservedBits { .. } => Some(Type::unit()),
        SchemaDispatchCasePayload::Schema { schema_name } => {
            if recursive_dispatch_payload_case_is_eligible(
                module,
                schema,
                field,
                dispatch,
                schema_name,
            ) {
                return schema_recursive_dispatch_payload_type(module, schema);
            }
            let nested = schema_dispatch_payload_schema(module, schema, schema_name)?;
            if schema_payload_has_generalized_reserved_byte_prefix(nested) {
                return None;
            }
            schema_encode_value_type(module, nested)
        }
    }
}

fn schema_repeat_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    repeat: &SchemaRepeatSpec,
    stack: &mut Vec<String>,
) -> Option<Type> {
    match &repeat.payload {
        SchemaRepeatPayload::Primitive { .. } => Some(Type::int()),
        SchemaRepeatPayload::ReservedBits { .. } => Some(Type::unit()),
        SchemaRepeatPayload::ByteView { .. } => Some(Type::named("ByteView", Vec::new())),
        SchemaRepeatPayload::Schema { schema_name } => {
            let nested = schema_dispatch_payload_schema(module, schema, schema_name)?;
            if schema_payload_has_generalized_reserved_byte_prefix(nested) {
                return None;
            }
            schema_decode_value_type_inner(module, nested, stack)
        }
    }
}

fn schema_decode_value_type_inner(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<Type> {
    let fields = schema_decode_record_fields_inner(module, schema, stack)?;
    Some(Type::Record(
        fields.into_iter().map(|(name, ty, _)| (name, ty)).collect(),
    ))
}

pub(crate) fn schema_recursive_dispatch_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    let schema_name = schema.name.as_deref()?;
    schema.fields.iter().find_map(|field| {
        closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))
            .filter(|dispatch| {
                recursive_dispatch_payload_is_eligible(schema, field, dispatch, schema_name)
            })
            .and_then(|dispatch| {
                schema_recursive_dispatch_helper_payload_type(module, schema, &dispatch)
            })
    })
}

pub(crate) fn schema_imported_recursive_dispatch_payload_type(
    _module: &SurfaceModule,
    _schema: &SchemaDecl,
    _dispatch: &SchemaDispatchSpec,
) -> Option<Type> {
    None
}

pub(crate) fn schema_recursive_dispatch_helper_payload_type(
    _module: &SurfaceModule,
    _schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
) -> Option<Type> {
    dispatch
        .cases
        .iter()
        .any(|case| matches!(case.payload, SchemaDispatchCasePayload::Primitive { .. }))
        .then_some(Type::int())
}

pub(crate) fn recursive_dispatch_payload_is_eligible(
    schema: &SchemaDecl,
    _field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    schema.name.as_deref() == Some(schema_name)
        && dispatch_has_recursive_schema_payload_case(dispatch, schema_name)
        && dispatch.length_field.is_some()
        && dispatch_has_non_recursive_primitive_payload_case(dispatch)
}

pub(crate) fn recursive_dispatch_payload_case_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    if recursive_dispatch_payload_is_eligible(schema, field, dispatch, schema_name) {
        return true;
    }
    imported_recursive_dispatch_payload_case_is_eligible(module, schema, dispatch, schema_name)
        && schema_imported_recursive_dispatch_payload_type(module, schema, dispatch).is_some()
}

pub(crate) fn imported_recursive_dispatch_payload_case_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    schema_name.contains("::")
        && dispatch.length_field.is_some()
        && dispatch_has_non_recursive_payload_case(module, schema, dispatch)
        && recursive_dispatch_payload_target_is_eligible(module, schema, schema_name)
}

pub(crate) fn recursive_dispatch_decode_only_payload_case_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    imported_recursive_dispatch_decode_only_payload_case_is_eligible(
        module,
        schema,
        dispatch,
        schema_name,
    ) || (!schema_name.contains("::")
        && dispatch.length_field.is_some()
        && dispatch_has_non_recursive_primitive_payload_case(dispatch)
        && recursive_dispatch_payload_target_is_eligible(module, schema, schema_name))
}

fn imported_recursive_dispatch_decode_only_payload_case_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    schema_name.contains("::")
        && dispatch.length_field.is_some()
        && dispatch_has_non_recursive_primitive_payload_case(dispatch)
        && recursive_dispatch_payload_target_is_eligible(module, schema, schema_name)
}

pub(crate) fn schema_has_eligible_recursive_dispatch_payload(schema: &SchemaDecl) -> bool {
    let Some(schema_name) = schema.name.as_deref() else {
        return false;
    };
    schema.fields.iter().any(|field| {
        closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))
            .is_some_and(|dispatch| {
                recursive_dispatch_payload_is_eligible(schema, field, &dispatch, schema_name)
            })
    })
}

pub(crate) fn schema_has_recursive_dispatch_payload(schema: &SchemaDecl) -> bool {
    let Some(schema_name) = schema.name.as_deref() else {
        return false;
    };
    schema.fields.iter().any(|field| {
        closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))
            .is_some_and(|dispatch| {
                dispatch.cases.iter().any(|case| {
                    matches!(
                        &case.payload,
                        SchemaDispatchCasePayload::Schema { schema_name: payload_name }
                            if payload_name == schema_name
                    )
                })
            })
    })
}

fn recursive_dispatch_payload_target_is_eligible(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    schema_name: &str,
) -> bool {
    schema_dispatch_payload_schema(module, schema, schema_name)
        .is_some_and(schema_has_eligible_recursive_dispatch_payload)
}

fn dispatch_has_non_recursive_payload_case(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    dispatch.cases.iter().any(|case| match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => true,
        SchemaDispatchCasePayload::ReservedBits { .. } => true,
        SchemaDispatchCasePayload::Schema { schema_name } => {
            !recursive_dispatch_payload_target_is_eligible(module, schema, schema_name)
        }
    })
}

fn dispatch_has_non_recursive_primitive_payload_case(dispatch: &SchemaDispatchSpec) -> bool {
    dispatch
        .cases
        .iter()
        .any(|case| matches!(case.payload, SchemaDispatchCasePayload::Primitive { .. }))
}

fn dispatch_has_recursive_schema_payload_case(
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
) -> bool {
    dispatch.cases.iter().any(|case| {
        matches!(
            &case.payload,
            SchemaDispatchCasePayload::Schema { schema_name: payload_name }
                if payload_name == schema_name
        )
    })
}

pub(crate) fn same_module_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    schema_name: &str,
) -> Option<&'a SchemaDecl> {
    if schema_name.contains("::") {
        return None;
    }
    let current_index = module
        .schemas
        .iter()
        .position(|candidate| candidate.node_id == schema.node_id)?;
    module
        .schemas
        .iter()
        .enumerate()
        .find_map(|(index, candidate)| {
            (candidate.name.as_deref() == Some(schema_name)
                && candidate.module_name.as_deref() == schema.module_name.as_deref()
                && candidate.format.as_ref().map(|format| format.name.as_str()) == Some("binary")
                && index < current_index)
                .then_some(candidate)
        })
}

pub(crate) fn schema_dispatch_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    schema_name: &str,
) -> Option<&'a SchemaDecl> {
    let segments = schema_payload_name_path(schema_name)?;
    match segments.as_slice() {
        [name] => same_module_schema(module, schema, name),
        [_, .., name] => {
            let use_decl = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                schema.module_name.as_deref(),
            )?;
            let target_module = Some(use_decl.name.as_str());
            module.schemas.iter().find(|candidate| {
                candidate.name.as_deref() == Some(name)
                    && candidate.module_name.as_deref() == target_module
                    && candidate.visibility == Visibility::Public
                    && candidate.format.as_ref().map(|format| format.name.as_str())
                        == Some("binary")
            })
        }
        _ => None,
    }
}

pub(crate) fn schema_decode_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    schema_decode_value_type_inner(module, schema, &mut Vec::new())
}

pub(super) fn schema_encode_function_signatures(module: &SurfaceModule) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .filter_map(|schema| schema_encode_function_signature_for_schema(module, schema))
        .collect()
}

pub(super) fn schema_validate_function_signatures(
    module: &SurfaceModule,
) -> Vec<FunctionSignature> {
    module
        .schemas
        .iter()
        .filter_map(|schema| schema_validate_function_signature_for_schema(module, schema))
        .collect()
}

fn schema_validate_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return None;
    }
    let fields = schema_decode_record_fields(module, schema)?
        .into_iter()
        .map(|(name, ty, _)| (name, ty))
        .collect::<Vec<_>>();
    let decoded_type = Type::Record(fields);
    Some(FunctionSignature {
        name: schema_validate_function_name(schema_name),
        target_name: format!("{SCHEMA_VALIDATE_TARGET_PREFIX}{schema_name}"),
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

fn schema_encode_function_signature_for_schema(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<FunctionSignature> {
    let schema_name = schema.name.as_ref()?;
    if schema.format.is_none() {
        let value_type = Type::Record(format_neutral_schema_encode_record_fields(module, schema)?);
        return Some(FunctionSignature {
            name: schema_encode_function_name(schema_name),
            target_name: format!("{SCHEMA_NEUTRAL_ENCODE_TARGET_PREFIX}{schema_name}"),
            module_name: schema.module_name.clone(),
            visibility: schema.visibility,
            params: vec![value_type.clone()],
            variadic: None,
            return_type: Type::named("Result", vec![value_type, Type::string()]),
            effects: Vec::new(),
            node_id: schema.node_id,
            span: schema.span.clone(),
        });
    }
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return None;
    }
    let (fields, exact_width_field_names) =
        schema_encode::schema_encode_schema_fields(module, schema)?;
    let value_fields =
        schema_encode_value_fields(module, schema, &fields, &exact_width_field_names)?;
    let byte_chunk = Type::named("ByteChunk", Vec::new());
    let encode_error = Type::named("EncodeError", Vec::new());
    Some(FunctionSignature {
        name: schema_encode_function_name(schema_name),
        target_name: format!("{SCHEMA_ENCODE_TARGET_PREFIX}{schema_name}"),
        module_name: schema.module_name.clone(),
        visibility: schema.visibility,
        params: vec![Type::Record(value_fields)],
        variadic: None,
        return_type: Type::named("Result", vec![byte_chunk, encode_error]),
        effects: Vec::new(),
        node_id: schema.node_id,
        span: schema.span.clone(),
    })
}

pub(crate) fn schema_encode_value_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<Type> {
    schema_encode_function_signature_for_schema(module, schema)
        .and_then(|signature| signature.params.into_iter().next())
}

fn schema_encode_value_fields(
    _module: &SurfaceModule,
    _schema: &SchemaDecl,
    schema_fields: &[(String, Type)],
    _exact_width_field_names: &[String],
) -> Option<Vec<(String, Type)>> {
    Some(schema_fields.to_vec())
}

pub(crate) fn schema_decode_function_name(schema_name: &str) -> String {
    format!("byte_decode_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_decode_step_function_name(schema_name: &str) -> String {
    format!("byte_decode_step_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_encode_function_name(schema_name: &str) -> String {
    format!("byte_encode_{}", snake_case_identifier(schema_name))
}

pub(crate) fn schema_validate_function_name(schema_name: &str) -> String {
    format!("validate_{}", snake_case_identifier(schema_name))
}

fn snake_case_identifier(name: &str) -> String {
    let mut out = String::new();
    let mut previous_was_lower_or_digit = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            if ch.is_ascii_uppercase() {
                if previous_was_lower_or_digit && !out.ends_with('_') {
                    out.push('_');
                }
                out.push(ch.to_ascii_lowercase());
                previous_was_lower_or_digit = false;
            } else {
                out.push(ch);
                previous_was_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
            }
        } else if !out.is_empty() && !out.ends_with('_') {
            out.push('_');
            previous_was_lower_or_digit = false;
        }
    }
    out.trim_matches('_').to_string()
}
