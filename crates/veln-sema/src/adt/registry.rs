use std::collections::{BTreeMap, BTreeSet, HashMap};

use veln_ast::{PublicAlias, PublicAliasKind, SurfaceModule, UseDecl, UseOrigin, Visibility};
use veln_core::CoreType;

use crate::name_recovery::{
    normal_use_decls, public_alias_has_invalid_target_leaf, use_decl_matches_import_path,
};
use crate::semantic_model::Type;
use crate::source_less_names::InvalidStandardSymbolCase;

use super::descriptors::{AdtConstructor, AdtDescriptor, AdtVariantDescriptor};
use super::lookup_validation::{
    companion_access_targets, constructor_matches_visible_path, imported_module_path_matches,
    same_descriptor, source_descriptor, validate_adt_lookup_descriptors,
};

#[derive(Clone, Debug)]
pub(crate) struct AdtRegistry {
    descriptors: Vec<AdtDescriptor>,
    descriptors_by_type_name: HashMap<String, Vec<usize>>,
    descriptors_by_identity: HashMap<String, Vec<usize>>,
    variants_by_name: HashMap<String, Vec<(usize, usize)>>,
    companion_access_targets: BTreeMap<String, String>,
    annotation_types: BTreeMap<(Option<String>, String), Type>,
    type_alias_identities: BTreeSet<(Option<String>, String)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConstructorLookup<'a> {
    Found(AdtConstructor<'a>),
    Ambiguous,
    Missing,
}

impl AdtRegistry {
    pub(super) fn from_parts(
        descriptors: Vec<AdtDescriptor>,
        companion_access_targets: BTreeMap<String, String>,
    ) -> Self {
        let annotation_types = descriptors
            .iter()
            .filter_map(|descriptor| {
                if descriptor.module_name.as_deref() != Some("std::prelude") {
                    return None;
                }
                let ty = match descriptor.type_name.as_str() {
                    "WallTime" => Type::wall_time(),
                    "SourceLocation" => Type::source_location(),
                    _ => return None,
                };
                Some((descriptor_identity(descriptor), ty))
            })
            .collect();
        Self::from_parts_with_annotation_types(
            descriptors,
            companion_access_targets,
            annotation_types,
        )
    }

    fn from_parts_with_annotation_types(
        descriptors: Vec<AdtDescriptor>,
        companion_access_targets: BTreeMap<String, String>,
        annotation_types: BTreeMap<(Option<String>, String), Type>,
    ) -> Self {
        let mut descriptors_by_type_name = HashMap::<String, Vec<usize>>::new();
        let mut descriptors_by_identity = HashMap::<String, Vec<usize>>::new();
        let mut variants_by_name = HashMap::<String, Vec<(usize, usize)>>::new();
        for (descriptor_index, descriptor) in descriptors.iter().enumerate() {
            descriptors_by_type_name
                .entry(descriptor.type_name.clone())
                .or_default()
                .push(descriptor_index);
            descriptors_by_identity
                .entry(descriptor.identity())
                .or_default()
                .push(descriptor_index);
            for (variant_index, variant) in descriptor.variants.iter().enumerate() {
                variants_by_name
                    .entry(variant.name.clone())
                    .or_default()
                    .push((descriptor_index, variant_index));
            }
        }
        for indices in descriptors_by_identity.values_mut() {
            indices.sort_by_key(|index| descriptors[*index].nominal_identity.is_some());
        }
        Self {
            descriptors,
            descriptors_by_type_name,
            descriptors_by_identity,
            variants_by_name,
            companion_access_targets,
            annotation_types,
            type_alias_identities: BTreeSet::new(),
        }
    }

    fn descriptors_named(&self, name: &str) -> impl DoubleEndedIterator<Item = &AdtDescriptor> {
        self.descriptors_by_type_name
            .get(name)
            .into_iter()
            .flatten()
            .map(|index| &self.descriptors[*index])
    }

    fn variants_named(
        &self,
        name: &str,
    ) -> impl Iterator<Item = (&AdtDescriptor, &AdtVariantDescriptor)> {
        self.variants_by_name.get(name).into_iter().flatten().map(
            |(descriptor_index, variant_index)| {
                let descriptor = &self.descriptors[*descriptor_index];
                (descriptor, &descriptor.variants[*variant_index])
            },
        )
    }

    #[cfg(test)]
    pub(crate) fn from_validated_parts_for_test(
        descriptors: Vec<AdtDescriptor>,
        companion_access_targets: BTreeMap<String, String>,
    ) -> Result<Self, InvalidStandardSymbolCase> {
        validate_adt_lookup_descriptors("adt", &descriptors)?;
        Ok(Self::from_parts(descriptors, companion_access_targets))
    }

    pub(crate) fn from_validated_source_less_descriptors(
        descriptors: Vec<AdtDescriptor>,
    ) -> Result<Self, InvalidStandardSymbolCase> {
        validate_adt_lookup_descriptors("adt", &descriptors)?;
        Ok(Self::from_parts(descriptors, Default::default()))
    }

    pub(crate) fn from_module_with_base(module: &SurfaceModule, base: &Self) -> Self {
        let mut descriptors = base.descriptors.clone();
        let source_descriptors = module
            .types
            .iter()
            .filter_map(source_descriptor)
            .collect::<Vec<_>>();
        remove_replaced_standard_descriptors(module, &source_descriptors, &mut descriptors);

        let mut annotation_types = base.annotation_types.clone();
        extend_source_annotation_types(&source_descriptors, &mut annotation_types);

        let mut alias_targets = descriptors.clone();
        alias_targets.extend(source_descriptors.clone());
        let aliases = type_alias_descriptors(module, &alias_targets);
        let mut type_alias_identities = base.type_alias_identities.clone();
        type_alias_identities.extend(aliases.iter().map(descriptor_identity));
        extend_alias_annotation_types(module, &alias_targets, &mut annotation_types);

        descriptors.extend(aliases);
        let source_descriptor_start = descriptors.len();
        descriptors.extend(source_descriptors);
        let mut companion_targets = base.companion_access_targets.clone();
        companion_targets.extend(companion_access_targets(module));
        let mut registry = Self::from_parts_with_annotation_types(
            descriptors,
            companion_targets,
            annotation_types,
        );
        registry.type_alias_identities = type_alias_identities;
        registry.canonicalize_source_payload_types(module, source_descriptor_start);
        registry
    }

    fn canonicalize_source_payload_types(
        &mut self,
        module: &SurfaceModule,
        source_descriptor_start: usize,
    ) {
        let lookup = self.clone();
        let uses = normal_use_decls(module);
        let no_quarantined_uses = Vec::new();
        let no_effects = Vec::new();
        let no_effect_access_targets = BTreeMap::new();
        for descriptor in &mut self.descriptors[source_descriptor_start..] {
            for variant in &mut descriptor.variants {
                for field in &mut variant.payload_fields {
                    let super::descriptors::AdtPayloadType::Concrete(ty) = &mut field.ty else {
                        continue;
                    };
                    *ty = crate::types::canonicalize_type_effects(
                        ty.clone(),
                        &uses,
                        &no_quarantined_uses,
                        descriptor.module_name.as_deref(),
                        &no_effects,
                        &lookup,
                        &no_effect_access_targets,
                    );
                }
            }
        }
    }

    pub(crate) fn descriptors(&self) -> &[AdtDescriptor] {
        &self.descriptors
    }

    pub(crate) fn annotation_type_for_descriptor(
        &self,
        descriptor: &AdtDescriptor,
    ) -> Option<&Type> {
        self.annotation_types.get(&descriptor_identity(descriptor))
    }

    pub(crate) fn standard_subset(&self, module_names: &BTreeSet<String>) -> Self {
        let descriptors = self
            .descriptors
            .iter()
            .filter(|descriptor| {
                descriptor
                    .module_name
                    .as_deref()
                    .is_none_or(|module_name| module_names.contains(module_name))
            })
            .cloned()
            .collect();
        let companion_access_targets = self
            .companion_access_targets
            .iter()
            .filter(|(module, target)| {
                module_names.contains(module.as_str()) && module_names.contains(target.as_str())
            })
            .map(|(module, target)| (module.clone(), target.clone()))
            .collect();
        let annotation_types = self
            .annotation_types
            .iter()
            .filter(|((module_name, _), _)| {
                module_name
                    .as_ref()
                    .is_none_or(|module_name| module_names.contains(module_name))
            })
            .map(|(identity, ty)| (identity.clone(), ty.clone()))
            .collect();
        let mut registry = Self::from_parts_with_annotation_types(
            descriptors,
            companion_access_targets,
            annotation_types,
        );
        registry.type_alias_identities = self
            .type_alias_identities
            .iter()
            .filter(|(module_name, _)| {
                module_name
                    .as_ref()
                    .is_none_or(|module_name| module_names.contains(module_name))
            })
            .cloned()
            .collect();
        registry
    }

    pub(crate) fn descriptor_for_type(&self, ty: &Type) -> Option<&AdtDescriptor> {
        let (identity, args) = match ty {
            Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
                (identity, args)
            }
            _ => return None,
        };
        self.descriptors_by_identity
            .get(identity)
            .into_iter()
            .flatten()
            .map(|index| {
                #[cfg(test)]
                descriptor_lookup_counters::record_candidate_scan();
                &self.descriptors[*index]
            })
            .find(|descriptor| descriptor.type_parameters.len() == args.len())
    }

    pub(crate) fn descriptor_for_type_in_module(
        &self,
        ty: &Type,
        module_name: Option<&str>,
    ) -> Option<&AdtDescriptor> {
        let (name, args) = match ty {
            Type::Named { name, args, .. } | Type::VariantRefinement { name, args, .. } => {
                (name, args)
            }
            _ => return None,
        };
        if name.contains("::") {
            return None;
        }
        self.descriptors_named(name).rev().find(|descriptor| {
            descriptor.module_name.as_deref() == module_name
                && descriptor.type_parameters.len() == args.len()
        })
    }

    pub(crate) fn descriptor_for_type_prefer_module(
        &self,
        ty: &Type,
        module_name: Option<&str>,
    ) -> Option<&AdtDescriptor> {
        self.descriptor_for_type_in_module(ty, module_name)
            .or_else(|| self.descriptor_for_type(ty))
    }

    pub(crate) fn descriptor_for_type_path(
        &self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        self.descriptor_for_type_path_with_arity(name, Some(args_len), current_module, uses)
    }

    pub(crate) fn descriptor_for_type_path_any_arity(
        &self,
        name: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        self.descriptor_for_type_path_with_arity(name, None, current_module, uses)
    }

    pub(crate) fn type_path_is_alias(
        &self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> bool {
        self.descriptor_for_type_path(name, args_len, current_module, uses)
            .is_some_and(|descriptor| {
                self.type_alias_identities
                    .contains(&descriptor_identity(descriptor))
            })
    }

    fn descriptor_for_type_path_with_arity(
        &self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        let arity_matches = |descriptor: &&AdtDescriptor| {
            args_len.is_none_or(|args_len| descriptor.type_parameters.len() == args_len)
        };
        if !name.contains("::") {
            return self
                .descriptors_named(name)
                .rev()
                .find(|descriptor| {
                    descriptor.module_name.as_deref() == current_module && arity_matches(descriptor)
                })
                .or_else(|| {
                    self.descriptors_named(name).find(|descriptor| {
                        descriptor.module_name.is_none() && arity_matches(descriptor)
                    })
                })
                .or_else(|| {
                    let mut imported = self.descriptors_named(name).filter(|descriptor| {
                        arity_matches(descriptor)
                            && descriptor.visibility == Visibility::Public
                            && descriptor.module_name.as_ref().is_some_and(|module| {
                                uses.iter().any(|use_decl| &use_decl.name == module)
                            })
                    });
                    let descriptor = imported.next()?;
                    imported.next().is_none().then_some(descriptor)
                });
        }
        let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
        let type_name = segments.last()?;
        self.descriptors_named(type_name).rev().find(|descriptor| {
            arity_matches(descriptor)
                && self.descriptor_visible(descriptor, &segments, current_module, uses, true)
        })
    }

    pub(crate) fn descriptor_for_core_type(&self, ty: &CoreType) -> Option<&AdtDescriptor> {
        let CoreType::Named { name, args } = ty else {
            return None;
        };
        self.descriptors_named(name).find(|descriptor| {
            descriptor.type_name == *name && descriptor.type_parameters.len() == args.len()
        })
    }

    pub(crate) fn constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> ConstructorLookup<'_> {
        self.lookup_constructor(segments, current_module, uses, true)
    }

    pub(crate) fn constructor_candidates(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Vec<AdtConstructor<'_>> {
        self.lookup_constructor_candidates(segments, current_module, uses, true)
    }

    pub(crate) fn nullary_constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> ConstructorLookup<'_> {
        match self.constructor(segments, current_module, uses) {
            ConstructorLookup::Found(constructor)
                if constructor.variant.payload_fields.is_empty() =>
            {
                ConstructorLookup::Found(constructor)
            }
            ConstructorLookup::Found(_) => ConstructorLookup::Missing,
            other => other,
        }
    }

    pub(crate) fn constructor_for_descriptor(
        &self,
        segments: &[String],
        descriptor: &AdtDescriptor,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<AdtConstructor<'_>> {
        match self.constructor(segments, current_module, uses) {
            ConstructorLookup::Found(constructor)
                if same_descriptor(constructor.descriptor, descriptor) =>
            {
                return Some(constructor);
            }
            ConstructorLookup::Found(_) if segments.len() == 1 => {}
            ConstructorLookup::Ambiguous => {}
            _ => return None,
        }

        let mut matches = Vec::new();
        let name = segments.last()?;
        for (candidate, variant) in self.variants_named(name) {
            if !same_descriptor(candidate, descriptor)
                || !self.descriptor_visible(candidate, segments, current_module, uses, true)
            {
                continue;
            }
            if constructor_matches_visible_path(candidate, variant, segments, uses, current_module)
                && self.variant_visible(candidate, variant, current_module, uses, segments)
            {
                matches.push(AdtConstructor {
                    descriptor: candidate,
                    variant,
                });
            }
        }
        match matches.as_slice() {
            [constructor] => Some(*constructor),
            _ => None,
        }
    }

    fn lookup_constructor(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        include_imports: bool,
    ) -> ConstructorLookup<'_> {
        let mut matches =
            self.lookup_constructor_candidates(segments, current_module, uses, include_imports);
        prefer_current_module_constructors(&mut matches, segments, current_module);
        match matches.as_slice() {
            [] => ConstructorLookup::Missing,
            [constructor] => ConstructorLookup::Found(*constructor),
            _ => ConstructorLookup::Ambiguous,
        }
    }

    fn lookup_constructor_candidates(
        &self,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        include_imports: bool,
    ) -> Vec<AdtConstructor<'_>> {
        let mut matches = Vec::new();
        let Some(name) = segments.last() else {
            return matches;
        };
        for (descriptor, variant) in self.variants_named(name) {
            #[cfg(test)]
            constructor_lookup_counters::record_candidate_scan();
            if !self.descriptor_visible(descriptor, segments, current_module, uses, include_imports)
            {
                continue;
            }
            if constructor_matches_visible_path(descriptor, variant, segments, uses, current_module)
                && self.variant_visible(descriptor, variant, current_module, uses, segments)
            {
                matches.push(AdtConstructor {
                    descriptor,
                    variant,
                });
            }
        }
        deduplicate_nominal_constructors(&mut matches);
        matches
    }

    fn descriptor_visible(
        &self,
        descriptor: &AdtDescriptor,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
        include_imports: bool,
    ) -> bool {
        if descriptor.module_name.is_none() {
            return true;
        }
        let same_module = descriptor.module_name.as_deref() == current_module;
        if same_module {
            return true;
        }
        if !include_imports {
            return false;
        }
        if descriptor.visibility != Visibility::Public {
            return self.companion_private_access_allowed(
                descriptor,
                segments,
                current_module,
                uses,
            );
        }
        let Some(first) = segments.first() else {
            return false;
        };
        if let Some(module_name) = uses
            .iter()
            .find(|use_decl| {
                use_decl.module_name.as_deref() == current_module && use_decl.alias == *first
            })
            .map(|use_decl| use_decl.name.as_str())
        {
            return descriptor.module_name.as_deref() == Some(module_name);
        }
        if imported_descriptor_path_matches(descriptor, segments, current_module, uses) {
            return true;
        }
        segments.len() <= 2
            && uses.iter().any(|use_decl| {
                use_decl.module_name.as_deref() == current_module
                    && descriptor.module_name.as_deref() == Some(use_decl.name.as_str())
            })
    }

    fn variant_visible(
        &self,
        descriptor: &AdtDescriptor,
        variant: &AdtVariantDescriptor,
        current_module: Option<&str>,
        uses: &[UseDecl],
        segments: &[String],
    ) -> bool {
        if descriptor.module_name.is_none() || descriptor.module_name.as_deref() == current_module {
            return true;
        }
        if self.companion_private_access_allowed(descriptor, segments, current_module, uses) {
            return true;
        }
        if segments.len() > 2 {
            return variant.visibility == Visibility::Public
                && descriptor.visibility == Visibility::Public;
        }
        variant.visibility == Visibility::Public
    }

    fn companion_private_access_allowed(
        &self,
        descriptor: &AdtDescriptor,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> bool {
        let Some(current_module) = current_module else {
            return false;
        };
        let Some(target_module) = descriptor.module_name.as_deref() else {
            return false;
        };
        if !self
            .companion_access_targets
            .get(current_module)
            .is_some_and(|allowed| allowed == target_module)
        {
            return false;
        }
        let Some(first) = segments.first() else {
            return false;
        };
        uses.iter().any(|use_decl| {
            use_decl.package.is_none()
                && use_decl.module_name.as_deref() == Some(current_module)
                && use_decl.alias == *first
                && use_decl.name == target_module
        })
    }
}

fn deduplicate_nominal_constructors(constructors: &mut Vec<AdtConstructor<'_>>) {
    let mut unique = Vec::<AdtConstructor<'_>>::with_capacity(constructors.len());
    for constructor in constructors.drain(..) {
        let duplicate = unique.iter_mut().find(|candidate| {
            candidate.descriptor.identity() == constructor.descriptor.identity()
                && candidate.variant.name == constructor.variant.name
        });
        match duplicate {
            Some(candidate)
                if candidate.descriptor.nominal_identity.is_some()
                    && constructor.descriptor.nominal_identity.is_none() =>
            {
                *candidate = constructor;
            }
            Some(_) => {}
            None => unique.push(constructor),
        }
    }
    *constructors = unique;
}

fn remove_replaced_standard_descriptors(
    module: &SurfaceModule,
    source_descriptors: &[AdtDescriptor],
    descriptors: &mut Vec<AdtDescriptor>,
) {
    let standard_source_types = source_descriptors
        .iter()
        .filter(|descriptor| descriptor.module_name.as_deref() == Some("std::prelude"))
        .map(|descriptor| descriptor.type_name.as_str())
        .chain(
            module
                .aliases
                .iter()
                .filter(|alias| {
                    alias.kind == PublicAliasKind::Type
                        && alias.module_name.as_deref() == Some("std::prelude")
                })
                .filter_map(|alias| alias.name.as_deref()),
        )
        .collect::<Vec<_>>();
    descriptors.retain(|descriptor| {
        matches!(descriptor.type_name.as_str(), "Option" | "Result" | "List")
            || !standard_source_types.contains(&descriptor.type_name.as_str())
    });
}

fn extend_source_annotation_types(
    source_descriptors: &[AdtDescriptor],
    annotation_types: &mut BTreeMap<(Option<String>, String), Type>,
) {
    for descriptor in source_descriptors {
        if descriptor.module_name.as_deref() == Some("std::prelude") {
            let ty = match descriptor.type_name.as_str() {
                "WallTime" => Type::wall_time(),
                "SourceLocation" => Type::source_location(),
                _ => continue,
            };
            annotation_types.insert(descriptor_identity(descriptor), ty);
        }
    }
}

fn extend_alias_annotation_types(
    module: &SurfaceModule,
    alias_targets: &[AdtDescriptor],
    annotation_types: &mut BTreeMap<(Option<String>, String), Type>,
) {
    let uses = normal_use_decls(module);
    for alias in &module.aliases {
        let Some(alias_name) = alias.name.as_ref() else {
            continue;
        };
        let Some(target) = descriptor_for_alias_target(
            &alias.target,
            &uses,
            alias_targets,
            alias.module_name.as_deref(),
        ) else {
            continue;
        };
        let Some(annotation_type) = annotation_types.get(&descriptor_identity(target)).cloned()
        else {
            continue;
        };
        annotation_types.insert(
            (alias.module_name.clone(), alias_name.clone()),
            annotation_type,
        );
    }
}

fn prefer_current_module_constructors<'a>(
    matches: &mut Vec<AdtConstructor<'a>>,
    segments: &[String],
    current_module: Option<&str>,
) {
    if segments.len() != 1 {
        return;
    }
    let Some(current_module) = current_module else {
        return;
    };
    if !matches
        .iter()
        .any(|constructor| constructor.descriptor.module_name.as_deref() == Some(current_module))
    {
        return;
    }
    matches.retain(|constructor| {
        constructor.descriptor.module_name.as_deref() == Some(current_module)
    });
}

fn imported_descriptor_path_matches(
    descriptor: &AdtDescriptor,
    segments: &[String],
    current_module: Option<&str>,
    uses: &[UseDecl],
) -> bool {
    let Some(type_index) = descriptor_type_segment_index(descriptor, segments) else {
        return false;
    };
    imported_module_path_matches(descriptor, &segments[..type_index], uses, current_module)
}

fn descriptor_type_segment_index(descriptor: &AdtDescriptor, segments: &[String]) -> Option<usize> {
    segments
        .iter()
        .enumerate()
        .rev()
        .find_map(|(index, segment)| (segment == &descriptor.type_name).then_some(index))
        .filter(|index| *index > 0)
}

fn type_alias_descriptors(
    module: &SurfaceModule,
    descriptors: &[AdtDescriptor],
) -> Vec<AdtDescriptor> {
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
        }
    }

    fn resolve(mut self) -> Vec<AdtDescriptor> {
        for start in 0..self.aliases.len() {
            if self.memo[start].is_none() {
                self.resolve_from(start);
            }
        }
        self.resolved
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

fn descriptor_for_alias_target<'a>(
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

#[cfg(test)]
pub(in crate::adt) mod type_alias_resolution_counters {
    use std::cell::Cell;

    thread_local! {
        static WORK: Cell<usize> = const { Cell::new(0) };
    }

    pub(in crate::adt) fn reset() {
        WORK.set(0);
    }

    pub(super) fn record_work() {
        WORK.set(WORK.get() + 1);
    }

    pub(in crate::adt) fn work() -> usize {
        WORK.get()
    }
}

fn descriptor_identity(descriptor: &AdtDescriptor) -> (Option<String>, String) {
    (descriptor.module_name.clone(), descriptor.type_name.clone())
}

#[cfg(test)]
pub(super) mod constructor_lookup_counters {
    use std::cell::Cell;

    thread_local! {
        static CANDIDATE_SCANS: Cell<usize> = const { Cell::new(0) };
    }

    pub(in crate::adt) fn reset() {
        CANDIDATE_SCANS.set(0);
    }

    pub(super) fn record_candidate_scan() {
        CANDIDATE_SCANS.set(CANDIDATE_SCANS.get() + 1);
    }

    pub(in crate::adt) fn candidate_scans() -> usize {
        CANDIDATE_SCANS.get()
    }
}

#[cfg(test)]
pub(super) mod descriptor_lookup_counters {
    use std::cell::Cell;

    thread_local! {
        static CANDIDATE_SCANS: Cell<usize> = const { Cell::new(0) };
    }

    pub(in crate::adt) fn reset() {
        CANDIDATE_SCANS.set(0);
    }

    pub(super) fn record_candidate_scan() {
        CANDIDATE_SCANS.set(CANDIDATE_SCANS.get() + 1);
    }

    pub(in crate::adt) fn candidate_scans() -> usize {
        CANDIDATE_SCANS.get()
    }
}
