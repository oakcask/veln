use std::collections::{BTreeSet, HashMap};

use veln_ast::{FunctionKind, SurfaceModule, Visibility};
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

    merge_variant_refinement_base_diagnostics(
        &mut diagnostics,
        check_variant_refinement_bases(module, environment),
    );

    diagnostics
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
    }
}

fn lower_analyzed_surface_module_with_environment(
    module: &SurfaceModule,
    mut diagnostics: Vec<Diagnostic>,
    environment: &TypeEnvironment,
    mode: SurfaceLoweringMode,
) -> LoweredSurfaceModule {
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        return LoweredSurfaceModule {
            diagnostics,
            core: None,
            ir: None,
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
    }
}
