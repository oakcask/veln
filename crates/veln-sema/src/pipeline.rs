use std::collections::{BTreeSet, HashMap};
use std::hash::Hash;

use veln_ast::{FunctionKind, SurfaceModule, Visibility};
use veln_core::CheckedProgram;
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_ir::{TypedProgram, lower_checked_core};

use crate::analysis::{
    VariantDiagnosticInterner, check_declared_effect_labels, check_duplicate_constructor_names,
    check_duplicate_effect_names, check_duplicate_function_names, check_duplicate_schema_names,
    check_duplicate_type_names, check_duplicate_use_aliases, check_function_body,
    check_handler_declarations, check_module_boundary,
    check_public_adt_variant_refinement_visibility, check_public_aliases,
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

mod diagnostic_ordering;
mod identifier_casing;
mod publication;

use diagnostic_ordering::{
    merge_variant_refinement_base_diagnostics, order_delayed_parse_diagnostics,
    order_variant_refinement_visibility_diagnostics,
};
use identifier_casing::check_invalid_name_casing;
pub use identifier_casing::{
    CanonicalTypeAnnotationIdentity, CanonicalTypeAnnotationWork,
    VariantRefinementClassificationWork, canonical_type_annotation_identities_with_context,
    canonical_type_annotation_identities_with_context_and_work,
    classified_project_qualified_path_segments,
    classified_project_qualified_path_segments_with_context,
    classified_project_qualified_path_segments_with_context_and_work,
};
use publication::unpublished_public_declarations;

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

type FunctionPublicationIdentity = (Option<String>, String);

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
    order_delayed_parse_diagnostics(&mut diagnostics);

    diagnostics
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
    pub(crate) recovery_materialization: usize,
    pub(crate) recovery_materialized_type_nodes: usize,
    pub(crate) recovery_type_nodes_cloned: usize,
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
            recovery_materialization: 0,
            recovery_materialized_type_nodes: 0,
            recovery_type_nodes_cloned: 0,
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
pub(crate) fn record_variant_refinement_recovery_materialization(amount: usize) {
    update_variant_refinement_base_work(|work| work.recovery_materialization += amount);
}

#[cfg(not(test))]
pub(crate) fn record_variant_refinement_recovery_materialization(_amount: usize) {}

#[cfg(test)]
pub(crate) fn record_variant_refinement_recovery_type_materialization(amount: usize) {
    update_variant_refinement_base_work(|work| {
        work.recovery_materialized_type_nodes += amount;
    });
}

#[cfg(not(test))]
pub(crate) fn record_variant_refinement_recovery_type_materialization(_amount: usize) {}

#[cfg(test)]
pub(crate) fn record_variant_refinement_recovery_type_clones(amount: usize) {
    update_variant_refinement_base_work(|work| {
        work.recovery_type_nodes_cloned += amount;
    });
}

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

    diagnostics.extend(
        module
            .module_values
            .iter()
            .map(module_value_not_executable_diagnostic),
    );
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
    diagnostics.extend(check_schema_field_primitives(module, environment));
    diagnostics.extend(check_schema_type_references(module));
    diagnostics.extend(check_handler_declarations(module, environment));

    diagnostics
}

fn module_value_not_executable_diagnostic(value: &veln_ast::ModuleValueDecl) -> Diagnostic {
    let name = value.name.as_deref().unwrap_or("<missing>");
    let mut diagnostic = Diagnostic::new(
        "module.value_not_executable",
        Severity::Error,
        DiagnosticKind::Module,
        format!("module value declaration `{name}` is not executable"),
        Some(value.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_value")),
            ("name", JsonValue::string(name)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(
            "The declaration is available to parsing and formatting, but semantic analysis and code generation do not support module values yet.",
        ),
    )]));
    diagnostic
}

#[derive(Clone, Copy)]
struct DiagnosticSpan<'a>(&'a veln_source::SourceSpan);

impl PartialEq for DiagnosticSpan<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for DiagnosticSpan<'_> {}

impl Hash for DiagnosticSpan<'_> {
    fn hash<Hasher>(&self, state: &mut Hasher)
    where
        Hasher: std::hash::Hasher,
    {
        self.0.file.hash(state);
        self.0.start.line.hash(state);
        self.0.start.column.hash(state);
        self.0.start.offset.hash(state);
        self.0.end.line.hash(state);
        self.0.end.column.hash(state);
        self.0.end.offset.hash(state);
        self.0.original_path().hash(state);
        self.0
            .original_start()
            .map(|position| (position.line, position.column, position.offset))
            .hash(state);
        self.0
            .original_end()
            .map(|position| (position.line, position.column, position.offset))
            .hash(state);
    }
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleValueGateWork {
    pub(crate) indexed_values: usize,
    pub(crate) scanned_diagnostics: usize,
    pub(crate) span_index_lookups: usize,
    pub(crate) emitted_canonical_diagnostics: usize,
}

#[cfg(test)]
impl ModuleValueGateWork {
    pub(crate) fn total(self) -> usize {
        self.indexed_values
            + self.scanned_diagnostics
            + self.span_index_lookups
            + self.emitted_canonical_diagnostics
    }
}

#[cfg(test)]
thread_local! {
    static MODULE_VALUE_GATE_WORK: std::cell::Cell<ModuleValueGateWork> =
        const { std::cell::Cell::new(ModuleValueGateWork {
            indexed_values: 0,
            scanned_diagnostics: 0,
            span_index_lookups: 0,
            emitted_canonical_diagnostics: 0,
        }) };
}

#[cfg(test)]
fn record_module_value_gate_work(update: impl FnOnce(&mut ModuleValueGateWork)) {
    MODULE_VALUE_GATE_WORK.with(|work| {
        let mut current = work.get();
        update(&mut current);
        work.set(current);
    });
}

#[cfg(test)]
pub(crate) fn reset_module_value_gate_work() {
    MODULE_VALUE_GATE_WORK.set(ModuleValueGateWork::default());
}

#[cfg(test)]
pub(crate) fn take_module_value_gate_work() -> ModuleValueGateWork {
    MODULE_VALUE_GATE_WORK.replace(ModuleValueGateWork::default())
}

fn enforce_module_value_lowering_gate(
    module: &SurfaceModule,
    diagnostics: Vec<Diagnostic>,
) -> Vec<Diagnostic> {
    let mut value_by_span = HashMap::with_capacity(module.module_values.len());
    for (index, value) in module.module_values.iter().enumerate() {
        value_by_span.insert(DiagnosticSpan(&value.span), index);
        #[cfg(test)]
        record_module_value_gate_work(|work| work.indexed_values += 1);
    }

    let mut emitted = vec![false; module.module_values.len()];
    let mut repaired = Vec::with_capacity(diagnostics.len() + module.module_values.len());
    for diagnostic in diagnostics {
        #[cfg(test)]
        record_module_value_gate_work(|work| work.scanned_diagnostics += 1);
        let value_index = if diagnostic.id == "module.value_not_executable" {
            diagnostic.span.as_ref().and_then(|span| {
                #[cfg(test)]
                record_module_value_gate_work(|work| work.span_index_lookups += 1);
                value_by_span.get(&DiagnosticSpan(span)).copied()
            })
        } else {
            None
        };
        let Some(value_index) = value_index else {
            repaired.push(diagnostic);
            continue;
        };
        if emitted[value_index] {
            continue;
        }
        repaired.push(module_value_not_executable_diagnostic(
            &module.module_values[value_index],
        ));
        emitted[value_index] = true;
        #[cfg(test)]
        record_module_value_gate_work(|work| work.emitted_canonical_diagnostics += 1);
    }

    for (value_index, value) in module.module_values.iter().enumerate() {
        if emitted[value_index] {
            continue;
        }
        repaired.push(module_value_not_executable_diagnostic(value));
        #[cfg(test)]
        record_module_value_gate_work(|work| work.emitted_canonical_diagnostics += 1);
    }
    repaired
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
    diagnostics: Vec<Diagnostic>,
    environment: &TypeEnvironment,
    mode: SurfaceLoweringMode,
) -> LoweredSurfaceModule {
    let mut diagnostics = enforce_module_value_lowering_gate(module, diagnostics);
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
