use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use veln_ast::{PublicAlias, UseDecl, Visibility};
use veln_core::CoreType;
use veln_source::{SourcePath, SourceSpan};

use crate::semantic_model::Type;

use super::descriptors::{AdtConstructor, AdtDescriptor, AdtVariantDescriptor};
use super::lookup_validation::{
    constructor_matches_visible_path, imported_module_path_matches, same_descriptor,
};

mod aliases;
mod build;
mod constructors;
mod lookup_helpers;

#[derive(Clone, Debug)]
pub(crate) struct AdtRegistry {
    descriptors: Vec<AdtDescriptor>,
    variant_declaration_orders: Vec<Arc<VariantDeclarationOrder>>,
    descriptors_by_type_name: HashMap<String, Vec<usize>>,
    descriptors_by_source_identity: HashMap<(Option<String>, String), Vec<usize>>,
    descriptors_by_identity: HashMap<String, Vec<usize>>,
    variants_by_name: HashMap<String, Vec<(usize, usize)>>,
    variants_by_descriptor: HashMap<DescriptorKey, HashMap<String, usize>>,
    companion_access_targets: BTreeMap<String, String>,
    annotation_types: BTreeMap<(Option<String>, String), Type>,
    type_alias_identities: BTreeSet<(Option<String>, String)>,
    resolved_type_alias_declarations: BTreeSet<TypeAliasDeclarationIdentity>,
    declaration_spans: HashMap<String, SourceSpan>,
    variant_declaration_spans: HashMap<String, HashMap<String, SourceSpan>>,
    qualified_import_modules: HashMap<(Option<String>, String), Vec<String>>,
    local_descriptor_start: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum QualifiedSourceIdentity {
    Source((Option<String>, String)),
    Ambiguous,
    CompilerKnown(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TypeAliasDeclarationIdentity {
    module_name: Option<String>,
    file: SourcePath,
    start: usize,
    end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct DescriptorKey {
    module_name: Option<String>,
    type_name: String,
    nominal_identity: Option<String>,
    arity: usize,
}

impl DescriptorKey {
    fn new(descriptor: &AdtDescriptor) -> Self {
        Self {
            module_name: descriptor.module_name.clone(),
            type_name: descriptor.type_name.clone(),
            nominal_identity: descriptor.nominal_identity.clone(),
            arity: descriptor.type_parameters.len(),
        }
    }
}

impl TypeAliasDeclarationIdentity {
    fn new(alias: &PublicAlias) -> Self {
        let span = alias.span.resolved_or_generated();
        Self {
            module_name: alias.module_name.clone(),
            file: span.file,
            start: span.start.offset,
            end: span.end.offset,
        }
    }
}

#[derive(Debug)]
pub(crate) struct VariantDeclarationOrder {
    names: Vec<String>,
    coverage_cases: Vec<String>,
    ranks: HashMap<String, usize>,
}

impl VariantDeclarationOrder {
    pub(crate) fn len(&self) -> usize {
        self.names.len()
    }

    pub(crate) fn rank(&self, name: &str) -> Option<usize> {
        self.ranks.get(name).copied()
    }

    pub(crate) fn name(&self, rank: usize) -> Option<&str> {
        self.names.get(rank).map(String::as_str)
    }

    pub(crate) fn coverage_case(&self, rank: usize) -> Option<&str> {
        self.coverage_cases.get(rank).map(String::as_str)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConstructorLookup<'a> {
    Found(AdtConstructor<'a>),
    Ambiguous,
    Missing,
}

#[derive(Clone, Copy)]
pub(crate) enum ConstructorShape {
    Nullary,
    Payload,
}

impl ConstructorShape {
    fn matches(self, constructor: AdtConstructor<'_>) -> bool {
        match self {
            Self::Nullary => constructor.variant.payload_fields.is_empty(),
            Self::Payload => !constructor.variant.payload_fields.is_empty(),
        }
    }
}

impl AdtRegistry {
    pub(crate) fn quarantine_source_variant_payload_types(
        &mut self,
        invalid: &HashSet<(Option<String>, String, String)>,
    ) -> usize {
        let mut work = 0;
        let mut invalid_identities = HashMap::<String, HashSet<String>>::new();
        for descriptor in &self.descriptors {
            work += 1;
            for variant in &descriptor.variants {
                work += 1;
                if invalid.contains(&(
                    descriptor.module_name.clone(),
                    descriptor.type_name.clone(),
                    variant.name.clone(),
                )) {
                    invalid_identities
                        .entry(descriptor.identity())
                        .or_default()
                        .insert(variant.name.clone());
                }
            }
        }
        for descriptor in &mut self.descriptors {
            work += 1;
            let identity = descriptor.identity();
            for variant in &mut descriptor.variants {
                work += 1;
                if !invalid_identities
                    .get(&identity)
                    .is_some_and(|variants| variants.contains(&variant.name))
                {
                    continue;
                }
                for field in &mut variant.payload_fields {
                    field.ty = super::descriptors::AdtPayloadType::Concrete(Type::Unknown);
                }
            }
        }
        work
    }

    pub(crate) fn resolves_type_alias(&self, alias: &PublicAlias) -> bool {
        self.resolved_type_alias_declarations
            .contains(&TypeAliasDeclarationIdentity::new(alias))
    }

    fn descriptors_named(&self, name: &str) -> impl DoubleEndedIterator<Item = &AdtDescriptor> {
        self.descriptors_by_type_name
            .get(name)
            .into_iter()
            .flatten()
            .map(|index| self.descriptor_lookup_candidate(*index))
    }

    fn descriptor_lookup_candidate(&self, index: usize) -> &AdtDescriptor {
        #[cfg(test)]
        descriptor_lookup_counters::record_candidate_scan();
        &self.descriptors[index]
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

    pub(crate) fn declaration_span_for_type(&self, ty: &Type) -> Option<&SourceSpan> {
        let descriptor = self.descriptor_for_type(ty)?;
        self.declaration_spans.get(&descriptor.identity())
    }

    pub(crate) fn declaration_span(&self, descriptor: &AdtDescriptor) -> Option<&SourceSpan> {
        self.declaration_spans.get(&descriptor.identity())
    }

    pub(crate) fn variant_for_descriptor<'a>(
        &'a self,
        descriptor: &'a AdtDescriptor,
        name: &str,
    ) -> Option<&'a AdtVariantDescriptor> {
        let variant_index = self
            .variants_by_descriptor
            .get(&DescriptorKey::new(descriptor))?
            .get(name)?;
        descriptor.variants.get(*variant_index)
    }

    pub(crate) fn variant_declaration_span(
        &self,
        descriptor: &AdtDescriptor,
        variant: &AdtVariantDescriptor,
    ) -> Option<&SourceSpan> {
        self.variant_declaration_spans
            .get(&descriptor.identity())?
            .get(&variant.name)
    }

    pub(crate) fn variant_declaration_order_for_type(
        &self,
        ty: &Type,
    ) -> Option<Arc<VariantDeclarationOrder>> {
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
            .copied()
            .find(|index| self.descriptors[*index].type_parameters.len() == args.len())
            .map(|index| Arc::clone(&self.variant_declaration_orders[index]))
    }

    pub(crate) fn variant_declaration_order_for_descriptor(
        &self,
        descriptor: &AdtDescriptor,
    ) -> Option<Arc<VariantDeclarationOrder>> {
        self.descriptors_by_identity
            .get(&descriptor.identity())
            .into_iter()
            .flatten()
            .copied()
            .find(|index| {
                self.descriptors[*index].type_parameters.len() == descriptor.type_parameters.len()
            })
            .map(|index| Arc::clone(&self.variant_declaration_orders[index]))
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
        self.descriptors_by_source_identity
            .get(&(module_name.map(str::to_string), name.clone()))
            .into_iter()
            .flatten()
            .rev()
            .map(|index| self.descriptor_lookup_candidate(*index))
            .find(|descriptor| descriptor.type_parameters.len() == args.len())
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

    pub(crate) fn unique_descriptor_for_type_path(
        &self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        if name.contains("::") {
            let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
            let type_name = segments.last()?;
            let identity =
                self.qualified_source_identity(type_name, &segments, current_module, uses);
            return self.unique_qualified_descriptor_for_identity(
                &identity,
                Some(args_len),
                &segments,
                current_module,
                uses,
            );
        }
        let descriptor = self.descriptor_for_type_path(name, args_len, current_module, uses)?;
        let indices = self.descriptors_by_type_name.get(&descriptor.type_name)?;
        let selected_index = indices
            .iter()
            .copied()
            .find(|index| std::ptr::eq(self.descriptor_lookup_candidate(*index), descriptor))?;
        let selected_is_local = selected_index >= self.local_descriptor_start;
        let mut matches = indices.iter().copied().filter_map(|index| {
            let candidate = self.descriptor_lookup_candidate(index);
            ((index >= self.local_descriptor_start) == selected_is_local
                && candidate.type_name == descriptor.type_name
                && candidate.module_name == descriptor.module_name
                && candidate.type_parameters.len() == args_len)
                .then_some(candidate)
        });
        let unique = matches.next()?;
        matches.next().is_none().then_some(unique)
    }

    pub(crate) fn descriptor_is_type_alias(&self, descriptor: &AdtDescriptor) -> bool {
        self.type_alias_identities
            .contains(&(descriptor.module_name.clone(), descriptor.type_name.clone()))
    }

    pub(crate) fn descriptor_for_type_path_any_arity(
        &self,
        name: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        self.descriptor_for_type_path_with_arity(name, None, current_module, uses)
    }

    pub(crate) fn unique_descriptor_for_type_path_any_arity(
        &self,
        name: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        if name.contains("::") {
            self.unique_qualified_descriptor_any_arity(name, current_module, uses)
        } else {
            self.unique_unqualified_descriptor_any_arity(name, current_module, uses)
        }
    }

    fn unique_unqualified_descriptor_any_arity(
        &self,
        name: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        let indices = self
            .descriptors_by_type_name
            .get(name)
            .map(Vec::as_slice)
            .unwrap_or_default();
        let descriptors = |local| {
            indices
                .iter()
                .copied()
                .filter(move |index| (*index >= self.local_descriptor_start) == local)
                .map(|index| self.descriptor_lookup_candidate(index))
        };
        let local = descriptors(true)
            .filter(|descriptor| descriptor.module_name.as_deref() == current_module);
        let builtin = descriptors(false).filter(|descriptor| descriptor.module_name.is_none());
        let imported = indices
            .iter()
            .map(|index| self.descriptor_lookup_candidate(*index))
            .filter(|descriptor| {
                descriptor.visibility == Visibility::Public
                    && descriptor
                        .module_name
                        .as_ref()
                        .is_some_and(|module| uses.iter().any(|use_decl| &use_decl.name == module))
            });
        Self::unique_candidate(local)
            .or_else(|| Self::unique_candidate(builtin))
            .or_else(|| Self::unique_candidate(imported))
            .flatten()
    }

    fn unique_qualified_descriptor_any_arity(
        &self,
        name: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
        let type_name = segments.last()?;
        let identity = self.qualified_source_identity(type_name, &segments, current_module, uses);
        self.unique_qualified_descriptor_for_identity(
            &identity,
            None,
            &segments,
            current_module,
            uses,
        )
    }

    fn qualified_source_identity(
        &self,
        type_name: &str,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> QualifiedSourceIdentity {
        self.exact_qualified_source_identity(type_name, segments, current_module, uses)
            .unwrap_or_else(|| {
                self.recovered_qualified_source_identity(type_name, segments, current_module, uses)
                    .unwrap_or_else(|| {
                        QualifiedSourceIdentity::CompilerKnown(type_name.to_string())
                    })
            })
    }

    fn exact_qualified_source_identity(
        &self,
        type_name: &str,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<QualifiedSourceIdentity> {
        let module_path = segments[..segments.len().saturating_sub(1)].join("::");
        if current_module == Some(module_path.as_str())
            && self.source_identity_exists(current_module, type_name)
        {
            return Some(QualifiedSourceIdentity::Source((
                current_module.map(str::to_string),
                type_name.to_string(),
            )));
        }
        self.exact_imported_source_identity(type_name, &module_path, current_module, uses)
    }

    fn exact_imported_source_identity(
        &self,
        type_name: &str,
        module_path: &str,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<QualifiedSourceIdentity> {
        let indexed = self
            .qualified_import_modules
            .get(&(current_module.map(str::to_string), module_path.to_string()));
        let fallback;
        let modules = if let Some(indexed) = indexed {
            indexed
        } else if self.qualified_import_modules.is_empty() {
            fallback = uses
                .iter()
                .filter_map(|use_decl| {
                    super::lookup_validation::record_import_path_comparison();
                    (crate::name_recovery::use_decl_matches_import_path(
                        use_decl,
                        module_path,
                        current_module,
                    ) && use_decl.module_name.as_deref() == current_module)
                        .then(|| use_decl.name.clone())
                })
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            &fallback
        } else {
            return None;
        };
        let mut candidates = modules
            .iter()
            .filter(|module| self.source_identity_exists(Some(module), type_name));
        let module = candidates.next()?;
        if candidates.next().is_some() {
            return Some(QualifiedSourceIdentity::Ambiguous);
        }
        Some(QualifiedSourceIdentity::Source((
            Some(module.clone()),
            type_name.to_string(),
        )))
    }

    fn source_identity_exists(&self, module_name: Option<&str>, type_name: &str) -> bool {
        self.descriptors_by_source_identity
            .contains_key(&(module_name.map(str::to_string), type_name.to_string()))
    }

    fn recovered_qualified_source_identity(
        &self,
        type_name: &str,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<QualifiedSourceIdentity> {
        let mut recovered = segments.to_vec();
        let mut selected = None;
        for index in 0..segments.len().saturating_sub(1) {
            let Some(first) = segments[index].as_bytes().first().copied() else {
                continue;
            };
            if !first.is_ascii_uppercase() {
                continue;
            }
            recovered[index].replace_range(..1, &(first as char).to_ascii_lowercase().to_string());
            if let Some(identity) =
                self.exact_qualified_source_identity(type_name, &recovered, current_module, uses)
            {
                if identity == QualifiedSourceIdentity::Ambiguous
                    || selected
                        .as_ref()
                        .is_some_and(|selected| selected != &identity)
                {
                    return Some(QualifiedSourceIdentity::Ambiguous);
                }
                selected = Some(identity);
            }
            recovered[index] = segments[index].clone();
        }
        selected
    }

    fn unique_qualified_descriptor_for_identity(
        &self,
        identity: &QualifiedSourceIdentity,
        args_len: Option<usize>,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        let candidates = self
            .qualified_descriptor_candidates(identity)
            .filter(|descriptor| {
                Self::descriptor_matches_arity(descriptor, args_len)
                    && self.qualified_descriptor_visible(
                        descriptor,
                        identity,
                        segments,
                        current_module,
                        uses,
                    )
            });
        Self::unique_candidate(candidates).flatten()
    }

    fn qualified_descriptor_candidates<'a>(
        &'a self,
        identity: &QualifiedSourceIdentity,
    ) -> impl DoubleEndedIterator<Item = &'a AdtDescriptor> {
        let indices = match identity {
            QualifiedSourceIdentity::Source(identity) => self
                .descriptors_by_source_identity
                .get(identity)
                .map(Vec::as_slice)
                .unwrap_or_default(),
            QualifiedSourceIdentity::CompilerKnown(type_name) => self
                .descriptors_by_source_identity
                .get(&(None, type_name.clone()))
                .map(Vec::as_slice)
                .unwrap_or_default(),
            QualifiedSourceIdentity::Ambiguous => &[],
        };
        indices
            .iter()
            .copied()
            .map(|index| self.descriptor_lookup_candidate(index))
    }

    fn qualified_descriptor_visible(
        &self,
        descriptor: &AdtDescriptor,
        identity: &QualifiedSourceIdentity,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> bool {
        match identity {
            QualifiedSourceIdentity::Source((module_name, _)) => {
                if module_name.as_deref() == current_module {
                    return true;
                }
                descriptor.visibility == Visibility::Public
                    || self.companion_private_access_allowed(
                        descriptor,
                        segments,
                        current_module,
                        uses,
                    )
            }
            QualifiedSourceIdentity::CompilerKnown(_) => descriptor.module_name.is_none(),
            QualifiedSourceIdentity::Ambiguous => false,
        }
    }

    fn unique_candidate<'a>(
        mut candidates: impl Iterator<Item = &'a AdtDescriptor>,
    ) -> Option<Option<&'a AdtDescriptor>> {
        let candidate = candidates.next()?;
        Some(candidates.next().is_none().then_some(candidate))
    }

    pub(crate) fn canonical_descriptor_for<'a>(
        &'a self,
        descriptor: &'a AdtDescriptor,
    ) -> &'a AdtDescriptor {
        self.descriptors_by_identity
            .get(&descriptor.identity())
            .into_iter()
            .flatten()
            .map(|index| &self.descriptors[*index])
            .find(|candidate| {
                candidate.nominal_identity.is_none()
                    && candidate.type_parameters.len() == descriptor.type_parameters.len()
            })
            .unwrap_or(descriptor)
    }

    pub(crate) fn canonical_type_name_for_descriptor(&self, descriptor: &AdtDescriptor) -> String {
        self.canonical_descriptor_for(descriptor).type_name.clone()
    }

    fn descriptor_for_type_path_with_arity(
        &self,
        name: &str,
        args_len: Option<usize>,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        if !name.contains("::") {
            return self.descriptor_for_unqualified_type_path(name, args_len, current_module, uses);
        }
        let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
        let type_name = segments.last()?;
        let identity = self.qualified_source_identity(type_name, &segments, current_module, uses);
        self.qualified_descriptor_candidates(&identity)
            .rev()
            .find(|descriptor| {
                Self::descriptor_matches_arity(descriptor, args_len)
                    && self.qualified_descriptor_visible(
                        descriptor,
                        &identity,
                        &segments,
                        current_module,
                        uses,
                    )
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
pub(crate) mod descriptor_lookup_counters {
    use std::cell::Cell;

    thread_local! {
        static CANDIDATE_SCANS: Cell<usize> = const { Cell::new(0) };
    }

    pub(crate) fn reset() {
        CANDIDATE_SCANS.set(0);
    }

    pub(super) fn record_candidate_scan() {
        CANDIDATE_SCANS.set(CANDIDATE_SCANS.get() + 1);
    }

    pub(crate) fn candidate_scans() -> usize {
        CANDIDATE_SCANS.get()
    }
}
