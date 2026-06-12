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
    IrSchemaDecodeDispatch, IrSchemaDecodeDispatchCase, IrSchemaDecodeField,
    IrSchemaDecodeMappingField, IrSchemaDecodeSpec, TypedProgram, lower_checked_core,
};

use crate::analysis::{
    check_codec_decode_signatures, check_codec_encode_signatures, check_codec_schema_references,
    check_declared_effect_labels, check_duplicate_codec_names, check_duplicate_constructor_names,
    check_duplicate_function_names, check_duplicate_type_names, check_duplicate_use_aliases,
    check_function_body, check_module_boundary, check_public_aliases,
    check_public_function_boundary, check_reserved_prelude_aliases, check_schema_field_primitives,
    check_schema_mappings, check_schema_type_references, check_test_declaration_boundary,
};
use crate::lowering::lower_surface_module_to_core;
use crate::types::{
    TypeEnvironment, closed_dispatch_schema_primitive, exact_width_schema_primitive,
    exact_width_schema_primitive_max_value, schema_decode_function_name,
    schema_decode_mapping_fields,
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
        .filter_map(|schema| {
            let schema_name = schema.name.as_ref()?;
            if schema.format.as_ref()?.name != "binary" {
                return None;
            }
            let mut decoded_field_names = Vec::new();
            let mut fields = Vec::new();
            for field in &schema.fields {
                if let Some(width) = exact_width_schema_primitive(&field.ty) {
                    decoded_field_names.push(field.name.clone());
                    fields.push(IrSchemaDecodeField {
                        name: field.name.clone(),
                        width,
                        max_value: exact_width_schema_primitive_max_value(&field.ty)?,
                        predicate: field
                            .where_clause
                            .as_ref()
                            .map(|where_clause| where_clause.predicate.clone()),
                        dispatch: None,
                    });
                    continue;
                }
                let dispatch = closed_dispatch_schema_primitive(&field.ty)?;
                if !decoded_field_names.contains(&dispatch.tag_field) {
                    return None;
                }
                decoded_field_names.push(field.name.clone());
                fields.push(IrSchemaDecodeField {
                    name: field.name.clone(),
                    width: 0,
                    max_value: 0,
                    predicate: None,
                    dispatch: Some(IrSchemaDecodeDispatch {
                        tag_field: dispatch.tag_field,
                        cases: dispatch
                            .cases
                            .into_iter()
                            .map(|case| IrSchemaDecodeDispatchCase {
                                tag: case.tag,
                                width: case.width,
                            })
                            .collect(),
                    }),
                });
            }
            Some(IrSchemaDecodeSpec {
                schema_name: schema_name.clone(),
                function_name: schema_decode_function_name(schema_name),
                fields,
                mapping: schema_decode_mapping_fields(module, schema)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|field| IrSchemaDecodeMappingField {
                        target: field.target,
                        source: field.source,
                    })
                    .collect(),
            })
        })
        .collect()
}
