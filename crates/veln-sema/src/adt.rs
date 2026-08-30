use std::collections::{BTreeMap, BTreeSet, HashMap};

use veln_ast::{PublicAliasKind, SurfaceModule, TypeDecl, UseDecl, Visibility};
use veln_core::CoreType;
use veln_project::companion_access_target;

use crate::builtin_type_syntax::{BUILTIN_TYPE_SYNTAX_DESCRIPTORS, BuiltinTypeSyntaxRegistry};
use crate::name_recovery::{normal_use_decls, public_alias_has_invalid_target_leaf};
use crate::semantic_model::Type;
use crate::source_less_names::{
    InvalidStandardSymbolCase, InvalidStandardSymbolReason, SourceLessNameClass,
    validate_source_less_lookup_segment,
};
use crate::standard_names::PRELUDE_MODULE;
use crate::type_annotation_parser::parse_type_annotation_with_arity;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AdtVariantKind {
    OptionSome,
    OptionNone,
    ResultOk,
    ResultErr,
    ListNil,
    ListCons,
    Source,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdtDescriptor {
    pub(crate) type_name: String,
    pub(crate) name_class: SourceLessNameClass,
    pub(crate) module_name: Option<String>,
    pub(crate) type_parameters: Vec<String>,
    pub(crate) variants: Vec<AdtVariantDescriptor>,
    pub(crate) diagnostic_name: String,
    pub(crate) propagation: Option<ResultPropagationDescriptor>,
    pub(crate) visibility: Visibility,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdtVariantDescriptor {
    pub(crate) name: String,
    pub(crate) name_class: SourceLessNameClass,
    pub(crate) kind: AdtVariantKind,
    pub(crate) payload_fields: Vec<AdtPayloadField>,
    pub(crate) coverage_case: String,
    pub(crate) visibility: Visibility,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdtPayloadField {
    pub(crate) name: String,
    pub(crate) ty: AdtPayloadType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum AdtPayloadType {
    TypeParameter(usize),
    SelfType,
    Concrete(Type),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ResultPropagationDescriptor {
    pub(crate) value_parameter_index: usize,
    pub(crate) error_parameter_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdtConstructor<'a> {
    pub(crate) descriptor: &'a AdtDescriptor,
    pub(crate) variant: &'a AdtVariantDescriptor,
}

#[derive(Clone, Debug)]
pub(crate) struct AdtRegistry {
    descriptors: Vec<AdtDescriptor>,
    descriptors_by_type_name: HashMap<String, Vec<usize>>,
    variants_by_name: HashMap<String, Vec<(usize, usize)>>,
    companion_access_targets: BTreeMap<String, String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ConstructorLookup<'a> {
    Found(AdtConstructor<'a>),
    Ambiguous,
    Missing,
}

#[cfg(test)]
mod constructor_lookup_counters {
    use std::cell::Cell;

    thread_local! {
        static CANDIDATE_SCANS: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn reset() {
        CANDIDATE_SCANS.set(0);
    }

    pub(super) fn record_candidate_scan() {
        CANDIDATE_SCANS.set(CANDIDATE_SCANS.get() + 1);
    }

    pub(super) fn candidate_scans() -> usize {
        CANDIDATE_SCANS.get()
    }
}

impl AdtRegistry {
    fn from_parts(
        descriptors: Vec<AdtDescriptor>,
        companion_access_targets: BTreeMap<String, String>,
    ) -> Self {
        let mut descriptors_by_type_name = HashMap::<String, Vec<usize>>::new();
        let mut variants_by_name = HashMap::<String, Vec<(usize, usize)>>::new();
        for (descriptor_index, descriptor) in descriptors.iter().enumerate() {
            descriptors_by_type_name
                .entry(descriptor.type_name.clone())
                .or_default()
                .push(descriptor_index);
            for (variant_index, variant) in descriptor.variants.iter().enumerate() {
                variants_by_name
                    .entry(variant.name.clone())
                    .or_default()
                    .push((descriptor_index, variant_index));
            }
        }
        Self {
            descriptors,
            descriptors_by_type_name,
            variants_by_name,
            companion_access_targets,
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
        let aliases = type_alias_descriptors(module, &source_descriptors);
        descriptors.extend(aliases);
        descriptors.extend(source_descriptors);
        let mut companion_targets = base.companion_access_targets.clone();
        companion_targets.extend(companion_access_targets(module));
        Self::from_parts(descriptors, companion_targets)
    }

    pub(crate) fn descriptors(&self) -> &[AdtDescriptor] {
        &self.descriptors
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
        Self::from_parts(descriptors, companion_access_targets)
    }

    pub(crate) fn descriptor_for_type(&self, ty: &Type) -> Option<&AdtDescriptor> {
        let Type::Named { name, args } = ty else {
            return None;
        };
        self.descriptors_named(name).find(|descriptor| {
            descriptor.type_name == *name && descriptor.type_parameters.len() == args.len()
        })
    }

    pub(crate) fn descriptor_for_type_in_module(
        &self,
        ty: &Type,
        module_name: Option<&str>,
    ) -> Option<&AdtDescriptor> {
        let Type::Named { name, args } = ty else {
            return None;
        };
        if name.contains("::") {
            return None;
        }
        self.descriptors_named(name).rev().find(|descriptor| {
            descriptor.module_name.as_deref() == module_name
                && descriptor.type_parameters.len() == args.len()
        })
    }

    pub(crate) fn descriptor_for_type_path(
        &self,
        name: &str,
        args_len: usize,
        current_module: Option<&str>,
        uses: &[UseDecl],
    ) -> Option<&AdtDescriptor> {
        if !name.contains("::") {
            return self.descriptors_named(name).rev().find(|descriptor| {
                descriptor.module_name.as_deref() == current_module
                    && descriptor.type_parameters.len() == args_len
            });
        }
        let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
        let type_name = segments.last()?;
        self.descriptors_named(type_name).rev().find(|descriptor| {
            descriptor.type_parameters.len() == args_len
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
            ConstructorLookup::Ambiguous
                if descriptor_allows_expected_constructor_disambiguation(descriptor) => {}
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

fn descriptor_allows_expected_constructor_disambiguation(descriptor: &AdtDescriptor) -> bool {
    matches!(
        descriptor.module_name.as_deref(),
        None | Some("std::prelude")
    ) && matches!(descriptor.type_name.as_str(), "DecodeStep" | "EncodeStep")
        && descriptor.visibility == Visibility::Public
}

fn type_alias_descriptors(
    module: &SurfaceModule,
    descriptors: &[AdtDescriptor],
) -> Vec<AdtDescriptor> {
    let uses = normal_use_decls(module);
    module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Type)
        .filter_map(|alias| {
            let name = alias.name.clone()?;
            if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
                return None;
            }
            if public_alias_has_invalid_target_leaf(module, alias, Some(veln_ast::NameClass::Type))
            {
                return None;
            }
            let target = descriptor_for_alias_target(
                &alias.target,
                &uses,
                descriptors,
                alias.module_name.as_deref(),
            )?;
            let mut descriptor = target.clone();
            descriptor.type_name = name;
            descriptor.module_name = alias.module_name.clone();
            descriptor.visibility = Visibility::Public;
            Some(descriptor)
        })
        .collect()
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
            .find(|descriptor| descriptor.type_name == *name),
        [alias, name] => {
            let module_name = uses
                .iter()
                .find(|use_decl| {
                    use_decl.module_name.as_deref() == current_module && use_decl.alias == *alias
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

mod builtin_codec_descriptors;
mod builtin_core_descriptors;
mod builtin_descriptors;
mod lookup_validation;
mod runtime_base_variants;
mod runtime_connection_variants;
mod runtime_hpack_variants;
mod runtime_peer_limit_variants;
mod runtime_protocol_variants;
mod runtime_root_descriptors;
mod runtime_support_descriptors;
mod type_operations;
mod unification;

pub(crate) use builtin_descriptors::build_builtin_descriptors;
pub(crate) use lookup_validation::validate_adt_lookup_descriptors;
use lookup_validation::{
    companion_access_targets, constructor_matches_visible_path, same_descriptor, source_descriptor,
};
#[cfg(test)]
use type_operations::raw_builtin_descriptors_for_test;
#[cfg(test)]
pub(crate) use type_operations::validate_builtin_adt_descriptors;
pub(crate) use type_operations::{
    adt_args, constructed_type, constructed_type_from_args, core_constructed_type,
    core_constructed_type_from_args, core_list_part, core_list_type, core_option_part,
    core_option_type, core_payload_type, core_payload_type_with_args, core_result_parts,
    core_result_type, list_part, list_type, merge_core_type_args_from_payload,
    merge_type_args_from_payload, option_part, option_type, payload_type, payload_type_with_args,
    result_parts, result_type,
};

#[cfg(test)]
#[path = "adt/tests.rs"]
mod tests;
