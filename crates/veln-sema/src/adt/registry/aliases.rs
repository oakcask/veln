use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use veln_ast::{PublicAlias, PublicAliasKind, SurfaceModule, UseDecl, Visibility};

use crate::name_recovery::{normal_use_decls, public_alias_has_invalid_target_leaf};

#[cfg(test)]
use super::type_alias_resolution_counters;
use super::{
    DescriptorKey, TypeAliasDeclarationIdentity, TypeAliasExposureDeclaration,
    TypeAliasExposureNode, TypeAliasExposureNodeId, TypeAliasExposurePathRecord,
    descriptor_identity,
};
use crate::adt::descriptors::AdtDescriptor;

pub(super) fn type_alias_descriptors(
    module: &SurfaceModule,
    descriptors: &[AdtDescriptor],
    inherited_exposure_segments: &[Arc<[TypeAliasExposureNode]>],
    inherited_exposure_paths: &std::collections::BTreeMap<
        DescriptorKey,
        TypeAliasExposurePathRecord,
    >,
) -> ResolvedTypeAliases {
    let uses = normal_use_decls(module);
    let aliases = eligible_type_aliases(module);
    let targets = TypeAliasTargetIndex::new(descriptors, &aliases, &uses);
    let steps = aliases
        .iter()
        .map(|alias| targets.target(alias))
        .collect::<Vec<_>>();
    TypeAliasResolver::new(
        descriptors,
        &aliases,
        &steps,
        inherited_exposure_segments,
        inherited_exposure_paths,
    )
    .resolve()
}

pub(super) struct ResolvedTypeAliases {
    pub(super) descriptors: Vec<AdtDescriptor>,
    pub(super) declarations: Vec<TypeAliasDeclarationIdentity>,
    pub(super) exposure_segments: Vec<Arc<[TypeAliasExposureNode]>>,
    pub(super) exposure_paths:
        std::collections::BTreeMap<DescriptorKey, TypeAliasExposurePathRecord>,
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

type TypeAliasKey = (Option<String>, Option<String>, String);

struct TypeAliasResolver<'a> {
    descriptors: &'a [AdtDescriptor],
    aliases: &'a [&'a PublicAlias],
    steps: &'a [TypeAliasTarget],
    memo: Vec<Option<Option<AdtDescriptor>>>,
    visiting: Vec<bool>,
    resolved: Vec<AdtDescriptor>,
    resolved_declarations: Vec<TypeAliasDeclarationIdentity>,
    exposure_segments: Vec<Arc<[TypeAliasExposureNode]>>,
    local_segment: usize,
    local_nodes: Vec<TypeAliasExposureNode>,
    exposure_paths: std::collections::BTreeMap<DescriptorKey, TypeAliasExposurePathRecord>,
}

impl<'a> TypeAliasResolver<'a> {
    fn new(
        descriptors: &'a [AdtDescriptor],
        aliases: &'a [&'a PublicAlias],
        steps: &'a [TypeAliasTarget],
        inherited_exposure_segments: &[Arc<[TypeAliasExposureNode]>],
        inherited_exposure_paths: &std::collections::BTreeMap<
            DescriptorKey,
            TypeAliasExposurePathRecord,
        >,
    ) -> Self {
        Self {
            descriptors,
            aliases,
            steps,
            memo: vec![None; aliases.len()],
            visiting: vec![false; aliases.len()],
            resolved: Vec::new(),
            resolved_declarations: Vec::new(),
            exposure_segments: inherited_exposure_segments.to_vec(),
            local_segment: inherited_exposure_segments.len(),
            local_nodes: Vec::new(),
            exposure_paths: inherited_exposure_paths.clone(),
        }
    }

    fn resolve(mut self) -> ResolvedTypeAliases {
        for start in 0..self.aliases.len() {
            if self.memo[start].is_none() {
                self.resolve_from(start);
            }
        }
        if !self.local_nodes.is_empty() {
            self.exposure_segments.push(self.local_nodes.into());
        }
        ResolvedTypeAliases {
            descriptors: self.resolved,
            declarations: self.resolved_declarations,
            exposure_segments: self.exposure_segments,
            exposure_paths: self.exposure_paths,
        }
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
                let (inherited, terminal) = target
                    .as_ref()
                    .map(|target| {
                        let inherited = self.exposure_paths.get(&DescriptorKey::new(target));
                        inherited.map_or_else(
                            || (None, Arc::new(target.clone())),
                            |inherited| {
                                (Some(inherited.outer_node), Arc::clone(&inherited.terminal))
                            },
                        )
                    })
                    .expect("resolved alias has a target");
                let outer_node = TypeAliasExposureNodeId {
                    segment: self.local_segment,
                    offset: self.local_nodes.len(),
                };
                self.local_nodes.push(TypeAliasExposureNode {
                    declaration: alias_exposure_declaration(self.aliases[index]),
                    inherited,
                });
                let exposure_path = TypeAliasExposurePathRecord {
                    outer_node,
                    terminal,
                };
                #[cfg(test)]
                type_alias_resolution_counters::record_provenance_node();
                self.exposure_paths
                    .insert(DescriptorKey::new(descriptor), exposure_path);
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

fn alias_exposure_declaration(alias: &PublicAlias) -> TypeAliasExposureDeclaration {
    let name = alias.name.as_deref().expect("filtered alias has a name");
    let identity = match alias.module_name.as_deref() {
        None | Some("std::prelude") => name.to_string(),
        Some(module) => format!("{module}::{name}"),
    };
    TypeAliasExposureDeclaration {
        identity,
        span: alias.span.clone(),
    }
}

fn alias_descriptor(alias: &PublicAlias, target: &AdtDescriptor) -> AdtDescriptor {
    let mut descriptor = target.clone();
    descriptor.nominal_identity =
        (alias.module_name.as_deref() != Some("std::prelude")).then(|| target.identity());
    descriptor.type_name = alias.name.clone().expect("filtered alias has a name");
    descriptor.package_name = alias.package_name.clone();
    descriptor.module_name = alias.module_name.clone();
    descriptor.visibility = Visibility::Public;
    descriptor
}

struct TypeAliasTargetIndex {
    descriptor_first: HashMap<TypeAliasKey, usize>,
    descriptor_last: HashMap<TypeAliasKey, usize>,
    prelude_public: HashMap<String, TypeAliasTarget>,
    ambiguous_prelude_public: HashSet<String>,
    opaque_public: HashMap<String, usize>,
    alias_first: HashMap<TypeAliasKey, usize>,
    alias_last: HashMap<TypeAliasKey, usize>,
    imports: HashMap<TypeAliasKey, TypeAliasImportTargets>,
}

#[derive(Default)]
struct TypeAliasImportTargets {
    exact: HashSet<(Option<String>, String)>,
    implicit: HashSet<(Option<String>, String)>,
}

impl TypeAliasTargetIndex {
    fn new(descriptors: &[AdtDescriptor], aliases: &[&PublicAlias], uses: &[UseDecl]) -> Self {
        let mut index = Self {
            descriptor_first: HashMap::new(),
            descriptor_last: HashMap::new(),
            prelude_public: HashMap::new(),
            ambiguous_prelude_public: HashSet::new(),
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
                self.index_prelude_target(
                    descriptor.type_name.clone(),
                    TypeAliasTarget::Descriptor(position),
                );
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
                alias.package_name.clone(),
                alias.module_name.clone(),
                alias.name.clone().expect("filtered alias has a name"),
            );
            self.alias_first.entry(key.clone()).or_insert(position);
            self.alias_last.insert(key.clone(), position);
            if alias.module_name.as_deref() == Some("std::prelude") {
                self.index_prelude_target(key.2, TypeAliasTarget::Alias(position));
            }
        }
    }

    fn index_imports(&mut self, uses: &[UseDecl]) {
        for use_decl in uses {
            let owner_package = use_decl.importing_package.clone();
            let module = use_decl.module_name.clone();
            let target = {
                let target = use_decl.resolved_target();
                (target.package, target.module)
            };
            for path in crate::name_recovery::use_decl_exact_paths(use_decl) {
                self.imports
                    .entry((owner_package.clone(), module.clone(), path))
                    .or_default()
                    .exact
                    .insert(target.clone());
            }
            if crate::name_recovery::use_decl_matches_implicit_import_alias(
                use_decl,
                &use_decl.alias,
                module.as_deref(),
            ) {
                self.imports
                    .entry((owner_package, module, use_decl.alias.clone()))
                    .or_default()
                    .implicit
                    .insert(target);
            }
        }
    }

    fn index_prelude_target(&mut self, name: String, target: TypeAliasTarget) {
        if self.prelude_public.insert(name.clone(), target).is_some() {
            self.ambiguous_prelude_public.insert(name);
        }
    }

    fn target(&self, alias: &PublicAlias) -> TypeAliasTarget {
        #[cfg(test)]
        type_alias_resolution_counters::record_work();
        let Some(name) = alias.target.last() else {
            return TypeAliasTarget::Missing;
        };
        if alias.target.len() == 1 {
            let key = (
                alias.package_name.clone(),
                alias.module_name.clone(),
                name.clone(),
            );
            match self.local_target(&key) {
                IndexedTarget::Unique(target) => return target,
                IndexedTarget::Ambiguous => return TypeAliasTarget::Missing,
                IndexedTarget::Absent => {}
            }
            if !self.ambiguous_prelude_public.contains(name)
                && let Some(&target) = self.prelude_public.get(name)
            {
                return target;
            }
            if let Some(&index) = self.opaque_public.get(name) {
                return TypeAliasTarget::Descriptor(index);
            }
            return TypeAliasTarget::Missing;
        }
        let qualifier = alias.target[..alias.target.len() - 1].join("::");
        let key = (
            alias.package_name.clone(),
            alias.module_name.clone(),
            qualifier,
        );
        let Some(imports) = self.imports.get(&key) else {
            return TypeAliasTarget::Missing;
        };
        let candidates = if imports.exact.is_empty() {
            &imports.implicit
        } else {
            &imports.exact
        };
        let mut candidates = candidates.iter();
        let Some((package, module)) = candidates.next() else {
            return TypeAliasTarget::Missing;
        };
        if candidates.next().is_some() {
            return TypeAliasTarget::Missing;
        }
        let key = (package.clone(), Some(module.clone()), name.clone());
        match self.local_target(&key) {
            IndexedTarget::Unique(target) => target,
            IndexedTarget::Absent | IndexedTarget::Ambiguous => TypeAliasTarget::Missing,
        }
    }

    fn local_target(&self, key: &TypeAliasKey) -> IndexedTarget {
        let alias = indexed_position(&self.alias_first, &self.alias_last, key);
        let descriptor = indexed_position(&self.descriptor_first, &self.descriptor_last, key);
        match (alias, descriptor) {
            (IndexedPosition::Unique(index), IndexedPosition::Absent) => {
                IndexedTarget::Unique(TypeAliasTarget::Alias(index))
            }
            (IndexedPosition::Absent, IndexedPosition::Unique(index)) => {
                IndexedTarget::Unique(TypeAliasTarget::Descriptor(index))
            }
            (IndexedPosition::Absent, IndexedPosition::Absent) => IndexedTarget::Absent,
            _ => IndexedTarget::Ambiguous,
        }
    }
}

enum IndexedTarget {
    Absent,
    Unique(TypeAliasTarget),
    Ambiguous,
}

enum IndexedPosition {
    Absent,
    Unique(usize),
    Ambiguous,
}

fn indexed_position(
    first: &HashMap<TypeAliasKey, usize>,
    last: &HashMap<TypeAliasKey, usize>,
    key: &TypeAliasKey,
) -> IndexedPosition {
    match (first.get(key), last.get(key)) {
        (None, None) => IndexedPosition::Absent,
        (Some(first), Some(last)) if first == last => IndexedPosition::Unique(*first),
        (Some(_), Some(_)) => IndexedPosition::Ambiguous,
        _ => unreachable!("alias target indexes are populated together"),
    }
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
                descriptor.type_name == *name
                    && descriptor.module_name.as_deref() == current_module
                    && descriptor.package_name.as_deref()
                        == uses
                            .iter()
                            .find(|use_decl| use_decl.module_name.as_deref() == current_module)
                            .and_then(|use_decl| use_decl.importing_package.as_deref())
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
            let use_decl = crate::name_recovery::select_use_decl_for_path(
                uses.iter(),
                &module_path,
                current_module,
            )?;
            let target = use_decl.resolved_target();
            descriptors.iter().find(|descriptor| {
                descriptor.type_name == *name
                    && descriptor.package_name == target.package
                    && descriptor.module_name.as_deref() == Some(target.module.as_str())
            })
        }
        _ => None,
    }
}
