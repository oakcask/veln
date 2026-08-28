use super::*;

pub(crate) fn check_schema_type_references(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for function in &module.functions {
        let current_module = function.module_name.as_deref();
        for param in &function.params {
            if let Some(annotation) = &param.ty {
                push_schema_type_reference_diagnostics(
                    module,
                    current_module,
                    annotation,
                    param.node_id.display("param"),
                    param.span.clone(),
                    "parameter_type",
                    &mut diagnostics,
                );
            }
        }
        if let Some(return_type) = &function.return_type {
            push_schema_type_reference_diagnostics(
                module,
                current_module,
                return_type,
                function.node_id.display(function.kind.node_prefix()),
                function.span.clone(),
                "return_type",
                &mut diagnostics,
            );
        }
        for line in &function.body {
            let BodyLineKind::Let {
                annotation: Some(annotation),
                ..
            } = &line.kind
            else {
                continue;
            };
            push_schema_type_reference_diagnostics(
                module,
                current_module,
                annotation,
                line.node_id.display("let"),
                line.span.clone(),
                "local_annotation",
                &mut diagnostics,
            );
        }
    }

    for type_decl in &module.types {
        let current_module = type_decl.module_name.as_deref();
        for variant in &type_decl.variants {
            for field in &variant.fields {
                push_schema_type_reference_diagnostics(
                    module,
                    current_module,
                    &field.ty,
                    field.node_id.display("field"),
                    field.span.clone(),
                    "type_variant_field",
                    &mut diagnostics,
                );
            }
        }
    }

    diagnostics
}

pub(crate) fn check_schema_field_primitives(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for schema in &module.schemas {
        let format_name = schema.format.as_ref().map(|format| format.name.as_str());
        let mut decoded_fields = BTreeMap::<String, Type>::new();
        let mut field_bindings = BTreeSet::new();
        let adts = AdtRegistry::from_module(module);
        let context = SchemaFieldPrimitiveContext {
            module,
            schema,
            format_name,
            adts: &adts,
        };
        let mut state = SchemaFieldPrimitiveState {
            decoded_fields: &mut decoded_fields,
            field_bindings: &mut field_bindings,
            diagnostics: &mut diagnostics,
        };
        for field in &schema.fields {
            check_schema_field_primitive(context, field, &mut state);
        }
        check_schema_validation_clauses(schema, &decoded_fields, &mut diagnostics);
    }

    diagnostics
}

#[derive(Clone, Copy)]
struct SchemaFieldPrimitiveContext<'a> {
    module: &'a SurfaceModule,
    schema: &'a SchemaDecl,
    format_name: Option<&'a str>,
    adts: &'a AdtRegistry,
}

struct SchemaFieldPrimitiveState<'a> {
    decoded_fields: &'a mut BTreeMap<String, Type>,
    field_bindings: &'a mut BTreeSet<String>,
    diagnostics: &'a mut Vec<Diagnostic>,
}

fn check_schema_field_primitive(
    context: SchemaFieldPrimitiveContext<'_>,
    field: &SchemaField,
    state: &mut SchemaFieldPrimitiveState<'_>,
) {
    if !state.field_bindings.insert(field.name.clone()) {
        state
            .diagnostics
            .push(schema_composition_duplicate_binding_diagnostic(
                context.schema,
                field,
            ));
        return;
    }
    if schema_field_primitive_is_handled(
        context.module,
        context.schema,
        field,
        context.format_name,
        state.decoded_fields,
        state.diagnostics,
    ) {
        return;
    }
    if context.format_name.is_none() {
        check_format_neutral_schema_field(
            context.module,
            context.schema,
            field,
            context.adts,
            state.decoded_fields,
            state.diagnostics,
        );
        return;
    }
    check_schema_non_byte_view_multiple(context.schema, field, state.diagnostics);
    check_unresolved_schema_payload_name(context.module, context.schema, field, state.diagnostics);
}

fn schema_field_primitive_is_handled(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    check_schema_field_composition(module, schema, field, diagnostics)
        || check_direct_schema_primitive(schema, field, format_name, decoded_fields, diagnostics)
        || check_nested_lowercase_schema_primitives(schema, field, format_name, diagnostics)
        || (format_name == Some("binary")
            && check_binary_schema_field(module, schema, field, decoded_fields, diagnostics))
}

fn check_unresolved_schema_payload_name(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if schema_payload_name_path(&field.ty).is_none()
        || schema_composition_quarantine_is_sole_failure(module, schema, &field.ty)
    {
        return;
    }
    diagnostics.push(schema_composition_reference_diagnostic(
        module,
        schema,
        field,
        unresolved_schema_composition_reason(module, schema, &field.ty),
    ));
}

fn check_schema_validation_clauses(
    schema: &SchemaDecl,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (index, validation) in schema.validations.iter().enumerate() {
        if index > 0 {
            diagnostics.push(schema_validation_duplicate_diagnostic(schema, validation));
        } else {
            check_schema_validation_clause(schema, validation, decoded_fields, diagnostics);
        }
    }
}

pub(super) fn check_schema_field_composition(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if let Some(reason) = schema_composition_reference_blocker(module, schema, field) {
        diagnostics.push(schema_composition_reference_diagnostic(
            module, schema, field, reason,
        ));
        return true;
    }
    if schema_field_uses_existing_grammar_at_boundary(schema, &field.ty) {
        return false;
    }
    let Some(target) = schema_field_target(module, schema, &field.ty) else {
        return false;
    };

    let decode_eligible = schema_decode_value_type(module, target).is_some();
    let encode_eligible = schema_encode_value_type(module, target).is_some();
    if !decode_eligible {
        diagnostics.push(schema_composition_reference_diagnostic(
            module,
            schema,
            field,
            "decode_ineligible_target",
        ));
    }
    if !encode_eligible {
        diagnostics.push(schema_composition_reference_diagnostic(
            module,
            schema,
            field,
            "encode_ineligible_target",
        ));
    }
    !decode_eligible
}

pub(super) fn check_direct_schema_primitive(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if let Some(reserved) = lowercase_reserved_bits_schema_primitive(&field.ty) {
        check_lowercase_reserved_bits(schema, field, format_name, reserved, diagnostics);
        return true;
    }
    if let Some(primitive) = lowercase_schema_primitive(&field.ty) {
        check_lowercase_integer_primitive(
            schema,
            field,
            format_name,
            primitive,
            decoded_fields,
            diagnostics,
        );
        return true;
    }
    if let Some(primitive) = exact_width_binary_primitive_name(&field.ty) {
        check_exact_width_primitive(
            schema,
            field,
            format_name,
            primitive,
            decoded_fields,
            diagnostics,
        );
        return true;
    }
    if let Some(primitive) = reserved_bits_primitive(&field.ty) {
        check_reserved_bits(schema, field, format_name, primitive, diagnostics);
        return true;
    }
    false
}

pub(super) fn check_lowercase_reserved_bits(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    reserved: Result<(i64, i64), LowercaseSchemaPrimitiveError>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match (format_name, reserved) {
        (Some("binary"), Ok(reserved)) => {
            check_schema_non_byte_view_multiple(schema, field, diagnostics);
            check_reserved_bits_encode_shape(schema, field, reserved, diagnostics);
        }
        (Some("binary"), Err(reason)) | (_, Err(reason)) => {
            diagnostics.push(lowercase_schema_primitive_diagnostic(
                &field.ty,
                Some(schema),
                Some(field),
                field.node_id.display("schema-field"),
                field.span.clone(),
                reason,
            ));
        }
        (_, Ok(_)) => diagnostics.push(lowercase_schema_primitive_position_diagnostic(
            &field.ty,
            Some(schema),
            Some(field),
            field.node_id.display("schema-field"),
            field.span.clone(),
            "non_binary_format",
        )),
    }
}

pub(super) fn check_lowercase_integer_primitive(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    primitive: Result<LowercaseSchemaPrimitive, LowercaseSchemaPrimitiveError>,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match (format_name, primitive) {
        (Some("binary"), Ok(_)) => {
            check_schema_non_byte_view_multiple(schema, field, diagnostics);
            record_decoded_schema_field(schema, field, Type::int(), decoded_fields, diagnostics);
        }
        (Some("binary"), Err(reason)) | (_, Err(reason)) => {
            diagnostics.push(lowercase_schema_primitive_diagnostic(
                &field.ty,
                Some(schema),
                Some(field),
                field.node_id.display("schema-field"),
                field.span.clone(),
                reason,
            ));
        }
        (_, Ok(_)) => diagnostics.push(lowercase_schema_primitive_position_diagnostic(
            &field.ty,
            Some(schema),
            Some(field),
            field.node_id.display("schema-field"),
            field.span.clone(),
            "non_binary_format",
        )),
    }
}

pub(super) fn check_exact_width_primitive(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    primitive: &str,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if format_name != Some("binary") {
        diagnostics.push(exact_width_schema_primitive_diagnostic(
            primitive,
            Some(schema),
            Some(field),
            field.node_id.display("schema-field"),
            field.span.clone(),
            "non_binary_format",
        ));
        return;
    }
    check_schema_non_byte_view_multiple(schema, field, diagnostics);
    record_decoded_schema_field(schema, field, Type::int(), decoded_fields, diagnostics);
}

pub(super) fn check_reserved_bits(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    primitive: Result<(i64, i64), ReservedBitsArgumentReason>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if format_name != Some("binary") {
        diagnostics.push(reserved_bits_format_diagnostic(schema, field));
        return;
    }
    match primitive {
        Err(reason) => diagnostics.push(reserved_bits_argument_diagnostic(schema, field, reason)),
        Ok(reserved) => {
            check_schema_non_byte_view_multiple(schema, field, diagnostics);
            check_reserved_bits_encode_shape(schema, field, reserved, diagnostics);
        }
    }
}

pub(super) fn check_reserved_bits_encode_shape(
    schema: &SchemaDecl,
    field: &SchemaField,
    reserved: (i64, i64),
    diagnostics: &mut Vec<Diagnostic>,
) {
    let field_index = schema
        .fields
        .iter()
        .position(|schema_field| schema_field.node_id == field.node_id);
    if field_index
        .and_then(|index| supported_encode_reserved_bits(&schema.fields, index, reserved))
        .is_none()
    {
        diagnostics.push(reserved_bits_encode_shape_diagnostic(
            schema,
            field,
            field_index,
            reserved,
        ));
    }
}

pub(super) fn check_nested_lowercase_schema_primitives(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut pushed_diagnostic = false;
    for (primitive, reason) in lowercase_schema_primitive_nested_payloads(&field.ty) {
        let supported_dispatch = reason == "dispatch_payload"
            && schema_dispatch_payload_accepts_lowercase_primitive(primitive);
        let supported_repeat = reason == "repeat_payload"
            && schema_repeat_payload_accepts_lowercase_primitive(primitive);
        if format_name == Some("binary") && (supported_dispatch || supported_repeat) {
            continue;
        }
        let reason = if format_name == Some("binary") {
            reason
        } else {
            "non_binary_format"
        };
        diagnostics.push(lowercase_schema_primitive_position_diagnostic(
            primitive,
            Some(schema),
            Some(field),
            field.node_id.display("schema-field"),
            field.span.clone(),
            reason,
        ));
        pushed_diagnostic = true;
    }
    pushed_diagnostic
}

pub(super) fn check_binary_schema_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
        if check_schema_byte_view_reference(
            schema,
            field,
            &length_expr,
            decoded_fields,
            diagnostics,
        ) && check_schema_byte_view_multiple(schema, field, decoded_fields, diagnostics)
        {
            decoded_fields.insert(field.name.clone(), Type::named("ByteView", Vec::new()));
        }
        return true;
    }
    if let Some(repeat) = repeat_schema_primitive(&field.ty) {
        check_schema_non_byte_view_multiple(schema, field, diagnostics);
        if let Some(field_ty) =
            check_schema_repeat_field(module, schema, field, &repeat, decoded_fields, diagnostics)
        {
            record_decoded_schema_field(schema, field, field_ty, decoded_fields, diagnostics);
        }
        return true;
    }
    if let Some(field_ty) = binary_composed_schema_field_type(module, schema, field) {
        check_schema_non_byte_view_multiple(schema, field, diagnostics);
        record_decoded_schema_field(schema, field, field_ty, decoded_fields, diagnostics);
        return true;
    }
    if let Some(field_ty) = binary_schema_anonymous_record_decode_type(&field.ty) {
        check_schema_non_byte_view_multiple(schema, field, diagnostics);
        record_decoded_schema_field(schema, field, field_ty, decoded_fields, diagnostics);
        return true;
    }
    let Some(dispatch) = closed_dispatch_schema_primitive(&field.ty)
        .or_else(|| extension_dispatch_schema_primitive(&field.ty))
    else {
        return false;
    };
    check_schema_non_byte_view_multiple(schema, field, diagnostics);
    if let Some(field_ty) = check_schema_dispatch_field(
        module,
        schema,
        field,
        &dispatch,
        decoded_fields,
        diagnostics,
    ) {
        record_decoded_schema_field(schema, field, field_ty, decoded_fields, diagnostics);
    }
    true
}

pub(super) fn binary_composed_schema_field_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
) -> Option<Type> {
    let payload_schema = schema_field_target(module, schema, &field.ty)?;
    (payload_schema
        .format
        .as_ref()
        .map(|format| format.name.as_str())
        == Some("binary"))
    .then(|| schema_decode_value_type(module, payload_schema))
    .flatten()
}
