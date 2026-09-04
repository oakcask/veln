use crate::name_recovery::public_alias_has_invalid_target_leaf;
use std::collections::BTreeMap;
use veln_ast::{FunctionKind, PublicAliasKind, SurfaceModule, Visibility};
use veln_diagnostics::Diagnostic;
use veln_source::SourceSpan;

use super::module_boundaries::{
    alias_kind_mismatch_diagnostic, duplicate_name_diagnostic, function_target,
    normal_imported_use_for_path, private_alias_diagnostic, type_target,
    unresolved_alias_diagnostic,
};

type SeenNames = BTreeMap<(Option<String>, String), (String, SourceSpan)>;

fn record_name(
    seen: &mut SeenNames,
    diagnostics: &mut Vec<Diagnostic>,
    module_name: Option<&str>,
    name: &str,
    kind: (&'static str, &'static str),
    node_id: String,
    span: &SourceSpan,
) {
    let (namespace, subject) = kind;
    let key = (module_name.map(str::to_owned), name.to_string());
    if let Some((first_node_id, first_span)) = seen.get(&key) {
        diagnostics.push(duplicate_name_diagnostic(
            name,
            namespace,
            subject,
            node_id,
            span.clone(),
            first_node_id.clone(),
            first_span,
        ));
    } else {
        seen.insert(key, (node_id, span.clone()));
    }
}

pub(crate) fn check_duplicate_function_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = SeenNames::new();

    for function in &module.functions {
        let Some(name) = &function.name else {
            continue;
        };
        record_name(
            &mut seen,
            &mut diagnostics,
            function.module_name.as_deref(),
            name,
            ("function", "function declaration"),
            function.node_id.display(function.kind.node_prefix()),
            &function.span,
        );
    }
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
    {
        let Some(name) = &alias.name else {
            continue;
        };
        record_name(
            &mut seen,
            &mut diagnostics,
            alias.module_name.as_deref(),
            name,
            ("function", "function alias"),
            alias.node_id.display("alias"),
            &alias.span,
        );
    }

    diagnostics
}

pub(crate) fn check_duplicate_type_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = SeenNames::new();

    for type_decl in &module.types {
        let Some(name) = &type_decl.name else {
            continue;
        };
        record_name(
            &mut seen,
            &mut diagnostics,
            type_decl.module_name.as_deref(),
            name,
            ("type", "type declaration"),
            type_decl.node_id.display("type"),
            &type_decl.span,
        );
    }
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Type)
    {
        let Some(name) = &alias.name else {
            continue;
        };
        record_name(
            &mut seen,
            &mut diagnostics,
            alias.module_name.as_deref(),
            name,
            ("type", "type alias"),
            alias.node_id.display("alias"),
            &alias.span,
        );
    }

    diagnostics
}

pub(crate) fn check_duplicate_effect_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = SeenNames::new();

    for effect in &module.effects {
        let Some(name) = &effect.name else {
            continue;
        };
        record_name(
            &mut seen,
            &mut diagnostics,
            effect.module_name.as_deref(),
            name,
            ("effect", "effect declaration"),
            effect.node_id.display("effect"),
            &effect.span,
        );

        let mut operations = SeenNames::new();
        for operation in &effect.operations {
            let Some(operation_name) = &operation.name else {
                continue;
            };
            record_name(
                &mut operations,
                &mut diagnostics,
                None,
                operation_name,
                ("operation", "effect operation declaration"),
                operation.node_id.display("operation"),
                &operation.name_span,
            );
        }
    }

    diagnostics
}

pub(crate) fn check_duplicate_schema_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = SeenNames::new();

    for schema in &module.schemas {
        let Some(name) = &schema.name else {
            continue;
        };
        record_name(
            &mut seen,
            &mut diagnostics,
            schema.module_name.as_deref(),
            name,
            ("schema", "schema declaration"),
            schema.node_id.display("schema"),
            &schema.span,
        );
    }
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Schema)
    {
        let Some(name) = &alias.name else {
            continue;
        };
        record_name(
            &mut seen,
            &mut diagnostics,
            alias.module_name.as_deref(),
            name,
            ("schema", "schema alias"),
            alias.node_id.display("alias"),
            &alias.span,
        );
    }

    diagnostics
}

#[derive(Clone, Copy, Debug)]
pub(super) enum SchemaAliasCheckResolution {
    Resolved,
    Private,
    WrongKind(&'static str),
    Cyclic,
    Unresolved,
}

pub(super) fn codec_schema_wrong_kind(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
) -> Option<&'static str> {
    if module.functions.iter().any(|function| {
        function.kind == FunctionKind::Function
            && function.name.as_deref() == Some(name)
            && function.module_name.as_deref() == module_name
    }) {
        return Some("function");
    }
    if module.types.iter().any(|type_decl| {
        type_decl.name.as_deref() == Some(name) && type_decl.module_name.as_deref() == module_name
    }) {
        return Some("type");
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
            && !public_alias_has_invalid_target_leaf(module, alias, None)
    }) {
        return match alias.kind {
            PublicAliasKind::Function => Some("function"),
            PublicAliasKind::Type => Some("type"),
            PublicAliasKind::Schema => None,
        };
    }
    None
}

pub(crate) fn check_public_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut schema_alias_cache = BTreeMap::new();
    for alias in &module.aliases {
        if alias.name.is_none() {
            continue;
        }
        match alias.kind {
            PublicAliasKind::Function => diagnostics.extend(check_function_alias_target(
                module,
                alias,
                alias.module_name.as_deref(),
            )),
            PublicAliasKind::Type => diagnostics.extend(check_type_alias_target(
                module,
                alias,
                alias.module_name.as_deref(),
            )),
            PublicAliasKind::Schema => {
                match resolve_schema_alias_check_reference(
                    module,
                    &alias.target,
                    alias.module_name.as_deref(),
                    false,
                    &mut Vec::new(),
                    &mut schema_alias_cache,
                ) {
                    SchemaAliasCheckResolution::Resolved => {}
                    SchemaAliasCheckResolution::Private => {
                        diagnostics.push(private_alias_diagnostic(alias));
                    }
                    SchemaAliasCheckResolution::WrongKind(actual_kind) => {
                        diagnostics.push(alias_kind_mismatch_diagnostic(
                            alias,
                            "schema",
                            actual_kind,
                        ));
                    }
                    SchemaAliasCheckResolution::Cyclic => {
                        diagnostics.push(unresolved_alias_diagnostic(alias, "schema"));
                    }
                    SchemaAliasCheckResolution::Unresolved => {
                        diagnostics.push(unresolved_alias_diagnostic(alias, "schema"));
                    }
                }
            }
        }
    }
    diagnostics
}

pub(super) fn check_function_alias_target(
    module: &SurfaceModule,
    alias: &veln_ast::PublicAlias,
    module_name: Option<&str>,
) -> Option<Diagnostic> {
    if function_target(module, &alias.target, module_name).is_some() {
        None
    } else if type_target(module, &alias.target, module_name).is_some() {
        Some(alias_kind_mismatch_diagnostic(alias, "function", "type"))
    } else {
        Some(unresolved_alias_diagnostic(alias, "function"))
    }
}

pub(super) fn check_type_alias_target(
    module: &SurfaceModule,
    alias: &veln_ast::PublicAlias,
    module_name: Option<&str>,
) -> Option<Diagnostic> {
    if type_target(module, &alias.target, module_name).is_some() {
        None
    } else if function_target(module, &alias.target, module_name).is_some() {
        Some(alias_kind_mismatch_diagnostic(alias, "type", "function"))
    } else {
        Some(unresolved_alias_diagnostic(alias, "type"))
    }
}

pub(super) fn resolve_schema_alias_check_reference(
    module: &SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
    cache: &mut BTreeMap<(Option<String>, String), SchemaAliasCheckResolution>,
) -> SchemaAliasCheckResolution {
    match segments {
        [name] => resolve_schema_alias_check_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
            cache,
        ),
        [_, .., name] => {
            let Some(use_decl) = normal_imported_use_for_path(
                module,
                &segments[..segments.len() - 1],
                current_module,
            ) else {
                return SchemaAliasCheckResolution::Unresolved;
            };
            resolve_schema_alias_check_in_module(
                module,
                Some(&use_decl.name),
                name,
                false,
                visited_aliases,
                cache,
            )
        }
        _ => SchemaAliasCheckResolution::Unresolved,
    }
}

pub(super) fn resolve_schema_alias_check_in_module(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
    cache: &mut BTreeMap<(Option<String>, String), SchemaAliasCheckResolution>,
) -> SchemaAliasCheckResolution {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return if allow_private_schema || schema.visibility == Visibility::Public {
            SchemaAliasCheckResolution::Resolved
        } else {
            SchemaAliasCheckResolution::Private
        };
    }

    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    }) {
        return resolve_schema_alias_check_target(module, alias, visited_aliases, cache);
    }

    codec_schema_wrong_kind(module, module_name, name).map_or(
        SchemaAliasCheckResolution::Unresolved,
        SchemaAliasCheckResolution::WrongKind,
    )
}

pub(super) fn resolve_schema_alias_check_target(
    module: &SurfaceModule,
    alias: &veln_ast::PublicAlias,
    visited_aliases: &mut Vec<(Option<String>, String)>,
    cache: &mut BTreeMap<(Option<String>, String), SchemaAliasCheckResolution>,
) -> SchemaAliasCheckResolution {
    let Some(name) = &alias.name else {
        return SchemaAliasCheckResolution::Unresolved;
    };
    let key = (alias.module_name.clone(), name.clone());
    if let Some(resolution) = cache.get(&key) {
        return *resolution;
    }
    if visited_aliases.contains(&key) {
        return SchemaAliasCheckResolution::Cyclic;
    }

    visited_aliases.push(key.clone());
    let resolution = resolve_schema_alias_check_reference(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
        cache,
    );
    visited_aliases.pop();
    cache.insert(key, resolution);
    resolution
}
