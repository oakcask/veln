use std::collections::HashMap;

use veln_ast::{PublicAlias, PublicAliasKind, SurfaceModule, UseDecl, UseOrigin, Visibility};

use crate::name_recovery::{
    normal_use_decls, public_alias_has_invalid_target_leaf, use_decl_matches_import_path,
};

#[cfg(test)]
use super::type_alias_resolution_counters;
use super::{TypeAliasDeclarationIdentity, descriptor_identity};
use crate::adt::descriptors::AdtDescriptor;

pub(super) fn type_alias_descriptors(
    module: &SurfaceModule,
    descriptors: &[AdtDescriptor],
) -> (Vec<AdtDescriptor>, Vec<TypeAliasDeclarationIdentity>) {
    let uses = normal_use_decls(module);
    let aliases = eligible_type_aliases(module);
    let targets = TypeAliasTargetIndex::new(descriptors, &aliases, &uses);
    let steps = aliases
        .iter()
        .map(|alias| targets.target(alias))
        .collect::<Vec<_>>();
    TypeAliasResolver::new(descriptors, &aliases, &steps).resolve()
}

fn eligible_type_aliases(module: &SurfaceModule) -> Vec<&PublicAlias> {
    module
        .aliases
        .iter()
        .filter(|alias| {
            alias.kind == PublicAliasKind::Type
                && alias
                    .name
                    .as_deref()
                    .is_some_and(|name| name.as_bytes().first().is_some_and(u8::is_ascii_uppercase))
                && !public_alias_has_invalid_target_leaf(
                    module,
                    alias,
                    Some(veln_ast::NameClass::Type),
                )
        })
        .collect()
}

#[derive(Clone, Copy)]
enum TypeAliasTarget {
    Descriptor(usize),
    Alias(usize),
    Missing,
}

type TypeAliasKey = (Option<String>, String);

struct TypeAliasResolver<'a> {
    descriptors: &'a [AdtDescriptor],
    aliases: &'a [&'a PublicAlias],
    steps: &'a [TypeAliasTarget],
    memo: Vec<Option<Option<AdtDescriptor>>>,
    visiting: Vec<bool>,
    resolved: Vec<AdtDescriptor>,
    resolved_declarations: Vec<TypeAliasDeclarationIdentity>,
}

impl<'a> TypeAliasResolver<'a> {
    fn new(
        descriptors: &'a [AdtDescriptor],
        aliases: &'a [&'a PublicAlias],
        steps: &'a [TypeAliasTarget],
    ) -> Self {
        Self {
            descriptors,
            aliases,
            steps,
            memo: vec![None; aliases.len()],
            visiting: vec![false; aliases.len()],
            resolved: Vec::new(),
            resolved_declarations: Vec::new(),
        }
    }

    fn resolve(mut self) -> (Vec<AdtDescriptor>, Vec<TypeAliasDeclarationIdentity>) {
        for start in 0..self.aliases.len() {
            if self.memo[start].is_none() {
                self.resolve_from(start);
            }
        }
        (self.resolved, self.resolved_declarations)
    }

    fn resolve_from(&mut self, start: usize) {
        let (path, terminal) = self.trace_from(start);
        let mut target = terminal;
        for index in path.into_iter().rev() {
            self.visiting[index] = false;
            let descriptor = target
                .as_ref()
                .map(|target| alias_descriptor(self.aliases[index], target));
            self.memo[index] = Some(descriptor.clone());
            if let Some(descriptor) = &descriptor {
                self.resolved.push(descriptor.clone());
                self.resolved_declarations
                    .push(TypeAliasDeclarationIdentity::new(self.aliases[index]));
            }
            target = descriptor;
        }
    }

    fn trace_from(&mut self, start: usize) -> (Vec<usize>, Option<AdtDescriptor>) {
        let mut path = Vec::new();
        let mut current = start;
        loop {
            #[cfg(test)]
            type_alias_resolution_counters::record_work();
            if let Some(known) = &self.memo[current] {
                return (path, known.clone());
            }
            if self.visiting[current] {
                return (path, None);
            }
            self.visiting[current] = true;
            path.push(current);
            match self.steps[current] {
                TypeAliasTarget::Descriptor(index) => {
                    return (path, Some(self.descriptors[index].clone()));
                }
                TypeAliasTarget::Alias(index) => current = index,
                TypeAliasTarget::Missing => return (path, None),
            }
        }
    }
}

fn alias_descriptor(alias: &PublicAlias, target: &AdtDescriptor) -> AdtDescriptor {
    let mut descriptor = target.clone();
    descriptor.nominal_identity =
        (alias.module_name.as_deref() != Some("std::prelude")).then(|| target.identity());
    descriptor.type_name = alias.name.clone().expect("filtered alias has a name");
    descriptor.module_name = alias.module_name.clone();
    descriptor.visibility = Visibility::Public;
    descriptor
}

struct TypeAliasTargetIndex {
    descriptor_first: HashMap<TypeAliasKey, usize>,
    descriptor_last: HashMap<TypeAliasKey, usize>,
    prelude_public: HashMap<String, TypeAliasTarget>,
    opaque_public: HashMap<String, usize>,
    alias_first: HashMap<TypeAliasKey, usize>,
    alias_last: HashMap<TypeAliasKey, usize>,
    imports: HashMap<TypeAliasKey, String>,
}

impl TypeAliasTargetIndex {
    fn new(descriptors: &[AdtDescriptor], aliases: &[&PublicAlias], uses: &[UseDecl]) -> Self {
        let mut index = Self {
            descriptor_first: HashMap::new(),
            descriptor_last: HashMap::new(),
            prelude_public: HashMap::new(),
            opaque_public: HashMap::new(),
            alias_first: HashMap::new(),
            alias_last: HashMap::new(),
            imports: HashMap::new(),
        };
        index.index_descriptors(descriptors);
        index.index_aliases(aliases);
        index.index_imports(uses);
        index
    }

    fn index_descriptors(&mut self, descriptors: &[AdtDescriptor]) {
        for (position, descriptor) in descriptors.iter().enumerate() {
            let key = descriptor_identity(descriptor);
            self.descriptor_first.entry(key.clone()).or_insert(position);
            self.descriptor_last.insert(key, position);
            if descriptor.module_name.as_deref() == Some("std::prelude")
                && descriptor.visibility == Visibility::Public
            {
                self.prelude_public
                    .entry(descriptor.type_name.clone())
                    .or_insert(TypeAliasTarget::Descriptor(position));
            }
            if descriptor.module_name.is_none()
                && matches!(descriptor.type_name.as_str(), "NetListener" | "NetStream")
                && descriptor.visibility == Visibility::Public
            {
                self.opaque_public
                    .entry(descriptor.type_name.clone())
                    .or_insert(position);
            }
        }
    }

    fn index_aliases(&mut self, aliases: &[&PublicAlias]) {
        for (position, alias) in aliases.iter().enumerate() {
            let key = (
                alias.module_name.clone(),
                alias.name.clone().expect("filtered alias has a name"),
            );
            self.alias_first.entry(key.clone()).or_insert(position);
            self.alias_last.insert(key.clone(), position);
            if alias.module_name.as_deref() == Some("std::prelude") {
                self.prelude_public
                    .entry(key.1)
                    .or_insert(TypeAliasTarget::Alias(position));
            }
        }
    }

    fn index_imports(&mut self, uses: &[UseDecl]) {
        for use_decl in uses {
            let module = use_decl.module_name.clone();
            let target = use_decl.name.clone();
            self.imports
                .entry((module.clone(), target.clone()))
                .or_insert_with(|| target.clone());
            if is_standard_import(use_decl, module.as_deref())
                && let Some(relative) = target.strip_prefix("std::")
            {
                self.imports
                    .entry((module.clone(), relative.to_string()))
                    .or_insert_with(|| target.clone());
            }
            if use_decl.package.is_some()
                || !target.contains("::")
                || use_decl.origin == UseOrigin::ImplicitStandardPrelude
            {
                self.imports
                    .entry((module, use_decl.alias.clone()))
                    .or_insert(target);
            }
        }
    }

    fn target(&self, alias: &PublicAlias) -> TypeAliasTarget {
        #[cfg(test)]
        type_alias_resolution_counters::record_work();
        let Some(name) = alias.target.last() else {
            return TypeAliasTarget::Missing;
        };
        if alias.target.len() == 1 {
            let key = (alias.module_name.clone(), name.clone());
            if let Some(&index) = self.alias_last.get(&key) {
                return TypeAliasTarget::Alias(index);
            }
            if let Some(&index) = self.descriptor_last.get(&key) {
                return TypeAliasTarget::Descriptor(index);
            }
            if let Some(&target) = self.prelude_public.get(name) {
                return target;
            }
            if let Some(&index) = self.opaque_public.get(name) {
                return TypeAliasTarget::Descriptor(index);
            }
            return TypeAliasTarget::Missing;
        }
        let qualifier = alias.target[..alias.target.len() - 1].join("::");
        let key = (alias.module_name.clone(), qualifier);
        let Some(module) = self.imports.get(&key) else {
            return TypeAliasTarget::Missing;
        };
        let key = (Some(module.clone()), name.clone());
        if let Some(&index) = self.descriptor_first.get(&key) {
            return TypeAliasTarget::Descriptor(index);
        }
        self.alias_first
            .get(&key)
            .copied()
            .map_or(TypeAliasTarget::Missing, TypeAliasTarget::Alias)
    }
}

fn is_standard_import(use_decl: &UseDecl, module_name: Option<&str>) -> bool {
    use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
        || (use_decl.package.is_none() && module_name.is_some_and(|name| name.starts_with("std::")))
}

pub(super) fn descriptor_for_alias_target<'a>(
    segments: &[String],
    uses: &[UseDecl],
    descriptors: &'a [AdtDescriptor],
    current_module: Option<&str>,
) -> Option<&'a AdtDescriptor> {
    match segments {
        [name] => descriptors
            .iter()
            .rev()
            .find(|descriptor| {
                descriptor.type_name == *name && descriptor.module_name.as_deref() == current_module
            })
            .or_else(|| {
                descriptors.iter().find(|descriptor| {
                    descriptor.type_name == *name
                        && descriptor.module_name.as_deref() == Some("std::prelude")
                        && descriptor.visibility == Visibility::Public
                })
            })
            .or_else(|| {
                descriptors.iter().find(|descriptor| {
                    descriptor.type_name == *name
                        && descriptor.module_name.is_none()
                        && matches!(descriptor.type_name.as_str(), "NetListener" | "NetStream")
                        && descriptor.visibility == Visibility::Public
                })
            }),
        [_, .., name] => {
            let import_path = &segments[..segments.len() - 1];
            let module_path = import_path.join("::");
            let module_name = uses
                .iter()
                .find(|use_decl| {
                    use_decl_matches_import_path(use_decl, &module_path, current_module)
                })
                .map(|use_decl| use_decl.name.as_str())?;
            descriptors.iter().find(|descriptor| {
                descriptor.type_name == *name
                    && descriptor.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
}
