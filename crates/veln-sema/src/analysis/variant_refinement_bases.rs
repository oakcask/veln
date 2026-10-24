use std::collections::{HashMap, HashSet};

use veln_ast::{
    BodyLine, BodyLineKind, Expr, ExprKind, FunctionKind, SurfaceModule, TypePathSegments,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::diagnostics::{span_json, type_details};
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_annotation_with_recovery_slots;
use crate::types::{TypeEnvironment, TypeParameterIdentityContext, VariantRefinementBaseFailure};

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct DiagnosticRecordRenderWork {
    pub(crate) index_insertions: usize,
    pub(crate) pairing_lookups: usize,
    pub(crate) result_reordering_lookups: usize,
}

#[cfg(test)]
thread_local! {
    static DIAGNOSTIC_RECORD_RENDER_WORK: std::cell::Cell<DiagnosticRecordRenderWork> =
        const { std::cell::Cell::new(DiagnosticRecordRenderWork {
            index_insertions: 0,
            pairing_lookups: 0,
            result_reordering_lookups: 0,
        }) };
}

#[cfg(test)]
pub(crate) fn reset_diagnostic_record_render_work() {
    DIAGNOSTIC_RECORD_RENDER_WORK.set(DiagnosticRecordRenderWork::default());
}

#[cfg(test)]
pub(crate) fn take_diagnostic_record_render_work() -> DiagnosticRecordRenderWork {
    DIAGNOSTIC_RECORD_RENDER_WORK.replace(DiagnosticRecordRenderWork::default())
}

enum DiagnosticRecordRenderOperation {
    IndexInsertion,
    PairingLookup,
    ResultReorderingLookup,
}

fn record_diagnostic_record_render_work(operation: DiagnosticRecordRenderOperation) {
    #[cfg(test)]
    DIAGNOSTIC_RECORD_RENDER_WORK.set({
        let mut work = DIAGNOSTIC_RECORD_RENDER_WORK.get();
        match operation {
            DiagnosticRecordRenderOperation::IndexInsertion => work.index_insertions += 1,
            DiagnosticRecordRenderOperation::PairingLookup => work.pairing_lookups += 1,
            DiagnosticRecordRenderOperation::ResultReorderingLookup => {
                work.result_reordering_lookups += 1;
            }
        }
        work
    });
    #[cfg(not(test))]
    let _ = operation;
}

pub(crate) struct VariantRefinementBaseDiagnostic {
    pub(crate) diagnostic: Diagnostic,
    pub(crate) ordering_scope: SourceSpan,
    pub(crate) union_scope: Option<SourceSpan>,
}

struct VariantUnionBaseFailure {
    expected_base_type: String,
    actual_base_type: String,
    expected_span: SourceSpan,
    actual_span: SourceSpan,
    union_scope: SourceSpan,
}

struct RecoveredType {
    ty: Type,
    nodes: usize,
}

#[derive(Default)]
struct AnnotationRecovery {
    errors: Vec<(String, String, SourceSpan)>,
    union_base_failures: Vec<VariantUnionBaseFailure>,
    unknown_variant_occurrences: HashSet<PathOccurrence>,
    unavailable_variant_occurrences: HashSet<PathOccurrence>,
    unresolved_base_occurrences: HashSet<PathOccurrence>,
}

struct AlternativeRecovery {
    recovered: Option<RecoveredType>,
    invalid: bool,
    complete: bool,
}

#[derive(Clone, Copy)]
struct ParseDiagnosticLocation {
    context: &'static str,
    recovery_anchor: &'static str,
}

impl ParseDiagnosticLocation {
    const fn type_annotation(context: &'static str) -> Self {
        Self {
            context,
            recovery_anchor: "type annotation",
        }
    }

    const fn type_argument(context: &'static str) -> Self {
        Self {
            context,
            recovery_anchor: "type argument",
        }
    }
}

pub(crate) fn check_variant_refinement_bases(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<VariantRefinementBaseDiagnostic> {
    let mut diagnostics = Vec::new();

    collect_type_annotations(module, environment, &mut diagnostics);
    collect_effect_annotations(module, environment, &mut diagnostics);
    collect_schema_annotations(module, environment, &mut diagnostics);
    collect_function_annotations(module, environment, &mut diagnostics);
    collect_handler_annotations(module, environment, &mut diagnostics);

    diagnostics
}

fn collect_type_annotations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for decl in &module.types {
        for variant in &decl.variants {
            for field in &variant.fields {
                collect_declaration_annotation(
                    &field.ty_refinements,
                    &field.ty_paths,
                    decl.module_name.as_deref(),
                    environment,
                    &decl.params,
                    field.node_id.display("field"),
                    [
                        decl.node_id.display("type"),
                        variant.node_id.display("variant"),
                        field.node_id.display("field"),
                    ],
                    ParseDiagnosticLocation::type_annotation("type_variant"),
                    diagnostics,
                );
            }
        }
    }
}

fn collect_effect_annotations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for effect in &module.effects {
        for operation in &effect.operations {
            for param in &operation.params {
                collect_declaration_annotation(
                    &param.ty_refinements,
                    &param.ty_paths,
                    effect.module_name.as_deref(),
                    environment,
                    &[],
                    param.node_id.display("param"),
                    [
                        effect.node_id.display("effect"),
                        operation.node_id.display("operation"),
                        param.node_id.display("param"),
                    ],
                    ParseDiagnosticLocation::type_annotation("effect_operation"),
                    diagnostics,
                );
            }
            collect_declaration_annotation(
                &operation.return_type_refinements,
                &operation.return_type_paths,
                effect.module_name.as_deref(),
                environment,
                &[],
                operation.node_id.display("operation"),
                [
                    effect.node_id.display("effect"),
                    operation.node_id.display("operation"),
                ],
                ParseDiagnosticLocation::type_annotation("effect_operation"),
                diagnostics,
            );
        }
    }
}

fn collect_schema_annotations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for schema in &module.schemas {
        for field in &schema.fields {
            collect_declaration_annotation(
                &field.ty_refinements,
                &field.ty_paths,
                schema.module_name.as_deref(),
                environment,
                &[],
                field.node_id.display("schema-field"),
                [
                    schema.node_id.display("schema"),
                    field.node_id.display("schema-field"),
                ],
                ParseDiagnosticLocation::type_annotation("schema_field"),
                diagnostics,
            );
        }
    }
}

fn collect_function_annotations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for function in &module.functions {
        crate::pipeline::record_variant_refinement_annotation_work(1);
        for param in &function.params {
            crate::pipeline::record_variant_refinement_annotation_work(1);
            collect_annotation(
                &param.ty_refinements,
                &param.ty_paths,
                function.module_name.as_deref(),
                environment,
                &[],
                ParseDiagnosticLocation::type_annotation("function_parameters"),
                diagnostics,
            );
        }
        collect_annotation(
            &function.return_type_refinements,
            &function.return_type_paths,
            function.module_name.as_deref(),
            environment,
            &[],
            ParseDiagnosticLocation::type_annotation(match function.kind {
                FunctionKind::Function => "function_return",
                FunctionKind::Test => "test_return",
            }),
            diagnostics,
        );
        for line in &function.body {
            crate::pipeline::record_variant_refinement_annotation_work(1);
            collect_body_line(
                line,
                function.module_name.as_deref(),
                environment,
                diagnostics,
            );
        }
    }
}

fn collect_handler_annotations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for handler in &module.handlers {
        for param in &handler.params {
            collect_declaration_annotation(
                &param.ty_refinements,
                &param.ty_paths,
                handler.module_name.as_deref(),
                environment,
                &[],
                param.node_id.display("param"),
                [
                    handler.node_id.display("handler"),
                    param.node_id.display("param"),
                ],
                ParseDiagnosticLocation::type_annotation("handler_parameters"),
                diagnostics,
            );
        }
        for clause in &handler.operation_clauses {
            for param in &clause.params {
                collect_declaration_annotation(
                    &param.ty_refinements,
                    &param.ty_paths,
                    handler.module_name.as_deref(),
                    environment,
                    &[],
                    param.node_id.display("param"),
                    [
                        handler.node_id.display("handler"),
                        clause.node_id.display("clause"),
                        param.node_id.display("param"),
                    ],
                    ParseDiagnosticLocation::type_annotation("handler_operation"),
                    diagnostics,
                );
            }
            collect_expr(
                &clause.body,
                handler.module_name.as_deref(),
                environment,
                "handler_operation_clause",
                diagnostics,
            );
        }
    }
}

pub(crate) fn annotation_has_invalid_refinement(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
) -> bool {
    let type_parameter_context = TypeParameterIdentityContext::new(type_parameters);
    annotation_has_lowercase_final(refinements, current_module, environment)
        || annotation_has_type_parameter_base_arity_failure(
            refinements,
            current_module,
            environment,
            &type_parameter_context,
        )
        || annotation_has_base_failure(
            refinements,
            paths,
            current_module,
            environment,
            &type_parameter_context,
        )
        || (annotation_needs_recovery(
            refinements,
            paths,
            current_module,
            environment,
            &type_parameter_context,
        ) && {
            let recovery = analyze_annotation(
                refinements,
                paths,
                current_module,
                environment,
                type_parameters,
            );
            !recovery.unknown_variant_occurrences.is_empty()
                || !recovery.union_base_failures.is_empty()
        })
}

pub(crate) fn annotation_has_unavailable_refinement_identity(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
) -> bool {
    let type_parameter_context = TypeParameterIdentityContext::new(type_parameters);
    annotation_has_type_parameter_base_arity_failure(
        refinements,
        current_module,
        environment,
        &type_parameter_context,
    ) || annotation_has_ambiguous_module_qualifier(refinements, current_module, environment)
        || annotation_has_unavailable_declared_base(refinements, current_module, environment)
        || annotation_has_base_failure(
            refinements,
            paths,
            current_module,
            environment,
            &type_parameter_context,
        )
        || (annotation_needs_recovery(
            refinements,
            paths,
            current_module,
            environment,
            &type_parameter_context,
        ) && {
            let recovery = analyze_annotation(
                refinements,
                paths,
                current_module,
                environment,
                type_parameters,
            );
            (annotation_has_lowercase_final(refinements, current_module, environment)
                && !recovery.errors.is_empty())
                || !recovery.unknown_variant_occurrences.is_empty()
                || !recovery.unavailable_variant_occurrences.is_empty()
                || !recovery.unresolved_base_occurrences.is_empty()
                || !recovery.union_base_failures.is_empty()
        })
}

fn annotation_has_unavailable_declared_base(
    refinements: &[veln_ast::VariantRefinementType],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    refinements.iter().any(|refinement| {
        refinement.alternatives.iter().any(|alternative| {
            if !alternative_is_selected(alternative, current_module, environment) {
                return false;
            }
            let base = alternative.base.segments.join("::");
            environment
                .variant_refinement_base_has_unavailable_declared_identity(&base, current_module)
                || alternative.type_arguments.iter().any(|argument| {
                    annotation_has_unavailable_declared_base(
                        &argument.ty_refinements,
                        current_module,
                        environment,
                    )
                })
        })
    })
}

fn annotation_has_ambiguous_module_qualifier(
    refinements: &[veln_ast::VariantRefinementType],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    refinements.iter().any(|refinement| {
        refinement.alternatives.iter().any(|alternative| {
            if !alternative_is_selected(alternative, current_module, environment) {
                return false;
            }
            let base = alternative.base.segments.join("::");
            environment
                .variant_refinement_base_has_ambiguous_module_qualifier(&base, current_module)
                || alternative.type_arguments.iter().any(|argument| {
                    annotation_has_ambiguous_module_qualifier(
                        &argument.ty_refinements,
                        current_module,
                        environment,
                    )
                })
        })
    })
}

fn annotation_has_type_parameter_base_arity_failure(
    refinements: &[veln_ast::VariantRefinementType],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &TypeParameterIdentityContext<'_>,
) -> bool {
    refinements.iter().any(|refinement| {
        refinement.alternatives.iter().any(|alternative| {
            if !alternative_is_selected(alternative, current_module, environment) {
                return false;
            }
            let base = alternative.base.segments.join("::");
            (!base.contains("::")
                && type_parameters.ordinal(&base).is_some()
                && !alternative.type_arguments.is_empty())
                || alternative.type_arguments.iter().any(|argument| {
                    annotation_has_type_parameter_base_arity_failure(
                        &argument.ty_refinements,
                        current_module,
                        environment,
                        type_parameters,
                    )
                })
        })
    })
}

fn annotation_has_lowercase_final(
    refinements: &[veln_ast::VariantRefinementType],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    refinements.iter().any(|refinement| {
        refinement.alternatives.iter().any(|alternative| {
            alternative_is_selected(alternative, current_module, environment)
                && (alternative
                    .variant
                    .as_bytes()
                    .first()
                    .is_some_and(u8::is_ascii_lowercase)
                    || alternative.type_arguments.iter().any(|argument| {
                        annotation_has_lowercase_final(
                            &argument.ty_refinements,
                            current_module,
                            environment,
                        )
                    }))
        })
    })
}

fn alternative_is_selected(
    alternative: &veln_ast::VariantRefinementAlternative,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    environment.variant_refinement_syntax_candidate_is_selected(
        &alternative.base.segments,
        alternative.type_arguments.len(),
        &alternative.variant,
        current_module,
    )
}

fn annotation_needs_recovery(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &TypeParameterIdentityContext<'_>,
) -> bool {
    if paths
        .iter()
        .any(|path| recovered_path_needs_analysis(path, current_module, environment))
    {
        return true;
    }
    let mut pending = refinements.iter().collect::<Vec<_>>();
    while let Some(refinement) = pending.pop() {
        if refinement_has_selected_union(refinement, current_module, environment) {
            return true;
        }
        for alternative in &refinement.alternatives {
            if !alternative_is_selected(alternative, current_module, environment) {
                continue;
            }
            if refinement_alternative_needs_analysis(
                alternative,
                current_module,
                environment,
                type_parameters,
            ) {
                return true;
            }
            pending.extend(
                alternative
                    .type_arguments
                    .iter()
                    .flat_map(|argument| argument.ty_refinements.iter()),
            );
        }
    }
    false
}

fn recovered_path_needs_analysis(
    path: &TypePathSegments,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    environment
        .recovered_variant_refinement_base_failure(&path.segments, current_module)
        .is_some()
        || environment
            .recovered_variant_refinement_unknown_descriptor(&path.segments, current_module)
            .is_some()
}

fn refinement_has_selected_union(
    refinement: &veln_ast::VariantRefinementType,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    refinement.alternatives.len() > 1
        && refinement
            .alternatives
            .iter()
            .filter(|alternative| alternative_is_selected(alternative, current_module, environment))
            .count()
            > 1
}

fn refinement_alternative_needs_analysis(
    alternative: &veln_ast::VariantRefinementAlternative,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &TypeParameterIdentityContext<'_>,
) -> bool {
    if alternative
        .variant
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_lowercase)
    {
        return true;
    }
    let base = alternative.base.segments.join("::");
    environment
        .variant_refinement_base_failure(&base, alternative.type_arguments.len(), current_module)
        .is_some()
        || environment
            .type_path_arity_error_with_context(
                &base,
                alternative.type_arguments.len(),
                current_module,
                type_parameters,
            )
            .is_some()
        || environment.variant_refinement_base_is_unresolved(&base, current_module, type_parameters)
        || environment.variant_refinement_base_has_ambiguous_module_qualifier(&base, current_module)
        || environment
            .variant_refinement_unknown_descriptor(
                &base,
                alternative.type_arguments.len(),
                &alternative.variant,
                current_module,
            )
            .is_some()
}

fn annotation_has_base_failure(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &TypeParameterIdentityContext<'_>,
) -> bool {
    paths.iter().any(|path| {
        environment
            .recovered_variant_refinement_base_failure(&path.segments, current_module)
            .is_some()
    }) || refinements.iter().any(|refinement| {
        refinement.alternatives.iter().any(|alternative| {
            if !alternative_is_selected(alternative, current_module, environment) {
                return false;
            }
            let base = alternative.base.segments.join("::");
            environment
                .variant_refinement_base_failure(
                    &base,
                    alternative.type_arguments.len(),
                    current_module,
                )
                .is_some()
                || environment.variant_refinement_base_is_unresolved(
                    &base,
                    current_module,
                    type_parameters,
                )
                || alternative.type_arguments.iter().any(|argument| {
                    annotation_has_base_failure(
                        &argument.ty_refinements,
                        &argument.ty_paths,
                        current_module,
                        environment,
                        type_parameters,
                    )
                })
        })
    })
}

pub(crate) fn independent_annotation_errors(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> Vec<(String, String, SourceSpan)> {
    analyze_annotation(refinements, paths, current_module, environment, &[]).errors
}

fn analyze_annotation(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
) -> AnnotationRecovery {
    #[cfg(test)]
    {
        let (recovery, cloned_nodes) = crate::semantic_model::measure_type_clone_nodes(|| {
            analyze_annotation_unmeasured(
                refinements,
                paths,
                current_module,
                environment,
                type_parameters,
            )
        });
        crate::pipeline::record_variant_refinement_recovery_type_clones(cloned_nodes);
        recovery
    }
    #[cfg(not(test))]
    analyze_annotation_unmeasured(
        refinements,
        paths,
        current_module,
        environment,
        type_parameters,
    )
}

fn analyze_annotation_unmeasured(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
) -> AnnotationRecovery {
    let mut recovery = AnnotationRecovery::default();
    let mut visited_paths = HashMap::new();
    let mut materialized_refinements = HashMap::new();
    let declaration_type_parameters = TypeParameterIdentityContext::new(type_parameters);
    collect_independent_refinement_errors(
        refinements,
        paths,
        current_module,
        environment,
        &mut recovery.errors,
        &mut visited_paths,
        &mut recovery.union_base_failures,
        &mut recovery.unknown_variant_occurrences,
        &mut recovery.unavailable_variant_occurrences,
        &mut recovery.unresolved_base_occurrences,
        &declaration_type_parameters,
        &mut materialized_refinements,
    );
    collect_independent_structural_arity_errors(
        paths,
        current_module,
        environment,
        &mut recovery.errors,
        &mut visited_paths,
        &declaration_type_parameters,
    );
    recovery
}

fn collect_independent_structural_arity_errors(
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
    declaration_type_parameters: &TypeParameterIdentityContext<'_>,
) {
    for path in paths {
        crate::pipeline::record_variant_refinement_recovery_work(1);
        let path_span = joined_span(&path.segment_spans).resolved_or_generated();
        let occurrence = (
            path_span.file.clone(),
            path_span.start.offset,
            path_span.end.offset,
        );
        if visited_paths.get(&occurrence).copied() == Some(true) {
            continue;
        }
        let name = path.segments.join("::");
        if let Some(error) = environment.type_path_arity_error_with_context(
            &name,
            path.type_argument_count,
            current_module,
            declaration_type_parameters,
        ) {
            errors.push((path.written.clone(), error, path.application_span.clone()));
        }
    }
}

type PathOccurrence = (veln_source::SourcePath, usize, usize);

#[allow(clippy::too_many_arguments)]
fn collect_independent_path_errors(
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
    unknown_variant_occurrences: &mut HashSet<PathOccurrence>,
    unavailable_variant_occurrences: &mut HashSet<PathOccurrence>,
    declaration_type_parameters: &TypeParameterIdentityContext<'_>,
) -> bool {
    let mut annotation_is_invalid = false;
    for path in paths {
        let span = joined_span(&path.segment_spans).resolved_or_generated();
        let occurrence = (span.file, span.start.offset, span.end.offset);
        if let Some(path_is_invalid) = visited_paths.get(&occurrence) {
            annotation_is_invalid |= path_is_invalid;
            continue;
        }
        if path.segments.len() == 1
            && path.type_argument_count == 0
            && declaration_type_parameters
                .ordinal(&path.segments[0])
                .is_some()
        {
            visited_paths.insert(occurrence, false);
            continue;
        }
        let path_is_invalid = if environment
            .recovered_variant_refinement_base_failure(&path.segments, current_module)
            .is_some()
        {
            true
        } else if environment
            .recovered_variant_refinement_unknown_descriptor(&path.segments, current_module)
            .is_some()
        {
            unknown_variant_occurrences.insert(occurrence.clone());
            true
        } else if environment
            .recovered_variant_refinement_has_ambiguous_final(&path.segments, current_module)
        {
            unavailable_variant_occurrences.insert(occurrence.clone());
            true
        } else if let Some(error) = environment.recovered_variant_refinement_annotation_error(
            std::slice::from_ref(path),
            current_module,
        ) {
            errors.push((
                path.segments.join("::"),
                error,
                joined_span(&path.segment_spans),
            ));
            true
        } else {
            false
        };
        visited_paths.insert(occurrence, path_is_invalid);
        annotation_is_invalid |= path_is_invalid;
    }
    annotation_is_invalid
}

#[allow(clippy::too_many_arguments)]
fn collect_independent_refinement_errors(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
    union_base_failures: &mut Vec<VariantUnionBaseFailure>,
    unknown_variant_occurrences: &mut HashSet<PathOccurrence>,
    unavailable_variant_occurrences: &mut HashSet<PathOccurrence>,
    unresolved_base_occurrences: &mut HashSet<PathOccurrence>,
    declaration_type_parameters: &TypeParameterIdentityContext<'_>,
    materialized_refinements: &mut HashMap<PathOccurrence, Option<RecoveredType>>,
) -> bool {
    let mut annotation_is_invalid = collect_independent_path_errors(
        paths,
        current_module,
        environment,
        errors,
        visited_paths,
        unknown_variant_occurrences,
        unavailable_variant_occurrences,
        declaration_type_parameters,
    );
    for refinement in refinements {
        let (refinement_is_invalid, recovered_refinement) = collect_refinement_union(
            refinement,
            current_module,
            environment,
            errors,
            visited_paths,
            union_base_failures,
            unknown_variant_occurrences,
            unavailable_variant_occurrences,
            unresolved_base_occurrences,
            declaration_type_parameters,
            materialized_refinements,
        );
        annotation_is_invalid |= refinement_is_invalid;
        materialized_refinements.insert(span_occurrence(&refinement.span), recovered_refinement);
    }
    annotation_is_invalid
}

#[allow(clippy::too_many_arguments)]
fn collect_refinement_union(
    refinement: &veln_ast::VariantRefinementType,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
    union_base_failures: &mut Vec<VariantUnionBaseFailure>,
    unknown_variant_occurrences: &mut HashSet<PathOccurrence>,
    unavailable_variant_occurrences: &mut HashSet<PathOccurrence>,
    unresolved_base_occurrences: &mut HashSet<PathOccurrence>,
    declaration_type_parameters: &TypeParameterIdentityContext<'_>,
    materialized_refinements: &mut HashMap<PathOccurrence, Option<RecoveredType>>,
) -> (bool, Option<RecoveredType>) {
    let mut expected_span: Option<SourceSpan> = None;
    let mut recovered_refinement: Option<RecoveredType> = None;
    let mut recovered_variants = HashSet::new();
    let mut refinement_is_invalid = false;
    let mut refinement_is_complete = true;
    for alternative in &refinement.alternatives {
        if !alternative_is_selected(alternative, current_module, environment) {
            continue;
        }
        let alternative_recovery = collect_refinement_alternative_type(
            alternative,
            current_module,
            environment,
            errors,
            visited_paths,
            union_base_failures,
            unknown_variant_occurrences,
            unavailable_variant_occurrences,
            unresolved_base_occurrences,
            declaration_type_parameters,
            materialized_refinements,
        );
        refinement_is_invalid |= alternative_recovery.invalid;
        refinement_is_complete &= alternative_recovery.complete;
        let Some(canonical) = alternative_recovery.recovered else {
            continue;
        };
        if variant_refinement_base_identity_and_arguments(&canonical.ty).is_none() {
            refinement_is_invalid = true;
            refinement_is_complete = false;
            continue;
        }
        let alternative_is_invalid = merge_recovered_union_alternative(
            refinement,
            alternative,
            canonical,
            &mut expected_span,
            &mut recovered_refinement,
            &mut recovered_variants,
            union_base_failures,
        );
        refinement_is_invalid |= alternative_is_invalid;
        refinement_is_complete &= !alternative_is_invalid;
    }
    let recovered_refinement = refinement_is_complete
        .then_some(recovered_refinement)
        .flatten()
        .map(|recovered| RecoveredType {
            ty: canonicalize_recovered_refinement(recovered.ty, environment),
            nodes: recovered.nodes,
        });
    (refinement_is_invalid, recovered_refinement)
}

fn merge_recovered_union_alternative(
    refinement: &veln_ast::VariantRefinementType,
    alternative: &veln_ast::VariantRefinementAlternative,
    canonical: RecoveredType,
    expected_span: &mut Option<SourceSpan>,
    recovered_refinement: &mut Option<RecoveredType>,
    recovered_variants: &mut HashSet<String>,
    union_base_failures: &mut Vec<VariantUnionBaseFailure>,
) -> bool {
    let Some(established_span) = expected_span else {
        *expected_span = Some(alternative_span(alternative));
        if let Type::VariantRefinement { variants, .. } = &canonical.ty {
            recovered_variants.extend(variants.iter().cloned());
        }
        *recovered_refinement = Some(canonical);
        return false;
    };
    crate::pipeline::record_variant_refinement_recovery_work(1);
    let expected = recovered_refinement
        .as_ref()
        .expect("an expected span has a recovered refinement");
    if !same_variant_refinement_base(&expected.ty, &canonical.ty) {
        let (expected_base_type, actual_base_type) =
            variant_refinement_base_renders(&expected.ty, &canonical.ty);
        union_base_failures.push(VariantUnionBaseFailure {
            expected_base_type,
            actual_base_type,
            expected_span: established_span.clone(),
            actual_span: alternative_span(alternative),
            union_scope: refinement.span.clone(),
        });
        return true;
    }
    let expected = recovered_refinement
        .take()
        .expect("an expected span has a recovered refinement");
    let nodes = expected.nodes;
    let ty = merge_refinement_alternative(expected.ty, canonical.ty, recovered_variants);
    *recovered_refinement = Some(RecoveredType { ty, nodes });
    false
}

#[allow(clippy::too_many_arguments)]
fn collect_refinement_alternative_type(
    alternative: &veln_ast::VariantRefinementAlternative,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
    union_base_failures: &mut Vec<VariantUnionBaseFailure>,
    unknown_variant_occurrences: &mut HashSet<PathOccurrence>,
    unavailable_variant_occurrences: &mut HashSet<PathOccurrence>,
    unresolved_base_occurrences: &mut HashSet<PathOccurrence>,
    declaration_type_parameters: &TypeParameterIdentityContext<'_>,
    materialized_refinements: &mut HashMap<PathOccurrence, Option<RecoveredType>>,
) -> AlternativeRecovery {
    crate::pipeline::record_variant_refinement_recovery_work(1);
    let base = alternative.base.segments.join("::");
    let own_base_failure = environment
        .variant_refinement_base_failure(&base, alternative.type_arguments.len(), current_module)
        .is_some();
    let argument_types = collect_refinement_argument_types(
        &alternative.type_arguments,
        current_module,
        environment,
        errors,
        visited_paths,
        union_base_failures,
        unknown_variant_occurrences,
        unavailable_variant_occurrences,
        unresolved_base_occurrences,
        declaration_type_parameters,
        materialized_refinements,
    );
    if environment.variant_refinement_base_is_unresolved(
        &base,
        current_module,
        declaration_type_parameters,
    ) {
        unresolved_base_occurrences.insert(span_occurrence(&joined_span(
            &alternative.base.segment_spans,
        )));
        return invalid_alternative_recovery();
    }
    if let Some(error) = environment.type_path_arity_error_with_context(
        &base,
        alternative.type_arguments.len(),
        current_module,
        declaration_type_parameters,
    ) {
        let written = format!(
            "{}::{}",
            written_refinement_base(alternative),
            alternative.variant
        );
        crate::pipeline::record_variant_refinement_recovery_materialization(written.len());
        errors.push((written, error, alternative_span(alternative)));
        return invalid_alternative_recovery();
    }
    if own_base_failure || argument_types.is_none() {
        return invalid_alternative_recovery();
    }
    recover_refinement_alternative(
        alternative,
        current_module,
        environment,
        errors,
        unknown_variant_occurrences,
        unavailable_variant_occurrences,
        base,
        argument_types.expect("checked recovered argument types"),
    )
}

fn invalid_alternative_recovery() -> AlternativeRecovery {
    AlternativeRecovery {
        recovered: None,
        invalid: true,
        complete: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn recover_refinement_alternative(
    alternative: &veln_ast::VariantRefinementAlternative,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    unknown_variant_occurrences: &mut HashSet<PathOccurrence>,
    unavailable_variant_occurrences: &mut HashSet<PathOccurrence>,
    base: String,
    argument_types: Vec<RecoveredType>,
) -> AlternativeRecovery {
    let argument_nodes: usize = argument_types.iter().map(|argument| argument.nodes).sum();
    let argument_types = argument_types
        .into_iter()
        .map(|argument| argument.ty)
        .collect::<Vec<_>>();
    let argument_types = match recover_unknown_variant_alternative(
        alternative,
        current_module,
        environment,
        &base,
        argument_nodes,
        argument_types,
        unknown_variant_occurrences,
    ) {
        Ok(recovered) => {
            return AlternativeRecovery {
                recovered: Some(recovered),
                invalid: true,
                complete: false,
            };
        }
        Err(argument_types) => argument_types,
    };
    let argument_types = match recover_ambiguous_variant_base(
        alternative,
        current_module,
        environment,
        &base,
        argument_nodes,
        argument_types,
        unavailable_variant_occurrences,
    ) {
        Ok(recovered) => return recovered,
        Err(argument_types) => argument_types,
    };
    let shell = Type::variant_refinement(
        base,
        vec![Type::Unknown; argument_types.len()],
        vec![alternative.variant.clone()],
    );
    if record_refinement_annotation_error(alternative, current_module, environment, errors, &shell)
    {
        return invalid_alternative_recovery();
    }
    crate::pipeline::record_variant_refinement_recovery_materialization(argument_types.len() + 1);
    let recovered = refinement_with_arguments(
        environment.canonicalize_type_annotation(shell, current_module),
        argument_types,
    )
    .map(|ty| RecoveredType {
        ty,
        nodes: argument_nodes + 1,
    });
    crate::pipeline::record_variant_refinement_recovery_type_materialization(1);
    let complete = recovered.is_some();
    AlternativeRecovery {
        invalid: !complete
            || alternative
                .variant
                .as_bytes()
                .first()
                .is_some_and(u8::is_ascii_lowercase),
        recovered,
        complete,
    }
}

fn record_refinement_annotation_error(
    alternative: &veln_ast::VariantRefinementAlternative,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    shell: &Type,
) -> bool {
    let Some(error) = environment.variant_refinement_annotation_error(shell, current_module) else {
        return false;
    };
    let written = format!(
        "{}::{}",
        written_refinement_base(alternative),
        alternative.variant
    );
    crate::pipeline::record_variant_refinement_recovery_materialization(written.len());
    errors.push((written, error, alternative_span(alternative)));
    true
}

fn recover_ambiguous_variant_base(
    alternative: &veln_ast::VariantRefinementAlternative,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    base: &str,
    argument_nodes: usize,
    argument_types: Vec<Type>,
    unavailable_variant_occurrences: &mut HashSet<PathOccurrence>,
) -> Result<AlternativeRecovery, Vec<Type>> {
    let Some(descriptor) = environment.variant_refinement_ambiguous_recovery_descriptor(
        base,
        argument_types.len(),
        &alternative.variant,
        current_module,
    ) else {
        return Err(argument_types);
    };
    unavailable_variant_occurrences.insert(span_occurrence(&alternative.span));
    Ok(AlternativeRecovery {
        recovered: Some(RecoveredType {
            ty: Type::resolved_named(
                environment
                    .adts
                    .canonical_type_name_for_descriptor(descriptor),
                descriptor.identity(),
                argument_types,
            ),
            nodes: argument_nodes + 1,
        }),
        invalid: true,
        complete: false,
    })
}

#[allow(clippy::too_many_arguments)]
fn recover_unknown_variant_alternative(
    alternative: &veln_ast::VariantRefinementAlternative,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    base: &str,
    argument_nodes: usize,
    argument_types: Vec<Type>,
    unknown_variant_occurrences: &mut HashSet<PathOccurrence>,
) -> Result<RecoveredType, Vec<Type>> {
    let Some(descriptor) = environment.variant_refinement_unknown_descriptor(
        base,
        argument_types.len(),
        &alternative.variant,
        current_module,
    ) else {
        return Err(argument_types);
    };
    unknown_variant_occurrences.insert(span_occurrence(&alternative.span));
    Ok(RecoveredType {
        ty: Type::resolved_named(
            environment
                .adts
                .canonical_type_name_for_descriptor(descriptor),
            descriptor.identity(),
            argument_types,
        ),
        nodes: argument_nodes + 1,
    })
}

#[allow(clippy::too_many_arguments)]
fn collect_refinement_argument_types(
    arguments: &[veln_ast::VariantRefinementTypeArgument],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
    union_base_failures: &mut Vec<VariantUnionBaseFailure>,
    unknown_variant_occurrences: &mut HashSet<PathOccurrence>,
    unavailable_variant_occurrences: &mut HashSet<PathOccurrence>,
    unresolved_base_occurrences: &mut HashSet<PathOccurrence>,
    declaration_type_parameters: &TypeParameterIdentityContext<'_>,
    materialized_refinements: &mut HashMap<PathOccurrence, Option<RecoveredType>>,
) -> Option<Vec<RecoveredType>> {
    let mut recovered = Vec::with_capacity(arguments.len());
    let mut all_arguments_recovered = true;
    for argument in arguments {
        let argument = collect_refinement_argument_type(
            argument,
            current_module,
            environment,
            errors,
            visited_paths,
            union_base_failures,
            unknown_variant_occurrences,
            unavailable_variant_occurrences,
            unresolved_base_occurrences,
            declaration_type_parameters,
            materialized_refinements,
        );
        match argument {
            Some(argument) => recovered.push(argument),
            None => all_arguments_recovered = false,
        }
    }
    all_arguments_recovered.then_some(recovered)
}

#[allow(clippy::too_many_arguments)]
fn collect_refinement_argument_type(
    argument: &veln_ast::VariantRefinementTypeArgument,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
    union_base_failures: &mut Vec<VariantUnionBaseFailure>,
    unknown_variant_occurrences: &mut HashSet<PathOccurrence>,
    unavailable_variant_occurrences: &mut HashSet<PathOccurrence>,
    unresolved_base_occurrences: &mut HashSet<PathOccurrence>,
    declaration_type_parameters: &TypeParameterIdentityContext<'_>,
    materialized_refinements: &mut HashMap<PathOccurrence, Option<RecoveredType>>,
) -> Option<RecoveredType> {
    let argument_errors_start = errors.len();
    collect_independent_refinement_errors(
        &argument.ty_refinements,
        &argument.ty_paths,
        current_module,
        environment,
        errors,
        visited_paths,
        union_base_failures,
        unknown_variant_occurrences,
        unavailable_variant_occurrences,
        unresolved_base_occurrences,
        declaration_type_parameters,
        materialized_refinements,
    );
    if errors.len() != argument_errors_start {
        return None;
    }
    let (skeleton, recovery_slots) = type_argument_skeleton(argument);
    crate::pipeline::record_variant_refinement_recovery_materialization(skeleton.len());
    let ty = match parse_type_annotation_with_recovery_slots(&skeleton, &recovery_slots) {
        Ok(ty) => ty,
        Err(error) => {
            let written = render_type_argument(argument);
            errors.push((written, error, argument.span.clone()));
            return None;
        }
    };
    if let Some(error) = environment.variant_refinement_arity_error(&ty, current_module) {
        let written = render_type_argument(argument);
        errors.push((written, error, argument.span.clone()));
        return None;
    }
    if !environment.type_annotation_is_fully_resolved_with_context(
        &ty,
        current_module,
        declaration_type_parameters,
    ) {
        return None;
    }
    materialize_refinement_argument(
        argument,
        ty,
        current_module,
        environment,
        declaration_type_parameters,
        materialized_refinements,
    )
}

fn materialize_refinement_argument(
    argument: &veln_ast::VariantRefinementTypeArgument,
    ty: Type,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    declaration_type_parameters: &TypeParameterIdentityContext<'_>,
    materialized_refinements: &mut HashMap<PathOccurrence, Option<RecoveredType>>,
) -> Option<RecoveredType> {
    let mut child_refinements = argument.ty_refinements.iter().map(|refinement| {
        materialized_refinements
            .remove(&span_occurrence(&refinement.span))
            .flatten()
    });
    let mut replacements = Vec::new();
    if !collect_recovery_replacements(&ty, &mut child_refinements, &mut replacements)
        || child_refinements.next().is_some()
    {
        return None;
    }
    let replacement_nodes = replacements
        .iter()
        .map(|replacement| replacement.nodes)
        .sum::<usize>();
    let replacement_count = replacements.len();
    let mut replacement_types = replacements.into_iter().map(|replacement| replacement.ty);
    let canonical = environment.canonicalize_type_annotation_with_context(
        ty,
        current_module,
        declaration_type_parameters,
    );
    let canonical_nodes = type_node_count(&canonical);
    crate::pipeline::record_variant_refinement_recovery_materialization(canonical_nodes);
    let local_nodes = canonical_nodes - replacement_count;
    crate::pipeline::record_variant_refinement_recovery_type_materialization(local_nodes);
    let recovered_argument = replace_recovery_slots(canonical, &mut replacement_types);
    if replacement_types.next().is_some() {
        return None;
    }
    Some(RecoveredType {
        ty: recovered_argument,
        nodes: local_nodes + replacement_nodes,
    })
}

fn type_argument_skeleton(
    argument: &veln_ast::VariantRefinementTypeArgument,
) -> (String, Vec<std::ops::Range<usize>>) {
    let mut text = String::new();
    let mut slots = Vec::with_capacity(argument.ty_refinements.len());
    for (index, fragment) in argument.ty_fragments.iter().enumerate() {
        text.push_str(fragment);
        if index < argument.ty_refinements.len() {
            let start = text.len();
            text.push_str("unknown");
            slots.push(start..text.len());
        }
    }
    (text, slots)
}

fn collect_recovery_replacements(
    ty: &Type,
    child_refinements: &mut impl Iterator<Item = Option<RecoveredType>>,
    replacements: &mut Vec<RecoveredType>,
) -> bool {
    match ty {
        ty @ Type::Named { args, .. } => {
            if ty.is_recovery_slot() {
                let Some(Some(child)) = child_refinements.next() else {
                    return false;
                };
                replacements.push(child);
                return true;
            }
            args.iter()
                .all(|arg| collect_recovery_replacements(arg, child_refinements, replacements))
        }
        Type::VariantRefinement { args, .. } => args
            .iter()
            .all(|arg| collect_recovery_replacements(arg, child_refinements, replacements)),
        Type::Record(fields) => fields
            .iter()
            .all(|(_, ty)| collect_recovery_replacements(ty, child_refinements, replacements)),
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            params
                .iter()
                .all(|ty| collect_recovery_replacements(ty, child_refinements, replacements))
                && variadic.as_deref().is_none_or(|ty| {
                    collect_recovery_replacements(ty, child_refinements, replacements)
                })
                && collect_recovery_replacements(return_type, child_refinements, replacements)
        }
        Type::Unknown => true,
    }
}

fn replace_recovery_slots(ty: Type, replacements: &mut impl Iterator<Item = Type>) -> Type {
    if ty.is_recovery_slot() {
        return replacements.next().unwrap_or(Type::Unknown);
    }
    match ty {
        Type::Unknown => Type::Unknown,
        Type::Named {
            name,
            identity,
            args,
        } => Type::Named {
            name,
            identity,
            args: args
                .into_iter()
                .map(|arg| replace_recovery_slots(arg, replacements))
                .collect(),
        },
        Type::VariantRefinement {
            name,
            identity,
            args,
            variants,
            unresolved_alternatives,
        } => Type::VariantRefinement {
            name,
            identity,
            args: args
                .into_iter()
                .map(|arg| replace_recovery_slots(arg, replacements))
                .collect(),
            variants,
            unresolved_alternatives,
        },
        Type::Record(fields) => Type::Record(
            fields
                .into_iter()
                .map(|(name, ty)| (name, replace_recovery_slots(ty, replacements)))
                .collect(),
        ),
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => Type::Function {
            params: params
                .into_iter()
                .map(|ty| replace_recovery_slots(ty, replacements))
                .collect(),
            variadic: variadic.map(|ty| Box::new(replace_recovery_slots(*ty, replacements))),
            return_type: Box::new(replace_recovery_slots(*return_type, replacements)),
            effects,
        },
    }
}

fn span_occurrence(span: &SourceSpan) -> PathOccurrence {
    (span.file.clone(), span.start.offset, span.end.offset)
}

fn type_node_count(ty: &Type) -> usize {
    match ty {
        Type::Unknown => 1,
        Type::Named { args, .. } | Type::VariantRefinement { args, .. } => {
            1 + args.iter().map(type_node_count).sum::<usize>()
        }
        Type::Record(fields) => {
            1 + fields
                .iter()
                .map(|(_, field)| type_node_count(field))
                .sum::<usize>()
        }
        Type::Function {
            params,
            variadic,
            return_type,
            ..
        } => {
            1 + params.iter().map(type_node_count).sum::<usize>()
                + variadic.as_deref().map(type_node_count).unwrap_or_default()
                + type_node_count(return_type)
        }
    }
}

fn refinement_with_arguments(ty: Type, arguments: Vec<Type>) -> Option<Type> {
    match ty {
        Type::Named { name, identity, .. } => Some(Type::Named {
            name,
            identity,
            args: arguments,
        }),
        Type::VariantRefinement {
            name,
            identity,
            variants,
            unresolved_alternatives,
            ..
        } => Some(Type::VariantRefinement {
            name,
            identity,
            args: arguments,
            variants,
            unresolved_alternatives,
        }),
        _ => None,
    }
}

fn merge_refinement_alternative(
    expected: Type,
    actual: Type,
    seen_variants: &mut HashSet<String>,
) -> Type {
    if matches!(expected, Type::Named { .. }) {
        return expected;
    }
    let Type::VariantRefinement {
        name,
        identity,
        args,
        variants,
        unresolved_alternatives,
    } = expected
    else {
        unreachable!("union alternatives have a recovered variant-refinement base");
    };
    let Type::VariantRefinement {
        variants: actual_variants,
        ..
    } = actual
    else {
        return Type::VariantRefinement {
            name,
            identity,
            args,
            variants,
            unresolved_alternatives,
        };
    };
    let mut variants = variants.to_vec();
    for variant in actual_variants.iter() {
        if seen_variants.insert(variant.clone()) {
            variants.push(variant.clone());
        }
    }
    Type::VariantRefinement {
        name,
        identity,
        args,
        variants: variants.into(),
        unresolved_alternatives,
    }
}

fn canonicalize_recovered_refinement(ty: Type, environment: &TypeEnvironment) -> Type {
    let Some(declaration_order) = environment.adts.variant_declaration_order_for_type(&ty) else {
        return ty;
    };
    let Type::VariantRefinement {
        name,
        identity,
        args,
        variants,
        unresolved_alternatives,
    } = ty
    else {
        return ty;
    };
    let requested = variants.iter().map(String::as_str).collect::<HashSet<_>>();
    let canonical_variants = (0..declaration_order.len())
        .filter_map(|rank| declaration_order.name(rank))
        .filter(|variant| requested.contains(variant))
        .map(str::to_string)
        .collect::<Vec<_>>();
    if canonical_variants.len() == declaration_order.len() {
        Type::Named {
            name,
            identity,
            args,
        }
    } else {
        Type::VariantRefinement {
            name,
            identity,
            args,
            variants: canonical_variants.into(),
            unresolved_alternatives,
        }
    }
}

fn variant_refinement_base_identity_and_arguments(ty: &Type) -> Option<(&str, &[Type])> {
    match ty {
        Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
            Some((identity, args))
        }
        _ => None,
    }
}

fn same_variant_refinement_base(expected: &Type, actual: &Type) -> bool {
    match (
        variant_refinement_base_identity_and_arguments(expected),
        variant_refinement_base_identity_and_arguments(actual),
    ) {
        (Some((expected_identity, expected_args)), Some((actual_identity, actual_args))) => {
            expected_identity == actual_identity
                && crate::type_relations::invariant_args_match(expected_args, actual_args)
                && expected_args
                    .iter()
                    .zip(actual_args)
                    .all(|(expected, actual)| resolved_type_identities_match(expected, actual))
        }
        _ => false,
    }
}

fn resolved_type_identities_match(expected: &Type, actual: &Type) -> bool {
    match (expected, actual) {
        (
            Type::Named {
                identity: expected_identity,
                args: expected_args,
                ..
            }
            | Type::VariantRefinement {
                identity: expected_identity,
                args: expected_args,
                ..
            },
            Type::Named {
                identity: actual_identity,
                args: actual_args,
                ..
            }
            | Type::VariantRefinement {
                identity: actual_identity,
                args: actual_args,
                ..
            },
        ) => {
            expected_identity == actual_identity
                && expected_args.len() == actual_args.len()
                && expected_args
                    .iter()
                    .zip(actual_args)
                    .all(|(expected, actual)| resolved_type_identities_match(expected, actual))
        }
        (Type::Record(expected), Type::Record(actual)) => {
            crate::type_relations::record_fields_match_by_name(
                expected,
                actual,
                resolved_type_identities_match,
            )
        }
        (
            Type::Function {
                params: expected_params,
                variadic: expected_variadic,
                return_type: expected_return,
                ..
            },
            Type::Function {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                ..
            },
        ) => {
            expected_params.len() == actual_params.len()
                && expected_params
                    .iter()
                    .zip(actual_params)
                    .all(|(expected, actual)| resolved_type_identities_match(expected, actual))
                && match (expected_variadic, actual_variadic) {
                    (Some(expected), Some(actual)) => {
                        resolved_type_identities_match(expected, actual)
                    }
                    (None, None) => true,
                    _ => false,
                }
                && resolved_type_identities_match(expected_return, actual_return)
        }
        (Type::Unknown, Type::Unknown) => true,
        _ => true,
    }
}

fn alternative_span(alternative: &veln_ast::VariantRefinementAlternative) -> SourceSpan {
    let mut span = joined_span(&alternative.base.segment_spans);
    span.end = alternative.variant_span.end;
    span
}

fn variant_refinement_base_renders(expected: &Type, actual: &Type) -> (String, String) {
    let (
        Type::Named {
            name: expected_name,
            identity: expected_identity,
            args: expected_args,
        }
        | Type::VariantRefinement {
            name: expected_name,
            identity: expected_identity,
            args: expected_args,
            ..
        },
        Type::Named {
            name: actual_name,
            identity: actual_identity,
            args: actual_args,
        }
        | Type::VariantRefinement {
            name: actual_name,
            identity: actual_identity,
            args: actual_args,
            ..
        },
    ) = (expected, actual)
    else {
        unreachable!("successfully instantiated refinement has a named ADT base")
    };
    let (expected_name, actual_name) =
        if expected_name == actual_name && expected_identity != actual_identity {
            (expected_identity.as_str(), actual_identity.as_str())
        } else {
            (expected_name.as_str(), actual_name.as_str())
        };
    let mut expected_arguments = Vec::with_capacity(expected_args.len());
    let mut actual_arguments = Vec::with_capacity(actual_args.len());
    for (index, expected_argument) in expected_args.iter().enumerate() {
        if let Some(actual_argument) = actual_args.get(index) {
            let (expected_render, actual_render) =
                diagnostic_type_renders((expected_argument, actual_argument));
            expected_arguments.push(expected_render);
            actual_arguments.push(actual_render);
        } else {
            expected_arguments.push(expected_argument.render());
        }
    }
    actual_arguments.extend(
        actual_args
            .iter()
            .skip(expected_args.len())
            .map(Type::render),
    );
    let expected_render = if expected_arguments.is_empty() {
        expected_name.to_string()
    } else {
        format!("{expected_name}<{}>", expected_arguments.join(", "))
    };
    let actual_render = if actual_arguments.is_empty() {
        actual_name.to_string()
    } else {
        format!("{actual_name}<{}>", actual_arguments.join(", "))
    };
    (expected_render, actual_render)
}

fn diagnostic_type_renders(expected: (&Type, &Type)) -> (String, String) {
    let (expected, actual) = expected;
    diagnostic_named_type_renders(expected, actual)
        .or_else(|| diagnostic_variant_type_renders(expected, actual))
        .or_else(|| diagnostic_record_type_renders(expected, actual))
        .or_else(|| diagnostic_function_type_renders(expected, actual))
        .unwrap_or_else(|| (expected.render(), actual.render()))
}

fn diagnostic_named_type_renders(expected: &Type, actual: &Type) -> Option<(String, String)> {
    match (expected, actual) {
        (
            Type::Named {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
            },
            Type::Named {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
            },
        ) if expected_args.len() == actual_args.len() => {
            let (expected_name, actual_name) = diagnostic_type_names(
                expected_name,
                expected_identity,
                actual_name,
                actual_identity,
            );
            let (expected_arguments, actual_arguments) =
                diagnostic_type_arguments(expected_args, actual_args);
            if expected_arguments.is_empty() && expected_name == "Unit" && actual_name == "Unit" {
                Some(("()".to_string(), "()".to_string()))
            } else {
                Some((
                    diagnostic_type_application(expected_name, &expected_arguments),
                    diagnostic_type_application(actual_name, &actual_arguments),
                ))
            }
        }
        _ => None,
    }
}

fn diagnostic_variant_type_renders(expected: &Type, actual: &Type) -> Option<(String, String)> {
    match (expected, actual) {
        (
            Type::VariantRefinement {
                name: expected_name,
                identity: expected_identity,
                args: expected_args,
                variants: expected_variants,
                ..
            },
            Type::VariantRefinement {
                name: actual_name,
                identity: actual_identity,
                args: actual_args,
                variants: actual_variants,
                ..
            },
        ) if expected_args.len() == actual_args.len() => {
            let (expected_name, actual_name) = diagnostic_type_names(
                expected_name,
                expected_identity,
                actual_name,
                actual_identity,
            );
            let (expected_arguments, actual_arguments) =
                diagnostic_type_arguments(expected_args, actual_args);
            let expected_base = diagnostic_type_application(expected_name, &expected_arguments);
            let actual_base = diagnostic_type_application(actual_name, &actual_arguments);
            Some((
                expected_variants
                    .iter()
                    .map(|variant| format!("{expected_base}::{variant}"))
                    .collect::<Vec<_>>()
                    .join(" | "),
                actual_variants
                    .iter()
                    .map(|variant| format!("{actual_base}::{variant}"))
                    .collect::<Vec<_>>()
                    .join(" | "),
            ))
        }
        _ => None,
    }
}

fn diagnostic_record_type_renders(expected: &Type, actual: &Type) -> Option<(String, String)> {
    let (Type::Record(expected_fields), Type::Record(actual_fields)) = (expected, actual) else {
        return None;
    };
    if expected_fields.len() != actual_fields.len() {
        return None;
    }

    struct FieldPair<'a> {
        actual: &'a Type,
        rendered_actual: Option<String>,
    }

    let mut pairs_by_name = HashMap::with_capacity(actual_fields.len());
    for (name, actual) in actual_fields {
        record_diagnostic_record_render_work(DiagnosticRecordRenderOperation::IndexInsertion);
        pairs_by_name.entry(name.as_str()).or_insert(FieldPair {
            actual,
            rendered_actual: None,
        });
    }

    for (name, _) in expected_fields {
        record_diagnostic_record_render_work(DiagnosticRecordRenderOperation::PairingLookup);
        pairs_by_name.get(name.as_str())?;
    }

    let mut rendered_expected_fields = Vec::with_capacity(expected_fields.len());
    for (name, expected) in expected_fields {
        record_diagnostic_record_render_work(DiagnosticRecordRenderOperation::PairingLookup);
        let pair = pairs_by_name.get_mut(name.as_str())?;
        let (rendered_expected, rendered_actual) = diagnostic_type_renders((expected, pair.actual));
        if pair.rendered_actual.is_none() {
            pair.rendered_actual = Some(rendered_actual);
        }
        rendered_expected_fields.push(format!("{name}: {rendered_expected}"));
    }

    let mut rendered_actual_fields = Vec::with_capacity(actual_fields.len());
    for (name, _) in actual_fields {
        record_diagnostic_record_render_work(
            DiagnosticRecordRenderOperation::ResultReorderingLookup,
        );
        let rendered_actual = pairs_by_name.get(name.as_str())?.rendered_actual.as_ref()?;
        rendered_actual_fields.push(format!("{name}: {rendered_actual}"));
    }

    Some((
        format!("{{{}}}", rendered_expected_fields.join(", ")),
        format!("{{{}}}", rendered_actual_fields.join(", ")),
    ))
}

fn diagnostic_function_type_renders(expected: &Type, actual: &Type) -> Option<(String, String)> {
    match (expected, actual) {
        (
            Type::Function {
                params: expected_params,
                variadic: expected_variadic,
                return_type: expected_return,
                effects: expected_effects,
            },
            Type::Function {
                params: actual_params,
                variadic: actual_variadic,
                return_type: actual_return,
                effects: actual_effects,
            },
        ) if expected_params.len() == actual_params.len()
            && expected_variadic.is_some() == actual_variadic.is_some() =>
        {
            let (mut expected_params, mut actual_params): (Vec<_>, Vec<_>) = expected_params
                .iter()
                .zip(actual_params)
                .map(diagnostic_type_renders)
                .unzip();
            if let (Some(expected), Some(actual)) = (expected_variadic, actual_variadic) {
                let (expected, actual) = diagnostic_type_renders((expected, actual));
                expected_params.push(format!("...{expected}"));
                actual_params.push(format!("...{actual}"));
            }
            let (expected_return, actual_return) =
                diagnostic_type_renders((expected_return, actual_return));
            let expected_effects = if expected_effects.is_empty() {
                String::new()
            } else {
                format!(" effects [{}]", expected_effects.join(", "))
            };
            let actual_effects = if actual_effects.is_empty() {
                String::new()
            } else {
                format!(" effects [{}]", actual_effects.join(", "))
            };
            Some((
                format!(
                    "fn({}) -> {expected_return}{expected_effects}",
                    expected_params.join(", ")
                ),
                format!(
                    "fn({}) -> {actual_return}{actual_effects}",
                    actual_params.join(", ")
                ),
            ))
        }
        _ => None,
    }
}

fn diagnostic_type_names<'a>(
    expected_name: &'a str,
    expected_identity: &'a str,
    actual_name: &'a str,
    actual_identity: &'a str,
) -> (&'a str, &'a str) {
    if expected_name == actual_name && expected_identity != actual_identity {
        (
            if TypeParameterIdentityContext::is_identity(expected_identity) {
                expected_name
            } else {
                expected_identity
            },
            if TypeParameterIdentityContext::is_identity(actual_identity) {
                actual_name
            } else {
                actual_identity
            },
        )
    } else {
        (expected_name, actual_name)
    }
}

fn diagnostic_type_arguments(expected: &[Type], actual: &[Type]) -> (Vec<String>, Vec<String>) {
    expected
        .iter()
        .zip(actual)
        .map(diagnostic_type_renders)
        .unzip()
}

fn diagnostic_type_application(name: &str, arguments: &[String]) -> String {
    if arguments.is_empty() {
        name.to_string()
    } else {
        format!("{name}<{}>", arguments.join(", "))
    }
}

fn variant_union_base_diagnostic(
    expected_base_type: &str,
    actual_base_type: &str,
    expected_span: &SourceSpan,
    actual_span: SourceSpan,
    union_scope: SourceSpan,
) -> VariantRefinementBaseDiagnostic {
    let mut diagnostic = Diagnostic::new(
        "type.variant_union_base",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "variant-refinement base `{actual_base_type}` does not match expected base `{expected_base_type}`"
        ),
        Some(actual_span.clone()),
        JsonValue::object([
            ("expected_base_type", JsonValue::string(expected_base_type)),
            ("actual_base_type", JsonValue::string(actual_base_type)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("expected_variant_union_base")),
        (
            "message",
            JsonValue::string(format!(
                "The earliest independently instantiated base identity established `{expected_base_type}` as the union base."
            )),
        ),
        ("span", span_json(expected_span)),
    ]));
    VariantRefinementBaseDiagnostic {
        diagnostic,
        ordering_scope: actual_span,
        union_scope: Some(union_scope),
    }
}

fn collect_annotation(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
    parse_location: ParseDiagnosticLocation,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    crate::pipeline::record_variant_refinement_annotation_work(refinements.len() + paths.len() + 1);
    let type_parameter_context = TypeParameterIdentityContext::new(type_parameters);
    let recovery = if annotation_needs_recovery(
        refinements,
        paths,
        current_module,
        environment,
        &type_parameter_context,
    ) {
        analyze_annotation(
            refinements,
            paths,
            current_module,
            environment,
            type_parameters,
        )
    } else {
        AnnotationRecovery::default()
    };
    collect_annotation_diagnostics(
        refinements,
        paths,
        current_module,
        environment,
        &recovery,
        parse_location,
        diagnostics,
    );
}

fn collect_annotation_diagnostics(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    recovery: &AnnotationRecovery,
    parse_location: ParseDiagnosticLocation,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    diagnostics.extend(recovery.union_base_failures.iter().map(|failure| {
        variant_union_base_diagnostic(
            &failure.expected_base_type,
            &failure.actual_base_type,
            &failure.expected_span,
            failure.actual_span.clone(),
            failure.union_scope.clone(),
        )
    }));
    collect_refinements(
        refinements,
        current_module,
        environment,
        &recovery.unknown_variant_occurrences,
        &recovery.unresolved_base_occurrences,
        parse_location,
        diagnostics,
    );
    for path in paths {
        if let Some(failure) =
            environment.recovered_variant_refinement_base_failure(&path.segments, current_module)
        {
            let base_segments = &path.segments[..path.segments.len() - 1];
            let written_type = base_segments.join("::");
            let span = joined_span(&path.segment_spans[..base_segments.len()]);
            diagnostics.push(base_diagnostic(
                written_type,
                span,
                joined_span(&path.segment_spans),
                None,
                failure,
            ));
        } else if recovery
            .unknown_variant_occurrences
            .contains(&span_occurrence(&joined_span(&path.segment_spans)))
            && let Some(descriptor) = environment
                .recovered_variant_refinement_unknown_descriptor(&path.segments, current_module)
        {
            let variant_index = path.segments.len() - 1;
            diagnostics.push(unknown_variant_diagnostic_from_parts(
                descriptor.type_name.clone(),
                path.segments[variant_index].clone(),
                path.segment_spans[variant_index].clone(),
                joined_span(&path.segment_spans),
                None,
                &path.segments[..variant_index],
                descriptor,
                current_module,
                environment,
            ));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_declaration_annotation(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    type_parameters: &[String],
    node_id: String,
    origin_node_ids: impl IntoIterator<Item = String> + Clone,
    parse_location: ParseDiagnosticLocation,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    crate::pipeline::record_variant_refinement_annotation_work(refinements.len() + paths.len() + 1);
    let type_parameter_context = TypeParameterIdentityContext::new(type_parameters);
    let has_base_failure = annotation_has_base_failure(
        refinements,
        paths,
        current_module,
        environment,
        &type_parameter_context,
    );
    let recovery = if has_base_failure
        || annotation_needs_recovery(
            refinements,
            paths,
            current_module,
            environment,
            &type_parameter_context,
        ) {
        analyze_annotation(
            refinements,
            paths,
            current_module,
            environment,
            type_parameters,
        )
    } else {
        AnnotationRecovery::default()
    };
    collect_annotation_diagnostics(
        refinements,
        paths,
        current_module,
        environment,
        &recovery,
        parse_location,
        diagnostics,
    );
    if !has_base_failure
        && recovery.errors.is_empty()
        && recovery.union_base_failures.is_empty()
        && recovery.unknown_variant_occurrences.is_empty()
    {
        return;
    }
    for (annotation, error, span) in recovery.errors {
        let diagnostic = Diagnostic::new(
            "type.invalid_annotation",
            Severity::Error,
            DiagnosticKind::Type,
            format!("invalid type annotation `{annotation}`: {error}"),
            Some(span.clone()),
            type_details(
                node_id.clone(),
                "valid_type",
                annotation,
                "source",
                "source",
                "assignable",
                origin_node_ids.clone(),
            ),
        );
        diagnostics.push(VariantRefinementBaseDiagnostic {
            diagnostic,
            ordering_scope: span,
            union_scope: None,
        });
    }
}

fn collect_refinements(
    refinements: &[veln_ast::VariantRefinementType],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    unknown_variant_occurrences: &HashSet<PathOccurrence>,
    unresolved_base_occurrences: &HashSet<PathOccurrence>,
    parse_location: ParseDiagnosticLocation,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for refinement in refinements {
        crate::pipeline::record_variant_refinement_annotation_work(1);
        for alternative in &refinement.alternatives {
            if !alternative_is_selected(alternative, current_module, environment) {
                continue;
            }
            crate::pipeline::record_variant_refinement_annotation_work(1);
            collect_refinement_alternative_diagnostics(
                alternative,
                refinement,
                current_module,
                environment,
                unknown_variant_occurrences,
                unresolved_base_occurrences,
                parse_location,
                diagnostics,
            );
            for argument in &alternative.type_arguments {
                collect_refinements(
                    &argument.ty_refinements,
                    current_module,
                    environment,
                    unknown_variant_occurrences,
                    unresolved_base_occurrences,
                    parse_location,
                    diagnostics,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_refinement_alternative_diagnostics(
    alternative: &veln_ast::VariantRefinementAlternative,
    refinement: &veln_ast::VariantRefinementType,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    unknown_variant_occurrences: &HashSet<PathOccurrence>,
    unresolved_base_occurrences: &HashSet<PathOccurrence>,
    parse_location: ParseDiagnosticLocation,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    if combined_lowercase_refinement_candidate(alternative) {
        diagnostics.push(lowercase_final_parse_diagnostic(
            alternative,
            parse_location,
        ));
    }
    let base = alternative.base.segments.join("::");
    if unresolved_base_occurrences.contains(&span_occurrence(&joined_span(
        &alternative.base.segment_spans,
    ))) {
        diagnostics.push(unresolved_refinement_base_diagnostic(
            alternative,
            refinement,
        ));
    } else if let Some(failure) = environment.variant_refinement_base_failure(
        &base,
        alternative.type_arguments.len(),
        current_module,
    ) {
        diagnostics.push(base_diagnostic(
            written_refinement_base(alternative),
            joined_span(&alternative.base.segment_spans),
            alternative.span.clone(),
            Some(refinement.span.clone()),
            failure,
        ));
    } else if unknown_variant_occurrences.contains(&span_occurrence(&alternative.span))
        && let Some(descriptor) = environment.variant_refinement_unknown_descriptor(
            &base,
            alternative.type_arguments.len(),
            &alternative.variant,
            current_module,
        )
    {
        diagnostics.push(unknown_variant_diagnostic(
            alternative,
            refinement,
            descriptor,
            current_module,
            environment,
        ));
    }
}

fn unresolved_refinement_base_diagnostic(
    alternative: &veln_ast::VariantRefinementAlternative,
    refinement: &veln_ast::VariantRefinementType,
) -> VariantRefinementBaseDiagnostic {
    let symbol = alternative.base.segments.join("::");
    let diagnostic = Diagnostic::new(
        "name.unresolved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("unresolved type `{symbol}`"),
        Some(joined_span(&alternative.base.segment_spans)),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::Null),
            ("symbol", JsonValue::string(symbol)),
            ("namespace", JsonValue::string("type")),
            ("resolution_status", JsonValue::string("unresolved")),
            ("candidates", JsonValue::array([])),
        ]),
    );
    VariantRefinementBaseDiagnostic {
        diagnostic,
        ordering_scope: alternative.span.clone(),
        union_scope: Some(refinement.span.clone()),
    }
}

fn combined_lowercase_refinement_candidate(
    alternative: &veln_ast::VariantRefinementAlternative,
) -> bool {
    alternative.type_arguments.is_empty()
        && alternative
            .base
            .segments
            .last()
            .and_then(|segment| segment.as_bytes().first())
            .is_some_and(u8::is_ascii_lowercase)
        && alternative
            .variant
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_lowercase)
}

fn unknown_variant_diagnostic(
    alternative: &veln_ast::VariantRefinementAlternative,
    refinement: &veln_ast::VariantRefinementType,
    descriptor: &crate::adt::descriptors::AdtDescriptor,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> VariantRefinementBaseDiagnostic {
    unknown_variant_diagnostic_from_parts(
        resolved_refinement_base(alternative, descriptor),
        alternative.variant.clone(),
        alternative.variant_span.clone(),
        alternative.span.clone(),
        Some(refinement.span.clone()),
        &alternative.base.segments,
        descriptor,
        current_module,
        environment,
    )
}

fn lowercase_final_parse_diagnostic(
    alternative: &veln_ast::VariantRefinementAlternative,
    parse_location: ParseDiagnosticLocation,
) -> VariantRefinementBaseDiagnostic {
    let diagnostic = Diagnostic::new(
        "parse.variant_refinement_type",
        Severity::Error,
        DiagnosticKind::Parse,
        "variant refinement final segment must start with an ASCII uppercase letter",
        Some(alternative.variant_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("parse")),
            ("node_id", JsonValue::Null),
            ("parser_context", JsonValue::string(parse_location.context)),
            (
                "unexpected",
                JsonValue::object([
                    ("kind", JsonValue::string("identifier")),
                    ("text", JsonValue::string(alternative.variant.clone())),
                ]),
            ),
            (
                "expected",
                JsonValue::array([JsonValue::string("NamedAdtType::Variant")]),
            ),
            (
                "recovery",
                JsonValue::object([
                    ("strategy", JsonValue::string("insert_token")),
                    ("anchor", JsonValue::string(parse_location.recovery_anchor)),
                    ("dropped_token_count", JsonValue::Number(0)),
                ]),
            ),
        ]),
    );
    VariantRefinementBaseDiagnostic {
        diagnostic,
        ordering_scope: alternative.span.clone(),
        union_scope: None,
    }
}

fn resolved_refinement_base(
    alternative: &veln_ast::VariantRefinementAlternative,
    descriptor: &crate::adt::descriptors::AdtDescriptor,
) -> String {
    if alternative.type_arguments.is_empty() {
        return descriptor.type_name.clone();
    }
    let arguments = alternative
        .type_arguments
        .iter()
        .map(render_type_argument)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{}<{arguments}>", descriptor.type_name)
}

#[allow(clippy::too_many_arguments)]
fn unknown_variant_diagnostic_from_parts(
    base_type: String,
    variant: String,
    variant_span: SourceSpan,
    ordering_scope: SourceSpan,
    union_scope: Option<SourceSpan>,
    written_base_segments: &[String],
    descriptor: &crate::adt::descriptors::AdtDescriptor,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> VariantRefinementBaseDiagnostic {
    let mut diagnostic = Diagnostic::new(
        "type.variant_refinement_unknown",
        Severity::Error,
        DiagnosticKind::Type,
        format!("variant `{variant}` is not a variant of `{base_type}`"),
        Some(variant_span),
        JsonValue::object([
            ("base_type", JsonValue::string(base_type)),
            ("variant", JsonValue::string(variant)),
        ]),
    );
    let resolved_identity = descriptor.identity();
    let mut base_related = vec![
        ("kind", JsonValue::string("base_type_declaration")),
        (
            "message",
            JsonValue::string(format!(
                "Type `{}` is the resolved ADT for this refinement.",
                descriptor.type_name
            )),
        ),
        (
            "resolved_identity",
            JsonValue::string(resolved_identity.clone()),
        ),
    ];
    if let Some(span) = environment.adts.declaration_span(descriptor) {
        base_related.push(("span", span_json(span)));
    }
    diagnostic.related.push(JsonValue::object(base_related));
    for visible in descriptor.variants.iter().filter(|candidate| {
        environment.variant_refinement_variant_visible(
            descriptor,
            candidate,
            written_base_segments,
            current_module,
        )
    }) {
        let mut related = vec![
            ("kind", JsonValue::string("visible_variant_declaration")),
            (
                "message",
                JsonValue::string(format!(
                    "Visible variant `{}` is declared here.",
                    visible.name
                )),
            ),
            ("variant", JsonValue::string(&visible.name)),
            ("resolved_identity", JsonValue::string(&resolved_identity)),
        ];
        if let Some(span) = environment
            .adts
            .variant_declaration_span(descriptor, visible)
        {
            related.push(("span", span_json(span)));
        }
        diagnostic.related.push(JsonValue::object(related));
    }
    VariantRefinementBaseDiagnostic {
        diagnostic,
        ordering_scope,
        union_scope,
    }
}

fn collect_body_line(
    line: &BodyLine,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    match &line.kind {
        BodyLineKind::Let {
            annotation_structure,
            expr,
            ..
        } => {
            collect_annotation(
                &annotation_structure.variant_refinements,
                &annotation_structure.paths,
                current_module,
                environment,
                &[],
                ParseDiagnosticLocation::type_annotation("let_statement"),
                diagnostics,
            );
            collect_expr(
                expr,
                current_module,
                environment,
                "let_statement",
                diagnostics,
            );
        }
        BodyLineKind::Expr { expr } => {
            collect_expr(
                expr,
                current_module,
                environment,
                "expression_line",
                diagnostics,
            );
        }
        BodyLineKind::Defer { body, .. } => {
            for line in body {
                collect_body_line(line, current_module, environment, diagnostics);
            }
        }
    }
}

fn collect_expr(
    expr: &Expr,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    parser_context: &'static str,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    if let ExprKind::TypeApply {
        type_arg_paths,
        type_arg_refinements,
        ..
    } = &expr.kind
    {
        for (refinements, paths) in type_arg_refinements.iter().zip(type_arg_paths) {
            collect_declaration_annotation(
                refinements,
                paths,
                current_module,
                environment,
                &[],
                expr.node_id.display("expr"),
                [expr.node_id.display("expr")],
                ParseDiagnosticLocation::type_argument(parser_context),
                diagnostics,
            );
        }
    }
    if let ExprKind::Begin { body, .. } = &expr.kind {
        for line in body {
            collect_body_line(line, current_module, environment, diagnostics);
        }
    } else {
        expr.for_each_child(&mut |child| {
            collect_expr(
                child,
                current_module,
                environment,
                parser_context,
                diagnostics,
            )
        });
    }
}

fn base_diagnostic(
    written_type: String,
    span: SourceSpan,
    ordering_scope: SourceSpan,
    union_scope: Option<SourceSpan>,
    failure: VariantRefinementBaseFailure,
) -> VariantRefinementBaseDiagnostic {
    let message = match failure.reason {
        "not_adt" => format!("variant-refinement base `{written_type}` is not a finite ADT"),
        "opaque" => {
            format!("variant-refinement base `{written_type}` is opaque at this annotation")
        }
        _ => unreachable!("closed provider-independent base reason"),
    };
    let mut diagnostic = Diagnostic::new(
        "type.variant_refinement_base",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(span),
        JsonValue::object([
            ("written_type", JsonValue::string(written_type)),
            ("reason", JsonValue::string(failure.reason)),
        ]),
    );
    let mut related = vec![
        ("kind", JsonValue::string(failure.related_kind)),
        ("message", JsonValue::string(failure.related_message)),
        (
            "resolved_identity",
            JsonValue::string(failure.resolved_identity),
        ),
    ];
    if let Some(provider) = failure.provider {
        related.push(("provider", JsonValue::string(provider)));
    }
    if let Some(declaration_span) = failure.declaration_span {
        related.push(("span", span_json(&declaration_span)));
    }
    diagnostic.related.push(JsonValue::object(related));
    VariantRefinementBaseDiagnostic {
        diagnostic,
        ordering_scope,
        union_scope,
    }
}

fn joined_span(spans: &[SourceSpan]) -> SourceSpan {
    let mut span = spans.first().expect("type path has a segment").clone();
    span.end = spans.last().expect("type path has a segment").end;
    span
}

fn written_refinement_base(alternative: &veln_ast::VariantRefinementAlternative) -> String {
    let base = alternative.base.segments.join("::");
    if alternative.type_arguments.is_empty() {
        return base;
    }
    let arguments = alternative
        .type_arguments
        .iter()
        .map(render_type_argument)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{base}<{arguments}>")
}

fn render_type_argument(argument: &veln_ast::VariantRefinementTypeArgument) -> String {
    let mut text = String::new();
    for (index, fragment) in argument.ty_fragments.iter().enumerate() {
        text.push_str(fragment);
        if let Some(refinement) = argument.ty_refinements.get(index) {
            text.push_str(&render_refinement(refinement));
        }
    }
    text
}

fn render_refinement(refinement: &veln_ast::VariantRefinementType) -> String {
    refinement
        .alternatives
        .iter()
        .map(|alternative| {
            format!(
                "{}::{}",
                written_refinement_base(alternative),
                alternative.variant
            )
        })
        .collect::<Vec<_>>()
        .join(" | ")
}
