use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::adt::registry::AdtRegistry;
use veln_ast::{NameClass, PublicAliasKind, SchemaField, SurfaceModule, UseDecl, Visibility};
use veln_source::SourceSpan;

use super::schema_types::format_neutral_schema_first_unsupported_encode_field;
use super::signatures::FunctionSignature;
use crate::name_recovery::public_alias_has_invalid_target_leaf;

#[derive(Clone)]
pub(super) struct SchemaSymbolTable {
    schemas: Vec<SchemaSymbol>,
    aliases: Vec<SchemaAliasSymbol>,
    uses: Vec<UseDecl>,
}

#[derive(Clone)]
struct SchemaSymbol {
    name: String,
    module_name: Option<String>,
    package_name: Option<String>,
    visibility: Visibility,
    span: SourceSpan,
    unsupported_format_neutral_encode_field: Option<SchemaField>,
}

#[derive(Clone)]
struct SchemaAliasSymbol {
    name: String,
    module_name: Option<String>,
    package_name: Option<String>,
    target: Vec<String>,
}

pub(super) struct ResolvedSchemaSymbol {
    pub(super) name: String,
    pub(super) module_name: Option<String>,
    pub(super) package_name: Option<String>,
    pub(super) span: SourceSpan,
    pub(super) unsupported_format_neutral_encode_field: Option<SchemaField>,
}

pub(super) struct SchemaAliasTarget {
    pub(super) target: Vec<String>,
    pub(super) module_name: Option<String>,
    pub(super) package_name: Option<String>,
}

impl SchemaSymbolTable {
    pub(super) fn schema_path_is_ambiguous(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
    ) -> bool {
        let [_, ..] = segments else {
            return false;
        };
        let module_path = segments[..segments.len() - 1].join("::");
        crate::name_recovery::use_decl_path_is_ambiguous(
            self.uses
                .iter()
                .filter(|use_decl| use_decl.importing_package.as_deref() == current_package),
            &module_path,
            current_module,
        )
    }

    pub(super) fn extend(&mut self, other: Self) {
        self.schemas.extend(other.schemas);
        self.aliases.extend(other.aliases);
        self.uses.extend(other.uses);
    }

    pub(super) fn standard_subset(&self, module_names: &BTreeSet<String>) -> Self {
        Self {
            schemas: selected_symbols(&self.schemas, module_names, |symbol| {
                symbol.module_name.as_deref()
            }),
            aliases: selected_symbols(&self.aliases, module_names, |symbol| {
                symbol.module_name.as_deref()
            }),
            uses: selected_symbols(&self.uses, module_names, |use_decl| {
                use_decl.module_name.as_deref()
            }),
        }
    }

    pub(super) fn from_module(module: &SurfaceModule, adts: &AdtRegistry) -> Self {
        let schemas = module
            .schemas
            .iter()
            .filter_map(|schema| {
                Some(SchemaSymbol {
                    name: schema.name.clone()?,
                    module_name: schema.module_name.clone(),
                    package_name: schema.package_name.clone(),
                    visibility: schema.visibility,
                    span: schema.span.clone(),
                    unsupported_format_neutral_encode_field:
                        format_neutral_schema_first_unsupported_encode_field(module, schema, adts),
                })
            })
            .collect();
        let aliases = module
            .aliases
            .iter()
            .filter(|alias| alias.kind == PublicAliasKind::Schema)
            .filter(|alias| !public_alias_has_invalid_target_leaf(module, alias, None))
            .filter_map(|alias| {
                Some(SchemaAliasSymbol {
                    name: alias.name.clone()?,
                    module_name: alias.module_name.clone(),
                    package_name: alias.package_name.clone(),
                    target: alias.target.clone(),
                })
            })
            .collect();
        Self {
            schemas,
            aliases,
            uses: module.uses.clone(),
        }
    }

    pub(super) fn private_schema(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
    ) -> bool {
        self.schema_path(
            segments,
            current_module,
            current_package,
            uses,
            true,
            companion_access_targets,
            &mut Vec::new(),
        ) == SchemaPathLookup::Private
    }

    pub(super) fn schema_alias_target(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<SchemaAliasTarget> {
        match segments {
            [name] => self.schema_alias_target_in_module(current_module, current_package, name),
            [_, .., name] => {
                let use_decl = imported_use_for_path_in_owner(
                    uses,
                    &segments[..segments.len() - 1],
                    current_module,
                    current_package,
                )?;
                self.schema_alias_target_in_module(
                    Some(&use_decl.name),
                    use_decl.resolved_target().package.as_deref(),
                    name,
                )
            }
            _ => None,
        }
    }

    fn schema_alias_target_in_module(
        &self,
        module_name: Option<&str>,
        package_name: Option<&str>,
        name: &str,
    ) -> Option<SchemaAliasTarget> {
        let alias = self.aliases.iter().find(|alias| {
            alias.name == name
                && alias.module_name.as_deref() == module_name
                && alias.package_name.as_deref() == package_name
        })?;
        Some(SchemaAliasTarget {
            target: alias.target.clone(),
            module_name: alias.module_name.clone(),
            package_name: alias.package_name.clone(),
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn schema_target_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
        allow_private_local_schema: bool,
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, Option<String>, String)>,
    ) -> Option<ResolvedSchemaSymbol> {
        match segments {
            [name] => self.schema_target_in_module(
                current_module,
                current_package,
                name,
                allow_private_local_schema,
                uses,
                companion_access_targets,
                visited_aliases,
            ),
            [_, .., name] => {
                let use_decl = imported_use_for_path_in_owner(
                    uses,
                    &segments[..segments.len() - 1],
                    current_module,
                    current_package,
                )?;
                self.schema_target_in_module(
                    Some(&use_decl.name),
                    use_decl.resolved_target().package.as_deref(),
                    name,
                    companion_private_schema_access_allowed(
                        use_decl,
                        current_module,
                        companion_access_targets,
                    ),
                    uses,
                    companion_access_targets,
                    visited_aliases,
                )
            }
            _ => None,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn schema_target_in_module(
        &self,
        module_name: Option<&str>,
        package_name: Option<&str>,
        name: &str,
        allow_private_schema: bool,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, Option<String>, String)>,
    ) -> Option<ResolvedSchemaSymbol> {
        if let Some(schema) = self.schemas.iter().find(|schema| {
            schema.name == name
                && schema.module_name.as_deref() == module_name
                && schema.package_name.as_deref() == package_name
        }) {
            return (allow_private_schema || schema.visibility == Visibility::Public).then(|| {
                ResolvedSchemaSymbol {
                    name: schema.name.clone(),
                    module_name: schema.module_name.clone(),
                    package_name: schema.package_name.clone(),
                    span: schema.span.clone(),
                    unsupported_format_neutral_encode_field: schema
                        .unsupported_format_neutral_encode_field
                        .clone(),
                }
            });
        }
        let alias = self.aliases.iter().find(|alias| {
            alias.name == name
                && alias.module_name.as_deref() == module_name
                && alias.package_name.as_deref() == package_name
        })?;
        let key = (
            alias.package_name.clone(),
            alias.module_name.clone(),
            alias.name.clone(),
        );
        if visited_aliases.contains(&key) {
            return None;
        }
        visited_aliases.push(key);
        let result = self.schema_target_path(
            &alias.target,
            alias.module_name.as_deref(),
            alias.package_name.as_deref(),
            uses,
            false,
            companion_access_targets,
            visited_aliases,
        );
        visited_aliases.pop();
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn schema_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        current_package: Option<&str>,
        uses: &[UseDecl],
        allow_private_local_schema: bool,
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, Option<String>, String)>,
    ) -> SchemaPathLookup {
        match segments {
            [name] => self.schema_in_module(
                current_module,
                current_package,
                name,
                allow_private_local_schema,
                uses,
                companion_access_targets,
                visited_aliases,
            ),
            [_, .., name] => {
                let Some(use_decl) = imported_use_for_path_in_owner(
                    uses,
                    &segments[..segments.len() - 1],
                    current_module,
                    current_package,
                ) else {
                    return SchemaPathLookup::Missing;
                };
                self.schema_in_module(
                    Some(&use_decl.name),
                    use_decl.resolved_target().package.as_deref(),
                    name,
                    companion_private_schema_access_allowed(
                        use_decl,
                        current_module,
                        companion_access_targets,
                    ),
                    uses,
                    companion_access_targets,
                    visited_aliases,
                )
            }
            _ => SchemaPathLookup::Missing,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn schema_in_module(
        &self,
        module_name: Option<&str>,
        package_name: Option<&str>,
        name: &str,
        allow_private_schema: bool,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, Option<String>, String)>,
    ) -> SchemaPathLookup {
        if let Some(schema) = self.schemas.iter().find(|schema| {
            schema.name == name
                && schema.module_name.as_deref() == module_name
                && schema.package_name.as_deref() == package_name
        }) {
            return if allow_private_schema || schema.visibility == Visibility::Public {
                SchemaPathLookup::Visible
            } else {
                SchemaPathLookup::Private
            };
        }
        let Some(alias) = self.aliases.iter().find(|alias| {
            alias.name == name
                && alias.module_name.as_deref() == module_name
                && alias.package_name.as_deref() == package_name
        }) else {
            return SchemaPathLookup::Missing;
        };
        let key = (
            alias.package_name.clone(),
            alias.module_name.clone(),
            alias.name.clone(),
        );
        if visited_aliases.contains(&key) {
            return SchemaPathLookup::Missing;
        }
        visited_aliases.push(key);
        let result = self.schema_path(
            &alias.target,
            alias.module_name.as_deref(),
            alias.package_name.as_deref(),
            uses,
            false,
            companion_access_targets,
            visited_aliases,
        );
        visited_aliases.pop();
        result
    }
}

#[derive(Clone)]
pub(super) struct NamedSymbol {
    pub(super) name: String,
    pub(super) package_name: Option<String>,
    pub(super) module_name: Option<String>,
    visibility: Visibility,
}

pub(super) trait SymbolVisibility {
    fn visibility(&self) -> Visibility;
}

impl SymbolVisibility for FunctionSignature {
    fn visibility(&self) -> Visibility {
        self.visibility
    }
}

impl SymbolVisibility for NamedSymbol {
    fn visibility(&self) -> Visibility {
        self.visibility
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SchemaPathLookup {
    Visible,
    Private,
    Missing,
}

pub(super) fn named_type_symbols(module: &SurfaceModule) -> Vec<NamedSymbol> {
    let mut symbols = module
        .types
        .iter()
        .filter_map(|ty| {
            let name = ty.name.clone()?;
            if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
                return None;
            }
            Some(NamedSymbol {
                name,
                module_name: ty.module_name.clone(),
                package_name: ty.package_name.clone(),
                visibility: ty.visibility,
            })
        })
        .collect::<Vec<_>>();
    symbols.extend(
        module
            .aliases
            .iter()
            .filter(|alias| alias.kind == PublicAliasKind::Type)
            .filter(|alias| {
                !public_alias_has_invalid_target_leaf(module, alias, Some(NameClass::Type))
            })
            .filter_map(|alias| {
                let name = alias.name.clone()?;
                if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
                    return None;
                }
                Some(NamedSymbol {
                    name,
                    module_name: alias.module_name.clone(),
                    package_name: alias.package_name.clone(),
                    visibility: Visibility::Public,
                })
            }),
    );
    symbols
}

fn selected_symbols<T: Clone>(
    symbols: &[T],
    selected_modules: &BTreeSet<String>,
    module_name: impl for<'a> Fn(&'a T) -> Option<&'a str>,
) -> Vec<T> {
    symbols
        .iter()
        .filter(|symbol| {
            module_name(symbol).is_none_or(|module_name| selected_modules.contains(module_name))
        })
        .cloned()
        .collect()
}

pub(super) fn imported_use_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let mut packages = uses
        .iter()
        .filter(|use_decl| use_decl.module_name.as_deref() == current_module)
        .map(|use_decl| use_decl.importing_package.as_deref())
        .collect::<HashSet<_>>();
    let current_package = if packages.remove(&None) {
        None
    } else if packages.len() == 1 {
        packages.into_iter().next().flatten()
    } else {
        return None;
    };
    imported_use_for_path_in_owner(uses, segments, current_module, current_package)
}

pub(super) fn imported_use_for_path_in_owner<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
    current_package: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    crate::name_recovery::select_use_decl_for_path(
        uses.iter()
            .filter(|use_decl| use_decl.importing_package.as_deref() == current_package),
        &module_path,
        current_module,
    )
}

pub(super) fn companion_private_schema_access_allowed(
    use_decl: &UseDecl,
    current_module: Option<&str>,
    companion_access_targets: &BTreeMap<String, String>,
) -> bool {
    use_decl.package.is_none()
        && current_module.is_some_and(|current_module| {
            companion_access_targets
                .get(current_module)
                .is_some_and(|allowed| allowed == use_decl.name.as_str())
        })
}
