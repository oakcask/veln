use std::collections::{BTreeMap, BTreeSet};

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
    SchemaRepeatPayload, repeat_schema_primitive, schema_length_expression,
    schema_payload_name_path,
};
use crate::types::schema_types::schema_field_uses_existing_grammar;

#[cfg(test)]
thread_local! {
    static SCHEMA_ALIAS_TARGET_IMPORT_INDEX_ENTRIES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static SCHEMA_ALIAS_TARGET_IMPORT_LOOKUPS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static SCHEMA_ALIAS_CHAIN_RESOLUTION_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_schema_alias_target_import_index_entry() {
    SCHEMA_ALIAS_TARGET_IMPORT_INDEX_ENTRIES
        .set(SCHEMA_ALIAS_TARGET_IMPORT_INDEX_ENTRIES.get() + 1);
}

#[cfg(test)]
fn record_schema_alias_target_import_lookup() {
    SCHEMA_ALIAS_TARGET_IMPORT_LOOKUPS.set(SCHEMA_ALIAS_TARGET_IMPORT_LOOKUPS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_schema_alias_target_import_work() {
    SCHEMA_ALIAS_TARGET_IMPORT_INDEX_ENTRIES.set(0);
    SCHEMA_ALIAS_TARGET_IMPORT_LOOKUPS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_alias_target_import_work() -> (usize, usize) {
    (
        SCHEMA_ALIAS_TARGET_IMPORT_INDEX_ENTRIES.get(),
        SCHEMA_ALIAS_TARGET_IMPORT_LOOKUPS.get(),
    )
}

#[cfg(test)]
fn record_schema_alias_chain_resolution_visit() {
    SCHEMA_ALIAS_CHAIN_RESOLUTION_VISITS.set(SCHEMA_ALIAS_CHAIN_RESOLUTION_VISITS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_schema_alias_chain_resolution_work() {
    SCHEMA_ALIAS_CHAIN_RESOLUTION_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_alias_chain_resolution_work() -> usize {
    SCHEMA_ALIAS_CHAIN_RESOLUTION_VISITS.get()
}

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
    pub direct_target_module: Option<String>,
    pub direct_target_name: String,
    pub direct_target_is_alias: bool,
}

pub fn schema_repeat_count_expression_is_valid(text: &str) -> bool {
    schema_length_expression(text).is_some()
}

pub fn resolved_schema_aliases(module: &SurfaceModule) -> Vec<ResolvedSchemaAlias> {
    let imports = SchemaAliasTargetImportIndex::new(module);
    let mut alias_counts = BTreeMap::new();
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Schema)
    {
        let Some(name) = alias.name.as_deref() else {
            continue;
        };
        *alias_counts
            .entry((alias.module_name.as_deref(), name))
            .or_insert(0usize) += 1;
    }
    let mut schemas = BTreeMap::<_, Vec<_>>::new();
    for schema in &module.schemas {
        let Some(name) = schema.name.as_deref() else {
            continue;
        };
        schemas
            .entry((schema.module_name.as_deref(), name))
            .or_default()
            .push(schema);
    }
    module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Schema)
        .filter(|alias| {
            alias.name.as_deref().is_some_and(|name| {
                alias_counts.get(&(alias.module_name.as_deref(), name)) == Some(&1)
            })
        })
        .filter(|alias| {
            alias
                .name
                .as_deref()
                .is_some_and(|name| !schemas.contains_key(&(alias.module_name.as_deref(), name)))
        })
        .filter_map(|alias| {
            let target =
                direct_public_schema_alias_target_with_index(module, alias, &schemas, &imports)?;
            Some(ResolvedSchemaAlias {
                alias_span: alias.span.clone(),
                alias_module: alias.module_name.clone(),
                alias_name: alias.name.clone()?,
                target_span: target.span.clone(),
                target_module: target.module_name.clone(),
                target_name: target.name.clone()?,
                direct_target_module: target.module_name.clone(),
                direct_target_name: target.name.clone()?,
                direct_target_is_alias: false,
            })
        })
        .collect()
}

/// Resolves public schema aliases through other public schema aliases.
///
/// This is intentionally separate from `resolved_schema_aliases`: workspace
/// navigation keeps its direct-schema-only contract, while dependency
/// navigation opts into the bounded chain behavior explicitly.
pub fn resolved_schema_alias_chains(module: &SurfaceModule) -> Vec<ResolvedSchemaAlias> {
    let imports = SchemaAliasTargetImportIndex::new(module);
    let mut aliases = BTreeMap::new();
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Schema)
    {
        let Some(name) = alias.name.as_deref() else {
            continue;
        };
        if !name
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_uppercase())
        {
            continue;
        }
        aliases
            .entry((alias.module_name.as_deref(), name))
            .or_insert_with(Vec::new)
            .push(alias);
    }
    let mut schemas = BTreeMap::<_, Vec<_>>::new();
    for schema in &module.schemas {
        let Some(name) = schema.name.as_deref() else {
            continue;
        };
        schemas
            .entry((schema.module_name.as_deref(), name))
            .or_default()
            .push(schema);
    }
    let mut results = Vec::new();
    let mut memo = BTreeMap::new();
    for candidates in aliases.values() {
        let [alias] = candidates.as_slice() else {
            continue;
        };
        let Some((target, direct_target_module, direct_target_name, direct_target_is_alias)) =
            resolve_schema_alias_chain(module, alias, &imports, &aliases, &schemas, &mut memo)
        else {
            continue;
        };
        results.push(ResolvedSchemaAlias {
            alias_span: alias.span.clone(),
            alias_module: alias.module_name.clone(),
            alias_name: alias.name.clone().unwrap_or_default(),
            target_span: target.span.clone(),
            target_module: target.module_name.clone(),
            target_name: target.name.clone().unwrap_or_default(),
            direct_target_module,
            direct_target_name,
            direct_target_is_alias,
        });
    }
    results
}

fn resolve_schema_alias_chain<'a>(
    module: &'a SurfaceModule,
    alias: &'a PublicAlias,
    imports: &SchemaAliasTargetImportIndex<'a>,
    aliases: &BTreeMap<(Option<&'a str>, &'a str), Vec<&'a PublicAlias>>,
    schemas: &BTreeMap<(Option<&'a str>, &'a str), Vec<&'a SchemaDecl>>,
    memo: &mut BTreeMap<(Option<&'a str>, &'a str), Option<&'a SchemaDecl>>,
) -> Option<(&'a SchemaDecl, Option<String>, String, bool)> {
    alias.name.as_deref()?;
    let mut path = Vec::new();
    let mut positions = BTreeMap::new();
    let mut current = alias;
    let (target, target_module, target_name, direct_target_is_alias) = loop {
        #[cfg(test)]
        record_schema_alias_chain_resolution_visit();
        let identity = (current.module_name.as_deref(), current.name.as_deref()?);
        if let Some(result) = memo.get(&identity) {
            let Some(target) = result.as_ref().copied() else {
                for path_identity in &path {
                    memo.insert(*path_identity, None);
                }
                return None;
            };
            for path_identity in &path {
                memo.insert(*path_identity, Some(target));
            }
            let (target_module, target_name) =
                direct_schema_alias_target_identity_with_index(alias, imports)?;
            let direct_target_is_alias = aliases.contains_key(&(target_module, target_name));
            break (
                target,
                target_module.map(str::to_string),
                target_name.to_string(),
                direct_target_is_alias,
            );
        }
        if positions.insert(identity, path.len()).is_some()
            || public_alias_has_invalid_target_leaf(module, current, None)
        {
            for identity in path {
                memo.insert(identity, None);
            }
            return None;
        }
        path.push(identity);
        let Some((next_module, next_name)) =
            direct_schema_alias_target_identity_with_index(current, imports)
        else {
            memoize_schema_alias_failure(&path, memo);
            return None;
        };
        if schemas.contains_key(&(next_module, next_name))
            && aliases.contains_key(&(next_module, next_name))
        {
            for identity in &path {
                memo.insert(*identity, None);
            }
            return None;
        }
        if let Some(targets) = schemas.get(&(next_module, next_name)) {
            let [target] = targets.as_slice() else {
                for identity in &path {
                    memo.insert(*identity, None);
                }
                return None;
            };
            if target.visibility != Visibility::Public {
                for identity in &path {
                    memo.insert(*identity, None);
                }
                return None;
            }
            for identity in &path {
                memo.insert(*identity, Some(*target));
            }
            break (
                *target,
                next_module.map(str::to_string),
                next_name.to_string(),
                false,
            );
        }
        let Some(candidates) = aliases.get(&(next_module, next_name)) else {
            memoize_schema_alias_failure(&path, memo);
            return None;
        };
        let [target_alias] = candidates.as_slice() else {
            memoize_schema_alias_failure(&path, memo);
            return None;
        };
        current = target_alias;
    };
    Some((target, target_module, target_name, direct_target_is_alias))
}

fn memoize_schema_alias_failure<'a>(
    path: &[(Option<&'a str>, &'a str)],
    memo: &mut BTreeMap<(Option<&'a str>, &'a str), Option<&'a SchemaDecl>>,
) {
    for identity in path {
        memo.insert(*identity, None);
    }
}

fn direct_public_schema_alias_target_with_index<'a>(
    module: &'a SurfaceModule,
    alias: &PublicAlias,
    schemas: &BTreeMap<(Option<&'a str>, &'a str), Vec<&'a SchemaDecl>>,
    imports: &SchemaAliasTargetImportIndex<'a>,
) -> Option<&'a SchemaDecl> {
    if public_alias_has_invalid_target_leaf(module, alias, None) {
        return None;
    }
    let (target_module, target_name) =
        direct_schema_alias_target_identity_with_index(alias, imports)?;
    let mut candidates = schemas
        .get(&(target_module, target_name))?
        .iter()
        .copied()
        .filter(|target| target.visibility == Visibility::Public);
    let target = candidates.next()?;
    candidates.next().is_none().then_some(target)
}

#[derive(Clone, Copy)]
struct IndexedSchemaAliasTargetImport<'a> {
    use_decl: &'a veln_ast::UseDecl,
    valid: bool,
}

struct SchemaAliasTargetImportIndex<'a> {
    exact: BTreeMap<(Option<String>, String), Vec<IndexedSchemaAliasTargetImport<'a>>>,
    implicit: BTreeMap<(Option<String>, String), Vec<IndexedSchemaAliasTargetImport<'a>>>,
}

impl<'a> SchemaAliasTargetImportIndex<'a> {
    fn new(module: &'a SurfaceModule) -> Self {
        let invalid_name_spans = module
            .invalid_names
            .iter()
            .filter(|invalid| {
                invalid.class == veln_ast::NameClass::Module
                    && invalid.occurrence == veln_ast::NameOccurrence::PathSegment
            })
            .map(|invalid| {
                (
                    invalid.span.file.as_str(),
                    invalid.span.start.offset,
                    invalid.span.end.offset,
                )
            })
            .collect::<BTreeSet<_>>();
        let mut index = Self {
            exact: BTreeMap::new(),
            implicit: BTreeMap::new(),
        };
        for use_decl in &module.uses {
            #[cfg(test)]
            record_schema_alias_target_import_index_entry();
            let candidate = IndexedSchemaAliasTargetImport {
                use_decl,
                valid: !use_decl.name_spans.iter().any(|span| {
                    invalid_name_spans.contains(&(
                        span.file.as_str(),
                        span.start.offset,
                        span.end.offset,
                    ))
                }),
            };
            let declaring_module = use_decl.module_name.clone();
            index
                .exact
                .entry((declaring_module.clone(), use_decl.name.clone()))
                .or_default()
                .push(candidate);
            if let Some(package_relative) = standard_package_relative_import(use_decl) {
                index
                    .exact
                    .entry((declaring_module.clone(), package_relative.to_string()))
                    .or_default()
                    .push(candidate);
            }
            index
                .implicit
                .entry((declaring_module, use_decl.alias.clone()))
                .or_default()
                .push(candidate);
        }
        index
    }

    fn resolve(
        &self,
        declaring_module: Option<&str>,
        qualifier: &str,
    ) -> Option<&'a veln_ast::UseDecl> {
        #[cfg(test)]
        record_schema_alias_target_import_lookup();
        let key = (declaring_module.map(str::to_string), qualifier.to_string());
        if let Some(candidates) = self.exact.get(&key) {
            return unique_valid_local_import(candidates);
        }
        unique_valid_local_import(self.implicit.get(&key)?)
    }
}

fn standard_package_relative_import(use_decl: &veln_ast::UseDecl) -> Option<&str> {
    let package_relative = use_decl.name.strip_prefix("std::")?;
    (use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
        || (use_decl.package.is_none()
            && use_decl
                .module_name
                .as_deref()
                .is_some_and(|module| module.starts_with("std::"))))
    .then_some(package_relative)
}

fn unique_valid_local_import<'a>(
    candidates: &[IndexedSchemaAliasTargetImport<'a>],
) -> Option<&'a veln_ast::UseDecl> {
    let [candidate] = candidates else {
        return None;
    };
    (candidate.valid && candidate.use_decl.package.is_none()).then_some(candidate.use_decl)
}

fn direct_schema_alias_target_identity_with_index<'a>(
    alias: &'a PublicAlias,
    imports: &SchemaAliasTargetImportIndex<'a>,
) -> Option<(Option<&'a str>, &'a str)> {
    Some(match alias.target.as_slice() {
        [name] => (alias.module_name.as_deref(), name.as_str()),
        [qualifiers @ .., name] => {
            let qualifier = qualifiers.join("::");
            let use_decl = imports.resolve(alias.module_name.as_deref(), &qualifier)?;
            (Some(use_decl.name.as_str()), name.as_str())
        }
        [] => return None,
    })
}

fn direct_public_schema_alias_target<'a>(
    module: &'a SurfaceModule,
    alias: &PublicAlias,
) -> Option<&'a SchemaDecl> {
    if public_alias_has_invalid_target_leaf(module, alias, None) {
        return None;
    }
    let (target_module, target_name) = direct_schema_alias_target_identity(module, alias)?;
    let mut candidates = module.schemas.iter().filter(|schema| {
        schema.name.as_deref() == Some(target_name)
            && schema.module_name.as_deref() == target_module
            && schema.visibility == Visibility::Public
    });
    let target = candidates.next()?;
    candidates.next().is_none().then_some(target)
}

fn direct_schema_alias_target_identity<'a>(
    module: &'a SurfaceModule,
    alias: &'a PublicAlias,
) -> Option<(Option<&'a str>, &'a str)> {
    Some(match alias.target.as_slice() {
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
    })
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
