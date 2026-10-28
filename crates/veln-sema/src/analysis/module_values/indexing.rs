use std::collections::{BTreeSet, HashMap};

use veln_ast::{NameClass, NameOccurrence, PublicAliasKind, SurfaceModule};

use crate::types::{ModuleValueIdentity, ModuleValueIndex};

pub(super) fn initially_invalid_values(
    module: &SurfaceModule,
    colliding_identities: &BTreeSet<ModuleValueIdentity>,
) -> Vec<bool> {
    module
        .module_values
        .iter()
        .map(|value| {
            value.name.as_ref().is_some_and(|name| {
                colliding_identities.contains(&(
                    value.package_name.clone(),
                    value.module_name.clone(),
                    name.clone(),
                ))
            })
        })
        .collect()
}

pub(super) fn module_value_indices(
    module: &SurfaceModule,
    colliding_identities: &BTreeSet<ModuleValueIdentity>,
) -> HashMap<ModuleValueIdentity, ModuleValueIndex> {
    let mut indices = HashMap::new();
    for (index, value) in module.module_values.iter().enumerate() {
        if let Some(name) = &value.name {
            let identity = (
                value.package_name.clone(),
                value.module_name.clone(),
                name.clone(),
            );
            let indexed = if colliding_identities.contains(&identity) {
                ModuleValueIndex::Collision
            } else {
                ModuleValueIndex::Unique(index)
            };
            indices.entry(identity).or_insert(indexed);
        }
    }
    indices
}

pub(super) fn invalid_cased_module_value_identities(
    module: &SurfaceModule,
) -> BTreeSet<ModuleValueIdentity> {
    let invalid_declaration_spans = module
        .invalid_names
        .iter()
        .filter(|invalid| {
            invalid.class == NameClass::ValueBinding
                && invalid.occurrence == NameOccurrence::Declaration
        })
        .map(|invalid| {
            (
                invalid.span.file.clone(),
                invalid.span.start.offset,
                invalid.span.end.offset,
            )
        })
        .collect::<BTreeSet<_>>();
    module
        .module_values
        .iter()
        .filter_map(|value| {
            let name = value.name.as_ref()?;
            let name_span = value.name_span.as_ref()?;
            invalid_declaration_spans
                .contains(&(
                    name_span.file.clone(),
                    name_span.start.offset,
                    name_span.end.offset,
                ))
                .then(|| {
                    (
                        value.package_name.clone(),
                        value.module_name.clone(),
                        name.clone(),
                    )
                })
        })
        .collect()
}

pub(super) fn colliding_module_value_identities(
    module: &SurfaceModule,
) -> BTreeSet<ModuleValueIdentity> {
    let mut counts = HashMap::<ModuleValueIdentity, usize>::new();
    for function in &module.functions {
        if let Some(name) = &function.name {
            *counts
                .entry((
                    function.package_name.clone(),
                    function.module_name.clone(),
                    name.clone(),
                ))
                .or_default() += 1;
        }
    }
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
    {
        if let Some(name) = &alias.name {
            *counts
                .entry((
                    alias.package_name.clone(),
                    alias.module_name.clone(),
                    name.clone(),
                ))
                .or_default() += 1;
        }
    }
    for value in &module.module_values {
        if let Some(name) = &value.name {
            *counts
                .entry((
                    value.package_name.clone(),
                    value.module_name.clone(),
                    name.clone(),
                ))
                .or_default() += 1;
        }
    }
    counts
        .into_iter()
        .filter_map(|(identity, count)| (count > 1).then_some(identity))
        .collect()
}
