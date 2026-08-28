use super::*;

pub(super) fn check_schema_dispatch_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    let mut valid =
        check_schema_dispatch_references(schema, field, dispatch, decoded_fields, diagnostics);
    let payload_types =
        collect_schema_dispatch_payload_types(module, schema, field, dispatch, diagnostics);
    valid &= !payload_types.mixed && !payload_types.resolution_failed;

    let recursive_dispatch_payload =
        schema_dispatch_has_recursive_payload(module, schema, field, dispatch);
    reconcile_schema_dispatch_payload_types(
        SchemaDispatchFieldContext {
            module,
            schema,
            field,
            dispatch,
        },
        &payload_types,
        recursive_dispatch_payload,
        &mut valid,
        diagnostics,
    )?;

    if !valid {
        return None;
    }
    let payload_ty = if recursive_dispatch_payload {
        schema_recursive_dispatch_helper_payload_type(module, schema, dispatch)?
    } else {
        payload_types.expected?
    };
    if dispatch.preserves_unknown {
        Some(Type::named("SchemaDispatchPayload", vec![payload_ty]))
    } else {
        Some(payload_ty)
    }
}

pub(super) fn check_schema_dispatch_references(
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let tag_valid = check_schema_dispatch_reference(
        schema,
        field,
        decoded_fields,
        &dispatch.tag_field,
        "tag",
        diagnostics,
    );
    let length_valid = dispatch.length_field.as_ref().is_none_or(|length_field| {
        check_schema_dispatch_reference(
            schema,
            field,
            decoded_fields,
            length_field,
            "length",
            diagnostics,
        )
    });
    tag_valid && length_valid
}

pub(super) struct SchemaDispatchPayloadTypes {
    expected: Option<Type>,
    mixed: bool,
    resolution_failed: bool,
}

#[derive(Clone, Copy)]
pub(super) struct SchemaDispatchFieldContext<'a> {
    module: &'a SurfaceModule,
    schema: &'a SchemaDecl,
    field: &'a SchemaField,
    dispatch: &'a SchemaDispatchSpec,
}

pub(super) fn collect_schema_dispatch_payload_types(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    diagnostics: &mut Vec<Diagnostic>,
) -> SchemaDispatchPayloadTypes {
    let mut result = SchemaDispatchPayloadTypes {
        expected: None,
        mixed: false,
        resolution_failed: false,
    };
    for case in &dispatch.cases {
        let Some(payload_ty) = resolve_schema_dispatch_case_payload_type(
            module,
            schema,
            field,
            dispatch,
            case,
            diagnostics,
        ) else {
            result.resolution_failed = true;
            continue;
        };
        if let Some(expected) = &result.expected {
            result.mixed |= expected != &payload_ty;
        } else {
            result.expected = Some(payload_ty);
        }
    }
    result
}

pub(super) fn resolve_schema_dispatch_case_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    case: &SchemaDispatchCase,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
        SchemaDispatchCasePayload::ReservedBits { .. } => Some(Type::unit()),
        SchemaDispatchCasePayload::Schema { schema_name } => resolve_schema_dispatch_named_payload(
            module,
            schema,
            field,
            dispatch,
            case.tag,
            schema_name,
            diagnostics,
        ),
    }
}

pub(super) fn resolve_schema_dispatch_named_payload(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    tag: i64,
    schema_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if schema.name.as_deref() == Some(schema_name) {
        return resolve_self_schema_dispatch_payload(
            module,
            schema,
            field,
            dispatch,
            tag,
            schema_name,
            diagnostics,
        );
    }
    let payload_schema = resolve_schema_dispatch_payload_schema(
        module,
        schema,
        field,
        tag,
        schema_name,
        diagnostics,
    )?;
    resolve_external_schema_dispatch_payload(
        SchemaDispatchFieldContext {
            module,
            schema,
            field,
            dispatch,
        },
        tag,
        schema_name,
        payload_schema,
        diagnostics,
    )
}

pub(super) fn resolve_self_schema_dispatch_payload(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    tag: i64,
    schema_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if !recursive_dispatch_payload_is_eligible(schema, field, dispatch, schema_name) {
        push_recursive_dispatch_payload_blocker(
            schema,
            field,
            dispatch,
            tag,
            schema_name,
            schema,
            diagnostics,
        );
        return None;
    }
    schema_recursive_dispatch_payload_type(module, schema).or_else(|| {
        diagnostics.push(incompatible_schema_dispatch_payload_diagnostic(
            module,
            schema,
            field,
            tag,
            schema_name,
            schema,
            SchemaHelperAvailability {
                decode: false,
                encode: false,
            },
        ));
        None
    })
}

pub(super) fn resolve_external_schema_dispatch_payload(
    context: SchemaDispatchFieldContext<'_>,
    tag: i64,
    schema_name: &str,
    payload_schema: &SchemaDecl,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if schema_has_recursive_dispatch_payload(payload_schema)
        && !(recursive_dispatch_payload_case_is_eligible(
            context.module,
            context.schema,
            context.field,
            context.dispatch,
            schema_name,
        ) || recursive_dispatch_decode_only_payload_case_is_eligible(
            context.module,
            context.schema,
            context.dispatch,
            schema_name,
        ))
    {
        push_recursive_dispatch_payload_blocker(
            context.schema,
            context.field,
            context.dispatch,
            tag,
            schema_name,
            payload_schema,
            diagnostics,
        );
        return None;
    }
    schema_dispatch_payload_helper_type(
        context.module,
        context.schema,
        context.field,
        tag,
        schema_name,
        payload_schema,
        diagnostics,
    )
}

pub(super) fn push_recursive_dispatch_payload_blocker(
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    tag: i64,
    schema_name: &str,
    payload_schema: &SchemaDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let blocker =
        recursive_dispatch_payload_blocker(schema, field, dispatch, schema_name, payload_schema);
    diagnostics.push(schema_dispatch_payload_diagnostic(
        schema,
        field,
        tag,
        schema_name,
        blocker.reason,
        blocker.message,
        [(
            "recursive_helper_fact",
            JsonValue::string(blocker.fact.to_string()),
        )],
    ));
}

pub(super) fn schema_dispatch_has_recursive_payload(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    dispatch.cases.iter().any(|case| {
        matches!(
            &case.payload,
            SchemaDispatchCasePayload::Schema { schema_name }
                    if recursive_dispatch_payload_case_is_eligible(
                        module,
                        schema,
                        field,
                        dispatch,
                        schema_name,
                    ) || recursive_dispatch_decode_only_payload_case_is_eligible(
                        module,
                        schema,
                        dispatch,
                        schema_name,
                    )
        )
    })
}

pub(super) fn reconcile_schema_dispatch_payload_types(
    context: SchemaDispatchFieldContext<'_>,
    payload_types: &SchemaDispatchPayloadTypes,
    recursive_dispatch_payload: bool,
    valid: &mut bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<()> {
    if !payload_types.mixed {
        return Some(());
    }
    if recursive_dispatch_payload {
        *valid = !payload_types.resolution_failed;
        return Some(());
    }
    if payload_types.resolution_failed {
        *valid = false;
        return Some(());
    }
    let expected = payload_types.expected.as_ref()?;
    let Some((case, payload_ty)) = context.dispatch.cases.iter().find_map(|case| {
        let payload_ty =
            schema_dispatch_case_known_payload_type(context.module, context.schema, case)?;
        (&payload_ty != expected).then_some((case, payload_ty))
    }) else {
        return Some(());
    };
    diagnostics.push(schema_dispatch_payload_diagnostic(
        context.schema,
        context.field,
        case.tag,
        schema_dispatch_case_payload_name(&case.payload),
        "incompatible_payload_type",
        format!(
            "dispatch payload case `{}` decodes as `{}`, but earlier cases decode as `{}`",
            case.tag,
            payload_ty.render(),
            expected.render()
        ),
        [
            ("expected", JsonValue::string(expected.render())),
            ("actual", JsonValue::string(payload_ty.render())),
        ],
    ));
    Some(())
}

pub(super) fn schema_dispatch_case_known_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    case: &SchemaDispatchCase,
) -> Option<Type> {
    match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
        SchemaDispatchCasePayload::ReservedBits { .. } => Some(Type::unit()),
        SchemaDispatchCasePayload::Schema { schema_name }
            if schema.name.as_deref() == Some(schema_name.as_str()) =>
        {
            schema_recursive_dispatch_payload_type(module, schema)
        }
        SchemaDispatchCasePayload::Schema { schema_name } => {
            schema_dispatch_payload_schema(module, schema, schema_name)
                .and_then(|payload_schema| schema_decode_value_type(module, payload_schema))
        }
    }
}

pub(super) struct RecursiveDispatchPayloadBlocker {
    reason: &'static str,
    fact: &'static str,
    message: String,
}

pub(super) fn recursive_dispatch_payload_blocker(
    _parent_schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
    payload_schema: &SchemaDecl,
) -> RecursiveDispatchPayloadBlocker {
    if dispatch.length_field.is_none() {
        return RecursiveDispatchPayloadBlocker {
            reason: "recursive_payload_missing_length_bound",
            fact: "recursive dispatch payloads require a length-bounded parent dispatch field",
            message: format!(
                "dispatch payload schema `{schema_name}` requires parent dispatch field `{}` to include a length field",
                field.name
            ),
        };
    }
    if !dispatch
        .cases
        .iter()
        .any(|case| matches!(case.payload, SchemaDispatchCasePayload::Primitive { .. }))
    {
        return RecursiveDispatchPayloadBlocker {
            reason: "recursive_payload_missing_primitive_base_case",
            fact: "recursive dispatch parents require a non-recursive primitive base case",
            message: format!(
                "dispatch payload schema `{schema_name}` requires parent dispatch field `{}` to include a non-recursive primitive case",
                field.name
            ),
        };
    }
    if !schema_has_eligible_recursive_dispatch_payload(payload_schema) {
        return RecursiveDispatchPayloadBlocker {
            reason: "recursive_payload_missing_bounded_helper",
            fact: "recursive dispatch payload schemas must expose a bounded recursive helper",
            message: format!(
                "dispatch payload schema `{schema_name}` does not expose a bounded recursive helper"
            ),
        };
    }
    RecursiveDispatchPayloadBlocker {
        reason: "recursive_payload_ineligible_parent",
        fact: "recursive dispatch payloads require a length-bounded parent with recursive helper support and a non-recursive base case",
        message: format!(
            "dispatch payload schema `{schema_name}` does not satisfy recursive dispatch helper requirements"
        ),
    }
}

pub(super) fn check_schema_dispatch_reference(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    reference: &str,
    role: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(ty) = schema_field_reference_type(decoded_fields, reference) else {
        let reason = if schema_field_declared_after(schema, field, reference) {
            "forward_field_reference"
        } else {
            "unknown_field_reference"
        };
        let mut diagnostic = schema_dispatch_reference_diagnostic(
            schema,
            field,
            reference,
            role,
            reason,
            format!("dispatch {role} field `{reference}` must be an earlier decoded `Int` field"),
            [],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, role);
        diagnostics.push(diagnostic);
        return false;
    };
    if ty != &Type::int() {
        let mut diagnostic = schema_dispatch_reference_diagnostic(
            schema,
            field,
            reference,
            role,
            "incompatible_field_reference",
            format!(
                "dispatch {role} field `{reference}` decodes as `{}`, not `Int`",
                ty.render()
            ),
            [("actual", JsonValue::string(ty.render()))],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, role);
        diagnostics.push(diagnostic);
        return false;
    }
    true
}
