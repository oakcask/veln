use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use veln_ast::{PublicAliasKind, SurfaceModule, UseDecl, Visibility};
use veln_source::SourceSpan;

use crate::name_recovery::normal_use_decls;
use crate::semantic_model::Type;
use crate::source_less_names::InvalidStandardSymbolCase;

use super::aliases::{descriptor_for_alias_target, type_alias_descriptors};
use super::{
    AdtRegistry, DescriptorKey, ImportedDescriptorCandidates, QualifiedImportModules,
    QualifiedRecoveryModules, TypeAliasDeclarationIdentity, VariantDeclarationOrder,
    descriptor_identity,
};
use crate::adt::descriptors::AdtDescriptor;
use crate::adt::lookup_validation::{
    companion_access_targets, source_descriptor, validate_adt_lookup_descriptors,
};

struct RegistryIndexes {
    variant_declaration_orders: Vec<Arc<VariantDeclarationOrder>>,
    descriptors_by_type_name: HashMap<String, Vec<usize>>,
    descriptors_by_source_identity: HashMap<(Option<String>, String), Vec<usize>>,
    descriptors_by_identity: HashMap<String, Vec<usize>>,
    variants_by_name: HashMap<String, Vec<(usize, usize)>>,
    variants_by_descriptor: HashMap<DescriptorKey, HashMap<String, Option<usize>>>,
}

impl RegistryIndexes {
    fn from_descriptors(descriptors: &[AdtDescriptor]) -> Self {
        let mut indexes = Self {
            variant_declaration_orders: variant_declaration_orders(descriptors),
            descriptors_by_type_name: HashMap::new(),
            descriptors_by_source_identity: HashMap::new(),
            descriptors_by_identity: HashMap::new(),
            variants_by_name: HashMap::new(),
            variants_by_descriptor: HashMap::new(),
        };
        for (descriptor_index, descriptor) in descriptors.iter().enumerate() {
            indexes.index_descriptor(descriptor_index, descriptor);
        }
        indexes.sort_identity_candidates(descriptors);
        indexes
    }

    fn index_descriptor(&mut self, descriptor_index: usize, descriptor: &AdtDescriptor) {
        self.descriptors_by_type_name
            .entry(descriptor.type_name.clone())
            .or_default()
            .push(descriptor_index);
        self.descriptors_by_source_identity
            .entry(descriptor_identity(descriptor))
            .or_default()
            .push(descriptor_index);
        self.descriptors_by_identity
            .entry(descriptor.identity())
            .or_default()
            .push(descriptor_index);
        for (variant_index, variant) in descriptor.variants.iter().enumerate() {
            self.variants_by_name
                .entry(variant.name.clone())
                .or_default()
                .push((descriptor_index, variant_index));
            self.variants_by_descriptor
                .entry(DescriptorKey::new(descriptor))
                .or_default()
                .entry(variant.name.clone())
                .and_modify(|unique_index| *unique_index = None)
                .or_insert(Some(variant_index));
        }
    }

    fn sort_identity_candidates(&mut self, descriptors: &[AdtDescriptor]) {
        for indices in self.descriptors_by_identity.values_mut() {
            indices.sort_by_key(|index| descriptors[*index].nominal_identity.is_some());
        }
    }
}

fn variant_declaration_orders(descriptors: &[AdtDescriptor]) -> Vec<Arc<VariantDeclarationOrder>> {
    descriptors
        .iter()
        .map(|descriptor| {
            let names = descriptor
                .variants
                .iter()
                .map(|variant| variant.name.clone())
                .collect::<Vec<_>>();
            let ranks = names
                .iter()
                .enumerate()
                .map(|(rank, name)| (name.clone(), rank))
                .collect();
            let coverage_cases = descriptor
                .variants
                .iter()
                .map(|variant| variant.coverage_case.clone())
                .collect();
            Arc::new(VariantDeclarationOrder {
                names,
                coverage_cases,
                ranks,
            })
        })
        .collect()
}

impl AdtRegistry {
    pub(in crate::adt) fn from_parts(
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
        let RegistryIndexes {
            variant_declaration_orders,
            descriptors_by_type_name,
            descriptors_by_source_identity,
            descriptors_by_identity,
            variants_by_name,
            variants_by_descriptor,
        } = RegistryIndexes::from_descriptors(&descriptors);
        let local_descriptor_start = descriptors.len();
        Self {
            descriptors,
            variant_declaration_orders,
            descriptors_by_type_name,
            descriptors_by_source_identity,
            descriptors_by_identity,
            variants_by_name,
            variants_by_descriptor,
            companion_access_targets,
            annotation_types,
            type_alias_identities: BTreeSet::new(),
            resolved_type_alias_declarations: BTreeSet::new(),
            declaration_spans: HashMap::new(),
            variant_declaration_spans: HashMap::new(),
            qualified_import_modules: HashMap::new(),
            qualified_import_recovery_modules: HashMap::new(),
            imported_descriptors_by_type: HashMap::new(),
            imported_descriptors_by_type_arity: HashMap::new(),
            source_import_indexes_ready: false,
            local_descriptor_start,
        }
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
        let (aliases, resolved_alias_declarations) = type_alias_descriptors(module, &alias_targets);
        let mut type_alias_identities = base.type_alias_identities.clone();
        type_alias_identities.extend(aliases.iter().map(descriptor_identity));
        let mut resolved_type_alias_declarations = base.resolved_type_alias_declarations.clone();
        resolved_type_alias_declarations.extend(resolved_alias_declarations);
        extend_alias_annotation_types(module, &alias_targets, &mut annotation_types);

        let local_descriptor_start = descriptors.len();
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
        registry.finish_source_registry(
            base,
            module,
            local_descriptor_start,
            source_descriptor_start,
            type_alias_identities,
            resolved_type_alias_declarations,
        );
        registry
    }

    fn finish_source_registry(
        &mut self,
        base: &Self,
        module: &SurfaceModule,
        local_descriptor_start: usize,
        source_descriptor_start: usize,
        type_alias_identities: BTreeSet<(Option<String>, String)>,
        resolved_type_alias_declarations: BTreeSet<TypeAliasDeclarationIdentity>,
    ) {
        self.type_alias_identities = type_alias_identities;
        self.resolved_type_alias_declarations = resolved_type_alias_declarations;
        self.declaration_spans = base.declaration_spans.clone();
        self.declaration_spans
            .extend(source_declaration_spans(module));
        self.variant_declaration_spans = base.variant_declaration_spans.clone();
        self.variant_declaration_spans
            .extend(source_variant_declaration_spans(module));
        let uses = normal_use_decls(module);
        self.qualified_import_modules = qualified_import_modules(&uses);
        self.qualified_import_recovery_modules =
            qualified_import_recovery_modules(&self.qualified_import_modules);
        let imported_descriptors = imported_descriptor_candidates(&self.descriptors, &uses);
        self.imported_descriptors_by_type = imported_descriptors.any_arity;
        self.imported_descriptors_by_type_arity = imported_descriptors.exact_arity;
        self.source_import_indexes_ready = true;
        self.local_descriptor_start = local_descriptor_start;
        self.canonicalize_source_payload_types(module, source_descriptor_start);
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
                    let crate::adt::descriptors::AdtPayloadType::Concrete(ty) = &mut field.ty
                    else {
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
        let companion_access_targets = retained_companion_access_targets(self, module_names);
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
        registry.type_alias_identities = retained_type_alias_identities(self, module_names);
        registry.resolved_type_alias_declarations = self
            .resolved_type_alias_declarations
            .iter()
            .filter(|identity| {
                identity
                    .module_name
                    .as_ref()
                    .is_none_or(|module_name| module_names.contains(module_name))
            })
            .cloned()
            .collect();
        registry.declaration_spans = retained_declaration_spans(self, &registry);
        registry.variant_declaration_spans = retained_variant_declaration_spans(self, &registry);
        registry
    }
}

fn source_declaration_spans(module: &SurfaceModule) -> HashMap<String, SourceSpan> {
    module
        .types
        .iter()
        .filter_map(|decl| {
            let name = decl.name.as_ref()?;
            name.as_bytes()
                .first()
                .is_some_and(u8::is_ascii_uppercase)
                .then(|| {
                    let identity = match decl.module_name.as_deref() {
                        None | Some("std::prelude") => name.clone(),
                        Some(module) => format!("{module}::{name}"),
                    };
                    (identity, decl.span.clone())
                })
        })
        .collect()
}

fn source_variant_declaration_spans(
    module: &SurfaceModule,
) -> HashMap<String, HashMap<String, SourceSpan>> {
    let mut spans = HashMap::<String, HashMap<String, SourceSpan>>::new();
    for declaration in &module.types {
        let Some(name) = declaration.name.as_ref() else {
            continue;
        };
        let identity = match declaration.module_name.as_deref() {
            None | Some("std::prelude") => name.clone(),
            Some(module) => format!("{module}::{name}"),
        };
        let variants = spans.entry(identity).or_default();
        for variant in &declaration.variants {
            if let Some(name) = &variant.name {
                variants.insert(name.clone(), variant.span.clone());
            }
        }
    }
    spans
}

fn retained_declaration_spans(
    source: &AdtRegistry,
    subset: &AdtRegistry,
) -> HashMap<String, SourceSpan> {
    source
        .declaration_spans
        .iter()
        .filter(|(identity, _)| {
            subset
                .descriptors_by_identity
                .contains_key(identity.as_str())
        })
        .map(|(identity, span)| (identity.clone(), span.clone()))
        .collect()
}

fn retained_variant_declaration_spans(
    source: &AdtRegistry,
    subset: &AdtRegistry,
) -> HashMap<String, HashMap<String, SourceSpan>> {
    source
        .variant_declaration_spans
        .iter()
        .filter(|(identity, _)| {
            subset
                .descriptors_by_identity
                .contains_key(identity.as_str())
        })
        .map(|(identity, spans)| (identity.clone(), spans.clone()))
        .collect()
}

fn retained_companion_access_targets(
    registry: &AdtRegistry,
    module_names: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    registry
        .companion_access_targets
        .iter()
        .filter(|(module, target)| {
            module_names.contains(module.as_str()) && module_names.contains(target.as_str())
        })
        .map(|(module, target)| (module.clone(), target.clone()))
        .collect()
}

fn retained_type_alias_identities(
    registry: &AdtRegistry,
    module_names: &BTreeSet<String>,
) -> BTreeSet<(Option<String>, String)> {
    registry
        .type_alias_identities
        .iter()
        .filter(|(module_name, _)| {
            module_name
                .as_ref()
                .is_none_or(|module_name| module_names.contains(module_name))
        })
        .cloned()
        .collect()
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

fn qualified_import_modules(
    uses: &[UseDecl],
) -> HashMap<(Option<String>, String), QualifiedImportModules> {
    let mut modules = HashMap::<(Option<String>, String), QualifiedImportModules>::new();
    for use_decl in uses {
        let mut exact_paths = vec![use_decl.name.clone()];
        if let Some(relative) = use_decl.name.strip_prefix("std::") {
            exact_paths.push(relative.to_string());
        }
        exact_paths.sort();
        exact_paths.dedup();
        for path in exact_paths {
            crate::adt::lookup_validation::record_import_path_comparison();
            modules
                .entry((use_decl.module_name.clone(), path))
                .or_default()
                .exact
                .insert(use_decl.name.clone());
        }
        if crate::name_recovery::use_decl_matches_implicit_import_alias(
            use_decl,
            &use_decl.alias,
            use_decl.module_name.as_deref(),
        ) {
            crate::adt::lookup_validation::record_import_path_comparison();
            modules
                .entry((use_decl.module_name.clone(), use_decl.alias.clone()))
                .or_default()
                .implicit
                .insert(use_decl.name.clone());
        }
    }
    modules
}

fn qualified_import_recovery_modules(
    exact: &HashMap<(Option<String>, String), QualifiedImportModules>,
) -> HashMap<(Option<String>, String), Vec<QualifiedRecoveryModules>> {
    let mut recovered = HashMap::<_, Vec<_>>::new();
    for ((owner, module_path), modules) in exact {
        recovered
            .entry((owner.clone(), normalized_qualified_path(module_path)))
            .or_default()
            .push(QualifiedRecoveryModules {
                module_path: module_path.clone(),
                modules: modules.clone(),
            });
    }
    recovered
}

fn normalized_qualified_path(path: &str) -> String {
    let mut normalized = String::with_capacity(path.len());
    for (index, segment) in path.split("::").enumerate() {
        if index > 0 {
            normalized.push_str("::");
        }
        if let Some(first) = segment.as_bytes().first().copied() {
            if first.is_ascii() {
                normalized.push((first as char).to_ascii_lowercase());
                normalized.push_str(&segment[1..]);
            } else {
                normalized.push_str(segment);
            }
        }
    }
    normalized
}

fn imported_descriptor_candidates(
    descriptors: &[AdtDescriptor],
    uses: &[UseDecl],
) -> ImportedDescriptorCandidates {
    let mut owners_by_imported_module = HashMap::<String, BTreeSet<Option<String>>>::new();
    for use_decl in uses {
        crate::adt::lookup_validation::record_import_path_comparison();
        owners_by_imported_module
            .entry(use_decl.name.clone())
            .or_default()
            .insert(use_decl.module_name.clone());
    }

    let mut any_arity = HashMap::new();
    let mut exact_arity = HashMap::new();
    for (index, descriptor) in descriptors.iter().enumerate() {
        let Some(module_name) = descriptor.module_name.as_ref() else {
            continue;
        };
        if descriptor.visibility != Visibility::Public {
            continue;
        }
        crate::adt::lookup_validation::record_import_path_comparison();
        let Some(owners) = owners_by_imported_module.get(module_name) else {
            continue;
        };
        for owner in owners {
            crate::adt::lookup_validation::record_import_path_comparison();
            record_imported_candidate(
                &mut any_arity,
                (owner.clone(), descriptor.type_name.clone()),
                index,
            );
            record_imported_candidate(
                &mut exact_arity,
                (
                    owner.clone(),
                    descriptor.type_name.clone(),
                    descriptor.type_parameters.len(),
                ),
                index,
            );
        }
    }
    ImportedDescriptorCandidates {
        any_arity,
        exact_arity,
    }
}

fn record_imported_candidate<K: std::hash::Hash + Eq>(
    candidates: &mut HashMap<K, Option<usize>>,
    key: K,
    index: usize,
) {
    crate::adt::lookup_validation::record_import_path_comparison();
    match candidates.entry(key) {
        std::collections::hash_map::Entry::Vacant(entry) => {
            entry.insert(Some(index));
        }
        std::collections::hash_map::Entry::Occupied(mut entry) => {
            entry.insert(None);
        }
    }
}
