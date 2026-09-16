use veln_ast::{SchemaDecl, SurfaceModule, Visibility};
use veln_source::SourceSpan;

use crate::analysis::boundary::schema_composition::{
    SchemaIdentity, schema_composition_reaches, schema_composition_reference_blocker,
    schema_field_has_ordinary_type_target,
};
use crate::analysis::boundary::schema_repeat_resolution::companion_private_schema_access_allowed;
use crate::name_recovery::schema_composition_imported_use_for_path;
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
    let target = schema_composition_target_for_path(module, schema, &path)?;
    if !direct_schema_composition_target_is_supported(module, schema, field, target) {
        return None;
    }
    Some((path, target))
}

fn schema_composition_target_for_path<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    path: &[String],
) -> Option<&'a SchemaDecl> {
    match path {
        [name] => local_schema_target(module, schema, name),
        [_, .., name] => imported_schema_target(module, schema, path, name),
        _ => None,
    }
}

fn local_schema_target<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    name: &str,
) -> Option<&'a SchemaDecl> {
    module.schemas.iter().find(|candidate| {
        candidate.name.as_deref() == Some(name)
            && candidate.module_name.as_deref() == schema.module_name.as_deref()
    })
}

fn imported_schema_target<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    path: &[String],
    name: &str,
) -> Option<&'a SchemaDecl> {
    let use_decl = schema_composition_imported_use_for_path(
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

fn direct_schema_composition_target_is_supported(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &veln_ast::SchemaField,
    target: &SchemaDecl,
) -> bool {
    !schema_field_has_ordinary_type_target(module, schema, &field.ty)
        && schema_format_name(schema) == schema_format_name(target)
        && !schema_composition_reaches(module, target, schema, &mut Vec::new())
}

fn navigation_repeat_payload_target<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    schema_name: &str,
) -> Option<&'a SchemaDecl> {
    if schema_format_name(schema) != Some("binary") {
        return None;
    }
    let path = schema_payload_name_path(schema_name)?;
    let target = repeat_payload_target_for_path(module, schema, &path)?;
    (schema_format_name(target) == Some("binary")).then_some(target)
}

fn repeat_payload_target_for_path<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    path: &[String],
) -> Option<&'a SchemaDecl> {
    match path {
        [name] => prior_local_schema_target(module, schema, name),
        [_, .., name] => imported_schema_target(module, schema, path, name),
        _ => None,
    }
}

fn prior_local_schema_target<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    name: &str,
) -> Option<&'a SchemaDecl> {
    let current_index = module
        .schemas
        .iter()
        .position(|candidate| SchemaIdentity::of(candidate) == SchemaIdentity::of(schema))?;
    let (target_index, target) = module.schemas.iter().enumerate().find(|(_, candidate)| {
        candidate.name.as_deref() == Some(name)
            && candidate.module_name.as_deref() == schema.module_name.as_deref()
    })?;
    (target_index < current_index).then_some(target)
}

fn schema_format_name(schema: &SchemaDecl) -> Option<&str> {
    schema.format.as_ref().map(|format| format.name.as_str())
}
