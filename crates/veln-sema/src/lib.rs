//! Name, type, effect, contract, and hole analysis.

mod adt;
mod analysis;
mod call_resolution;
mod contracts;
mod diagnostics;
mod effects;
mod lowering;
mod prelude;
mod repair_candidates;
mod standard_symbols;
#[cfg(test)]
mod tests;
mod types;

use veln_ast::{FunctionKind, SurfaceModule, Visibility};
use veln_core::CheckedProgram;
use veln_diagnostics::{Diagnostic, Severity};
use veln_ir::{
    IrSchemaDecodeDispatch, IrSchemaDecodeDispatchCase, IrSchemaDecodeField, IrSchemaDecodeMapping,
    IrSchemaDecodeMappingExpr, IrSchemaDecodeMappingField, IrSchemaDecodeMappingRecordField,
    IrSchemaDecodeMappingSelector, IrSchemaDecodeSpec, IrSchemaRepeat, IrSchemaReservedBits,
    TypedProgram, lower_checked_core,
};

use crate::analysis::{
    check_codec_decode_signatures, check_codec_encode_signatures, check_codec_schema_references,
    check_declared_effect_labels, check_duplicate_codec_names, check_duplicate_constructor_names,
    check_duplicate_function_names, check_duplicate_schema_names, check_duplicate_type_names,
    check_duplicate_use_aliases, check_function_body, check_module_boundary, check_public_aliases,
    check_public_function_boundary, check_reserved_prelude_aliases, check_schema_field_primitives,
    check_schema_mappings, check_schema_type_references, check_test_declaration_boundary,
};
use crate::lowering::lower_surface_module_to_core;
use crate::types::{
    SchemaDecodeMappingExpr, SchemaDispatchCasePayload, SchemaDispatchSpec, SchemaRepeatPayload,
    Type, TypeEnvironment, byte_view_schema_primitive, closed_dispatch_schema_primitive,
    exact_width_schema_primitive, exact_width_schema_primitive_little_endian,
    exact_width_schema_primitive_max_value, extension_dispatch_schema_primitive,
    flag_schema_primitive, repeat_schema_primitive, reserved_bits_schema_primitive,
    schema_decode_function_name, schema_decode_mapping_fields, schema_decode_mappings,
    schema_decode_value_type, schema_dispatch_payload_schema, schema_length_expression_references,
    supported_encode_reserved_bits,
};

#[derive(Clone, Debug)]
pub struct LoweredSurfaceModule {
    pub diagnostics: Vec<Diagnostic>,
    pub core: Option<CheckedProgram>,
    pub ir: Option<TypedProgram>,
}

pub fn analyze_surface_module(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let environment = TypeEnvironment::from_module(module);

    diagnostics.extend(check_duplicate_function_names(module));
    diagnostics.extend(check_duplicate_type_names(module));
    diagnostics.extend(check_duplicate_schema_names(module));
    diagnostics.extend(check_duplicate_codec_names(module));
    diagnostics.extend(check_duplicate_constructor_names(module));
    diagnostics.extend(check_module_boundary(module));
    diagnostics.extend(check_duplicate_use_aliases(module));
    diagnostics.extend(check_reserved_prelude_aliases(module));
    diagnostics.extend(check_public_aliases(module));
    diagnostics.extend(check_codec_schema_references(module));
    diagnostics.extend(check_codec_decode_signatures(module));
    diagnostics.extend(check_codec_encode_signatures(module));
    diagnostics.extend(check_schema_field_primitives(module));
    diagnostics.extend(check_schema_mappings(module));
    diagnostics.extend(check_schema_type_references(module));

    for function in &module.functions {
        diagnostics.extend(check_declared_effect_labels(function));
        if function.visibility == Visibility::Public {
            diagnostics.extend(check_public_function_boundary(function));
        }
        if function.kind == FunctionKind::Test {
            diagnostics.extend(check_test_declaration_boundary(function));
        }
        diagnostics.extend(check_function_body(function, &environment));
    }

    diagnostics
}

pub fn lower_checked_surface_module(module: &SurfaceModule) -> LoweredSurfaceModule {
    lower_analyzed_surface_module(module, analyze_surface_module(module))
}

pub fn lower_analyzed_surface_module(
    module: &SurfaceModule,
    mut diagnostics: Vec<Diagnostic>,
) -> LoweredSurfaceModule {
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return LoweredSurfaceModule {
            diagnostics,
            core: None,
            ir: None,
        };
    }

    let environment = TypeEnvironment::from_module(module);
    let lowered_core = lower_surface_module_to_core(module, &environment);
    diagnostics.extend(lowered_core.diagnostics);
    let ir = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        None
    } else {
        lower_checked_core(&lowered_core.program)
            .ok()
            .map(|mut ir| {
                ir.schema_decoders = schema_decode_specs(module);
                ir
            })
    };

    LoweredSurfaceModule {
        diagnostics,
        core: Some(lowered_core.program),
        ir,
    }
}

fn schema_decode_specs(module: &SurfaceModule) -> Vec<IrSchemaDecodeSpec> {
    module
        .schemas
        .iter()
        .filter_map(|schema| schema_decode_spec(module, schema))
        .collect()
}

fn schema_decode_spec(
    module: &SurfaceModule,
    schema: &veln_ast::SchemaDecl,
) -> Option<IrSchemaDecodeSpec> {
    schema_decode_spec_inner(module, schema, &mut Vec::new())
}

fn schema_decode_spec_inner(
    module: &SurfaceModule,
    schema: &veln_ast::SchemaDecl,
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
    schema: &veln_ast::SchemaDecl,
    stack: &mut Vec<String>,
) -> Option<IrSchemaDecodeSpec> {
    let schema_name = schema.name.as_ref()?;
    let mut decoded_field_types = std::collections::BTreeMap::<String, Type>::new();
    let mut fields = Vec::new();
    for (index, field) in schema.fields.iter().enumerate() {
        if let Some(reserved) = reserved_bits_schema_primitive(&field.ty) {
            let (bit_width, expected_value) =
                supported_encode_reserved_bits(&schema.fields, index, reserved)?;
            fields.push(IrSchemaDecodeField {
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
            });
            continue;
        }
        if let Some(width) = exact_width_schema_primitive(&field.ty) {
            decoded_field_types.insert(field.name.clone(), Type::int());
            fields.push(IrSchemaDecodeField {
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
            });
            continue;
        }
        if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
            if length_expr
                .references()
                .into_iter()
                .any(|reference| decoded_field_types.get(reference) != Some(&Type::int()))
            {
                return None;
            }
            decoded_field_types.insert(field.name.clone(), Type::named("ByteView", Vec::new()));
            fields.push(IrSchemaDecodeField {
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
            });
            continue;
        }
        if let Some(repeat) = repeat_schema_primitive(&field.ty) {
            if schema_length_expression_references(&repeat.count_field)?
                .into_iter()
                .any(|reference| decoded_field_types.get(reference) != Some(&Type::int()))
            {
                return None;
            }
            if let SchemaRepeatPayload::ByteView { length_field } = &repeat.payload
                && decoded_field_types.get(length_field) != Some(&Type::int())
            {
                return None;
            }
            let (element_ty, ir_repeat) = ir_schema_repeat(module, schema, repeat, stack)?;
            decoded_field_types.insert(field.name.clone(), Type::named("List", vec![element_ty]));
            fields.push(IrSchemaDecodeField {
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
            });
            continue;
        }
        let dispatch = closed_dispatch_schema_primitive(&field.ty)
            .or_else(|| extension_dispatch_schema_primitive(&field.ty))?;
        if decoded_field_types.get(&dispatch.tag_field) != Some(&Type::int())
            || dispatch.length_field.as_ref().is_some_and(|length_field| {
                decoded_field_types.get(length_field) != Some(&Type::int())
            })
        {
            return None;
        }
        let field_ty = schema_dispatch_field_type(module, schema, &dispatch)?;
        decoded_field_types.insert(field.name.clone(), field_ty);
        fields.push(IrSchemaDecodeField {
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
                cases: dispatch
                    .cases
                    .into_iter()
                    .map(|case| ir_schema_dispatch_case(module, schema, case, stack))
                    .collect::<Option<Vec<_>>>()?,
            }),
            reserved_bits: None,
        });
    }
    Some(IrSchemaDecodeSpec {
        schema_name: schema_name.clone(),
        function_name: schema_decode_function_name(schema_name),
        fields,
        validation: schema
            .validations
            .first()
            .map(|validation| validation.predicate.clone()),
        mapping: schema_decode_mapping_fields(module, schema)
            .unwrap_or_default()
            .into_iter()
            .map(|field| IrSchemaDecodeMappingField {
                target: field.target,
                source: field.source,
                expr: ir_schema_mapping_expr(field.expr),
            })
            .collect(),
        mapping_alternatives: schema_decode_mappings(module, schema)
            .unwrap_or_default()
            .into_iter()
            .map(|mapping| IrSchemaDecodeMapping {
                selector: mapping
                    .selector
                    .map(|selector| IrSchemaDecodeMappingSelector {
                        field: selector.field,
                        value: selector.value,
                    }),
                fields: mapping
                    .fields
                    .into_iter()
                    .map(|field| IrSchemaDecodeMappingField {
                        target: field.target,
                        source: field.source,
                        expr: ir_schema_mapping_expr(field.expr),
                    })
                    .collect(),
            })
            .collect(),
    })
}

fn ir_schema_mapping_expr(expr: SchemaDecodeMappingExpr) -> IrSchemaDecodeMappingExpr {
    match expr {
        SchemaDecodeMappingExpr::Field(name) => IrSchemaDecodeMappingExpr::Field(name),
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
        SchemaDecodeMappingExpr::Converter { function, arg } => {
            IrSchemaDecodeMappingExpr::Converter {
                function,
                arg: Box::new(ir_schema_mapping_expr(*arg)),
            }
        }
        SchemaDecodeMappingExpr::Binary { op, left, right } => IrSchemaDecodeMappingExpr::Binary {
            op,
            left: Box::new(ir_schema_mapping_expr(*left)),
            right: Box::new(ir_schema_mapping_expr(*right)),
        },
    }
}

fn ir_schema_dispatch_case(
    module: &SurfaceModule,
    schema: &veln_ast::SchemaDecl,
    case: crate::types::SchemaDispatchCase,
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
    })
}

fn ir_schema_repeat(
    module: &SurfaceModule,
    schema: &veln_ast::SchemaDecl,
    repeat: crate::types::SchemaRepeatSpec,
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
    schema: &veln_ast::SchemaDecl,
    dispatch: &SchemaDispatchSpec,
) -> Option<Type> {
    let mut payload_types = dispatch
        .cases
        .iter()
        .map(|case| match &case.payload {
            SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
            SchemaDispatchCasePayload::Schema { schema_name } => {
                let payload_schema = schema_dispatch_payload_schema(module, schema, schema_name)?;
                schema_decode_value_type(module, payload_schema)
            }
        })
        .collect::<Option<Vec<_>>>()?;
    let payload_ty = payload_types.pop()?;
    if payload_types.iter().any(|ty| ty != &payload_ty) {
        return None;
    }
    if dispatch.length_field.is_some() {
        Some(Type::named("SchemaDispatchPayload", vec![payload_ty]))
    } else {
        Some(payload_ty)
    }
}
