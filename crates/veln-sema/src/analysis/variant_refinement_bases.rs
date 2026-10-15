use std::collections::HashMap;

use veln_ast::{BodyLine, BodyLineKind, Expr, ExprKind, SurfaceModule, TypePathSegments};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::diagnostics::{span_json, type_details};
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_annotation;
use crate::types::{TypeEnvironment, VariantRefinementBaseFailure};

pub(crate) struct VariantRefinementBaseDiagnostic {
    pub(crate) diagnostic: Diagnostic,
    pub(crate) ordering_scope: SourceSpan,
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
                    field.node_id.display("field"),
                    [
                        decl.node_id.display("type"),
                        variant.node_id.display("variant"),
                        field.node_id.display("field"),
                    ],
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
                    param.node_id.display("param"),
                    [
                        effect.node_id.display("effect"),
                        operation.node_id.display("operation"),
                        param.node_id.display("param"),
                    ],
                    diagnostics,
                );
            }
            collect_declaration_annotation(
                &operation.return_type_refinements,
                &operation.return_type_paths,
                effect.module_name.as_deref(),
                environment,
                operation.node_id.display("operation"),
                [
                    effect.node_id.display("effect"),
                    operation.node_id.display("operation"),
                ],
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
                field.node_id.display("schema-field"),
                [
                    schema.node_id.display("schema"),
                    field.node_id.display("schema-field"),
                ],
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
                diagnostics,
            );
        }
        collect_annotation(
            &function.return_type_refinements,
            &function.return_type_paths,
            function.module_name.as_deref(),
            environment,
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
                param.node_id.display("param"),
                [
                    handler.node_id.display("handler"),
                    param.node_id.display("param"),
                ],
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
                    param.node_id.display("param"),
                    [
                        handler.node_id.display("handler"),
                        clause.node_id.display("clause"),
                        param.node_id.display("param"),
                    ],
                    diagnostics,
                );
            }
            collect_expr(
                &clause.body,
                handler.module_name.as_deref(),
                environment,
                diagnostics,
            );
        }
    }
}

pub(crate) fn annotation_has_base_failure(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    paths.iter().any(|path| {
        environment
            .recovered_variant_refinement_base_failure(&path.segments, current_module)
            .is_some()
    }) || refinements.iter().any(|refinement| {
        refinement.alternatives.iter().any(|alternative| {
            let base = alternative.base.segments.join("::");
            environment
                .variant_refinement_base_failure(
                    &base,
                    alternative.type_arguments.len(),
                    current_module,
                )
                .is_some()
                || alternative.type_arguments.iter().any(|argument| {
                    annotation_has_base_failure(
                        &argument.ty_refinements,
                        &argument.ty_paths,
                        current_module,
                        environment,
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
    let mut errors = Vec::new();
    let mut visited_paths = HashMap::new();
    collect_independent_refinement_errors(
        refinements,
        paths,
        current_module,
        environment,
        &mut errors,
        &mut visited_paths,
    );
    errors
}

type PathOccurrence = (veln_source::SourcePath, usize, usize);

fn collect_independent_refinement_errors(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
) -> bool {
    let mut annotation_is_invalid = false;
    for path in paths {
        let span = joined_span(&path.segment_spans).resolved_or_generated();
        let occurrence = (span.file, span.start.offset, span.end.offset);
        if let Some(path_is_invalid) = visited_paths.get(&occurrence) {
            annotation_is_invalid |= path_is_invalid;
            continue;
        }
        let path_is_invalid = if environment
            .recovered_variant_refinement_base_failure(&path.segments, current_module)
            .is_some()
        {
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
    for refinement in refinements {
        let mut expected_union_base = None;
        for alternative in &refinement.alternatives {
            let Some((written, canonical)) = collect_refinement_alternative_type(
                alternative,
                current_module,
                environment,
                errors,
                visited_paths,
            ) else {
                annotation_is_invalid = true;
                continue;
            };
            if let Some(expected) = &expected_union_base {
                crate::pipeline::record_variant_refinement_recovery_work(1);
                if !same_variant_refinement_base(expected, &canonical) {
                    errors.push((
                        written,
                        "variant refinement alternatives must resolve to declared variants of one ADT"
                            .to_string(),
                        alternative_span(alternative),
                    ));
                    annotation_is_invalid = true;
                }
            } else {
                expected_union_base = Some(canonical);
            }
        }
    }
    annotation_is_invalid
}

fn collect_refinement_alternative_type(
    alternative: &veln_ast::VariantRefinementAlternative,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
) -> Option<(String, Type)> {
    crate::pipeline::record_variant_refinement_recovery_work(1);
    let base = alternative.base.segments.join("::");
    let own_base_failure = environment
        .variant_refinement_base_failure(&base, alternative.type_arguments.len(), current_module)
        .is_some();
    let child_is_invalid = collect_refinement_argument_errors(
        &alternative.type_arguments,
        current_module,
        environment,
        errors,
        visited_paths,
    );
    let written = format!(
        "{}::{}",
        written_refinement_base(alternative),
        alternative.variant
    );
    let ty = match parse_type_annotation(&written) {
        Ok(ty) => ty,
        Err(error) => {
            if !child_is_invalid {
                errors.push((written, error, alternative_span(alternative)));
            }
            return None;
        }
    };
    if let Some(error) = environment.variant_refinement_arity_error(&ty, current_module) {
        errors.push((written, error, alternative_span(alternative)));
        return None;
    }
    if own_base_failure || child_is_invalid {
        return None;
    }
    if let Some(error) = environment.variant_refinement_annotation_error(&ty, current_module) {
        errors.push((written, error, alternative_span(alternative)));
        return None;
    }
    let canonical = environment.canonicalize_type_annotation(ty, current_module);
    Some((written, canonical))
}

fn collect_refinement_argument_errors(
    arguments: &[veln_ast::VariantRefinementTypeArgument],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String, SourceSpan)>,
    visited_paths: &mut HashMap<PathOccurrence, bool>,
) -> bool {
    let mut child_is_invalid = false;
    for argument in arguments {
        let argument_errors_start = errors.len();
        let argument_is_invalid = collect_independent_refinement_errors(
            &argument.ty_refinements,
            &argument.ty_paths,
            current_module,
            environment,
            errors,
            visited_paths,
        );
        child_is_invalid |= argument_is_invalid;
        if argument_is_invalid || errors.len() != argument_errors_start {
            continue;
        }
        let written_argument = render_type_argument(argument);
        let error = match parse_type_annotation(&written_argument) {
            Ok(ty) => environment.variant_refinement_arity_error(&ty, current_module),
            Err(error) => Some(error),
        };
        if let Some(error) = error {
            errors.push((written_argument, error, argument.span.clone()));
            child_is_invalid = true;
        }
    }
    child_is_invalid
}

fn same_variant_refinement_base(expected: &Type, actual: &Type) -> bool {
    fn identity_and_arguments(ty: &Type) -> Option<(&str, &[Type])> {
        match ty {
            Type::Named { identity, args, .. } | Type::VariantRefinement { identity, args, .. } => {
                Some((identity, args))
            }
            _ => None,
        }
    }

    match (
        identity_and_arguments(expected),
        identity_and_arguments(actual),
    ) {
        (Some((expected_identity, expected_args)), Some((actual_identity, actual_args))) => {
            expected_identity == actual_identity
                && crate::type_relations::invariant_args_match(expected_args, actual_args)
        }
        _ => false,
    }
}

fn alternative_span(alternative: &veln_ast::VariantRefinementAlternative) -> SourceSpan {
    let mut span = joined_span(&alternative.base.segment_spans);
    span.end = alternative.variant_span.end;
    span
}

fn collect_annotation(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    crate::pipeline::record_variant_refinement_annotation_work(refinements.len() + paths.len() + 1);
    collect_refinements(refinements, current_module, environment, diagnostics);
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
                failure,
            ));
        }
    }
}

fn collect_declaration_annotation(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    node_id: String,
    origin_node_ids: impl IntoIterator<Item = String> + Clone,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    collect_annotation(refinements, paths, current_module, environment, diagnostics);
    if !annotation_has_base_failure(refinements, paths, current_module, environment) {
        return;
    }
    for (annotation, error, span) in
        independent_annotation_errors(refinements, paths, current_module, environment)
    {
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
        });
    }
}

fn collect_refinements(
    refinements: &[veln_ast::VariantRefinementType],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<VariantRefinementBaseDiagnostic>,
) {
    for refinement in refinements {
        crate::pipeline::record_variant_refinement_annotation_work(1);
        for alternative in &refinement.alternatives {
            crate::pipeline::record_variant_refinement_annotation_work(1);
            let base = alternative.base.segments.join("::");
            if let Some(failure) = environment.variant_refinement_base_failure(
                &base,
                alternative.type_arguments.len(),
                current_module,
            ) {
                diagnostics.push(base_diagnostic(
                    written_refinement_base(alternative),
                    joined_span(&alternative.base.segment_spans),
                    alternative.span.clone(),
                    failure,
                ));
            }
            for argument in &alternative.type_arguments {
                collect_refinements(
                    &argument.ty_refinements,
                    current_module,
                    environment,
                    diagnostics,
                );
            }
        }
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
                diagnostics,
            );
            collect_expr(expr, current_module, environment, diagnostics);
        }
        BodyLineKind::Expr { expr } => {
            collect_expr(expr, current_module, environment, diagnostics);
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
                expr.node_id.display("expr"),
                [expr.node_id.display("expr")],
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
            collect_expr(child, current_module, environment, diagnostics)
        });
    }
}

fn base_diagnostic(
    written_type: String,
    span: SourceSpan,
    ordering_scope: SourceSpan,
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
