use std::collections::{BTreeSet, HashMap};

use veln_ast::{BodyLine, Param};

use super::boundary::{
    duplicate_name_diagnostic, exact_width_binary_primitive_name,
    exact_width_schema_primitive_diagnostic, format_neutral_schema_encode_helper_diagnostic,
    lowercase_schema_primitive_position_diagnostic, type_contains_unknown,
};
use super::repair_reasoning::*;
use super::*;
use crate::effect_rows::{collect_effect_row_substitution, instantiate_effect_rows};
use crate::effects::prelude_effect_origin;
use crate::schema::primitives::lowercase_schema_primitive;
use crate::source_less_lookup::qualified_symbol;
use crate::types::signatures::{
    FunctionSignature, SchemaReferenceErrorKind, UserEffectPathResolution,
};

type LocalNameDeclaration = (String, SourceSpan);
type ScopedLocalName = (String, Option<LocalNameDeclaration>);

pub(crate) fn check_function_body(
    function: &Function,
    environment: &TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
) -> Vec<Diagnostic> {
    let mut checker =
        FunctionChecker::for_source_declaration(function, environment, variant_diagnostics);
    checker.check_body();
    std::mem::take(&mut checker.diagnostics)
}

#[cfg(test)]
fn record_alias_group_created() {
    transparent_alias_work::group_created();
}

#[cfg(not(test))]
fn record_alias_group_created() {}

#[cfg(test)]
fn record_alias_group_released() {
    transparent_alias_work::group_released();
}

#[cfg(not(test))]
fn record_alias_group_released() {}

#[cfg(test)]
fn record_alias_member_retained() {
    transparent_alias_work::member_retained();
}

#[cfg(not(test))]
fn record_alias_member_retained() {}

#[cfg(test)]
fn record_alias_member_released() {
    transparent_alias_work::member_released();
}

#[cfg(not(test))]
fn record_alias_member_released() {}

#[cfg(test)]
fn record_alias_group_lookup() {
    transparent_alias_work::group_lookup();
}

#[cfg(not(test))]
fn record_alias_group_lookup() {}

#[cfg(test)]
fn record_alias_member_lookup() {
    transparent_alias_work::member_lookup();
}

#[cfg(not(test))]
fn record_alias_member_lookup() {}

#[cfg(test)]
fn record_alias_refinement_retained() {
    transparent_alias_work::refinement_retained();
}

#[cfg(not(test))]
fn record_alias_refinement_retained() {}

#[cfg(test)]
fn record_alias_refinement_released() {
    transparent_alias_work::refinement_released();
}

#[cfg(not(test))]
fn record_alias_refinement_released() {}

fn json_string_field_is(value: &JsonValue, field: &str, expected: &str) -> bool {
    matches!(
        value,
        JsonValue::Object(entries) if entries.iter().any(|(name, value)| {
            name == field && value.as_text() == Some(expected)
        })
    )
}

fn valid_value_binding_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

fn invalid_value_binding_name(name: &str) -> bool {
    !valid_value_binding_name(name)
}

fn type_contains_variant_refinement(ty: &Type) -> bool {
    match ty {
        Type::VariantRefinement { .. } => true,
        Type::Named { args, .. } => args.iter().any(type_contains_variant_refinement),
        Type::Record(fields) => fields
            .iter()
            .any(|(_, ty)| type_contains_variant_refinement(ty)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params.iter().any(type_contains_variant_refinement)
                || variadic
                    .as_deref()
                    .is_some_and(type_contains_variant_refinement)
                || type_contains_variant_refinement(return_type)
        }
        Type::Unknown => false,
    }
}

fn inferred_control_flow_recovery_type(ty: &Type) -> Type {
    match ty {
        Type::VariantRefinement {
            name,
            identity,
            args,
            ..
        } => Type::resolved_named(name, identity, args.clone()),
        ty => ty.clone(),
    }
}

fn inferred_aggregate_member_type_with_expected(ty: Type, expected: &Type) -> Type {
    if expected != &Type::Unknown && is_assignable(expected, &ty) {
        expected.clone()
    } else {
        ty
    }
}

fn literal_or_unit_pattern_type(pattern: &Pattern) -> Option<Type> {
    match &pattern.kind {
        PatternKind::StringLiteral(_) => Some(Type::string()),
        PatternKind::IntLiteral(_) => Some(Type::int()),
        PatternKind::FloatLiteral(_) => Some(Type::float()),
        PatternKind::BoolLiteral(_) => Some(Type::bool()),
        PatternKind::Unit => Some(Type::unit()),
        _ => None,
    }
}

fn literal_or_unit_pattern_is_compatible(pattern: &Pattern, expected: &Type) -> Option<bool> {
    let actual = literal_or_unit_pattern_type(pattern)?;
    Some(expected != &Type::Unknown && is_assignable(expected, &actual))
}

pub(in crate::analysis) struct FunctionChecker<'a> {
    pub(super) function: &'a Function,
    pub(super) environment: &'a TypeEnvironment,
    pub(super) supports_callsite_modifier: bool,
    pub(super) bindings: Vec<Binding>,
    binding_positions: HashMap<String, Vec<usize>>,
    next_transparent_alias_group: usize,
    transparent_alias_groups: Vec<TransparentAliasGroup>,
    transparent_alias_refinement_frames: Vec<usize>,
    stable_field_path_refinements: HashMap<StableFieldPath, Vec<Type>>,
    stable_field_path_refinement_frames: Vec<StableFieldPath>,
    invalid_binding_recoveries: Vec<InvalidBindingRecovery>,
    omitted_local_bindings: Vec<OmittedLocalBinding>,
    pub(super) local_names: BTreeMap<String, LocalNameDeclaration>,
    local_name_scopes: Vec<Vec<ScopedLocalName>>,
    captured_local_bindings: BTreeSet<String>,
    defer_capture_boundaries: Vec<usize>,
    pub(super) inferred_effects: Vec<EffectUse>,
    pub(super) inferred_return_type: Option<Type>,
    inferred_return_unknown_is_diagnosed: bool,
    pub(super) diagnostics: Vec<Diagnostic>,
    suppressed_diagnostic_indices: BTreeSet<usize>,
    defer_blocks: Vec<SourceSpan>,
    refined_match_domains: adt_and_match::RefinedMatchDomainCache,
    variant_diagnostics: &'a mut VariantDiagnosticInterner,
}

struct TransparentAliasGroup {
    feasible_type: Type,
    active_refinements: Vec<Type>,
    member_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct StableFieldPath {
    root_alias_group: usize,
    fields: Vec<String>,
}

pub(in crate::analysis) struct PatternBinding {
    name: String,
    ty: Type,
    node_id: NodeId,
    span: SourceSpan,
}

struct InvalidBindingRecovery {
    name: String,
    ty: Type,
}

struct OmittedLocalBinding {
    name: String,
    node_id: NodeId,
    span: SourceSpan,
    deferred_initializer_diagnostic: Option<usize>,
}

struct EffectBoundary {
    kind: &'static str,
    diagnostic_id: &'static str,
    subject: &'static str,
}

pub(super) struct DeferRestrictionDiagnostic {
    pub(super) id: &'static str,
    pub(super) message: String,
    pub(super) node_id: String,
    pub(super) span: SourceSpan,
    pub(super) reason: &'static str,
    pub(super) repair: &'static str,
    pub(super) repair_span: SourceSpan,
}

impl EffectBoundary {
    fn for_function(function: &Function) -> Option<Self> {
        if function.kind == FunctionKind::Test {
            return Some(Self {
                kind: "test_declaration",
                diagnostic_id: "effect.missing_test",
                subject: "test declaration",
            });
        }
        if function.visibility == Visibility::Public {
            return Some(Self {
                kind: "public_function",
                diagnostic_id: "effect.missing_public",
                subject: "public function",
            });
        }
        None
    }
}

#[derive(Clone, Copy)]
enum MatchDomain {
    Bool,
    Adt,
}

impl MatchDomain {
    pub(super) fn from_type(
        ty: &Type,
        environment: &TypeEnvironment,
        current_module: Option<&str>,
    ) -> Option<Self> {
        match ty {
            Type::Named { name, args, .. } if name == "Bool" && args.is_empty() => Some(Self::Bool),
            _ => environment
                .adts
                .descriptor_for_type_prefer_module(ty, current_module)
                .map(|_| Self::Adt),
        }
    }

    pub(super) fn cases(
        self,
        ty: &Type,
        environment: &TypeEnvironment,
        current_module: Option<&str>,
    ) -> Vec<String> {
        match self {
            Self::Bool => vec!["false".to_string(), "true".to_string()],
            Self::Adt => environment
                .adts
                .descriptor_for_type_prefer_module(ty, current_module)
                .into_iter()
                .flat_map(|descriptor| descriptor.variants.iter())
                .map(|variant| variant.coverage_case.clone())
                .collect(),
        }
    }
}

struct PatternCoverage {
    catches_all: bool,
    cases: Vec<String>,
}

fn match_pattern_coverage(
    pattern: &Pattern,
    domain: &MatchDomain,
    scrutinee_type: &Type,
    environment: &TypeEnvironment,
    current_module: Option<&str>,
) -> PatternCoverage {
    match &pattern.kind {
        PatternKind::Wildcard | PatternKind::Binding(_) => PatternCoverage {
            catches_all: true,
            cases: Vec::new(),
        },
        PatternKind::BoolLiteral(value) if matches!(domain, MatchDomain::Bool) => PatternCoverage {
            catches_all: false,
            cases: vec![(if *value { "true" } else { "false" }).to_string()],
        },
        PatternKind::Constructor { name, .. } => {
            if invalid_qualified_constructor_pattern(name) {
                return PatternCoverage {
                    catches_all: false,
                    cases: Vec::new(),
                };
            }
            let case = match domain {
                MatchDomain::Adt => environment
                    .adts
                    .descriptor_for_type_prefer_module(scrutinee_type, current_module)
                    .and_then(|descriptor| {
                        environment
                            .adts
                            .constructor_for_descriptor(
                                name,
                                descriptor,
                                current_module,
                                &environment.uses,
                            )
                            .map(|constructor| constructor.variant.coverage_case.clone())
                    }),
                MatchDomain::Bool => None,
            };
            PatternCoverage {
                catches_all: false,
                cases: case.into_iter().collect(),
            }
        }
        _ => PatternCoverage {
            catches_all: false,
            cases: Vec::new(),
        },
    }
}

impl<'a> FunctionChecker<'a> {
    pub(super) fn for_source_declaration(
        function: &'a Function,
        environment: &'a TypeEnvironment,
        variant_diagnostics: &'a mut VariantDiagnosticInterner,
    ) -> Self {
        Self::new(
            function,
            environment,
            function.kind == FunctionKind::Function,
            variant_diagnostics,
        )
    }

    pub(super) fn for_synthetic_declaration(
        function: &'a Function,
        environment: &'a TypeEnvironment,
        variant_diagnostics: &'a mut VariantDiagnosticInterner,
    ) -> Self {
        Self::new(function, environment, false, variant_diagnostics)
    }

    fn new(
        function: &'a Function,
        environment: &'a TypeEnvironment,
        supports_callsite_modifier: bool,
        variant_diagnostics: &'a mut VariantDiagnosticInterner,
    ) -> Self {
        Self {
            function,
            environment,
            supports_callsite_modifier,
            bindings: Vec::new(),
            binding_positions: HashMap::new(),
            next_transparent_alias_group: 0,
            transparent_alias_groups: Vec::new(),
            transparent_alias_refinement_frames: Vec::new(),
            stable_field_path_refinements: HashMap::new(),
            stable_field_path_refinement_frames: Vec::new(),
            invalid_binding_recoveries: Vec::new(),
            omitted_local_bindings: Vec::new(),
            local_names: BTreeMap::new(),
            local_name_scopes: Vec::new(),
            captured_local_bindings: BTreeSet::new(),
            defer_capture_boundaries: Vec::new(),
            inferred_effects: Vec::new(),
            inferred_return_type: None,
            inferred_return_unknown_is_diagnosed: false,
            diagnostics: Vec::new(),
            suppressed_diagnostic_indices: BTreeSet::new(),
            defer_blocks: Vec::new(),
            refined_match_domains: adt_and_match::RefinedMatchDomainCache::default(),
            variant_diagnostics,
        }
    }

    pub(super) fn fresh_transparent_alias_group(&mut self, feasible_type: Type) -> Option<usize> {
        if feasible_type != Type::Unknown
            && !matches!(feasible_type, Type::Record(_))
            && self
                .environment
                .adts
                .descriptor_for_type_prefer_module(
                    &feasible_type,
                    self.function.module_name.as_deref(),
                )
                .is_none()
        {
            return None;
        }
        let group = self.next_transparent_alias_group;
        self.next_transparent_alias_group += 1;
        self.transparent_alias_groups.push(TransparentAliasGroup {
            feasible_type,
            active_refinements: Vec::new(),
            member_count: 0,
        });
        record_alias_group_created();
        Some(group)
    }

    pub(super) fn push_binding(&mut self, binding: Binding) {
        self.binding_positions
            .entry(binding.name.clone())
            .or_default()
            .push(self.bindings.len());
        if let Some(group) = binding.transparent_alias_group {
            self.transparent_alias_groups[group].member_count += 1;
            record_alias_member_retained();
        }
        self.bindings.push(binding);
    }

    pub(super) fn truncate_bindings(&mut self, len: usize) {
        for binding in &self.bindings[len..] {
            let positions = self
                .binding_positions
                .get_mut(&binding.name)
                .expect("binding position entry");
            positions.pop();
            if positions.is_empty() {
                self.binding_positions.remove(&binding.name);
            }
            if let Some(group) = binding.transparent_alias_group {
                self.transparent_alias_groups[group].member_count -= 1;
                record_alias_member_released();
            }
        }
        self.bindings.truncate(len);
        while self
            .transparent_alias_groups
            .last()
            .is_some_and(|group| group.member_count == 0 && group.active_refinements.is_empty())
        {
            self.transparent_alias_groups.pop();
            self.next_transparent_alias_group -= 1;
            record_alias_group_released();
        }
    }

    pub(super) fn visible_binding_index(&self, name: &str) -> Option<usize> {
        self.binding_positions
            .get(name)
            .and_then(|positions| positions.last().copied())
    }

    fn alias_group_type(&self, group: usize) -> &Type {
        record_alias_group_lookup();
        self.transparent_alias_groups[group]
            .active_refinements
            .last()
            .unwrap_or(&self.transparent_alias_groups[group].feasible_type)
    }

    pub(super) fn binding_type(&self, index: usize) -> Type {
        record_alias_member_lookup();
        let binding = &self.bindings[index];
        binding.transparent_alias_group.map_or_else(
            || binding.ty.clone(),
            |group| {
                record_alias_group_lookup();
                let alias_group = &self.transparent_alias_groups[group];
                let mut persistent = binding.ty.clone();
                if is_assignable(&persistent, &alias_group.feasible_type) {
                    adt::merge_type_holes(&mut persistent, &alias_group.feasible_type);
                }
                alias_group.active_refinements.last().map_or_else(
                    || persistent.clone(),
                    |refinement| transparent_alias_presented_type(&persistent, refinement),
                )
            },
        )
    }

    pub(super) fn effective_visible_bindings(&self) -> Vec<(usize, Type)> {
        self.bindings
            .iter()
            .enumerate()
            .filter(|(index, binding)| self.visible_binding_index(&binding.name) == Some(*index))
            .map(|(index, _)| (index, self.binding_type(index)))
            .collect()
    }

    pub(super) fn set_binding_type(&mut self, index: usize, ty: Type) {
        self.bindings[index].ty = ty.clone();
        if let Some(group) = self.bindings[index].transparent_alias_group {
            let feasible_type = &mut self.transparent_alias_groups[group].feasible_type;
            if is_assignable(&ty, feasible_type) {
                adt::merge_type_holes(feasible_type, &ty);
            }
        }
    }

    pub(super) fn alias_group_match_type(&self, group: usize) -> Type {
        self.alias_group_type(group).clone()
    }

    pub(super) fn push_alias_group_refinement(&mut self, group: usize, refinement: Type) {
        self.transparent_alias_groups[group]
            .active_refinements
            .push(refinement);
        self.transparent_alias_refinement_frames.push(group);
        record_alias_refinement_retained();
    }

    pub(super) fn alias_refinement_frame_count(&self) -> usize {
        self.transparent_alias_refinement_frames.len()
    }

    pub(super) fn restore_alias_refinement_frames(&mut self, len: usize) {
        while self.transparent_alias_refinement_frames.len() > len {
            let group = self
                .transparent_alias_refinement_frames
                .pop()
                .expect("alias refinement frame");
            self.transparent_alias_groups[group]
                .active_refinements
                .pop()
                .expect("active alias refinement");
            record_alias_refinement_released();
        }
    }

    fn push_stable_field_path_refinement(&mut self, path: StableFieldPath, refinement: Type) {
        self.stable_field_path_refinements
            .entry(path.clone())
            .or_default()
            .push(refinement);
        self.stable_field_path_refinement_frames.push(path);
    }

    fn stable_field_path_refinement_frame_count(&self) -> usize {
        self.stable_field_path_refinement_frames.len()
    }

    fn restore_stable_field_path_refinement_frames(&mut self, len: usize) {
        while self.stable_field_path_refinement_frames.len() > len {
            let path = self
                .stable_field_path_refinement_frames
                .pop()
                .expect("stable field path refinement frame");
            let refinements = self
                .stable_field_path_refinements
                .get_mut(&path)
                .expect("stable field path refinements");
            refinements
                .pop()
                .expect("active stable field path refinement");
            if refinements.is_empty() {
                self.stable_field_path_refinements.remove(&path);
            }
        }
    }

    pub(super) fn check_body(&mut self) {
        self.check_function_annotations();
        self.check_contracts();
        let function = self.function;
        for (index, line) in function.body.iter().enumerate() {
            self.check_body_line(index, line);
        }
        self.check_implicit_unit_return();
        self.check_omitted_local_inference_complete();
        self.check_private_inference_complete();
        self.check_effect_boundaries();
        self.remove_suppressed_diagnostics();
    }
}

impl Drop for FunctionChecker<'_> {
    fn drop(&mut self) {
        self.restore_alias_refinement_frames(0);
        self.truncate_bindings(0);
        for _ in self.transparent_alias_groups.drain(..) {
            record_alias_group_released();
        }
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct TransparentAliasWork {
    pub(crate) group_lookups: usize,
    pub(crate) member_lookups: usize,
    pub(crate) groups_created: usize,
    pub(crate) peak_retained_groups: usize,
    pub(crate) peak_retained_members: usize,
    pub(crate) peak_active_refinements: usize,
    pub(crate) retained_groups: usize,
    pub(crate) retained_members: usize,
    pub(crate) active_refinements: usize,
}

#[cfg(test)]
pub(crate) fn reset_transparent_alias_work() {
    transparent_alias_work::reset();
}

#[cfg(test)]
pub(crate) fn take_transparent_alias_work() -> TransparentAliasWork {
    transparent_alias_work::take()
}

#[cfg(test)]
mod transparent_alias_work {
    use super::TransparentAliasWork;
    use std::cell::Cell;

    thread_local! {
        static WORK: Cell<TransparentAliasWork> = Cell::new(TransparentAliasWork::default());
    }

    fn update(f: impl FnOnce(&mut TransparentAliasWork)) {
        WORK.set({
            let mut work = WORK.get();
            f(&mut work);
            work
        });
    }

    pub(super) fn reset() {
        WORK.set(TransparentAliasWork::default());
    }

    pub(super) fn take() -> TransparentAliasWork {
        WORK.replace(TransparentAliasWork::default())
    }

    pub(super) fn group_created() {
        update(|work| {
            work.groups_created += 1;
            work.retained_groups += 1;
            work.peak_retained_groups = work.peak_retained_groups.max(work.retained_groups);
        });
    }

    pub(super) fn group_released() {
        update(|work| work.retained_groups -= 1);
    }

    pub(super) fn member_retained() {
        update(|work| {
            work.retained_members += 1;
            work.peak_retained_members = work.peak_retained_members.max(work.retained_members);
        });
    }

    pub(super) fn member_released() {
        update(|work| work.retained_members -= 1);
    }

    pub(super) fn group_lookup() {
        update(|work| work.group_lookups += 1);
    }

    pub(super) fn member_lookup() {
        update(|work| work.member_lookups += 1);
    }

    pub(super) fn refinement_retained() {
        update(|work| {
            work.active_refinements += 1;
            work.peak_active_refinements =
                work.peak_active_refinements.max(work.active_refinements);
        });
    }

    pub(super) fn refinement_released() {
        update(|work| work.active_refinements -= 1);
    }
}

fn transparent_alias_presented_type(presentation: &Type, fact: &Type) -> Type {
    let Type::VariantRefinement {
        identity: fact_identity,
        args: fact_args,
        variants: fact_variants,
        ..
    } = fact
    else {
        return presentation.clone();
    };
    match presentation {
        Type::Named {
            name,
            identity,
            args,
        } if identity == fact_identity && args == fact_args => {
            Type::resolved_variant_refinement_shared(
                name,
                identity,
                args.clone(),
                std::sync::Arc::clone(fact_variants),
            )
        }
        Type::VariantRefinement {
            name,
            identity,
            args,
            variants,
            ..
        } if identity == fact_identity && args == fact_args => {
            if variants.len() <= fact_variants.len() {
                return presentation.clone();
            }
            Type::resolved_variant_refinement_shared(
                name,
                identity,
                args.clone(),
                std::sync::Arc::clone(fact_variants),
            )
        }
        _ => presentation.clone(),
    }
}

mod adt_and_match;
#[cfg(test)]
pub(crate) use adt_and_match::{
    RefinedMatchCoverageWork, RefinedMatchDiagnosticWork, reset_refined_match_coverage_work,
    reset_refined_match_diagnostic_work, take_refined_match_coverage_work,
    take_refined_match_diagnostic_work,
};
mod annotations_and_effects;
mod body_lines;
mod collections_and_operators;
mod contract_validation;
mod control_flow_results;
mod diagnostics_and_repairs;
pub(crate) use diagnostics_and_repairs::VariantDiagnosticInterner;
#[cfg(test)]
pub(crate) use diagnostics_and_repairs::{
    reset_retained_variant_diagnostic_key_variants, take_retained_variant_diagnostic_key_variants,
};
mod expression_effects;
mod name_and_declared_calls;
mod patterns_and_exhaustiveness;
mod prelude_and_unresolved_calls;
mod recovery_helpers;

use recovery_helpers::*;
