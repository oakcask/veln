use veln_ast::{SchemaDecl, SurfaceModule, Visibility};
use veln_source::SourceSpan;

use crate::analysis::boundary::schema_composition::{
    SchemaIdentity, schema_composition_reaches, schema_composition_reference_blocker,
    schema_field_has_ordinary_type_target,
};
use crate::analysis::boundary::schema_repeat_resolution::companion_private_schema_access_allowed;
use crate::name_recovery::{normal_imported_use_for_path, use_decl_has_invalid_module_segment};
use crate::schema::primitives::{
    SchemaRepeatPayload, repeat_schema_primitive, schema_payload_name_path,
};
use crate::types::schema_types::schema_field_uses_existing_grammar;

#[derive(Clone, Debug)]
pub struct ResolvedSchemaCompositionReference {
    pub field_span: SourceSpan,
    pub path: Vec<String>,
    pub target_span: SourceSpan,
    pub target_module: Option<String>,
    pub target_name: String,
}

pub fn resolved_schema_composition_references(
    module: &SurfaceModule,
) -> Vec<ResolvedSchemaCompositionReference> {
    let mut references = Vec::new();
    for schema in &module.schemas {
        for field in &schema.fields {
            if let Some((path, target)) = direct_schema_composition_target(module, schema, field) {
                references.push(ResolvedSchemaCompositionReference {
                    field_span: field.span.clone(),
                    path,
                    target_span: target.span.clone(),
                    target_module: target.module_name.clone(),
                    target_name: target.name.clone().unwrap_or_default(),
                });
                continue;
            }
            let Some(repeat) = repeat_schema_primitive(&field.ty) else {
                continue;
            };
            let SchemaRepeatPayload::Schema { schema_name } = repeat.payload else {
                continue;
            };
            let Some(target) = navigation_repeat_payload_target(module, schema, &schema_name)
            else {
                continue;
            };
            let Some(path) = schema_payload_name_path(&schema_name) else {
                continue;
            };
            references.push(ResolvedSchemaCompositionReference {
                field_span: field.span.clone(),
                path,
                target_span: target.span.clone(),
                target_module: target.module_name.clone(),
                target_name: target.name.clone().unwrap_or_default(),
            });
        }
    }
    references
}

fn direct_schema_composition_target<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &veln_ast::SchemaField,
) -> Option<(Vec<String>, &'a SchemaDecl)> {
    if schema_field_uses_existing_grammar(schema, &field.ty)
        || schema_composition_reference_blocker(module, schema, field).is_some()
    {
        return None;
    }
    let path = schema_payload_name_path(&field.ty)?;
    let target = match path.as_slice() {
        [name] => module.schemas.iter().find(|candidate| {
            candidate.name.as_deref() == Some(name)
                && candidate.module_name.as_deref() == schema.module_name.as_deref()
        }),
        [_, .., name] => {
            let use_decl = navigation_imported_use_for_path(
                module,
                &path[..path.len() - 1],
                schema.module_name.as_deref(),
            )?;
            if use_decl.package.is_some() {
                return None;
            }
            module.schemas.iter().find(|candidate| {
                candidate.name.as_deref() == Some(name)
                    && candidate.module_name.as_deref() == Some(use_decl.name.as_str())
                    && (candidate.visibility == Visibility::Public
                        || companion_private_schema_access_allowed(module, schema, use_decl))
            })
        }
        _ => None,
    }?;
    if schema_field_has_ordinary_type_target(module, schema, &field.ty)
        || schema.format.as_ref().map(|format| format.name.as_str())
            != target.format.as_ref().map(|format| format.name.as_str())
        || schema_composition_reaches(module, target, schema, &mut Vec::new())
    {
        return None;
    }
    Some((path, target))
}

fn navigation_repeat_payload_target<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    schema_name: &str,
) -> Option<&'a SchemaDecl> {
    if schema.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
        return None;
    }
    let path = schema_payload_name_path(schema_name)?;
    let target = match path.as_slice() {
        [name] => {
            let current_index = module.schemas.iter().position(|candidate| {
                SchemaIdentity::of(candidate) == SchemaIdentity::of(schema)
            })?;
            let (target_index, target) =
                module.schemas.iter().enumerate().find(|(_, candidate)| {
                    candidate.name.as_deref() == Some(name)
                        && candidate.module_name.as_deref() == schema.module_name.as_deref()
                })?;
            (target_index < current_index).then_some(target)
        }
        [_, .., name] => {
            let use_decl = navigation_imported_use_for_path(
                module,
                &path[..path.len() - 1],
                schema.module_name.as_deref(),
            )?;
            if use_decl.package.is_some() {
                return None;
            }
            module.schemas.iter().find(|candidate| {
                candidate.name.as_deref() == Some(name)
                    && candidate.module_name.as_deref() == Some(use_decl.name.as_str())
                    && (candidate.visibility == Visibility::Public
                        || companion_private_schema_access_allowed(module, schema, use_decl))
            })
        }
        _ => None,
    }?;
    (target.format.as_ref().map(|format| format.name.as_str()) == Some("binary")).then_some(target)
}

fn navigation_imported_use_for_path<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a veln_ast::UseDecl> {
    normal_imported_use_for_path(module, segments, current_module).or_else(|| {
        let qualifier = segments.join("::");
        module.uses.iter().find(|use_decl| {
            use_decl.module_name.as_deref() == current_module
                && !use_decl_has_invalid_module_segment(module, use_decl)
                && use_decl.alias == qualifier
        })
    })
}
