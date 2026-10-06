use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;

use veln_ast::{PublicAliasKind, SurfaceModule};

use crate::name_recovery::normal_use_decls;
use crate::semantic_model::Type;
use crate::source_less_names::InvalidStandardSymbolCase;

use super::aliases::{descriptor_for_alias_target, type_alias_descriptors};
use super::{AdtRegistry, VariantDeclarationOrder, descriptor_identity};
use crate::adt::descriptors::AdtDescriptor;
use crate::adt::lookup_validation::{
    companion_access_targets, source_descriptor, validate_adt_lookup_descriptors,
};

struct RegistryIndexes {
    variant_declaration_orders: Vec<Arc<VariantDeclarationOrder>>,
    descriptors_by_type_name: HashMap<String, Vec<usize>>,
    descriptors_by_identity: HashMap<String, Vec<usize>>,
    variants_by_name: HashMap<String, Vec<(usize, usize)>>,
}

impl RegistryIndexes {
    fn from_descriptors(descriptors: &[AdtDescriptor]) -> Self {
        let mut indexes = Self {
            variant_declaration_orders: variant_declaration_orders(descriptors),
            descriptors_by_type_name: HashMap::new(),
            descriptors_by_identity: HashMap::new(),
            variants_by_name: HashMap::new(),
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
        self.descriptors_by_identity
            .entry(descriptor.identity())
            .or_default()
            .push(descriptor_index);
        for (variant_index, variant) in descriptor.variants.iter().enumerate() {
            self.variants_by_name
                .entry(variant.name.clone())
                .or_default()
                .push((descriptor_index, variant_index));
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
            Arc::new(VariantDeclarationOrder { names, ranks })
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
            descriptors_by_identity,
            variants_by_name,
        } = RegistryIndexes::from_descriptors(&descriptors);
        Self {
            descriptors,
            variant_declaration_orders,
            descriptors_by_type_name,
            descriptors_by_identity,
            variants_by_name,
            companion_access_targets,
            annotation_types,
            type_alias_identities: BTreeSet::new(),
            declaration_spans: HashMap::new(),
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
        registry.declaration_spans = base.declaration_spans.clone();
        registry
            .declaration_spans
            .extend(module.types.iter().filter_map(|decl| {
                let name = decl.name.as_ref()?;
                name.as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_uppercase)
                    .then(|| {
                        let identity = decl
                            .module_name
                            .as_ref()
                            .map_or_else(|| name.clone(), |module| format!("{module}::{name}"));
                        (identity, decl.span.clone())
                    })
            }));
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
        registry.declaration_spans = self
            .declaration_spans
            .iter()
            .filter(|(identity, _)| {
                registry
                    .descriptors_by_identity
                    .contains_key(identity.as_str())
            })
            .map(|(identity, span)| (identity.clone(), span.clone()))
            .collect();
        registry
    }
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
