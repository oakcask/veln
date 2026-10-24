use super::*;

pub(super) fn collect_valid_segments_from_body_line(
    line: &veln_ast::BodyLine,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    include_variant_refinements: bool,
    require_valid_variant_refinements: bool,
    output: &mut Vec<QualifiedPathSegment>,
    suppressed_locations: &mut HashMap<String, HashSet<(usize, usize, usize)>>,
) {
    match &line.kind {
        veln_ast::BodyLineKind::Let {
            pattern,
            annotation_structure,
            expr,
            ..
        } => {
            collect_valid_segments_from_pattern(pattern, current_module, environment, output);
            collect_type_path_segments(
                &annotation_structure.paths,
                &annotation_structure.variant_refinements,
                current_module,
                environment,
                output,
                suppressed_locations,
            );
            if include_variant_refinements {
                collect_variant_refinement_segments(
                    &annotation_structure.variant_refinements,
                    current_module,
                    environment,
                    &[],
                    require_valid_variant_refinements,
                    output,
                    suppressed_locations,
                );
            }
            collect_valid_segments_from_expr(
                expr,
                current_module,
                environment,
                include_variant_refinements,
                require_valid_variant_refinements,
                output,
                suppressed_locations,
            );
        }
        veln_ast::BodyLineKind::Expr { expr } => {
            collect_valid_segments_from_expr(
                expr,
                current_module,
                environment,
                include_variant_refinements,
                require_valid_variant_refinements,
                output,
                suppressed_locations,
            );
        }
        veln_ast::BodyLineKind::Defer { body, .. } => {
            for line in body {
                collect_valid_segments_from_body_line(
                    line,
                    current_module,
                    environment,
                    include_variant_refinements,
                    require_valid_variant_refinements,
                    output,
                    suppressed_locations,
                );
            }
        }
    }
}

pub(super) fn collect_valid_segments_from_expr(
    expr: &veln_ast::Expr,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    include_variant_refinements: bool,
    require_valid_variant_refinements: bool,
    output: &mut Vec<QualifiedPathSegment>,
    suppressed_locations: &mut HashMap<String, HashSet<(usize, usize, usize)>>,
) {
    match &expr.kind {
        veln_ast::ExprKind::NamePath {
            segments,
            segment_spans,
        } => {
            collect_valid_expr_path_segments(
                segments,
                segment_spans,
                current_module,
                environment,
                output,
            );
        }
        veln_ast::ExprKind::Call { callee, args } => {
            if let veln_ast::ExprKind::NamePath {
                segments,
                segment_spans,
            } = &callee.kind
            {
                collect_valid_call_path_segments(
                    segments,
                    segment_spans,
                    current_module,
                    environment,
                    output,
                );
            } else {
                collect_valid_segments_from_expr(
                    callee,
                    current_module,
                    environment,
                    include_variant_refinements,
                    require_valid_variant_refinements,
                    output,
                    suppressed_locations,
                );
            }
            for arg in args {
                collect_valid_segments_from_expr(
                    arg,
                    current_module,
                    environment,
                    include_variant_refinements,
                    require_valid_variant_refinements,
                    output,
                    suppressed_locations,
                );
            }
        }
        veln_ast::ExprKind::TypeApply {
            callee,
            type_arg_refinements,
            ..
        } => {
            collect_valid_segments_from_expr(
                callee,
                current_module,
                environment,
                include_variant_refinements,
                require_valid_variant_refinements,
                output,
                suppressed_locations,
            );
            if include_variant_refinements && require_valid_variant_refinements {
                for refinements in type_arg_refinements {
                    collect_variant_refinement_segments(
                        refinements,
                        current_module,
                        environment,
                        &[],
                        require_valid_variant_refinements,
                        output,
                        suppressed_locations,
                    );
                }
            }
        }
        veln_ast::ExprKind::Match { scrutinee, arms } => {
            collect_valid_segments_from_expr(
                scrutinee,
                current_module,
                environment,
                include_variant_refinements,
                require_valid_variant_refinements,
                output,
                suppressed_locations,
            );
            for arm in arms {
                collect_valid_segments_from_pattern(
                    &arm.pattern,
                    current_module,
                    environment,
                    output,
                );
                collect_valid_segments_from_expr(
                    &arm.expr,
                    current_module,
                    environment,
                    include_variant_refinements,
                    require_valid_variant_refinements,
                    output,
                    suppressed_locations,
                );
            }
        }
        veln_ast::ExprKind::Begin { body, .. } => {
            for line in body {
                collect_valid_segments_from_body_line(
                    line,
                    current_module,
                    environment,
                    include_variant_refinements,
                    require_valid_variant_refinements,
                    output,
                    suppressed_locations,
                );
            }
        }
        _ => expr.for_each_child(&mut |child| {
            collect_valid_segments_from_expr(
                child,
                current_module,
                environment,
                include_variant_refinements,
                require_valid_variant_refinements,
                output,
                suppressed_locations,
            );
        }),
    }
}

fn collect_valid_segments_from_pattern(
    pattern: &veln_ast::Pattern,
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    match &pattern.kind {
        veln_ast::PatternKind::Constructor {
            name,
            name_spans,
            args,
        } => {
            collect_valid_constructor_path_segments(
                name,
                name_spans,
                current_module,
                environment,
                output,
            );
            for arg in args {
                collect_valid_segments_from_pattern(arg, current_module, environment, output);
            }
        }
        veln_ast::PatternKind::Record(fields) => {
            for field in fields {
                collect_valid_segments_from_pattern(
                    &field.pattern,
                    current_module,
                    environment,
                    output,
                );
            }
        }
        _ => {}
    }
}

fn collect_valid_expr_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    if segments.len() < 2 {
        return;
    }
    if environment
        .function_path_for_value(segments, current_module)
        .is_some()
        || qualified_prelude_signature(segments, None).is_some()
        || qualified_prelude_builtin_signature_with_input(segments, None, None).is_some()
    {
        push_module_prefix_and_leaf(segments, segment_spans, NameClass::ValueBinding, output);
        return;
    }
    if matches!(
        environment
            .adts
            .nullary_constructor(segments, current_module, &environment.uses),
        crate::adt::registry::ConstructorLookup::Found(_)
    ) {
        push_constructor_path_segments(
            segments,
            segment_spans,
            current_module,
            environment,
            output,
        );
    }
}

fn collect_valid_call_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    if segments.len() < 2 {
        return;
    }
    if environment
        .function_path(segments, current_module)
        .is_some()
        || qualified_prelude_signature(segments, None).is_some()
        || qualified_prelude_builtin_signature_with_input(segments, None, None).is_some()
    {
        push_module_prefix_and_leaf(segments, segment_spans, NameClass::Function, output);
    } else if path_resolves_as_constructor(segments, current_module, environment) {
        push_constructor_path_segments(
            segments,
            segment_spans,
            current_module,
            environment,
            output,
        );
    }
}

fn collect_valid_constructor_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    if segments.len() < 2 || !path_resolves_as_constructor(segments, current_module, environment) {
        return;
    }
    push_constructor_path_segments(segments, segment_spans, current_module, environment, output);
}

pub(super) fn push_constructor_path_segments(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    output: &mut Vec<QualifiedPathSegment>,
) {
    let type_index = constructor_type_segment_index(segments, current_module, environment);
    for index in 0..segments.len() {
        let Some(span) = segment_spans.get(index) else {
            continue;
        };
        let role = if Some(index) == type_index {
            NameClass::Type
        } else if index + 1 == segments.len() {
            NameClass::Constructor
        } else {
            NameClass::Module
        };
        output.push(qualified_path_segment_from_parts(
            &segments[index],
            role,
            span,
            index,
            QualifiedPathSegmentEvidence::Resolved,
        ));
    }
}

fn constructor_type_segment_index(
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> Option<usize> {
    let crate::adt::registry::ConstructorLookup::Found(constructor) =
        environment
            .adts
            .constructor(segments, current_module, &environment.uses)
    else {
        return None;
    };
    segments[..segments.len().saturating_sub(1)]
        .iter()
        .rposition(|segment| segment == &constructor.descriptor.type_name)
}

fn push_module_prefix_and_leaf(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    leaf_role: NameClass,
    output: &mut Vec<QualifiedPathSegment>,
) {
    for index in 0..segments.len() {
        let Some(span) = segment_spans.get(index) else {
            continue;
        };
        let role = if index + 1 == segments.len() {
            leaf_role
        } else {
            NameClass::Module
        };
        output.push(qualified_path_segment_from_parts(
            &segments[index],
            role,
            span,
            index,
            QualifiedPathSegmentEvidence::Resolved,
        ));
    }
}

pub(super) fn qualified_path_segment_from_parts(
    name: &str,
    role: NameClass,
    span: &veln_source::SourceSpan,
    segment_index: usize,
    evidence: QualifiedPathSegmentEvidence,
) -> QualifiedPathSegment {
    QualifiedPathSegment {
        name: name.to_string(),
        role,
        occurrence: NameOccurrence::PathSegment,
        span: span.clone(),
        segment_index,
        evidence,
    }
}
