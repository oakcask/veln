use std::collections::{BTreeMap, BTreeSet};

use veln_ast::{PublicAlias, PublicAliasKind, SchemaDecl, SurfaceModule, Visibility};

use super::{
    ResolvedSchemaAlias, SchemaAliasTargetImportIndex,
    direct_schema_alias_target_identity_with_index,
};
use crate::name_recovery::InvalidAliasTargetIndex;

type AliasIdentity<'a> = (Option<&'a str>, &'a str);

#[cfg(test)]
thread_local! {
    static RESOLUTION_VISITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_resolution_visit() {
    RESOLUTION_VISITS.set(RESOLUTION_VISITS.get() + 1);
}

#[cfg(test)]
pub(crate) fn reset_schema_alias_chain_resolution_work() {
    RESOLUTION_VISITS.set(0);
}

#[cfg(test)]
pub(crate) fn schema_alias_chain_resolution_work() -> usize {
    RESOLUTION_VISITS.get()
}

struct SchemaAliasChainDeclarations<'a> {
    aliases: BTreeMap<AliasIdentity<'a>, Vec<&'a PublicAlias>>,
    schemas: BTreeMap<AliasIdentity<'a>, Vec<&'a SchemaDecl>>,
}

impl<'a> SchemaAliasChainDeclarations<'a> {
    fn new(module: &'a SurfaceModule) -> Self {
        Self {
            aliases: schema_alias_declarations(module),
            schemas: schema_declarations(module),
        }
    }

    fn unique_aliases(&self) -> Vec<&'a PublicAlias> {
        self.aliases
            .values()
            .filter_map(|candidates| match candidates.as_slice() {
                [alias] => Some(*alias),
                _ => None,
            })
            .collect()
    }
}

struct SchemaAliasChainResolver<'a> {
    imports: SchemaAliasTargetImportIndex<'a>,
    invalid_alias_targets: InvalidAliasTargetIndex,
    declarations: SchemaAliasChainDeclarations<'a>,
    memo: BTreeMap<AliasIdentity<'a>, Option<&'a SchemaDecl>>,
}

enum ChainStep<'a> {
    Alias(&'a PublicAlias),
    Schema(&'a SchemaDecl),
    Invalid,
}

impl<'a> SchemaAliasChainResolver<'a> {
    fn new(module: &'a SurfaceModule) -> Self {
        Self {
            imports: SchemaAliasTargetImportIndex::new(module),
            invalid_alias_targets: InvalidAliasTargetIndex::new(module),
            declarations: SchemaAliasChainDeclarations::new(module),
            memo: BTreeMap::new(),
        }
    }

    fn resolve(&mut self, alias: &'a PublicAlias) -> Option<ResolvedSchemaAlias> {
        let (direct_target_module, direct_target_name) =
            direct_schema_alias_target_identity_with_index(alias, &self.imports)?;
        let direct_target_is_alias = self
            .declarations
            .aliases
            .contains_key(&(direct_target_module, direct_target_name));
        let target = self.resolve_target(alias)?;
        Some(ResolvedSchemaAlias {
            alias_span: alias.span.clone(),
            alias_module: alias.module_name.clone(),
            alias_name: alias.name.clone()?,
            target_span: target.span.clone(),
            target_module: target.module_name.clone(),
            target_name: target.name.clone()?,
            direct_target_module: direct_target_module.map(str::to_string),
            direct_target_name: direct_target_name.to_string(),
            direct_target_is_alias,
        })
    }

    fn resolve_target(&mut self, alias: &'a PublicAlias) -> Option<&'a SchemaDecl> {
        let mut path = Vec::new();
        let mut visited = BTreeSet::new();
        let mut current = alias;
        loop {
            #[cfg(test)]
            record_resolution_visit();
            let identity = (current.module_name.as_deref(), current.name.as_deref()?);
            if let Some(&known) = self.memo.get(&identity) {
                self.complete(&path, known);
                return known;
            }
            if !visited.insert(identity) {
                self.complete(&path, None);
                return None;
            }
            path.push(identity);
            match self.successor(current) {
                ChainStep::Alias(alias) => current = alias,
                ChainStep::Schema(target) => {
                    self.complete(&path, Some(target));
                    return Some(target);
                }
                ChainStep::Invalid => {
                    self.complete(&path, None);
                    return None;
                }
            }
        }
    }

    fn successor(&self, alias: &'a PublicAlias) -> ChainStep<'a> {
        if self.invalid_alias_targets.contains(alias) {
            return ChainStep::Invalid;
        }
        let Some(identity) = direct_schema_alias_target_identity_with_index(alias, &self.imports)
        else {
            return ChainStep::Invalid;
        };
        if self.declarations.schemas.contains_key(&identity)
            && self.declarations.aliases.contains_key(&identity)
        {
            return ChainStep::Invalid;
        }
        if let Some(targets) = self.declarations.schemas.get(&identity) {
            return unique_public_schema(targets).map_or(ChainStep::Invalid, ChainStep::Schema);
        }
        self.declarations
            .aliases
            .get(&identity)
            .and_then(|candidates| match candidates.as_slice() {
                [target] => Some(ChainStep::Alias(target)),
                _ => None,
            })
            .unwrap_or(ChainStep::Invalid)
    }

    fn complete(&mut self, path: &[AliasIdentity<'a>], result: Option<&'a SchemaDecl>) {
        for &identity in path {
            self.memo.insert(identity, result);
        }
    }
}

/// Resolves public schema aliases through other public schema aliases.
///
/// This is intentionally separate from `super::resolved_schema_aliases`:
/// workspace navigation keeps its direct-schema-only contract, while
/// dependency navigation opts into the bounded chain behavior explicitly.
pub fn resolved_schema_alias_chains(module: &SurfaceModule) -> Vec<ResolvedSchemaAlias> {
    let mut resolver = SchemaAliasChainResolver::new(module);
    resolver
        .declarations
        .unique_aliases()
        .into_iter()
        .filter_map(|alias| resolver.resolve(alias))
        .collect()
}

fn schema_alias_declarations(
    module: &SurfaceModule,
) -> BTreeMap<AliasIdentity<'_>, Vec<&PublicAlias>> {
    let mut aliases = BTreeMap::new();
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Schema)
    {
        let Some(name) = alias.name.as_deref().filter(|name| valid_alias_name(name)) else {
            continue;
        };
        aliases
            .entry((alias.module_name.as_deref(), name))
            .or_insert_with(Vec::new)
            .push(alias);
    }
    aliases
}

fn valid_alias_name(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_uppercase())
}

fn schema_declarations(module: &SurfaceModule) -> BTreeMap<AliasIdentity<'_>, Vec<&SchemaDecl>> {
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
    schemas
}

fn unique_public_schema<'a>(targets: &[&'a SchemaDecl]) -> Option<&'a SchemaDecl> {
    let [target] = targets else {
        return None;
    };
    (target.visibility == Visibility::Public).then_some(*target)
}
