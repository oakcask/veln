use super::*;

pub(super) fn recovered_qualified_type_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<QualifiedPathSegment> {
    let mut invalid = Vec::new();
    for function in &module.functions {
        for line in &function.body {
            collect_recovered_qualified_type_segments_from_body_line(
                line,
                function.module_name.as_deref(),
                &function.span,
                environment,
                &mut invalid,
            );
        }
    }
    for handler in &module.handlers {
        for clause in &handler.operation_clauses {
            collect_recovered_qualified_type_segments_from_expr(
                &clause.body,
                handler.module_name.as_deref(),
                &handler.span,
                environment,
                &mut invalid,
            );
        }
    }
    invalid
        .into_iter()
        .map(|invalid| {
            invalid_name_to_classified_segment(
                &invalid,
                QualifiedPathSegmentEvidence::UniqueRecovery,
            )
        })
        .collect()
}

pub(super) fn recovered_qualified_module_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<QualifiedPathSegment> {
    recovered_qualified_segments(module, environment, push_recovered_module_segment)
}

pub(super) fn recovered_qualified_function_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<QualifiedPathSegment> {
    let mut invalid = Vec::new();
    for function in &module.functions {
        for line in &function.body {
            collect_recovered_qualified_function_segments_from_body_line(
                line,
                function.module_name.as_deref(),
                &function.span,
                environment,
                &mut invalid,
            );
        }
    }
    for handler in &module.handlers {
        for clause in &handler.operation_clauses {
            collect_recovered_qualified_function_segments_from_expr(
                &clause.body,
                handler.module_name.as_deref(),
                &handler.span,
                environment,
                &mut invalid,
            );
        }
    }
    invalid
        .into_iter()
        .map(|invalid| {
            invalid_name_to_classified_segment(
                &invalid,
                QualifiedPathSegmentEvidence::UniqueRecovery,
            )
        })
        .collect()
}

fn recovered_qualified_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    push: RecoveredQualifiedSegmentPush,
) -> Vec<QualifiedPathSegment> {
    let mut invalid = Vec::new();
    for function in &module.functions {
        for line in &function.body {
            collect_recovered_qualified_segments_from_body_line(
                line,
                function.module_name.as_deref(),
                &function.span,
                environment,
                push,
                &mut invalid,
            );
        }
    }
    for handler in &module.handlers {
        for clause in &handler.operation_clauses {
            collect_recovered_qualified_segments_from_expr(
                &clause.body,
                handler.module_name.as_deref(),
                &handler.span,
                environment,
                push,
                &mut invalid,
            );
        }
    }
    invalid
        .into_iter()
        .map(|invalid| {
            invalid_name_to_classified_segment(
                &invalid,
                QualifiedPathSegmentEvidence::UniqueRecovery,
            )
        })
        .collect()
}

fn invalid_name_to_classified_segment(
    invalid: &InvalidName,
    evidence: QualifiedPathSegmentEvidence,
) -> QualifiedPathSegment {
    QualifiedPathSegment {
        name: invalid.name.clone(),
        role: invalid.class,
        occurrence: invalid.occurrence,
        span: invalid.span.clone(),
        segment_index: invalid
            .segment_index
            .expect("classified path segment has segment index"),
        evidence,
    }
}

pub(super) fn push_recovered_qualified_type_segment(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    arg_count: Option<usize>,
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    if segments.len() < 2 {
        return;
    }
    let type_index = segments.len() - 2;
    let type_name = &segments[type_index];
    if type_name
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_uppercase)
    {
        return;
    }
    let Some(span) = segment_spans.get(type_index) else {
        return;
    };
    let mut corrected = segments.to_vec();
    corrected[type_index] = uppercase_initial(type_name);
    let recovered = match arg_count {
        Some(arg_count) => {
            matches!(
                environment
                    .adts
                    .constructor(&corrected, current_module, &environment.uses),
                crate::adt::registry::ConstructorLookup::Found(_)
            ) || environment.quarantined_import_constructor_recovery_candidate_count(
                &corrected,
                current_module,
                Some(arg_count),
            ) == 1
        }
        None => matches!(
            environment
                .adts
                .nullary_constructor(&corrected, current_module, &environment.uses),
            crate::adt::registry::ConstructorLookup::Found(_)
        ),
    };
    if !recovered {
        return;
    }
    invalid.push(InvalidName {
        name: type_name.clone(),
        class: NameClass::Type,
        occurrence: NameOccurrence::PathSegment,
        span: span.clone(),
        enclosing_function_span: Some(enclosing_function_span.clone()),
        segment_index: Some(type_index),
    });
}

pub(super) fn push_recovered_module_segment(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    if segments.len() < 2 {
        return;
    }
    for index in 0..segments.len() - 1 {
        let name = &segments[index];
        if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
            continue;
        }
        let Some(span) = segment_spans.get(index) else {
            continue;
        };
        let mut corrected = segments.to_vec();
        corrected[index] = lowercase_initial(name);
        let probe = InvalidName {
            name: name.clone(),
            class: NameClass::Module,
            occurrence: NameOccurrence::PathSegment,
            span: span.clone(),
            enclosing_function_span: Some(enclosing_function_span.clone()),
            segment_index: Some(index),
        };
        if index + 2 == corrected.len()
            && path_resolves_as_constructor(&corrected, current_module, environment)
        {
            continue;
        }
        if module_segment_role_is_fixed(&probe, &corrected, current_module, environment) {
            invalid.push(probe);
        }
    }
}

pub(super) fn push_recovered_function_segment(
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    enclosing_function_span: &veln_source::SourceSpan,
    environment: &TypeEnvironment,
    invalid: &mut Vec<InvalidName>,
) {
    if segments.len() < 2 {
        return;
    }
    let index = segments.len() - 1;
    let name = &segments[index];
    if !name.as_bytes().first().is_some_and(u8::is_ascii_uppercase) {
        return;
    }
    let Some(span) = segment_spans.get(index) else {
        return;
    };
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(name);
    if !environment
        .codec_call_path(segments, current_module)
        .is_empty()
    {
        return;
    }
    if environment
        .function_path(&corrected, current_module)
        .is_none()
    {
        return;
    }
    invalid.push(InvalidName {
        name: name.clone(),
        class: NameClass::Function,
        occurrence: NameOccurrence::PathSegment,
        span: span.clone(),
        enclosing_function_span: Some(enclosing_function_span.clone()),
        segment_index: Some(index),
    });
}
