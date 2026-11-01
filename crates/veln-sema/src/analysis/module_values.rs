use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};

use veln_ast::{BodyLine, BodyLineKind, Function, FunctionKind, SurfaceModule, Visibility};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};

use super::{FunctionChecker, VariantDiagnosticInterner, check_type_effect_labels};
use crate::diagnostics::span_json;
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_annotation;
use crate::types::{ModuleValueIdentity, ModuleValueIndex, TypeEnvironment};

mod admission;
mod availability;
mod check_state;
mod cycles;
mod indexing;
mod visibility;

use admission::{check_admitted_expr, collect_dependencies};
use check_state::ModuleValueCheckState;
use cycles::{cycle_diagnostics, strongly_connected_index_components};
use indexing::{
    colliding_module_value_identities, initially_invalid_values,
    invalid_cased_module_value_identities, module_value_indices,
};
use visibility::public_named_type_diagnostics;

#[derive(Clone)]
pub(super) struct Dependency {
    target: usize,
    span: veln_source::SourceSpan,
}

pub(super) struct DependencyCollector<'a> {
    dependencies: &'a mut Vec<Dependency>,
    collision_dependencies: &'a mut Vec<ModuleValueIdentity>,
}

pub(crate) struct ModuleValueAnalysis {
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) invalid: BTreeSet<ModuleValueIdentity>,
    invalid_indices: Vec<bool>,
    // Collisions, invalid casing, and initializer cycles cannot recover when
    // imported candidates change. Other failures are rechecked at each change.
    intrinsic_invalid: BTreeSet<ModuleValueIdentity>,
    dependencies: Vec<Vec<Dependency>>,
    collision_dependencies: Vec<Vec<ModuleValueIdentity>>,
    direct_causes: Vec<Vec<ModuleValuePublicationCause>>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ModuleValuePublicationProvenance {
    pub(crate) values: Vec<ModuleValuePublicationValue>,
    pub(crate) callable_collisions: Vec<(ModuleValueIdentity, Diagnostic)>,
}

#[derive(Clone, Debug)]
pub(crate) struct ModuleValuePublicationValue {
    pub(crate) identity: Option<ModuleValueIdentity>,
    pub(crate) invalid: bool,
    pub(crate) dependencies: Vec<usize>,
    pub(crate) collision_dependencies: Vec<ModuleValueIdentity>,
    pub(crate) direct_causes: Vec<ModuleValuePublicationCause>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModuleValuePublicationCause {
    pub(crate) package_name: Option<String>,
    pub(crate) diagnostic_id: String,
    pub(crate) diagnostic_message: Option<String>,
    pub(crate) span: veln_source::SourceSpan,
    pub(crate) diagnostic_index: Option<usize>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleValueAvailabilityWork {
    pub(crate) semantic_visits: usize,
    pub(crate) owner_index_entries: usize,
    pub(crate) owner_recheck_lookups: usize,
    pub(crate) dependency_index_entries: usize,
    pub(crate) peak_retained_dependency_state: usize,
    pub(crate) declaration_recheck_lookups: usize,
    pub(crate) retained_history_insertions: usize,
    // Identity-sized availability comparisons, copies, and traversals. Full
    // snapshot work must add one step per identity so adjacent-size tests see it.
    pub(crate) availability_state_steps: usize,
    pub(crate) availability_waves: usize,
    pub(crate) full_exposure_rebuild_entries: usize,
    pub(crate) availability_updates: usize,
    pub(crate) peak_pending_owners: usize,
    pub(crate) peak_retained_availability_identities: usize,
    pub(crate) retained_cyclic_component_identities: usize,
}

#[cfg(test)]
thread_local! {
    static MODULE_VALUE_AVAILABILITY_WORK:
        std::cell::Cell<ModuleValueAvailabilityWork> =
        const { std::cell::Cell::new(ModuleValueAvailabilityWork {
            semantic_visits: 0,
            owner_index_entries: 0,
            owner_recheck_lookups: 0,
            dependency_index_entries: 0,
            peak_retained_dependency_state: 0,
            declaration_recheck_lookups: 0,
            retained_history_insertions: 0,
            availability_state_steps: 0,
            availability_waves: 0,
            full_exposure_rebuild_entries: 0,
            availability_updates: 0,
            peak_pending_owners: 0,
            peak_retained_availability_identities: 0,
            retained_cyclic_component_identities: 0,
        }) };
}

#[cfg(test)]
fn update_module_value_availability_work(update: impl FnOnce(&mut ModuleValueAvailabilityWork)) {
    MODULE_VALUE_AVAILABILITY_WORK.set({
        let mut work = MODULE_VALUE_AVAILABILITY_WORK.get();
        update(&mut work);
        work
    });
}

#[cfg(test)]
pub(crate) fn reset_module_value_availability_work() {
    MODULE_VALUE_AVAILABILITY_WORK.set(ModuleValueAvailabilityWork::default());
}

#[cfg(test)]
pub(crate) fn take_module_value_availability_work() -> ModuleValueAvailabilityWork {
    MODULE_VALUE_AVAILABILITY_WORK.replace(ModuleValueAvailabilityWork::default())
}

#[cfg(test)]
pub(crate) fn record_module_value_exposure_rebuild(entries: usize) {
    update_module_value_availability_work(|work| {
        work.full_exposure_rebuild_entries += entries;
    });
}

#[cfg(not(test))]
pub(crate) fn record_module_value_exposure_rebuild(_entries: usize) {}

#[cfg(test)]
pub(crate) fn record_module_value_availability_update() {
    update_module_value_availability_work(|work| work.availability_updates += 1);
}

#[cfg(not(test))]
pub(crate) fn record_module_value_availability_update() {}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ModuleValueAnnotationDedupWork {
    pub(crate) indexed_diagnostics: usize,
    pub(crate) path_lookups: usize,
}

#[cfg(test)]
thread_local! {
    static MODULE_VALUE_ANNOTATION_DEDUP_WORK:
        std::cell::Cell<ModuleValueAnnotationDedupWork> =
        const { std::cell::Cell::new(ModuleValueAnnotationDedupWork {
            indexed_diagnostics: 0,
            path_lookups: 0,
        }) };
}

#[cfg(test)]
pub(crate) fn reset_module_value_annotation_dedup_work() {
    MODULE_VALUE_ANNOTATION_DEDUP_WORK.set(ModuleValueAnnotationDedupWork::default());
}

#[cfg(test)]
pub(crate) fn take_module_value_annotation_dedup_work() -> ModuleValueAnnotationDedupWork {
    MODULE_VALUE_ANNOTATION_DEDUP_WORK.replace(ModuleValueAnnotationDedupWork::default())
}

#[cfg(test)]
fn record_module_value_annotation_dedup_work(
    update: impl FnOnce(&mut ModuleValueAnnotationDedupWork),
) {
    MODULE_VALUE_ANNOTATION_DEDUP_WORK.set({
        let mut work = MODULE_VALUE_ANNOTATION_DEDUP_WORK.get();
        update(&mut work);
        work
    });
}

pub(crate) fn check_module_values(
    module: &SurfaceModule,
    environment: &mut TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
) -> ModuleValueAnalysis {
    let colliding_identities = colliding_module_value_identities(module);
    let mut unavailable = colliding_identities.clone();
    unavailable.extend(invalid_cased_module_value_identities(module));
    environment.quarantine_module_values(&unavailable);

    let initial =
        check_module_values_once(module, environment, variant_diagnostics, &BTreeSet::new());
    let reverse_dependencies = availability::reverse_dependencies(module, environment);
    let mut pending_owners = BTreeSet::new();
    availability::apply_availability_changes(
        &mut unavailable,
        &initial.invalid,
        environment,
        &mut pending_owners,
    );
    let stabilized = availability::stabilize_module_value_availability(
        module,
        environment,
        variant_diagnostics,
        unavailable,
        pending_owners,
        &reverse_dependencies,
        &initial.intrinsic_invalid,
    );

    let mut final_analysis = check_module_values_once(
        module,
        environment,
        variant_diagnostics,
        &stabilized.rejected,
    );
    attach_recurrent_causes(module, &mut final_analysis, stabilized.recurrent_causes);
    debug_assert_eq!(stabilized.unavailable, final_analysis.invalid);
    final_analysis
}

fn attach_recurrent_causes(
    module: &SurfaceModule,
    analysis: &mut ModuleValueAnalysis,
    recurrent_causes: Vec<(ModuleValueIdentity, Diagnostic)>,
) {
    let mut diagnostic_indices = analysis
        .diagnostics
        .iter()
        .enumerate()
        .map(|(index, diagnostic)| (module_value_diagnostic_key(diagnostic), index))
        .collect::<HashMap<_, _>>();
    let value_indices = module
        .module_values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| {
            Some((
                (
                    value.package_name.clone(),
                    value.module_name.clone(),
                    value.name.clone()?,
                ),
                index,
            ))
        })
        .collect::<HashMap<_, _>>();
    for (identity, diagnostic) in recurrent_causes {
        attach_recurrent_cause(
            identity,
            diagnostic,
            &mut diagnostic_indices,
            &value_indices,
            analysis,
        );
    }
}

fn attach_recurrent_cause(
    identity: ModuleValueIdentity,
    diagnostic: Diagnostic,
    diagnostic_indices: &mut HashMap<ModuleValueDiagnosticKey, usize>,
    value_indices: &HashMap<ModuleValueIdentity, usize>,
    analysis: &mut ModuleValueAnalysis,
) {
    let key = module_value_diagnostic_key(&diagnostic);
    let diagnostic_index = if let Some(index) = diagnostic_indices.get(&key) {
        *index
    } else {
        let index = analysis.diagnostics.len();
        diagnostic_indices.insert(key, index);
        analysis.diagnostics.push(diagnostic);
        index
    };
    let Some(value_index) = value_indices.get(&identity).copied() else {
        return;
    };
    let diagnostic = &analysis.diagnostics[diagnostic_index];
    let Some(span) = diagnostic.span.clone() else {
        return;
    };
    analysis.direct_causes[value_index].push(ModuleValuePublicationCause {
        package_name: identity.0,
        diagnostic_id: diagnostic.id.clone(),
        diagnostic_message: Some(diagnostic.message.as_str().to_owned()),
        span,
        diagnostic_index: Some(diagnostic_index),
    });
}

fn module_value_diagnostic_key(diagnostic: &Diagnostic) -> ModuleValueDiagnosticKey {
    (
        diagnostic.id.clone(),
        diagnostic.span.as_ref().map(span_identity),
        diagnostic.message.as_str().to_owned(),
    )
}
type ModuleValueDiagnosticKey = (String, Option<SpanIdentity>, String);

fn check_module_values_once(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
    forced_invalid: &BTreeSet<ModuleValueIdentity>,
) -> ModuleValueAnalysis {
    let colliding_identities = colliding_module_value_identities(module);
    let invalid_cased_identities = invalid_cased_module_value_identities(module);
    let mut intrinsic_invalid = colliding_identities.clone();
    intrinsic_invalid.extend(invalid_cased_identities.iter().cloned());
    intrinsic_invalid.extend(forced_invalid.iter().cloned());
    let value_indices = module_value_indices(module, &colliding_identities);
    let mut state = ModuleValueCheckState::new(module, &intrinsic_invalid);

    state.check_declarations(
        module,
        environment,
        variant_diagnostics,
        &invalid_cased_identities,
        &value_indices,
    );
    state.apply_cycle_failures(module, &mut intrinsic_invalid);
    state.finish(module, intrinsic_invalid)
}

fn publication_cause(
    package_name: Option<String>,
    diagnostic: &Diagnostic,
    diagnostic_index: Option<usize>,
) -> Option<ModuleValuePublicationCause> {
    Some(ModuleValuePublicationCause {
        package_name,
        diagnostic_id: diagnostic.id.clone(),
        diagnostic_message: Some(diagnostic.message.as_str().to_owned()),
        span: diagnostic.span.clone()?,
        diagnostic_index,
    })
}

fn invalid_module_value_identities(
    module: &SurfaceModule,
    invalid: &[bool],
) -> BTreeSet<ModuleValueIdentity> {
    module
        .module_values
        .iter()
        .enumerate()
        .filter(|(index, value)| invalid[*index] && value.name.is_some())
        .map(|(_, value)| {
            (
                value.package_name.clone(),
                value.module_name.clone(),
                value.name.clone().expect("filtered module value name"),
            )
        })
        .collect()
}

impl ModuleValueAnalysis {
    pub(crate) fn publication_provenance(
        &self,
        module: &SurfaceModule,
    ) -> ModuleValuePublicationProvenance {
        ModuleValuePublicationProvenance {
            values: module
                .module_values
                .iter()
                .enumerate()
                .map(|(index, value)| ModuleValuePublicationValue {
                    identity: value.name.as_ref().map(|name| {
                        (
                            value.package_name.clone(),
                            value.module_name.clone(),
                            name.clone(),
                        )
                    }),
                    invalid: self.invalid_indices[index],
                    dependencies: self.dependencies[index]
                        .iter()
                        .map(|dependency| dependency.target)
                        .collect(),
                    collision_dependencies: self.collision_dependencies[index].clone(),
                    direct_causes: self.direct_causes[index].clone(),
                })
                .collect(),
            callable_collisions: Vec::new(),
        }
    }
}

fn quarantined_import_causes(
    value: &veln_ast::ModuleValueDecl,
    environment: &TypeEnvironment,
) -> Vec<veln_source::SourceSpan> {
    let mut causes = Vec::new();
    collect_quarantined_import_causes(&value.initializer, value, environment, &mut causes);
    causes
}

fn collect_quarantined_import_causes(
    expr: &veln_ast::Expr,
    value: &veln_ast::ModuleValueDecl,
    environment: &TypeEnvironment,
    causes: &mut Vec<veln_source::SourceSpan>,
) {
    let veln_ast::ExprKind::NamePath { segments, .. } = &expr.kind else {
        expr.for_each_child(&mut |child| {
            collect_quarantined_import_causes(child, value, environment, causes)
        });
        return;
    };
    if let Some(use_decl) = environment.quarantined_import_value_recovery_use(
        segments,
        value.module_name.as_deref(),
        value.package_name.as_deref(),
    ) && let Some(alias_span) = use_decl.alias_span.as_ref()
        && !causes.contains(alias_span)
    {
        causes.push(alias_span.clone());
    }
    expr.for_each_child(&mut |child| {
        collect_quarantined_import_causes(child, value, environment, causes)
    });
}

pub(super) fn check_module_value(
    value: &veln_ast::ModuleValueDecl,
    index: usize,
    value_indices: &HashMap<ModuleValueIdentity, ModuleValueIndex>,
    environment: &TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
    dependencies: &mut DependencyCollector<'_>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let has_invalid_dependency =
        collect_value_dependencies(value, value_indices, environment, dependencies);
    let diagnostic_start = diagnostics.len();
    let parsed_annotation = parse_type_annotation(&value.annotation).ok();
    let expected = parsed_annotation.clone().map(|ty| {
        environment.canonicalize_type_annotation_in_owner(
            ty,
            value.module_name.as_deref(),
            value.package_name.as_deref(),
        )
    });
    append_module_value_diagnostics(
        value,
        index,
        environment,
        variant_diagnostics,
        expected.as_ref(),
        diagnostics,
    );

    let annotation_is_closed =
        annotation_is_closed(value, environment, parsed_annotation.is_some());
    if !annotation_is_closed {
        push_unresolved_annotation_paths(value, environment, diagnostic_start, diagnostics);
    }
    diagnostics.len() != diagnostic_start || !annotation_is_closed || has_invalid_dependency
}

fn collect_value_dependencies(
    value: &veln_ast::ModuleValueDecl,
    value_indices: &HashMap<ModuleValueIdentity, ModuleValueIndex>,
    environment: &TypeEnvironment,
    dependencies: &mut DependencyCollector<'_>,
) -> bool {
    let mut has_invalid_dependency = false;
    collect_dependencies(
        &value.initializer,
        value,
        value_indices,
        environment,
        &mut Vec::new(),
        dependencies,
        &mut has_invalid_dependency,
    );
    has_invalid_dependency
}

fn append_module_value_diagnostics(
    value: &veln_ast::ModuleValueDecl,
    index: usize,
    environment: &TypeEnvironment,
    variant_diagnostics: &mut VariantDiagnosticInterner,
    expected: Option<&Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_admitted_expr(
        &value.initializer,
        value,
        environment,
        expected,
        diagnostics,
    );

    let synthetic = synthetic_module_value_function(value, index);
    diagnostics.extend(check_type_effect_labels(
        &value.annotation,
        &value.annotation_span,
        &synthetic,
        environment,
        "module_value_type",
    ));
    let mut checker =
        FunctionChecker::for_synthetic_declaration(&synthetic, environment, variant_diagnostics);
    checker.check_body();
    diagnostics.append(&mut checker.diagnostics);
    diagnostics.extend(super::check_public_module_value_refinement_visibility(
        value,
        environment,
    ));
    diagnostics.extend(public_named_type_diagnostics(value, environment));
}

fn annotation_is_closed(
    value: &veln_ast::ModuleValueDecl,
    environment: &TypeEnvironment,
    annotation_was_parsed: bool,
) -> bool {
    annotation_was_parsed
        && value.annotation_paths.iter().all(|path| {
            environment.annotation_type_path_is_resolved_in_owner(
                &path.segments.join("::"),
                path.type_argument_count,
                value.module_name.as_deref(),
                value.package_name.as_deref(),
            )
        })
}

fn synthetic_module_value_function(value: &veln_ast::ModuleValueDecl, index: usize) -> Function {
    Function {
        node_id: value.node_id,
        module_name: value.module_name.clone(),
        package_name: value.package_name.clone(),
        kind: FunctionKind::Function,
        visibility: Visibility::Private,
        name: Some(format!("__module_value_{index}")),
        effect_binder: None,
        params: Vec::new(),
        return_binding: None,
        return_type: Some(value.annotation.clone()),
        return_type_span: Some(value.annotation_span.clone()),
        return_type_paths: value.annotation_paths.clone(),
        return_type_refinements: value.annotation_refinements.clone(),
        effects: Some(Vec::new()),
        effect_spans: Some(Vec::new()),
        callsite: None,
        contracts: Vec::new(),
        body: vec![BodyLine {
            node_id: value.node_id,
            kind: BodyLineKind::Expr {
                expr: value.initializer.clone(),
            },
            span: value.initializer.span.clone(),
        }],
        span: value.span.clone(),
    }
}

fn propagate_invalid_dependencies(dependencies: &[Vec<Dependency>], invalid: &mut [bool]) {
    let mut reverse = vec![Vec::new(); invalid.len()];
    for (source, edges) in dependencies.iter().enumerate() {
        for edge in edges {
            reverse[edge.target].push(source);
        }
    }
    let mut pending = invalid
        .iter()
        .enumerate()
        .filter_map(|(index, invalid)| invalid.then_some(index))
        .collect::<VecDeque<_>>();
    while let Some(target) = pending.pop_front() {
        for dependent in &reverse[target] {
            if !invalid[*dependent] {
                invalid[*dependent] = true;
                pending.push_back(*dependent);
            }
        }
    }
}

fn push_unresolved_annotation_paths(
    declaration: &veln_ast::ModuleValueDecl,
    environment: &TypeEnvironment,
    diagnostic_start: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let unresolved_spans = diagnostics[diagnostic_start..]
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .filter_map(|diagnostic| diagnostic.span.as_ref())
        .map(span_identity)
        .collect::<BTreeSet<_>>();
    #[cfg(test)]
    record_module_value_annotation_dedup_work(|work| {
        work.indexed_diagnostics += diagnostics.len() - diagnostic_start;
    });
    for path in &declaration.annotation_paths {
        let symbol = path.segments.join("::");
        #[cfg(test)]
        record_module_value_annotation_dedup_work(|work| work.path_lookups += 1);
        if environment.annotation_type_path_is_resolved_in_owner(
            &symbol,
            path.type_argument_count,
            declaration.module_name.as_deref(),
            declaration.package_name.as_deref(),
        ) || unresolved_spans.contains(&span_identity(&path.application_span))
        {
            continue;
        }
        diagnostics.push(Diagnostic::new(
            "name.unresolved",
            Severity::Error,
            DiagnosticKind::Name,
            format!("unresolved type `{symbol}`"),
            Some(path.application_span.clone()),
            JsonValue::object([
                ("phase", JsonValue::string("name")),
                ("node_id", JsonValue::Null),
                ("symbol", JsonValue::string(symbol)),
                ("namespace", JsonValue::string("type")),
                ("resolution_status", JsonValue::string("unresolved")),
                ("candidates", JsonValue::array([])),
            ]),
        ));
    }
}

type SpanIdentity = (
    veln_source::SourcePath,
    (usize, usize, usize),
    (usize, usize, usize),
    Option<veln_source::SourcePath>,
    Option<(usize, usize, usize)>,
    Option<(usize, usize, usize)>,
);

fn span_identity(span: &veln_source::SourceSpan) -> SpanIdentity {
    let position =
        |position: veln_source::LineCol| (position.line, position.column, position.offset);
    (
        span.file.clone(),
        position(span.start),
        position(span.end),
        span.original_path().cloned(),
        span.original_start().map(position),
        span.original_end().map(position),
    )
}
