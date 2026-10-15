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

#[derive(Clone, Debug)]
pub(crate) struct AdtRegistry {
    descriptors: Vec<AdtDescriptor>,
    variant_declaration_orders: Vec<Arc<VariantDeclarationOrder>>,
    descriptors_by_type_name: HashMap<String, Vec<usize>>,
    descriptors_by_identity: HashMap<String, Vec<usize>>,
    variants_by_name: HashMap<String, Vec<(usize, usize)>>,
    companion_access_targets: BTreeMap<String, String>,
    annotation_types: BTreeMap<(Option<String>, String), Type>,
    type_alias_identities: BTreeSet<(Option<String>, String)>,
    resolved_type_alias_declarations: BTreeSet<TypeAliasDeclarationIdentity>,
    declaration_spans: HashMap<String, SourceSpan>,
    local_descriptor_start: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TypeAliasDeclarationIdentity {
    module_name: Option<String>,
    file: SourcePath,
    start: usize,
    end: usize,
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
                .map(|index| &self.descriptors[index])
        };
        let local = descriptors(true)
            .filter(|descriptor| descriptor.module_name.as_deref() == current_module);
        let builtin = descriptors(false).filter(|descriptor| descriptor.module_name.is_none());
        let imported = indices
            .iter()
            .map(|index| &self.descriptors[*index])
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
        let qualified_source_identity = self
            .descriptors_named(type_name)
            .find(|descriptor| {
                self.descriptor_matches_qualified_source_identity(
                    descriptor,
                    &segments,
                    current_module,
                    uses,
                )
            })
            .map(descriptor_identity);
        let candidates = self.descriptors_named(type_name).filter(|descriptor| {
            qualified_source_identity.as_ref().map_or_else(
                || descriptor.module_name.is_none(),
                |identity| descriptor_identity(descriptor) == *identity,
            ) && self.descriptor_visible(descriptor, &segments, current_module, uses, true)
        });
        Self::unique_candidate(candidates).flatten()
    }

    fn descriptor_matches_qualified_source_identity(
        &self,
        descriptor: &AdtDescriptor,
        segments: &[String],
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> bool {
        let Some(module_name) = descriptor.module_name.as_deref() else {
            return false;
        };
        let module_segments = &segments[..segments.len() - 1];
        (descriptor.module_name.as_deref() == current_module
            && module_segments.join("::") == module_name)
            || imported_module_path_matches(descriptor, module_segments, uses, current_module)
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
