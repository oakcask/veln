use std::collections::BTreeMap;

use veln_ast::{SchemaDecl, SchemaField, SurfaceModule};
use veln_ir::{
    IrSchemaDecodeDispatch, IrSchemaDecodeDispatchCase, IrSchemaDecodeField, IrSchemaDecodeMapping,
    IrSchemaDecodeMappingExpr, IrSchemaDecodeMappingField, IrSchemaDecodeMappingRecordField,
    IrSchemaDecodeMappingSelector, IrSchemaDecodeSpec, IrSchemaRepeat, IrSchemaReservedBits,
};

use crate::schema::mapping::{
    SchemaDecodeMapping, SchemaDecodeMappingExpr, SchemaDecodeMappingField,
    SchemaDecodeMappingSelector, SchemaMappingSelectorComparison, schema_decode_mapping_fields,
    schema_decode_mappings,
};
use crate::types::{
    SchemaDispatchCase, SchemaDispatchCasePayload, SchemaDispatchSpec, SchemaRepeatPayload,
    SchemaRepeatSpec, Type, byte_view_schema_primitive, closed_dispatch_schema_primitive,
    exact_width_schema_primitive, exact_width_schema_primitive_little_endian,
    exact_width_schema_primitive_max_value, extension_dispatch_schema_primitive,
    flag_schema_primitive, recursive_dispatch_payload_case_is_eligible, repeat_schema_primitive,
    reserved_bits_schema_primitive, schema_decode_function_name, schema_decode_value_type,
    schema_dispatch_payload_schema, schema_length_expression_references,
    schema_recursive_dispatch_payload_type, selected_mappings_cover_closed_dispatch,
    supported_encode_reserved_bits,
};

pub(crate) fn schema_decode_specs(module: &SurfaceModule) -> Vec<IrSchemaDecodeSpec> {
    module
        .schemas
        .iter()
        .filter_map(|schema| schema_decode_spec(module, schema))
        .collect()
}

fn schema_decode_spec(module: &SurfaceModule, schema: &SchemaDecl) -> Option<IrSchemaDecodeSpec> {
    schema_decode_spec_inner(module, schema, &mut Vec::new())
}

fn schema_decode_spec_inner(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<IrSchemaDecodeSpec> {
    let schema_name = schema.name.as_ref()?;
    if schema.format.as_ref()?.name != "binary" {
        return None;
    }
    if stack.iter().any(|name| name == schema_name) {
        return None;
    }
    stack.push(schema_name.clone());
    let spec = schema_decode_spec_inner_after_push(module, schema, stack);
    stack.pop();
    spec
}

fn schema_decode_spec_inner_after_push(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<IrSchemaDecodeSpec> {
    let schema_name = schema.name.as_ref()?;
    let mut decoded_field_types = BTreeMap::<String, Type>::new();
    let mut fields = Vec::new();
    for (index, field) in schema.fields.iter().enumerate() {
        let decoded = ir_schema_field(module, schema, field, index, &decoded_field_types, stack)?;
        if let Some((ty, ir_field)) = decoded {
            if let Some(ty) = ty {
                decoded_field_types.insert(field.name.clone(), ty);
            }
            fields.push(ir_field);
        }
    }
    Some(IrSchemaDecodeSpec {
        schema_name: schema_name.clone(),
        function_name: schema_decode_function_name(schema_name),
        fields,
        validation: ir_schema_validation(schema),
        mapping: ir_schema_mapping_fields(module, schema),
        mapping_alternatives: ir_schema_mapping_alternatives(module, schema),
    })
}

fn ir_schema_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    index: usize,
    decoded_field_types: &BTreeMap<String, Type>,
    stack: &mut Vec<String>,
) -> Option<Option<(Option<Type>, IrSchemaDecodeField)>> {
    if let Some(field) = ir_schema_reserved_bits_field(schema, field, index) {
        return Some(Some((None, field?)));
    }
    if let Some(field) = ir_schema_exact_width_field(field) {
        return Some(Some((Some(Type::int()), field?)));
    }
    if let Some(field) = ir_schema_byte_view_field(field, decoded_field_types) {
        return Some(Some((Some(Type::named("ByteView", Vec::new())), field?)));
    }
    if let Some(decoded) = ir_schema_repeat_field(module, schema, field, decoded_field_types, stack)
    {
        return Some(Some(decoded?));
    }
    ir_schema_dispatch_field(module, schema, field, decoded_field_types, stack).map(Some)
}

fn ir_schema_reserved_bits_field(
    schema: &SchemaDecl,
    field: &SchemaField,
    index: usize,
) -> Option<Option<IrSchemaDecodeField>> {
    let reserved = reserved_bits_schema_primitive(&field.ty)?;
    let Some((bit_width, expected_value)) =
        supported_encode_reserved_bits(&schema.fields, index, reserved)
    else {
        return Some(None);
    };
    Some(Some(IrSchemaDecodeField {
        name: field.name.clone(),
        width: 0,
        max_value: 0,
        little_endian: false,
        flag_type: String::new(),
        predicate: None,
        length_field: None,
        repeat: None,
        dispatch: None,
        reserved_bits: Some(IrSchemaReservedBits {
            bit_width,
            expected_value,
        }),
    }))
}

fn ir_schema_exact_width_field(field: &SchemaField) -> Option<Option<IrSchemaDecodeField>> {
    let width = exact_width_schema_primitive(&field.ty)?;
    Some(Some(IrSchemaDecodeField {
        name: field.name.clone(),
        width,
        max_value: exact_width_schema_primitive_max_value(&field.ty)?,
        little_endian: exact_width_schema_primitive_little_endian(&field.ty),
        flag_type: flag_schema_primitive(&field.ty).unwrap_or("").to_string(),
        predicate: field
            .where_clause
            .as_ref()
            .map(|where_clause| where_clause.predicate.clone()),
        length_field: None,
        repeat: None,
        dispatch: None,
        reserved_bits: None,
    }))
}

fn ir_schema_byte_view_field(
    field: &SchemaField,
    decoded_field_types: &BTreeMap<String, Type>,
) -> Option<Option<IrSchemaDecodeField>> {
    let length_expr = byte_view_schema_primitive(&field.ty)?;
    if length_expr
        .references()
        .into_iter()
        .any(|reference| decoded_field_types.get(reference) != Some(&Type::int()))
    {
        return Some(None);
    }
    Some(Some(IrSchemaDecodeField {
        name: field.name.clone(),
        width: 0,
        max_value: 0,
        little_endian: false,
        flag_type: String::new(),
        predicate: None,
        length_field: Some(length_expr.render()),
        repeat: None,
        dispatch: None,
        reserved_bits: None,
    }))
}

fn ir_schema_repeat_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_field_types: &BTreeMap<String, Type>,
    stack: &mut Vec<String>,
) -> Option<Option<(Option<Type>, IrSchemaDecodeField)>> {
    let repeat = repeat_schema_primitive(&field.ty)?;
    if schema_length_expression_references(&repeat.count_field)?
        .into_iter()
        .any(|reference| decoded_field_types.get(reference) != Some(&Type::int()))
    {
        return Some(None);
    }
    if let SchemaRepeatPayload::ByteView { length_field } = &repeat.payload
        && decoded_field_types.get(length_field) != Some(&Type::int())
    {
        return Some(None);
    }
    let (element_ty, ir_repeat) = ir_schema_repeat(module, schema, repeat, stack)?;
    Some(Some((
        Some(Type::named("List", vec![element_ty])),
        IrSchemaDecodeField {
            name: field.name.clone(),
            width: 0,
            max_value: 0,
            little_endian: false,
            flag_type: String::new(),
            predicate: None,
            length_field: None,
            repeat: Some(ir_repeat),
            dispatch: None,
            reserved_bits: None,
        },
    )))
}

fn ir_schema_dispatch_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_field_types: &BTreeMap<String, Type>,
    stack: &mut Vec<String>,
) -> Option<(Option<Type>, IrSchemaDecodeField)> {
    let dispatch = closed_dispatch_schema_primitive(&field.ty)
        .or_else(|| extension_dispatch_schema_primitive(&field.ty))?;
    if decoded_field_types.get(&dispatch.tag_field) != Some(&Type::int())
        || dispatch
            .length_field
            .as_ref()
            .is_some_and(|length_field| decoded_field_types.get(length_field) != Some(&Type::int()))
    {
        return None;
    }
    let field_ty = schema_dispatch_field_type(module, schema, field, &dispatch)?;
    Some((
        Some(field_ty),
        IrSchemaDecodeField {
            name: field.name.clone(),
            width: 0,
            max_value: 0,
            little_endian: false,
            flag_type: String::new(),
            predicate: None,
            length_field: None,
            repeat: None,
            dispatch: Some(IrSchemaDecodeDispatch {
                tag_field: dispatch.tag_field,
                length_field: dispatch.length_field,
                preserves_unknown: dispatch.preserves_unknown,
                cases: dispatch
                    .cases
                    .into_iter()
                    .map(|case| ir_schema_dispatch_case(module, schema, case, stack))
                    .collect::<Option<Vec<_>>>()?,
            }),
            reserved_bits: None,
        },
    ))
}

fn ir_schema_mapping_field(field: SchemaDecodeMappingField) -> IrSchemaDecodeMappingField {
    IrSchemaDecodeMappingField {
        target: field.target,
        source: field.source,
        expr: ir_schema_mapping_expr(field.expr),
    }
}

fn ir_schema_validation(schema: &SchemaDecl) -> Option<String> {
    schema
        .validations
        .first()
        .map(|validation| validation.predicate.clone())
}

fn ir_schema_mapping_fields(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Vec<IrSchemaDecodeMappingField> {
    schema_decode_mapping_fields(module, schema)
        .unwrap_or_default()
        .into_iter()
        .map(ir_schema_mapping_field)
        .collect()
}

fn ir_schema_mapping_alternatives(
    module: &SurfaceModule,
    schema: &SchemaDecl,
) -> Vec<IrSchemaDecodeMapping> {
    schema_decode_mappings(module, schema)
        .unwrap_or_default()
        .into_iter()
        .map(ir_schema_mapping_alternative)
        .collect()
}

fn ir_schema_mapping_alternative(mapping: SchemaDecodeMapping) -> IrSchemaDecodeMapping {
    IrSchemaDecodeMapping {
        selector: mapping.selector.map(ir_schema_mapping_selector),
        fields: mapping
            .fields
            .into_iter()
            .map(ir_schema_mapping_field)
            .collect(),
    }
}

fn ir_schema_mapping_selector(
    selector: SchemaDecodeMappingSelector,
) -> IrSchemaDecodeMappingSelector {
    let simple = selector
        .predicate
        .as_ref()
        .and_then(|predicate| predicate.as_simple_comparison())
        .map(|(field, op, value)| {
            (
                field.to_string(),
                match op {
                    SchemaMappingSelectorComparison::Equal => "==",
                    SchemaMappingSelectorComparison::NotEqual => "!=",
                    SchemaMappingSelectorComparison::Less => "<",
                    SchemaMappingSelectorComparison::LessEqual => "<=",
                    SchemaMappingSelectorComparison::Greater => ">",
                    SchemaMappingSelectorComparison::GreaterEqual => ">=",
                }
                .to_string(),
                value,
            )
        });
    let expr = if simple.is_some() {
        None
    } else {
        Some(ir_schema_mapping_expr(selector.expr))
    };
    IrSchemaDecodeMappingSelector {
        text: selector.text,
        field: simple.as_ref().map(|(field, _, _)| field.clone()),
        operator: simple
            .as_ref()
            .map(|(_, op, _)| op.clone())
            .unwrap_or_default(),
        value: simple.map(|(_, _, value)| value).unwrap_or_default(),
        expr,
    }
}

fn ir_schema_mapping_expr(expr: SchemaDecodeMappingExpr) -> IrSchemaDecodeMappingExpr {
    match expr {
        SchemaDecodeMappingExpr::Field(name) => IrSchemaDecodeMappingExpr::Field(name),
        SchemaDecodeMappingExpr::Literal(value) => IrSchemaDecodeMappingExpr::Literal(value),
        SchemaDecodeMappingExpr::FieldAccess { base, field } => {
            IrSchemaDecodeMappingExpr::FieldAccess {
                base: Box::new(ir_schema_mapping_expr(*base)),
                field,
            }
        }
        SchemaDecodeMappingExpr::Record(fields) => IrSchemaDecodeMappingExpr::Record(
            fields
                .into_iter()
                .map(|field| IrSchemaDecodeMappingRecordField {
                    name: field.name,
                    expr: ir_schema_mapping_expr(field.expr),
                })
                .collect(),
        ),
        SchemaDecodeMappingExpr::Constructor { name, args } => {
            IrSchemaDecodeMappingExpr::Constructor {
                name,
                args: args.into_iter().map(ir_schema_mapping_expr).collect(),
            }
        }
        SchemaDecodeMappingExpr::Converter {
            function,
            inverse_function,
            args,
        } => IrSchemaDecodeMappingExpr::Converter {
            function,
            inverse_function,
            args: args
                .into_iter()
                .map(|arg| ir_schema_mapping_expr(arg.expr))
                .collect(),
        },
        SchemaDecodeMappingExpr::Prefix { op, expr } => IrSchemaDecodeMappingExpr::Prefix {
            op,
            expr: Box::new(ir_schema_mapping_expr(*expr)),
        },
        SchemaDecodeMappingExpr::Binary { op, left, right } => IrSchemaDecodeMappingExpr::Binary {
            op,
            left: Box::new(ir_schema_mapping_expr(*left)),
            right: Box::new(ir_schema_mapping_expr(*right)),
        },
    }
}

fn ir_schema_dispatch_case(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    case: SchemaDispatchCase,
    stack: &mut Vec<String>,
) -> Option<IrSchemaDecodeDispatchCase> {
    let width = match &case.payload {
        SchemaDispatchCasePayload::Primitive { width, .. } => *width,
        SchemaDispatchCasePayload::Schema { .. } => 0,
    };
    let little_endian = match &case.payload {
        SchemaDispatchCasePayload::Primitive { little_endian, .. } => *little_endian,
        SchemaDispatchCasePayload::Schema { .. } => false,
    };
    let payload_schema = match case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => None,
        SchemaDispatchCasePayload::Schema { schema_name } => {
            if schema.name.as_deref() == Some(schema_name.as_str()) {
                return Some(IrSchemaDecodeDispatchCase {
                    tag: case.tag,
                    width,
                    little_endian,
                    payload_schema: None,
                    payload_schema_name: Some(schema_name),
                });
            }
            let nested_schema = schema_dispatch_payload_schema(module, schema, &schema_name)?;
            Some(Box::new(schema_decode_spec_inner(
                module,
                nested_schema,
                stack,
            )?))
        }
    };
    Some(IrSchemaDecodeDispatchCase {
        tag: case.tag,
        width,
        little_endian,
        payload_schema,
        payload_schema_name: None,
    })
}

fn ir_schema_repeat(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    repeat: SchemaRepeatSpec,
    stack: &mut Vec<String>,
) -> Option<(Type, IrSchemaRepeat)> {
    let (element_ty, width, max_value, little_endian, byte_view_length_field, payload_schema) =
        match repeat.payload {
            SchemaRepeatPayload::Primitive {
                width,
                max_value,
                little_endian,
            } => (Type::int(), width, max_value, little_endian, None, None),
            SchemaRepeatPayload::ByteView { length_field } => (
                Type::named("ByteView", Vec::new()),
                0,
                0,
                false,
                Some(length_field),
                None,
            ),
            SchemaRepeatPayload::Schema { schema_name } => {
                let nested_schema = schema_dispatch_payload_schema(module, schema, &schema_name)?;
                let element_ty = schema_decode_value_type(module, nested_schema)?;
                let payload_schema = schema_decode_spec_inner(module, nested_schema, stack)?;
                (
                    element_ty,
                    0,
                    0,
                    false,
                    None,
                    Some(Box::new(payload_schema)),
                )
            }
        };
    Some((
        element_ty,
        IrSchemaRepeat {
            count_field: repeat.count_field,
            width,
            max_value,
            little_endian,
            byte_view_length_field,
            payload_schema,
        },
    ))
}

fn schema_dispatch_field_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
) -> Option<Type> {
    let mut payload_types = dispatch
        .cases
        .iter()
        .map(|case| match &case.payload {
            SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
            SchemaDispatchCasePayload::Schema { schema_name } => {
                if recursive_dispatch_payload_case_is_eligible(
                    module,
                    schema,
                    field,
                    dispatch,
                    schema_name,
                ) {
                    schema_recursive_dispatch_payload_type(module, schema)
                } else {
                    let payload_schema =
                        schema_dispatch_payload_schema(module, schema, schema_name)?;
                    schema_decode_value_type(module, payload_schema)
                }
            }
        })
        .collect::<Option<Vec<_>>>()?;
    let payload_ty = payload_types.pop()?;
    let recursive_payload = dispatch.cases.iter().any(|case| {
        matches!(
            &case.payload,
            SchemaDispatchCasePayload::Schema { schema_name }
                if recursive_dispatch_payload_case_is_eligible(
                    module,
                    schema,
                    field,
                    dispatch,
                    schema_name,
                )
        )
    });
    let payload_ty = if recursive_payload {
        schema_recursive_dispatch_payload_type(module, schema)?
    } else if payload_types.iter().any(|ty| ty != &payload_ty)
        && !selected_mappings_cover_closed_dispatch(schema, dispatch)
    {
        return None;
    } else {
        payload_ty
    };
    if dispatch.preserves_unknown {
        Some(Type::named("SchemaDispatchPayload", vec![payload_ty]))
    } else {
        Some(payload_ty)
    }
}
