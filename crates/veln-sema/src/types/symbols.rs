use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{PublicAliasKind, SchemaField, SurfaceModule, UseDecl, Visibility};
use veln_source::SourceSpan;

use super::schema_types::format_neutral_schema_first_unsupported_encode_field;
use super::signatures::FunctionSignature;

#[derive(Clone)]
pub(super) struct SchemaSymbolTable {
    schemas: Vec<SchemaSymbol>,
    aliases: Vec<SchemaAliasSymbol>,
}

#[derive(Clone)]
struct SchemaSymbol {
    name: String,
    module_name: Option<String>,
    visibility: Visibility,
    span: SourceSpan,
    unsupported_format_neutral_encode_field: Option<SchemaField>,
}

#[derive(Clone)]
struct SchemaAliasSymbol {
    name: String,
    module_name: Option<String>,
    target: Vec<String>,
}

pub(super) struct ResolvedSchemaSymbol {
    pub(super) name: String,
    pub(super) module_name: Option<String>,
    pub(super) span: SourceSpan,
    pub(super) unsupported_format_neutral_encode_field: Option<SchemaField>,
}

pub(super) struct SchemaAliasTarget {
    pub(super) target: Vec<String>,
    pub(super) module_name: Option<String>,
}

impl SchemaSymbolTable {
    pub(super) fn extend(&mut self, other: Self) {
        self.schemas.extend(other.schemas);
        self.aliases.extend(other.aliases);
    }

    pub(super) fn standard_subset(&self, module_names: &BTreeSet<String>) -> Self {
        Self {
            schemas: selected_symbols(&self.schemas, module_names, |symbol| {
                symbol.module_name.as_deref()
            }),
            aliases: selected_symbols(&self.aliases, module_names, |symbol| {
                symbol.module_name.as_deref()
            }),
        }
    }

    pub(super) fn from_module(module: &SurfaceModule) -> Self {
        let schemas = module
            .schemas
            .iter()
            .filter_map(|schema| {
                Some(SchemaSymbol {
                    name: schema.name.clone()?,
                    module_name: schema.module_name.clone(),
                    visibility: schema.visibility,
                    span: schema.span.clone(),
                    unsupported_format_neutral_encode_field:
                        format_neutral_schema_first_unsupported_encode_field(module, schema),
                })
            })
            .collect();
        let aliases = module
            .aliases
            .iter()
            .filter(|alias| alias.kind == PublicAliasKind::Schema)
            .filter_map(|alias| {
                Some(SchemaAliasSymbol {
                    name: alias.name.clone()?,
                    module_name: alias.module_name.clone(),
                    target: alias.target.clone(),
                })
            })
            .collect();
        Self { schemas, aliases }
    }

    pub(super) fn private_schema(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
    ) -> bool {
        self.schema_path(
            segments,
            current_module,
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
        uses: &[UseDecl],
    ) -> Option<SchemaAliasTarget> {
        match segments {
            [name] => self.schema_alias_target_in_module(current_module, name),
            [_, .., name] => {
                let use_decl =
                    imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
                self.schema_alias_target_in_module(Some(&use_decl.name), name)
            }
            _ => None,
        }
    }

    fn schema_alias_target_in_module(
        &self,
        module_name: Option<&str>,
        name: &str,
    ) -> Option<SchemaAliasTarget> {
        let alias = self
            .aliases
            .iter()
            .find(|alias| alias.name == name && alias.module_name.as_deref() == module_name)?;
        Some(SchemaAliasTarget {
            target: alias.target.clone(),
            module_name: alias.module_name.clone(),
        })
    }

    pub(super) fn schema_target_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        allow_private_local_schema: bool,
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> Option<ResolvedSchemaSymbol> {
        match segments {
            [name] => self.schema_target_in_module(
                current_module,
                name,
                allow_private_local_schema,
                uses,
                companion_access_targets,
                visited_aliases,
            ),
            [_, .., name] => {
                let use_decl =
                    imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)?;
                self.schema_target_in_module(
                    Some(&use_decl.name),
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

    fn schema_target_in_module(
        &self,
        module_name: Option<&str>,
        name: &str,
        allow_private_schema: bool,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> Option<ResolvedSchemaSymbol> {
        if let Some(schema) = self
            .schemas
            .iter()
            .find(|schema| schema.name == name && schema.module_name.as_deref() == module_name)
        {
            return (allow_private_schema || schema.visibility == Visibility::Public).then(|| {
                ResolvedSchemaSymbol {
                    name: schema.name.clone(),
                    module_name: schema.module_name.clone(),
                    span: schema.span.clone(),
                    unsupported_format_neutral_encode_field: schema
                        .unsupported_format_neutral_encode_field
                        .clone(),
                }
            });
        }
        let alias = self
            .aliases
            .iter()
            .find(|alias| alias.name == name && alias.module_name.as_deref() == module_name)?;
        let key = (alias.module_name.clone(), alias.name.clone());
        if visited_aliases.contains(&key) {
            return None;
        }
        visited_aliases.push(key);
        let result = self.schema_target_path(
            &alias.target,
            alias.module_name.as_deref(),
            uses,
            false,
            companion_access_targets,
            visited_aliases,
        );
        visited_aliases.pop();
        result
    }

    fn schema_path(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        allow_private_local_schema: bool,
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> SchemaPathLookup {
        match segments {
            [name] => self.schema_in_module(
                current_module,
                name,
                allow_private_local_schema,
                uses,
                companion_access_targets,
                visited_aliases,
            ),
            [_, .., name] => {
                let Some(use_decl) =
                    imported_use_for_path(uses, &segments[..segments.len() - 1], current_module)
                else {
                    return SchemaPathLookup::Missing;
                };
                self.schema_in_module(
                    Some(&use_decl.name),
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

    fn schema_in_module(
        &self,
        module_name: Option<&str>,
        name: &str,
        allow_private_schema: bool,
        uses: &[UseDecl],
        companion_access_targets: &BTreeMap<String, String>,
        visited_aliases: &mut Vec<(Option<String>, String)>,
    ) -> SchemaPathLookup {
        if let Some(schema) = self
            .schemas
            .iter()
            .find(|schema| schema.name == name && schema.module_name.as_deref() == module_name)
        {
            return if allow_private_schema || schema.visibility == Visibility::Public {
                SchemaPathLookup::Visible
            } else {
                SchemaPathLookup::Private
            };
        }
        let Some(alias) = self
            .aliases
            .iter()
            .find(|alias| alias.name == name && alias.module_name.as_deref() == module_name)
        else {
            return SchemaPathLookup::Missing;
        };
        let key = (alias.module_name.clone(), alias.name.clone());
        if visited_aliases.contains(&key) {
            return SchemaPathLookup::Missing;
        }
        visited_aliases.push(key);
        let result = self.schema_path(
            &alias.target,
            alias.module_name.as_deref(),
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
                visibility: ty.visibility,
            })
        })
        .collect::<Vec<_>>();
    symbols.extend(
        module
            .aliases
            .iter()
            .filter(|alias| alias.kind == PublicAliasKind::Type)
            .filter_map(|alias| {
                let name = alias.name.clone()?;
                if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
                    return None;
                }
                Some(NamedSymbol {
                    name,
                    module_name: alias.module_name.clone(),
                    visibility: Visibility::Public,
                })
            }),
    );
    symbols
}

pub(super) fn named_codec_symbols(module: &SurfaceModule) -> Vec<NamedSymbol> {
    module
        .codecs
        .iter()
        .filter_map(|codec| {
            Some(NamedSymbol {
                name: codec.name.clone()?,
                module_name: codec.module_name.clone(),
                visibility: codec.visibility,
            })
        })
        .collect()
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
    let module_path = segments.join("::");
    uses.iter().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
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
