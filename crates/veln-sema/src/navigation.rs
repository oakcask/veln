use veln_ast::{PublicAlias, PublicAliasKind, SchemaDecl, SurfaceModule, Visibility};
use veln_source::SourceSpan;

use crate::analysis::boundary::schema_composition::{
    SchemaIdentity, schema_composition_reaches, schema_composition_reference_blocker,
    schema_field_has_ordinary_type_target,
};
use crate::analysis::boundary::schema_repeat_resolution::companion_private_schema_access_allowed;
use crate::name_recovery::{
    public_alias_has_invalid_target_leaf, schema_composition_imported_use_for_path,
};
use crate::schema::primitives::{
    SchemaRepeatPayload, repeat_schema_primitive, schema_payload_name_path,
};
use crate::types::schema_types::schema_field_uses_existing_grammar;

#[derive(Clone, Debug)]
pub struct ResolvedSchemaCompositionReference {
    pub field_span: SourceSpan,
    pub path: Vec<String>,
    pub alias_span: Option<SourceSpan>,
    pub alias_module: Option<String>,
    pub alias_name: Option<String>,
    pub target_span: SourceSpan,
    pub target_module: Option<String>,
    pub target_name: String,
}

#[derive(Clone, Debug)]
pub struct ResolvedSchemaAlias {
    pub alias_span: SourceSpan,
    pub alias_module: Option<String>,
    pub alias_name: String,
    pub target_span: SourceSpan,
    pub target_module: Option<String>,
    pub target_name: String,
}

pub fn resolved_schema_aliases(module: &SurfaceModule) -> Vec<ResolvedSchemaAlias> {
    module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Schema)
        .filter(|alias| {
            module
                .aliases
                .iter()
                .filter(|candidate| {
                    candidate.kind == PublicAliasKind::Schema
                        && candidate.name == alias.name
                        && candidate.module_name == alias.module_name
                })
                .count()
                == 1
        })
        .filter(|alias| {
            !module
                .schemas
                .iter()
                .any(|schema| schema.name == alias.name && schema.module_name == alias.module_name)
        })
        .filter_map(|alias| {
            let target = direct_public_schema_alias_target(module, alias)?;
            Some(ResolvedSchemaAlias {
                alias_span: alias.span.clone(),
                alias_module: alias.module_name.clone(),
                alias_name: alias.name.clone()?,
                target_span: target.span.clone(),
                target_module: target.module_name.clone(),
                target_name: target.name.clone()?,
            })
        })
        .collect()
}

fn direct_public_schema_alias_target<'a>(
    module: &'a SurfaceModule,
    alias: &PublicAlias,
) -> Option<&'a SchemaDecl> {
    if public_alias_has_invalid_target_leaf(module, alias, None) {
        return None;
    }
    let (target_module, target_name) = match alias.target.as_slice() {
        [name] => (alias.module_name.as_deref(), name.as_str()),
        [qualifiers @ .., name] => {
            let use_decl = schema_composition_imported_use_for_path(
                module,
                qualifiers,
                alias.module_name.as_deref(),
            )?;
            if use_decl.package.is_some() {
                return None;
            }
            (Some(use_decl.name.as_str()), name.as_str())
        }
        [] => return None,
    };
    let mut candidates = module.schemas.iter().filter(|schema| {
        schema.name.as_deref() == Some(target_name)
            && schema.module_name.as_deref() == target_module
            && schema.visibility == Visibility::Public
    });
    let target = candidates.next()?;
    candidates.next().is_none().then_some(target)
}

pub fn resolved_schema_composition_references(
    module: &SurfaceModule,
) -> Vec<ResolvedSchemaCompositionReference> {
    let mut references = Vec::new();
    for schema in &module.schemas {
        for field in &schema.fields {
            if let Some((path, target)) = direct_schema_composition_target(module, schema, field) {
                let alias = direct_schema_alias_for_path(
                    module,
                    schema.module_name.as_deref(),
                    &path,
                    target,
                );
                references.push(ResolvedSchemaCompositionReference {
                    field_span: field.span.clone(),
                    path,
                    alias_span: alias.map(|alias| alias.span.clone()),
                    alias_module: alias.and_then(|alias| alias.module_name.clone()),
                    alias_name: alias.and_then(|alias| alias.name.clone()),
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
            let alias =
                direct_schema_alias_for_path(module, schema.module_name.as_deref(), &path, target);
            references.push(ResolvedSchemaCompositionReference {
                field_span: field.span.clone(),
                path,
                alias_span: alias.map(|alias| alias.span.clone()),
                alias_module: alias.and_then(|alias| alias.module_name.clone()),
                alias_name: alias.and_then(|alias| alias.name.clone()),
                target_span: target.span.clone(),
                target_module: target.module_name.clone(),
                target_name: target.name.clone().unwrap_or_default(),
            });
        }
    }
    references
}

fn direct_schema_alias_for_path<'a>(
    module: &'a SurfaceModule,
    current_module: Option<&str>,
    path: &[String],
    target: &SchemaDecl,
) -> Option<&'a PublicAlias> {
    let alias = schema_alias_for_path(module, current_module, path)?;
    let resolved_target = direct_public_schema_alias_target(module, alias)?;
    (resolved_target.span == target.span).then_some(alias)
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
    let direct = match path {
        [name] => local_schema_target(module, schema, name),
        [_, .., name] => imported_schema_target(module, schema, path, name),
        _ => None,
    };
    direct.or_else(|| {
        schema_alias_for_path(module, schema.module_name.as_deref(), path)
            .and_then(|alias| direct_public_schema_alias_target(module, alias))
    })
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
    let direct = match path {
        [name] => prior_local_schema_target(module, schema, name),
        [_, .., name] => imported_schema_target(module, schema, path, name),
        _ => None,
    };
    direct.or_else(|| {
        let alias = schema_alias_for_path(module, schema.module_name.as_deref(), path)?;
        let target = direct_public_schema_alias_target(module, alias)?;
        if target.module_name == schema.module_name {
            let current_index = module.schemas.iter().position(|candidate| {
                SchemaIdentity::of(candidate) == SchemaIdentity::of(schema)
            })?;
            let target_index = module.schemas.iter().position(|candidate| {
                SchemaIdentity::of(candidate) == SchemaIdentity::of(target)
            })?;
            (target_index < current_index).then_some(target)
        } else {
            Some(target)
        }
    })
}

fn schema_alias_for_path<'a>(
    module: &'a SurfaceModule,
    current_module: Option<&str>,
    path: &[String],
) -> Option<&'a PublicAlias> {
    let (alias_module, alias_name) = match path {
        [name] => (current_module, name.as_str()),
        [qualifiers @ .., name] => {
            let use_decl =
                schema_composition_imported_use_for_path(module, qualifiers, current_module)?;
            if use_decl.package.is_some() {
                return None;
            }
            (Some(use_decl.name.as_str()), name.as_str())
        }
        [] => return None,
    };
    let mut aliases = module.aliases.iter().filter(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(alias_name)
            && alias.module_name.as_deref() == alias_module
    });
    let alias = aliases.next()?;
    aliases.next().is_none().then_some(alias)
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
