use super::*;

type SchemaEncodeFields = (Vec<(String, Type)>, Vec<String>);

enum SchemaEncodeFieldKind<'a> {
    ReservedBits((i64, i64)),
    ExactWidth,
    Repeat(SchemaRepeatSpec),
    ByteView(ByteViewLengthExpr),
    NestedSchema(&'a SchemaDecl),
    AnonymousRecord(Type),
    Dispatch(SchemaDispatchSpec),
}

struct ResolvedSchemaEncodeField {
    ty: Option<Type>,
    exact_width: bool,
}

impl ResolvedSchemaEncodeField {
    fn omitted() -> Self {
        Self {
            ty: None,
            exact_width: false,
        }
    }

    fn visible(ty: Type) -> Self {
        Self {
            ty: Some(ty),
            exact_width: false,
        }
    }

    fn exact_width() -> Self {
        Self {
            ty: Some(Type::int()),
            exact_width: true,
        }
    }
}

pub(super) fn schema_encode_schema_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Option<SchemaEncodeFields> {
    let mut fields = Vec::new();
    let mut exact_width_field_names = Vec::new();
    let mut visible_field_types = BTreeMap::new();
    for (index, field) in schema.fields.iter().enumerate() {
        let resolved =
            resolve_schema_encode_field(module, schema, field, index, &visible_field_types)?;
        if resolved.exact_width {
            exact_width_field_names.push(field.name.clone());
        }
        if let Some(field_ty) = resolved.ty {
            fields.push((field.name.clone(), field_ty.clone()));
            visible_field_types.insert(field.name.clone(), field_ty);
        }
    }
    Some((fields, exact_width_field_names))
}

fn resolve_schema_encode_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    index: usize,
    visible_field_types: &BTreeMap<String, Type>,
) -> Option<ResolvedSchemaEncodeField> {
    match schema_encode_field_kind(module, schema, field)? {
        SchemaEncodeFieldKind::ReservedBits(reserved) => {
            supported_encode_reserved_bits(&schema.fields, index, reserved)?;
            Some(ResolvedSchemaEncodeField::omitted())
        }
        SchemaEncodeFieldKind::ExactWidth => Some(ResolvedSchemaEncodeField::exact_width()),
        SchemaEncodeFieldKind::Repeat(repeat) => {
            schema_encode_repeat_field_type(module, schema, visible_field_types, &repeat).map(
                |ty| ResolvedSchemaEncodeField {
                    ty,
                    exact_width: false,
                },
            )
        }
        SchemaEncodeFieldKind::ByteView(length_expr) => {
            schema_references_are_visible_int(visible_field_types, &length_expr.references())
                .then(|| ResolvedSchemaEncodeField::visible(Type::named("ByteView", Vec::new())))
        }
        SchemaEncodeFieldKind::NestedSchema(nested) => Some(ResolvedSchemaEncodeField::visible(
            schema_encode_value_type(module, nested)?,
        )),
        SchemaEncodeFieldKind::AnonymousRecord(record_ty) => {
            Some(ResolvedSchemaEncodeField::visible(record_ty))
        }
        SchemaEncodeFieldKind::Dispatch(dispatch) => Some(ResolvedSchemaEncodeField::visible(
            schema_encode_dispatch_field_type(
                module,
                schema,
                field,
                visible_field_types,
                &dispatch,
            )?,
        )),
    }
}

fn schema_encode_field_kind<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
) -> Option<SchemaEncodeFieldKind<'a>> {
    if let Some(reserved) = reserved_bits_schema_primitive(&field.ty) {
        return Some(SchemaEncodeFieldKind::ReservedBits(reserved));
    }
    if exact_width_schema_primitive(&field.ty).is_some() {
        return Some(SchemaEncodeFieldKind::ExactWidth);
    }
    if let Some(repeat) = repeat_schema_primitive(&field.ty) {
        return Some(SchemaEncodeFieldKind::Repeat(repeat));
    }
    if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
        return Some(SchemaEncodeFieldKind::ByteView(length_expr));
    }
    if let Some(nested) = schema_field_target(module, schema, &field.ty)
        && nested.format.as_ref().map(|format| format.name.as_str()) == Some("binary")
    {
        return Some(SchemaEncodeFieldKind::NestedSchema(nested));
    }
    if let Some(record_ty) = binary_schema_anonymous_record_decode_type(&field.ty) {
        return Some(SchemaEncodeFieldKind::AnonymousRecord(record_ty));
    }
    closed_dispatch_schema_primitive(&field.ty)
        .or_else(|| extension_dispatch_schema_primitive(&field.ty))
        .map(SchemaEncodeFieldKind::Dispatch)
}

fn schema_encode_repeat_field_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    visible_field_types: &BTreeMap<String, Type>,
    repeat: &SchemaRepeatSpec,
) -> Option<Option<Type>> {
    let count_references = schema_length_expression_references(&repeat.count_field)?;
    if !schema_references_are_visible_int(visible_field_types, &count_references) {
        return None;
    }
    if let SchemaRepeatPayload::ByteView { length_field } = &repeat.payload {
        let length_references = schema_length_expression_references(length_field)?;
        if !schema_references_are_visible_int(visible_field_types, &length_references) {
            return None;
        }
    }
    if let SchemaRepeatPayload::ReservedBits { .. } = &repeat.payload {
        return Some(None);
    }
    let element_ty = schema_repeat_payload_type(module, schema, repeat, &mut Vec::new())?;
    Some(Some(Type::named("List", vec![element_ty])))
}

fn schema_references_are_visible_int(
    visible_field_types: &BTreeMap<String, Type>,
    references: &[&str],
) -> bool {
    references.iter().all(|reference| {
        schema_field_reference_type(visible_field_types, reference) == Some(&Type::int())
    })
}

fn schema_encode_dispatch_field_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    visible_field_types: &BTreeMap<String, Type>,
    dispatch: &SchemaDispatchSpec,
) -> Option<Type> {
    let recursive_payload =
        recursive_dispatch_encode_payload_field(module, schema, field, dispatch);
    if schema_field_reference_type(visible_field_types, &dispatch.tag_field) != Some(&Type::int())
        || dispatch.length_field.as_ref().is_some_and(|length_field| {
            schema_field_reference_type(visible_field_types, length_field) != Some(&Type::int())
        })
        || (dispatch.length_field.is_some() && !dispatch.preserves_unknown && !recursive_payload)
    {
        return None;
    }
    let mut payload_types = dispatch
        .cases
        .iter()
        .map(|case| schema_encode_dispatch_case_type(module, schema, field, dispatch, case))
        .collect::<Option<Vec<_>>>()?;
    let payload_ty = payload_types.pop()?;
    if !recursive_payload && payload_types.iter().any(|ty| ty != &payload_ty) {
        return None;
    }
    Some(if dispatch.preserves_unknown {
        Type::named("SchemaDispatchPayload", vec![payload_ty])
    } else {
        payload_ty
    })
}

fn recursive_dispatch_encode_payload_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    schema_dispatch_has_schema_payload_where(dispatch, |schema_name| {
        recursive_dispatch_payload_case_is_eligible(module, schema, field, dispatch, schema_name)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use veln_ast::lower_surface_ast;
    use veln_source::SourceFile;
    use veln_syntax::parse;

    #[test]
    fn encode_value_type_preserves_mixed_field_kinds() {
        let source = SourceFile::new(
            "main.veln",
            concat!(
                "schema Child\n",
                "  format binary\n",
                "  code: UInt16be\n",
                "end\n",
                "schema Packet\n",
                "  format binary\n",
                "  length: UInt8\n",
                "  reserved: ReservedBits(16, 43981)\n",
                "  payload: ByteView(length)\n",
                "  count: UInt8\n",
                "  items: Repeat(count, UInt16be)\n",
                "  header: {kind: UInt8, flags: UInt16le}\n",
                "  child: Child\n",
                "  kind: UInt8\n",
                "  variant: ExtensionDispatch(kind, length, 1 => UInt8, 2 => UInt16be)\n",
                "end\n",
            ),
        );
        let module = lower_surface_ast(&parse(&source).tree);
        let packet = &module.schemas[1];

        assert_eq!(
            schema_encode_value_type(&module, packet),
            Some(Type::Record(vec![
                ("length".to_string(), Type::int()),
                ("payload".to_string(), Type::named("ByteView", Vec::new())),
                ("count".to_string(), Type::int()),
                ("items".to_string(), Type::named("List", vec![Type::int()])),
                (
                    "header".to_string(),
                    Type::Record(vec![
                        ("kind".to_string(), Type::int()),
                        ("flags".to_string(), Type::int()),
                    ]),
                ),
                (
                    "child".to_string(),
                    Type::Record(vec![("code".to_string(), Type::int())]),
                ),
                ("kind".to_string(), Type::int()),
                (
                    "variant".to_string(),
                    Type::named("SchemaDispatchPayload", vec![Type::int()]),
                ),
            ])),
        );
    }
}
