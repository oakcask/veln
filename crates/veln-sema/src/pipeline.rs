use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::hash::Hash;

use veln_ast::{FunctionKind, PublicAliasKind, SurfaceModule, Visibility};
use veln_core::CheckedProgram;
use veln_diagnostics::{Diagnostic, Severity};
use veln_ir::{TypedProgram, lower_checked_core};
use veln_source::SourcePath;

use crate::analysis::{
    VariantDiagnosticInterner, VariantRefinementBaseDiagnostic, check_declared_effect_labels,
    check_duplicate_constructor_names, check_duplicate_effect_names,
    check_duplicate_function_names, check_duplicate_schema_names, check_duplicate_type_names,
    check_duplicate_use_aliases, check_function_body, check_handler_declarations,
    check_module_boundary, check_public_adt_variant_refinement_visibility, check_public_aliases,
    check_public_function_boundary, check_reserved_prelude_aliases, check_schema_field_primitives,
    check_schema_type_references, check_test_declaration_boundary, check_variant_refinement_bases,
};
use crate::lowering::{
    lower_executable_surface_module_to_core, lower_project_surface_module_to_core,
    lower_surface_module_to_core,
};
use crate::schema;
use crate::source_less_lookup::validate_source_less_lookup_registries;
use crate::types::{
    ReusableStandardEnvironment, TypeEnvironment, prepare_current_reusable_standard_environment,
    prepare_reusable_standard_environment,
};

mod identifier_casing;

use identifier_casing::check_invalid_name_casing;
pub use identifier_casing::{
    CanonicalTypeAnnotationIdentity, CanonicalTypeAnnotationWork,
    VariantRefinementClassificationWork, canonical_type_annotation_identities_with_context,
    canonical_type_annotation_identities_with_context_and_work,
    classified_project_qualified_path_segments,
    classified_project_qualified_path_segments_with_context,
    classified_project_qualified_path_segments_with_context_and_work,
};

#[derive(Clone, Debug)]
pub struct LoweredSurfaceModule {
    pub diagnostics: Vec<Diagnostic>,
    pub core: Option<CheckedProgram>,
    pub ir: Option<TypedProgram>,
    pub unpublished_declarations: Vec<UnpublishedDeclaration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnpublishedDeclaration {
    pub kind: &'static str,
    pub module_name: Option<String>,
    pub name: String,
    pub span: veln_source::SourceSpan,
    pub causal_diagnostics: Vec<PublicationDiagnosticCause>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationDiagnosticCause {
    pub diagnostic_id: String,
    pub span: veln_source::SourceSpan,
}

impl Hash for PublicationDiagnosticCause {
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        self.diagnostic_id.hash(state);
        self.span.file.hash(state);
        self.span.start.line.hash(state);
        self.span.start.column.hash(state);
        self.span.start.offset.hash(state);
        self.span.end.line.hash(state);
        self.span.end.column.hash(state);
        self.span.end.offset.hash(state);
        self.span.original_path().hash(state);
        self.span
            .original_start()
            .map(|position| (position.line, position.column, position.offset))
            .hash(state);
        self.span
            .original_end()
            .map(|position| (position.line, position.column, position.offset))
            .hash(state);
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PublicationPropagationWork {
    pub reverse_edges: usize,
    pub admitted_cause_pairs: usize,
    pub propagation_attempts: usize,
}

#[cfg(test)]
thread_local! {
    static PUBLICATION_PROPAGATION_WORK: std::cell::Cell<PublicationPropagationWork> =
        const { std::cell::Cell::new(PublicationPropagationWork {
            reverse_edges: 0,
            admitted_cause_pairs: 0,
            propagation_attempts: 0,
        }) };
}

#[cfg(test)]
pub(crate) fn reset_publication_propagation_work() {
    PUBLICATION_PROPAGATION_WORK.set(PublicationPropagationWork::default());
}

#[cfg(test)]
pub(crate) fn publication_propagation_work() -> PublicationPropagationWork {
    PUBLICATION_PROPAGATION_WORK.get()
}

#[cfg(test)]
fn record_publication_propagation_work(update: impl FnOnce(&mut PublicationPropagationWork)) {
    PUBLICATION_PROPAGATION_WORK.set({
        let mut work = PUBLICATION_PROPAGATION_WORK.get();
        update(&mut work);
        work
    });
}

pub fn analyze_surface_module(module: &SurfaceModule) -> Vec<Diagnostic> {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return vec![failure.diagnostic()];
    }
    let environment = TypeEnvironment::from_module(module);
    analyze_surface_module_with_environment(module, &environment, true)
}

#[cfg(test)]
pub(crate) fn analyze_surface_module_with_base_for_test(
    module: &SurfaceModule,
    base: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let environment = TypeEnvironment::from_module_with_base_for_test(module, base);
    analyze_surface_module_with_environment(module, &environment, true)
}

#[cfg(test)]
pub(crate) fn check_project_surface_module_with_base_for_test(
    module: &SurfaceModule,
    base: &TypeEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    let environment = TypeEnvironment::from_module_with_base_for_test(module, base);
    check_project_surface_module_with_environment(module, environment)
}

pub fn check_project_surface_module(
    module: &SurfaceModule,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    if let Err(failure) = validate_source_less_lookup_registries() {
        let diagnostics = vec![failure.diagnostic()];
        return (diagnostics.clone(), lowered_internal_failure(diagnostics));
    }
    let environment = TypeEnvironment::from_module(module);
    check_project_surface_module_with_environment(module, environment)
}

pub fn check_project_surface_module_with_standard_environment(
    module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    if let Err(failure) = validate_source_less_lookup_registries() {
        let diagnostics = vec![failure.diagnostic()];
        return (diagnostics.clone(), lowered_internal_failure(diagnostics));
    }
    let environment = TypeEnvironment::from_module_with_standard(module, standard);
    check_project_surface_module_with_environment(module, environment)
}

pub fn check_project_surface_module_with_standard_modules_environment(
    application_module: &SurfaceModule,
    selected_standard_module_names: &BTreeSet<String>,
    standard: &ReusableStandardEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    if let Err(failure) = validate_source_less_lookup_registries() {
        let diagnostics = vec![failure.diagnostic()];
        return (diagnostics.clone(), lowered_internal_failure(diagnostics));
    }
    let environment = TypeEnvironment::from_application_module_with_standard_module_names(
        application_module,
        selected_standard_module_names,
        standard,
    );
    check_project_surface_module_with_environment(application_module, environment)
}

pub fn prepare_reusable_standard_surface_module_environment(
    module: &SurfaceModule,
) -> ReusableStandardEnvironment {
    validate_source_less_lookup_registries().expect("source-less lookup registries are valid");
    prepare_reusable_standard_environment(module)
}

pub fn validate_standard_symbol_registry_diagnostic() -> Result<(), Box<Diagnostic>> {
    validate_source_less_lookup_registries().map_err(|failure| Box::new(failure.diagnostic()))
}

pub fn prepare_current_reusable_standard_surface_module_environment(
    module: &SurfaceModule,
) -> ReusableStandardEnvironment {
    validate_source_less_lookup_registries().expect("source-less lookup registries are valid");
    prepare_current_reusable_standard_environment(module)
}

pub fn try_prepare_current_reusable_standard_surface_module_environment(
    module: &SurfaceModule,
) -> Result<ReusableStandardEnvironment, Box<Diagnostic>> {
    validate_standard_symbol_registry_diagnostic()?;
    Ok(prepare_current_reusable_standard_environment(module))
}

fn check_project_surface_module_with_environment(
    module: &SurfaceModule,
    environment: TypeEnvironment,
) -> (Vec<Diagnostic>, LoweredSurfaceModule) {
    if let Err(failure) = validate_source_less_lookup_registries() {
        let diagnostics = vec![failure.diagnostic()];
        return (diagnostics.clone(), lowered_internal_failure(diagnostics));
    }
    let validate_standard_bodies = should_validate_standard_bodies(module);
    let semantic_diagnostics =
        analyze_surface_module_with_environment(module, &environment, validate_standard_bodies);
    let checked = lower_analyzed_surface_module_with_environment(
        module,
        semantic_diagnostics.clone(),
        &environment,
        SurfaceLoweringMode::ProjectCheck,
    );
    (semantic_diagnostics, checked)
}

fn analyze_surface_module_with_environment(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    validate_standard_bodies: bool,
) -> Vec<Diagnostic> {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return vec![failure.diagnostic()];
    }
    let mut diagnostics = Vec::new();
    let mut variant_diagnostics = VariantDiagnosticInterner::default();

    diagnostics.extend(check_module_declarations(module, environment));

    for function in &module.functions {
        if !validate_standard_bodies
            && function
                .module_name
                .as_deref()
                .is_some_and(|module| module.starts_with("std::"))
        {
            continue;
        }
        diagnostics.extend(check_function_declaration_and_body(
            function,
            environment,
            &mut variant_diagnostics,
        ));
    }

    order_variant_refinement_visibility_diagnostics(&mut diagnostics);

    merge_variant_refinement_base_diagnostics(
        &mut diagnostics,
        check_variant_refinement_bases(module, environment),
    );

    diagnostics
}

fn order_variant_refinement_visibility_diagnostics(diagnostics: &mut [Diagnostic]) {
    let slots = diagnostics
        .iter()
        .enumerate()
        .filter_map(|(index, diagnostic)| {
            (diagnostic.id == "type.variant_refinement_private").then_some(index)
        })
        .collect::<Vec<_>>();
    if slots.len() < 2 {
        return;
    }

    let mut indices_by_file = HashMap::<SourcePath, Vec<usize>>::new();
    let mut file_order = Vec::new();
    let mut ordered_indices = Vec::with_capacity(slots.len());
    for &index in &slots {
        let Some(span) = diagnostics[index].span.as_ref() else {
            ordered_indices.push(index);
            continue;
        };
        match indices_by_file.entry(span.file.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                entry.get_mut().push(index);
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                file_order.push(span.file.clone());
                entry.insert(vec![index]);
            }
        }
    }
    for file in file_order {
        let indices = indices_by_file
            .get_mut(&file)
            .expect("visibility diagnostic file was indexed");
        radix_sort_diagnostic_indices(indices, diagnostics);
        ordered_indices.extend(indices.iter().copied());
    }

    let ordered = ordered_indices
        .into_iter()
        .map(|index| diagnostics[index].clone())
        .collect::<Vec<_>>();
    for (slot, diagnostic) in slots.into_iter().zip(ordered) {
        diagnostics[slot] = diagnostic;
    }
}

fn radix_sort_diagnostic_indices(indices: &mut Vec<usize>, diagnostics: &[Diagnostic]) {
    let mut scratch = vec![0; indices.len()];
    for use_end in [true, false] {
        for shift in (0..usize::BITS).step_by(8) {
            let mut counts = [0usize; 256];
            for &index in indices.iter() {
                let span = diagnostics[index]
                    .span
                    .as_ref()
                    .expect("indexed diagnostic has a source span");
                let key = if use_end {
                    span.end.offset
                } else {
                    span.start.offset
                };
                counts[(key >> shift) as u8 as usize] += 1;
            }
            let mut next = 0;
            for count in &mut counts {
                let start = next;
                next += *count;
                *count = start;
            }
            for &index in indices.iter() {
                let span = diagnostics[index]
                    .span
                    .as_ref()
                    .expect("indexed diagnostic has a source span");
                let key = if use_end {
                    span.end.offset
                } else {
                    span.start.offset
                };
                let bucket = (key >> shift) as u8 as usize;
                scratch[counts[bucket]] = index;
                counts[bucket] += 1;
            }
            std::mem::swap(indices, &mut scratch);
        }
    }
}

fn merge_variant_refinement_base_diagnostics(
    diagnostics: &mut Vec<Diagnostic>,
    base_diagnostics: Vec<VariantRefinementBaseDiagnostic>,
) {
    let (mut bases_by_file, ordered_indices) = index_base_diagnostics(&base_diagnostics);
    let mut bases = base_diagnostics.into_iter().map(Some).collect::<Vec<_>>();
    merge_existing_base_diagnostics(diagnostics, &mut bases_by_file, &mut bases);
    for index in ordered_indices {
        record_variant_refinement_base_work(1);
        if let Some(base) = bases[index].take() {
            diagnostics.push(base.diagnostic);
        }
    }
}

type BaseDiagnosticsByFile = HashMap<SourcePath, (Vec<usize>, usize)>;
type DiagnosticSpansByFile = HashMap<SourcePath, HashMap<(usize, usize), usize>>;

fn index_base_diagnostics(
    base_diagnostics: &[VariantRefinementBaseDiagnostic],
) -> (BaseDiagnosticsByFile, Vec<usize>) {
    let mut bases_by_file = BaseDiagnosticsByFile::new();
    let mut file_order = Vec::new();
    let mut ordered_indices = Vec::with_capacity(base_diagnostics.len());
    for (index, diagnostic) in base_diagnostics.iter().enumerate() {
        record_variant_refinement_base_work(1);
        if let Some(span) = diagnostic.diagnostic.span.as_ref() {
            match bases_by_file.entry(span.file.clone()) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    entry.get_mut().0.push(index);
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    file_order.push(span.file.clone());
                    entry.insert((vec![index], 0));
                }
            }
        } else {
            ordered_indices.push(index);
        }
    }
    for file in &file_order {
        let (indices, _) = bases_by_file
            .get_mut(file)
            .expect("base diagnostic file was indexed");
        radix_sort_base_diagnostic_indices(indices, base_diagnostics);
        ordered_indices.extend(indices.iter().copied());
    }
    (bases_by_file, ordered_indices)
}

fn merge_existing_base_diagnostics(
    diagnostics: &mut Vec<Diagnostic>,
    bases_by_file: &mut BaseDiagnosticsByFile,
    bases: &mut [Option<VariantRefinementBaseDiagnostic>],
) {
    let existing = std::mem::take(diagnostics);
    let (future_min_end, last_same_span) = index_existing_diagnostic_order(&existing);
    diagnostics.reserve(existing.len() + bases.len());
    for (existing_index, existing_diagnostic) in existing.into_iter().enumerate() {
        record_variant_refinement_base_work(1);
        let mut matching_bases = existing_diagnostic
            .span
            .as_ref()
            .and_then(|span| bases_by_file.get_mut(&span.file));
        while matching_bases.as_ref().is_some_and(|(indices, cursor)| {
            let Some(index) = indices.get(*cursor) else {
                return false;
            };
            record_variant_refinement_base_work(1);
            base_precedes_existing(
                bases[*index].as_ref().expect("pending base diagnostic"),
                &existing_diagnostic,
            ) && !future_existing_precedes_base(
                bases[*index].as_ref().expect("pending base diagnostic"),
                &existing_diagnostic,
                existing_index,
                future_min_end[existing_index],
                &last_same_span,
            )
        }) {
            let (indices, cursor) = matching_bases.as_mut().expect("matched base file");
            let index = indices[*cursor];
            *cursor += 1;
            diagnostics.push(
                bases[index]
                    .take()
                    .expect("pending base diagnostic")
                    .diagnostic,
            );
        }
        diagnostics.push(existing_diagnostic);
    }
}

fn index_existing_diagnostic_order(
    existing: &[Diagnostic],
) -> (Vec<Option<usize>>, DiagnosticSpansByFile) {
    let mut future_min_end = vec![None; existing.len()];
    let mut min_end_by_file = HashMap::<SourcePath, usize>::new();
    let mut last_same_span = DiagnosticSpansByFile::new();
    for (index, diagnostic) in existing.iter().enumerate().rev() {
        record_variant_refinement_base_work(1);
        let Some(span) = diagnostic.span.as_ref() else {
            continue;
        };
        future_min_end[index] = min_end_by_file.get(&span.file).copied();
        min_end_by_file
            .entry(span.file.clone())
            .and_modify(|end| *end = (*end).min(span.end.offset))
            .or_insert(span.end.offset);
        if diagnostic.id != "type.invalid_annotation" {
            last_same_span
                .entry(span.file.clone())
                .or_default()
                .entry((span.start.offset, span.end.offset))
                .or_insert(index);
        }
    }
    (future_min_end, last_same_span)
}

fn future_existing_precedes_base(
    base: &VariantRefinementBaseDiagnostic,
    existing: &Diagnostic,
    existing_index: usize,
    future_min_end: Option<usize>,
    last_same_span: &DiagnosticSpansByFile,
) -> bool {
    let Some(span) = base.diagnostic.span.as_ref() else {
        return false;
    };
    let later_alternative_casing = existing.id == "name.invalid_case"
        && existing.span.as_ref().is_some_and(|existing_span| {
            base.union_scope.as_ref().is_some_and(|union_scope| {
                existing_span.file == union_scope.file
                    && existing_span.start.offset >= base.ordering_scope.end.offset
                    && existing_span.start.offset >= union_scope.start.offset
                    && existing_span.end.offset <= union_scope.end.offset
            })
        });
    (future_min_end.is_some_and(|end| end <= span.start.offset) && !later_alternative_casing)
        || last_same_span
            .get(&span.file)
            .and_then(|positions| positions.get(&(span.start.offset, span.end.offset)))
            .is_some_and(|index| *index > existing_index)
}

fn radix_sort_base_diagnostic_indices(
    indices: &mut Vec<usize>,
    diagnostics: &[VariantRefinementBaseDiagnostic],
) {
    record_variant_refinement_base_work(indices.len());
    let mut scratch = vec![0; indices.len()];
    for use_end in [true, false] {
        for shift in (0..usize::BITS).step_by(8) {
            let mut counts = [0usize; 256];
            record_variant_refinement_base_work(counts.len());
            for &index in indices.iter() {
                record_variant_refinement_base_work(1);
                let span = diagnostics[index]
                    .diagnostic
                    .span
                    .as_ref()
                    .expect("indexed base diagnostic has a source span");
                let key = if use_end {
                    span.end.offset
                } else {
                    span.start.offset
                };
                counts[(key >> shift) as u8 as usize] += 1;
            }
            let mut next = 0;
            for count in &mut counts {
                record_variant_refinement_base_work(1);
                let start = next;
                next += *count;
                *count = start;
            }
            for &index in indices.iter() {
                record_variant_refinement_base_work(1);
                let span = diagnostics[index]
                    .diagnostic
                    .span
                    .as_ref()
                    .expect("indexed base diagnostic has a source span");
                let key = if use_end {
                    span.end.offset
                } else {
                    span.start.offset
                };
                let bucket = (key >> shift) as u8 as usize;
                scratch[counts[bucket]] = index;
                counts[bucket] += 1;
            }
            std::mem::swap(indices, &mut scratch);
        }
    }
}

fn base_precedes_existing(base: &VariantRefinementBaseDiagnostic, existing: &Diagnostic) -> bool {
    let (Some(span), Some(existing_span)) = (base.diagnostic.span.as_ref(), existing.span.as_ref())
    else {
        return false;
    };
    if existing.id == "name.invalid_case"
        && existing_span.file == base.ordering_scope.file
        && existing_span.start.offset >= base.ordering_scope.start.offset
        && existing_span.end.offset <= base.ordering_scope.end.offset
    {
        return false;
    }
    existing_span.file == span.file
        && (existing_span.start.offset >= span.end.offset
            || (existing.id == "type.invalid_annotation"
                && existing_span.start.offset == span.start.offset
                && existing_span.end.offset >= span.end.offset))
}

#[cfg(test)]
thread_local! {
    static VARIANT_REFINEMENT_BASE_WORK: std::cell::Cell<VariantRefinementBaseWork> =
        const { std::cell::Cell::new(VariantRefinementBaseWork::new()) };
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct VariantRefinementBaseWork {
    pub(crate) annotation_collection: usize,
    pub(crate) casing_collection: usize,
    pub(crate) enclosing_context: usize,
    pub(crate) recovery_comparison: usize,
    pub(crate) diagnostic_ordering: usize,
}

#[cfg(test)]
impl VariantRefinementBaseWork {
    const fn new() -> Self {
        Self {
            annotation_collection: 0,
            casing_collection: 0,
            enclosing_context: 0,
            recovery_comparison: 0,
            diagnostic_ordering: 0,
        }
    }
}

#[cfg(test)]
fn update_variant_refinement_base_work(update: impl FnOnce(&mut VariantRefinementBaseWork)) {
    VARIANT_REFINEMENT_BASE_WORK.with(|work| {
        let mut current = work.get();
        update(&mut current);
        work.set(current);
    });
}

#[cfg(test)]
pub(crate) fn record_variant_refinement_base_work(amount: usize) {
    update_variant_refinement_base_work(|work| work.diagnostic_ordering += amount);
}

#[cfg(not(test))]
pub(crate) fn record_variant_refinement_base_work(_amount: usize) {}

#[cfg(test)]
pub(crate) fn record_variant_refinement_annotation_work(amount: usize) {
    update_variant_refinement_base_work(|work| work.annotation_collection += amount);
}

#[cfg(not(test))]
pub(crate) fn record_variant_refinement_annotation_work(_amount: usize) {}

#[cfg(test)]
pub(crate) fn record_variant_refinement_casing_work(amount: usize) {
    update_variant_refinement_base_work(|work| work.casing_collection += amount);
}

#[cfg(not(test))]
pub(crate) fn record_variant_refinement_casing_work(_amount: usize) {}

#[cfg(test)]
pub(crate) fn record_variant_refinement_enclosing_context_work(amount: usize) {
    update_variant_refinement_base_work(|work| work.enclosing_context += amount);
}

#[cfg(not(test))]
pub(crate) fn record_variant_refinement_enclosing_context_work(_amount: usize) {}

#[cfg(test)]
pub(crate) fn record_variant_refinement_recovery_work(amount: usize) {
    update_variant_refinement_base_work(|work| work.recovery_comparison += amount);
}

#[cfg(not(test))]
pub(crate) fn record_variant_refinement_recovery_work(_amount: usize) {}

#[cfg(test)]
pub(crate) fn reset_variant_refinement_base_work() {
    VARIANT_REFINEMENT_BASE_WORK.with(|work| work.set(VariantRefinementBaseWork::new()));
}

#[cfg(test)]
pub(crate) fn take_variant_refinement_base_work() -> VariantRefinementBaseWork {
    VARIANT_REFINEMENT_BASE_WORK.with(|work| work.replace(VariantRefinementBaseWork::new()))
}

fn check_module_declarations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_invalid_name_casing(module, environment));
    diagnostics.extend(check_duplicate_function_names(module));
    diagnostics.extend(check_duplicate_type_names(module));
    diagnostics.extend(check_duplicate_effect_names(module));
    diagnostics.extend(check_duplicate_schema_names(module));
    diagnostics.extend(check_duplicate_constructor_names(module));
    diagnostics.extend(check_module_boundary(module));
    diagnostics.extend(check_duplicate_use_aliases(module));
    diagnostics.extend(check_reserved_prelude_aliases(module));
    diagnostics.extend(check_public_aliases(module, &environment.adts));
    for declaration in &module.types {
        diagnostics.extend(check_public_adt_variant_refinement_visibility(
            declaration,
            environment,
        ));
    }
    diagnostics.extend(check_schema_field_primitives(module));
    diagnostics.extend(check_schema_type_references(module));
    diagnostics.extend(check_handler_declarations(module, environment));

    diagnostics
}

fn check_function_declaration_and_body(
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    diagnostics.extend(check_declared_effect_labels(function, environment));
    let mut visibility_diagnostics = Vec::new();
    if function.visibility == Visibility::Public {
        let (visibility, boundary): (Vec<_>, Vec<_>) =
            check_public_function_boundary(function, environment)
                .into_iter()
                .partition(|diagnostic| diagnostic.id == "type.variant_refinement_private");
        diagnostics.extend(boundary);
        visibility_diagnostics = visibility;
    }
    if function.kind == FunctionKind::Test {
        diagnostics.extend(check_test_declaration_boundary(function));
    }
    diagnostics.extend(check_function_body(
        function,
        environment,
        variant_diagnostics,
    ));
    diagnostics.extend(visibility_diagnostics);

    diagnostics
}

pub fn lower_checked_surface_module(module: &SurfaceModule) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    lower_analyzed_surface_module(module, analyze_surface_module(module))
}

pub fn lower_project_reachable_surface_module(module: &SurfaceModule) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    let environment = TypeEnvironment::from_module(module);
    lower_project_reachable_surface_module_with_environment(module, environment)
}

pub fn lower_project_reachable_surface_module_with_standard_environment(
    module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    let environment = TypeEnvironment::from_module_with_standard(module, standard);
    lower_project_reachable_surface_module_with_environment(module, environment)
}

pub fn lower_project_reachable_surface_modules_with_standard_environment(
    reachable_module: &SurfaceModule,
    selected_standard_module: &SurfaceModule,
    standard: &ReusableStandardEnvironment,
) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    let environment = TypeEnvironment::from_application_module_with_standard(
        reachable_module,
        selected_standard_module,
        standard,
    );
    lower_project_reachable_surface_module_with_environment(reachable_module, environment)
}

fn lower_project_reachable_surface_module_with_environment(
    module: &SurfaceModule,
    environment: TypeEnvironment,
) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    let diagnostics = analyze_surface_module_with_environment(
        module,
        &environment,
        should_validate_standard_bodies(module),
    );
    lower_analyzed_surface_module_with_environment(
        module,
        diagnostics,
        &environment,
        SurfaceLoweringMode::Execution,
    )
}

mod helpers;
use helpers::should_validate_standard_bodies;

pub fn lower_analyzed_surface_module(
    module: &SurfaceModule,
    diagnostics: Vec<Diagnostic>,
) -> LoweredSurfaceModule {
    if let Err(failure) = validate_source_less_lookup_registries() {
        return lowered_internal_failure(vec![failure.diagnostic()]);
    }
    let environment = TypeEnvironment::from_module(module);
    lower_analyzed_surface_module_with_environment(
        module,
        diagnostics,
        &environment,
        SurfaceLoweringMode::Module,
    )
}

#[derive(Clone, Copy)]
enum SurfaceLoweringMode {
    Module,
    ProjectCheck,
    Execution,
}

fn lowered_internal_failure(diagnostics: Vec<Diagnostic>) -> LoweredSurfaceModule {
    LoweredSurfaceModule {
        diagnostics,
        core: None,
        ir: None,
        unpublished_declarations: Vec::new(),
    }
}

fn lower_analyzed_surface_module_with_environment(
    module: &SurfaceModule,
    mut diagnostics: Vec<Diagnostic>,
    environment: &TypeEnvironment,
    mode: SurfaceLoweringMode,
) -> LoweredSurfaceModule {
    let unpublished_declarations =
        unpublished_public_declarations(module, environment, &diagnostics);
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return LoweredSurfaceModule {
            diagnostics,
            core: None,
            ir: None,
            unpublished_declarations,
        };
    }

    let lowered_core = match mode {
        SurfaceLoweringMode::Module => lower_surface_module_to_core(module, environment),
        SurfaceLoweringMode::ProjectCheck => {
            lower_project_surface_module_to_core(module, environment)
        }
        SurfaceLoweringMode::Execution => {
            lower_executable_surface_module_to_core(module, environment)
        }
    };
    diagnostics.extend(lowered_core.diagnostics);
    let ir = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        None
    } else {
        lower_checked_core(&lowered_core.program)
            .ok()
            .map(|mut ir| {
                ir.schema_decoders = schema::ir::schema_decode_specs(module);
                ir
            })
    };

    LoweredSurfaceModule {
        diagnostics,
        core: Some(lowered_core.program),
        ir,
        unpublished_declarations,
    }
}

fn unpublished_public_declarations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &[Diagnostic],
) -> Vec<UnpublishedDeclaration> {
    let type_causes = type_publication_diagnostic_causes(module, environment, diagnostics);
    let function_causes =
        function_publication_diagnostic_causes(module, environment, diagnostics, &type_causes);
    let mut declarations = module
        .types
        .iter()
        .filter(|declaration| declaration.visibility == Visibility::Public)
        .filter_map(|declaration| {
            let name = declaration.name.as_ref()?;
            let descriptor = environment
                .adts
                .descriptor_for_type_path_including_quarantined(
                    name,
                    declaration.params.len(),
                    declaration.module_name.as_deref(),
                    &environment.uses,
                )?;
            environment
                .adts
                .descriptor_is_quarantined(descriptor)
                .then(|| UnpublishedDeclaration {
                    kind: "type",
                    module_name: declaration.module_name.clone(),
                    name: name.clone(),
                    span: declaration.span.clone(),
                    causal_diagnostics: type_causes
                        .get(&descriptor.identity())
                        .cloned()
                        .unwrap_or_default(),
                })
        })
        .chain(
            module
                .functions
                .iter()
                .filter(|function| {
                    function.kind == FunctionKind::Function
                        && function.visibility == Visibility::Public
                        && !environment.function_is_published(function)
                })
                .filter_map(|function| {
                    Some(UnpublishedDeclaration {
                        kind: "function",
                        module_name: function.module_name.clone(),
                        name: function.name.clone()?,
                        span: function.span.clone(),
                        causal_diagnostics: function_causes
                            .get(&(function.module_name.clone(), function.name.clone()?))
                            .cloned()
                            .unwrap_or_default(),
                    })
                }),
        )
        .collect::<Vec<_>>();
    declarations.extend(module.aliases.iter().filter_map(|alias| {
        let name = alias.name.as_ref()?;
        let unpublished = match alias.kind {
            PublicAliasKind::Type => environment
                .adts
                .descriptor_for_type_path_including_quarantined(
                    name,
                    0,
                    alias.module_name.as_deref(),
                    &environment.uses,
                )
                .is_some_and(|descriptor| environment.adts.descriptor_is_quarantined(descriptor)),
            PublicAliasKind::Function => environment.function(name).is_none(),
            _ => false,
        };
        let causal_diagnostics = match alias.kind {
            PublicAliasKind::Type => environment
                .adts
                .descriptor_for_type_path_including_quarantined(
                    name,
                    0,
                    alias.module_name.as_deref(),
                    &environment.uses,
                )
                .map(|descriptor| {
                    environment
                        .adts
                        .terminal_descriptor_for_alias(descriptor)
                        .unwrap_or_else(|| environment.adts.canonical_descriptor_for(descriptor))
                })
                .and_then(|descriptor| type_causes.get(&descriptor.identity()))
                .cloned()
                .unwrap_or_default(),
            PublicAliasKind::Function => function_causes
                .get(&(alias.module_name.clone(), name.clone()))
                .cloned()
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        unpublished.then(|| UnpublishedDeclaration {
            kind: match alias.kind {
                PublicAliasKind::Type => "type_alias",
                PublicAliasKind::Function => "function_alias",
                _ => "alias",
            },
            module_name: alias.module_name.clone(),
            name: name.clone(),
            span: alias.span.clone(),
            causal_diagnostics,
        })
    }));
    declarations
}

fn type_publication_diagnostic_causes(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &[Diagnostic],
) -> HashMap<String, Vec<PublicationDiagnosticCause>> {
    struct TypeCauseEntry {
        identity: String,
        dependencies: Vec<String>,
    }

    let mut causes = HashMap::new();
    let mut entries = Vec::new();
    for declaration in module
        .types
        .iter()
        .filter(|declaration| declaration.visibility == Visibility::Public)
    {
        let Some(name) = declaration.name.as_ref() else {
            continue;
        };
        let Some(descriptor) = environment
            .adts
            .descriptor_for_type_path_including_quarantined(
                name,
                declaration.params.len(),
                declaration.module_name.as_deref(),
                &environment.uses,
            )
        else {
            continue;
        };
        let identity = descriptor.identity();
        causes.insert(
            identity.clone(),
            diagnostic_causes_within(&declaration.span, diagnostics),
        );
        let dependencies =
            crate::analysis::adt_publication_dependency_identities(declaration, environment)
                .into_iter()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
        entries.push(TypeCauseEntry {
            identity,
            dependencies,
        });
    }

    let mut reverse_dependencies: HashMap<String, Vec<String>> = HashMap::new();
    let mut declaration_order = Vec::with_capacity(entries.len());
    for entry in entries {
        declaration_order.push(entry.identity.clone());
        for dependency in entry.dependencies {
            reverse_dependencies
                .entry(dependency)
                .or_default()
                .push(entry.identity.clone());
            #[cfg(test)]
            record_publication_propagation_work(|work| work.reverse_edges += 1);
        }
    }
    propagate_publication_causes(&declaration_order, &reverse_dependencies, &mut causes);
    causes
}

fn function_publication_diagnostic_causes(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &[Diagnostic],
    type_causes: &HashMap<String, Vec<PublicationDiagnosticCause>>,
) -> HashMap<(Option<String>, String), Vec<PublicationDiagnosticCause>> {
    let mut causes = HashMap::new();
    let mut declaration_order = Vec::new();
    for function in module.functions.iter().filter(|function| {
        function.kind == FunctionKind::Function && function.visibility == Visibility::Public
    }) {
        let Some(name) = function.name.as_ref() else {
            continue;
        };
        let mut declaration_causes = diagnostic_causes_within(&function.span, diagnostics);
        let dependencies =
            crate::analysis::function_publication_dependency_identities(function, environment)
                .into_iter()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
        let mut admitted_causes = declaration_causes.iter().cloned().collect::<HashSet<_>>();
        for cause in dependencies
            .iter()
            .flat_map(|dependency| type_causes.get(dependency).cloned().unwrap_or_default())
        {
            if admitted_causes.insert(cause.clone()) {
                declaration_causes.push(cause);
            }
        }
        let key = (function.module_name.clone(), name.clone());
        declaration_order.push(key.clone());
        causes.insert(key, declaration_causes);
    }
    let mut reverse_dependencies: HashMap<(Option<String>, String), Vec<(Option<String>, String)>> =
        HashMap::new();
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
    {
        let Some(name) = alias.name.as_ref() else {
            continue;
        };
        let key = (alias.module_name.clone(), name.clone());
        declaration_order.push(key.clone());
        causes.entry(key.clone()).or_default();
        let Some(target) = function_alias_target_key(module, alias) else {
            continue;
        };
        reverse_dependencies.entry(target).or_default().push(key);
        #[cfg(test)]
        record_publication_propagation_work(|work| work.reverse_edges += 1);
    }
    propagate_publication_causes(&declaration_order, &reverse_dependencies, &mut causes);
    causes
}

fn propagate_publication_causes<Key>(
    declaration_order: &[Key],
    reverse_dependencies: &HashMap<Key, Vec<Key>>,
    causes: &mut HashMap<Key, Vec<PublicationDiagnosticCause>>,
) where
    Key: Clone + Eq + Hash,
{
    let mut admitted = HashMap::<Key, HashSet<PublicationDiagnosticCause>>::new();
    let mut worklist = VecDeque::new();
    for declaration in declaration_order {
        let declaration_causes = causes.get(declaration).cloned().unwrap_or_default();
        let declaration_admitted = admitted.entry(declaration.clone()).or_default();
        for cause in declaration_causes {
            if declaration_admitted.insert(cause.clone()) {
                worklist.push_back((declaration.clone(), cause));
                #[cfg(test)]
                record_publication_propagation_work(|work| work.admitted_cause_pairs += 1);
            }
        }
    }

    while let Some((dependency, cause)) = worklist.pop_front() {
        let Some(dependents) = reverse_dependencies.get(&dependency) else {
            continue;
        };
        for dependent in dependents {
            #[cfg(test)]
            record_publication_propagation_work(|work| work.propagation_attempts += 1);
            if admitted
                .entry(dependent.clone())
                .or_default()
                .insert(cause.clone())
            {
                causes
                    .entry(dependent.clone())
                    .or_default()
                    .push(cause.clone());
                worklist.push_back((dependent.clone(), cause.clone()));
                #[cfg(test)]
                record_publication_propagation_work(|work| work.admitted_cause_pairs += 1);
            }
        }
    }
}

fn function_alias_target_key(
    module: &SurfaceModule,
    alias: &veln_ast::PublicAlias,
) -> Option<(Option<String>, String)> {
    let name = alias.target.last()?.clone();
    if alias.target.len() == 1 {
        return Some((alias.module_name.clone(), name));
    }
    let module_path = alias.target[..alias.target.len() - 1].join("::");
    let use_decl = module.uses.iter().find(|use_decl| {
        crate::name_recovery::use_decl_matches_import_path(
            use_decl,
            &module_path,
            alias.module_name.as_deref(),
        )
    })?;
    Some((
        Some(crate::name_recovery::resolved_import_module_name(
            use_decl,
            alias.module_name.as_deref(),
        )),
        name,
    ))
}

fn diagnostic_causes_within(
    declaration_span: &veln_source::SourceSpan,
    diagnostics: &[Diagnostic],
) -> Vec<PublicationDiagnosticCause> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .filter_map(|diagnostic| {
            let span = diagnostic.span.as_ref()?;
            (span.file == declaration_span.file
                && span.start.offset >= declaration_span.start.offset
                && span.end.offset <= declaration_span.end.offset)
                .then(|| PublicationDiagnosticCause {
                    diagnostic_id: diagnostic.id.clone(),
                    span: span.clone(),
                })
        })
        .collect()
}
