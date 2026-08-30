use super::*;

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

pub(super) fn schema_encode_dispatch_case_type(
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

pub(super) fn schema_repeat_payload_type(
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

pub(super) fn schema_decode_value_type_inner(
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

pub(crate) fn schema_dispatch_has_recursive_payload(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    schema_dispatch_has_schema_payload_where(dispatch, |schema_name| {
        recursive_dispatch_payload_case_is_eligible(module, schema, field, dispatch, schema_name)
            || recursive_dispatch_decode_only_payload_case_is_eligible(
                module,
                schema,
                dispatch,
                schema_name,
            )
    })
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
            let use_decl = normal_imported_use_for_path(
                module,
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
