use veln_ast::{BodyLine, BodyLineKind, Expr, ExprKind, SurfaceModule, TypePathSegments};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

use crate::diagnostics::span_json;
use crate::type_syntax::parse_type_annotation;
use crate::types::{TypeEnvironment, VariantRefinementBaseFailure};

pub(crate) fn check_variant_refinement_bases(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for decl in &module.types {
        for variant in &decl.variants {
            for field in &variant.fields {
                collect_annotation(
                    &field.ty_refinements,
                    &field.ty_paths,
                    decl.module_name.as_deref(),
                    environment,
                    &mut diagnostics,
                );
            }
        }
    }
    for effect in &module.effects {
        for operation in &effect.operations {
            for param in &operation.params {
                collect_annotation(
                    &param.ty_refinements,
                    &param.ty_paths,
                    effect.module_name.as_deref(),
                    environment,
                    &mut diagnostics,
                );
            }
            collect_annotation(
                &operation.return_type_refinements,
                &operation.return_type_paths,
                effect.module_name.as_deref(),
                environment,
                &mut diagnostics,
            );
        }
    }
    for schema in &module.schemas {
        for field in &schema.fields {
            collect_annotation(
                &field.ty_refinements,
                &field.ty_paths,
                schema.module_name.as_deref(),
                environment,
                &mut diagnostics,
            );
        }
    }
    for function in &module.functions {
        for param in &function.params {
            collect_annotation(
                &param.ty_refinements,
                &param.ty_paths,
                function.module_name.as_deref(),
                environment,
                &mut diagnostics,
            );
        }
        collect_annotation(
            &function.return_type_refinements,
            &function.return_type_paths,
            function.module_name.as_deref(),
            environment,
            &mut diagnostics,
        );
        for line in &function.body {
            collect_body_line(
                line,
                function.module_name.as_deref(),
                environment,
                &mut diagnostics,
            );
        }
    }
    for handler in &module.handlers {
        for param in &handler.params {
            collect_annotation(
                &param.ty_refinements,
                &param.ty_paths,
                handler.module_name.as_deref(),
                environment,
                &mut diagnostics,
            );
        }
        for clause in &handler.operation_clauses {
            for param in &clause.params {
                collect_annotation(
                    &param.ty_refinements,
                    &param.ty_paths,
                    handler.module_name.as_deref(),
                    environment,
                    &mut diagnostics,
                );
            }
            collect_expr(
                &clause.body,
                handler.module_name.as_deref(),
                environment,
                &mut diagnostics,
            );
        }
    }

    diagnostics
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
) -> Vec<(String, String)> {
    let mut errors = Vec::new();
    collect_independent_refinement_errors(
        refinements,
        paths,
        current_module,
        environment,
        &mut errors,
    );
    errors
}

fn collect_independent_refinement_errors(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    errors: &mut Vec<(String, String)>,
) {
    for path in paths {
        if environment
            .recovered_variant_refinement_base_failure(&path.segments, current_module)
            .is_none()
            && let Some(error) = environment.recovered_variant_refinement_annotation_error(
                std::slice::from_ref(path),
                current_module,
            )
        {
            errors.push((path.segments.join("::"), error));
        }
    }
    for refinement in refinements {
        for alternative in &refinement.alternatives {
            let base = alternative.base.segments.join("::");
            let own_base_failure = environment
                .variant_refinement_base_failure(
                    &base,
                    alternative.type_arguments.len(),
                    current_module,
                )
                .is_some();

            let child_errors_start = errors.len();
            for argument in &alternative.type_arguments {
                collect_independent_refinement_errors(
                    &argument.ty_refinements,
                    &argument.ty_paths,
                    current_module,
                    environment,
                    errors,
                );
            }

            let written = format!(
                "{}::{}",
                written_refinement_base(alternative),
                alternative.variant
            );
            let Some(ty) = parse_type_annotation(&written).ok() else {
                continue;
            };
            if let Some(error) = environment.variant_refinement_arity_error(&ty, current_module) {
                errors.push((written, error));
                continue;
            }
            if own_base_failure || errors.len() != child_errors_start {
                continue;
            }
            if let Some(error) =
                environment.variant_refinement_annotation_error(&ty, current_module)
            {
                errors.push((written, error));
            }
        }
    }
}

fn collect_annotation(
    refinements: &[veln_ast::VariantRefinementType],
    paths: &[TypePathSegments],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) {
    collect_refinements(refinements, current_module, environment, diagnostics);
    for path in paths {
        if let Some(failure) =
            environment.recovered_variant_refinement_base_failure(&path.segments, current_module)
        {
            let base_segments = &path.segments[..path.segments.len() - 1];
            let written_type = base_segments.join("::");
            let span = joined_span(&path.segment_spans[..base_segments.len()]);
            diagnostics.push(base_diagnostic(written_type, span, failure));
        }
    }
}

fn collect_refinements(
    refinements: &[veln_ast::VariantRefinementType],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for refinement in refinements {
        for alternative in &refinement.alternatives {
            let base = alternative.base.segments.join("::");
            if let Some(failure) = environment.variant_refinement_base_failure(
                &base,
                alternative.type_arguments.len(),
                current_module,
            ) {
                diagnostics.push(base_diagnostic(
                    written_refinement_base(alternative),
                    joined_span(&alternative.base.segment_spans),
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
    diagnostics: &mut Vec<Diagnostic>,
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
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let ExprKind::TypeApply {
        type_arg_paths,
        type_arg_refinements,
        ..
    } = &expr.kind
    {
        for (refinements, paths) in type_arg_refinements.iter().zip(type_arg_paths) {
            collect_annotation(refinements, paths, current_module, environment, diagnostics);
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
    failure: VariantRefinementBaseFailure,
) -> Diagnostic {
    let message = match failure.reason {
        "not_adt" => format!("variant-refinement base `{written_type}` is not a finite ADT"),
        "opaque" => {
            format!("variant-refinement base `{written_type}` is opaque at this annotation")
        }
        "variant_descriptor_unavailable" => format!(
            "variant-refinement base `{written_type}` has no public finite variant descriptor"
        ),
        _ => unreachable!("closed variant-refinement base reason"),
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
        ("kind", JsonValue::string("base_type_provider")),
        ("message", JsonValue::string(failure.related_message)),
        (
            "resolved_identity",
            JsonValue::string(failure.resolved_identity),
        ),
    ];
    if let Some(declaration_span) = failure.declaration_span {
        related.push(("span", span_json(&declaration_span)));
    }
    diagnostic.related.push(JsonValue::object(related));
    diagnostic
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
