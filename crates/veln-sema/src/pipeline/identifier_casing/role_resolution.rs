use super::*;

pub(super) fn invalid_function_segment_lacks_function_role(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::Function
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .filter(|occurrence| occurrence.call_role)
        .any(|occurrence| {
            invalid_function_segment_lacks_function_role_for_path(
                invalid,
                &occurrence.segments,
                &occurrence.segment_spans,
                occurrence.current_module.as_deref(),
                environment,
            )
        })
}

pub(super) fn invalid_function_segment_lacks_function_role_for_path(
    invalid: &InvalidName,
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    if index + 1 != segments.len() {
        return false;
    }
    let Some(span) = segment_spans.get(index) else {
        return false;
    };
    if span.file != invalid.span.file
        || span.start.offset != invalid.span.start.offset
        || span.end.offset != invalid.span.end.offset
    {
        return false;
    }
    environment
        .function_path(segments, current_module)
        .is_none()
        && environment
            .codec_call_path(segments, current_module)
            .is_empty()
        && !lowercase_corrected_function_path_resolves(
            invalid,
            segments,
            current_module,
            environment,
        )
}

pub(super) fn lowercase_corrected_function_path_resolves(
    invalid: &InvalidName,
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(&invalid.name);
    environment
        .function_path(&corrected, current_module)
        .is_some()
        || !environment
            .codec_call_path(&corrected, current_module)
            .is_empty()
}

pub(super) fn invalid_value_segment_lacks_value_role_for_path(
    invalid: &InvalidName,
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    let Some(span) = segment_spans.get(index) else {
        return false;
    };
    if span.file != invalid.span.file
        || span.start.offset != invalid.span.start.offset
        || span.end.offset != invalid.span.end.offset
    {
        return false;
    }
    if invalid.class == NameClass::Module {
        return !module_segment_role_is_fixed(invalid, segments, current_module, environment);
    }
    if !path_resolves_as_value(segments, current_module, environment)
        && !lowercase_corrected_value_path_resolves(invalid, segments, current_module, environment)
    {
        return true;
    }
    matches!(
        environment
            .adts
            .nullary_constructor(segments, current_module, &environment.uses),
        crate::adt::registry::ConstructorLookup::Found(_)
            | crate::adt::registry::ConstructorLookup::Ambiguous
    )
}

pub(super) fn module_segment_role_is_fixed(
    invalid: &InvalidName,
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    if index + 1 >= segments.len() {
        return false;
    }
    if type_qualified_constructor_path(invalid, segments, current_module, environment)
        || path_resolves_as_value(segments, current_module, environment)
        || environment
            .function_path(segments, current_module)
            .is_some()
        || !environment
            .codec_call_path(segments, current_module)
            .is_empty()
        || environment.quarantined_import_value_recovery_candidate_count(segments, current_module)
            == 1
        || environment.quarantined_import_constructor_recovery_candidate_count(
            segments,
            current_module,
            None,
        ) == 1
        || qualified_prelude_signature(segments, None).is_some()
        || qualified_prelude_builtin_signature_with_input(segments, None, None).is_some()
    {
        return true;
    }
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(&invalid.name);
    path_resolves_as_value(&corrected, current_module, environment)
        || matches!(
            environment
                .adts
                .constructor(&corrected, current_module, &environment.uses),
            crate::adt::registry::ConstructorLookup::Found(_)
        )
        || environment
            .function_path(&corrected, current_module)
            .is_some()
        || !environment
            .codec_call_path(&corrected, current_module)
            .is_empty()
        || environment.quarantined_import_value_recovery_candidate_count(&corrected, current_module)
            == 1
        || environment.quarantined_import_constructor_recovery_candidate_count(
            &corrected,
            current_module,
            None,
        ) == 1
        || qualified_prelude_signature(&corrected, None).is_some()
        || qualified_prelude_builtin_signature_with_input(&corrected, None, None).is_some()
}

pub(super) fn type_qualified_constructor_path(
    invalid: &InvalidName,
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    invalid.segment_index == Some(0)
        && environment
            .adts
            .constructor_candidates(segments, current_module, &environment.uses)
            .iter()
            .any(|constructor| constructor.descriptor.type_name == invalid.name)
}

pub(super) fn path_resolves_as_value(
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    environment
        .function_path_for_value(segments, current_module)
        .is_some()
        || matches!(
            environment
                .adts
                .nullary_constructor(segments, current_module, &environment.uses),
            crate::adt::registry::ConstructorLookup::Found(_)
                | crate::adt::registry::ConstructorLookup::Ambiguous
        )
}

pub(super) fn lowercase_corrected_value_path_resolves(
    invalid: &InvalidName,
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(&invalid.name);
    path_resolves_as_value(&corrected, current_module, environment)
}

pub(super) fn invalid_constructor_segment_lacks_constructor_role(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if !matches!(invalid.class, NameClass::Constructor | NameClass::Function)
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .filter(|occurrence| occurrence.call_role || occurrence.pattern_role)
        .any(|occurrence| {
            invalid_constructor_segment_lacks_constructor_role_for_path(
                invalid,
                &occurrence.segments,
                &occurrence.segment_spans,
                occurrence.current_module.as_deref(),
                environment,
                occurrence.pattern_role,
            )
        })
}

pub(super) fn invalid_constructor_segment_has_function_role(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::Constructor
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .filter(|occurrence| occurrence.call_role)
        .any(|occurrence| {
            invalid_constructor_segment_has_function_role_for_path(
                invalid,
                &occurrence.segments,
                &occurrence.segment_spans,
                occurrence.current_module.as_deref(),
                environment,
            )
        })
}

pub(super) fn invalid_constructor_segment_has_function_role_for_path(
    invalid: &InvalidName,
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    if index + 1 != segments.len() {
        return false;
    }
    let Some(span) = segment_spans.get(index) else {
        return false;
    };
    if span.file != invalid.span.file
        || span.start.offset != invalid.span.start.offset
        || span.end.offset != invalid.span.end.offset
    {
        return false;
    }
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(&invalid.name);
    for segment in corrected.iter_mut().take(index) {
        if segment
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_uppercase)
        {
            *segment = lowercase_initial(segment);
        }
    }
    environment
        .function_path(&corrected, current_module)
        .is_some()
        || !environment
            .codec_call_path(&corrected, current_module)
            .is_empty()
}

pub(super) fn invalid_constructor_segment_lacks_constructor_role_for_path(
    invalid: &InvalidName,
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
    pattern_role: bool,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    if index + 1 != segments.len() {
        return false;
    }
    let Some(span) = segment_spans.get(index) else {
        return false;
    };
    if span.file != invalid.span.file
        || span.start.offset != invalid.span.start.offset
        || span.end.offset != invalid.span.end.offset
    {
        return false;
    }
    match invalid.class {
        NameClass::Constructor => {
            if !pattern_role
                && segments.get(index.saturating_sub(1)).is_none_or(|segment| {
                    !segment
                        .as_bytes()
                        .first()
                        .is_some_and(u8::is_ascii_uppercase)
                })
            {
                return true;
            }
            environment
                .function_path(segments, current_module)
                .is_some()
                || !environment
                    .codec_call_path(segments, current_module)
                    .is_empty()
        }
        NameClass::Function => {
            matches!(
                environment
                    .adts
                    .constructor(segments, current_module, &environment.uses),
                crate::adt::registry::ConstructorLookup::Found(_)
                    | crate::adt::registry::ConstructorLookup::Ambiguous
            ) || !environment
                .codec_call_path(segments, current_module)
                .is_empty()
        }
        NameClass::Type | NameClass::Module | NameClass::ValueBinding => false,
    }
}

pub(super) fn invalid_type_segment_lacks_constructor_role(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::Type
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .filter(|occurrence| occurrence.call_role || occurrence.pattern_role)
        .any(|occurrence| {
            invalid_type_segment_lacks_constructor_role_for_path(
                invalid,
                &occurrence.segments,
                &occurrence.segment_spans,
                occurrence.current_module.as_deref(),
                environment,
            )
        })
}

pub(super) fn invalid_type_segment_has_module_role(
    invalid: &InvalidName,
    occurrences: &QualifiedPathOccurrenceIndex,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::Type
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index.is_none()
    {
        return false;
    }
    occurrences
        .occurrences_for(invalid)
        .iter()
        .filter(|occurrence| occurrence.call_role || occurrence.pattern_role)
        .any(|occurrence| {
            invalid_type_segment_has_module_role_for_path(
                invalid,
                &occurrence.segments,
                &occurrence.segment_spans,
                occurrence.current_module.as_deref(),
                environment,
            )
        })
}

pub(super) fn invalid_type_segment_has_module_role_for_path(
    invalid: &InvalidName,
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    let Some(span) = segment_spans.get(index) else {
        return false;
    };
    if span.file != invalid.span.file
        || span.start.offset != invalid.span.start.offset
        || span.end.offset != invalid.span.end.offset
        || index + 1 >= segments.len()
    {
        return false;
    }
    let mut corrected = segments.to_vec();
    corrected[index] = lowercase_initial(&invalid.name);
    if index + 2 == corrected.len()
        && path_resolves_as_constructor(&corrected, current_module, environment)
    {
        return false;
    }
    module_segment_role_is_fixed(invalid, &corrected, current_module, environment)
}

pub(super) fn invalid_type_segment_lacks_constructor_role_for_path(
    invalid: &InvalidName,
    segments: &[String],
    segment_spans: &[veln_source::SourceSpan],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    let Some(index) = invalid.segment_index else {
        return false;
    };
    let Some(span) = segment_spans.get(index) else {
        return false;
    };
    if span.file != invalid.span.file
        || span.start.offset != invalid.span.start.offset
        || span.end.offset != invalid.span.end.offset
    {
        return false;
    }
    if path_resolves_as_constructor(segments, current_module, environment) {
        return true;
    }
    if environment
        .function_path(segments, current_module)
        .is_some()
        || !environment
            .codec_call_path(segments, current_module)
            .is_empty()
    {
        return true;
    }
    if segments
        .last()
        .is_some_and(|leaf| leaf.as_bytes().first().is_some_and(u8::is_ascii_lowercase))
    {
        return true;
    }
    if segments.len() < 3 {
        return true;
    }
    if index + 2 != segments.len() {
        return false;
    }
    let mut corrected = segments.to_vec();
    corrected[index] = uppercase_initial(&invalid.name);
    !path_resolves_as_constructor(&corrected, current_module, environment)
        && environment.quarantined_import_constructor_recovery_candidate_count(
            &corrected,
            current_module,
            None,
        ) != 1
}

pub(super) fn path_resolves_as_constructor(
    segments: &[String],
    current_module: Option<&str>,
    environment: &TypeEnvironment,
) -> bool {
    matches!(
        environment
            .adts
            .constructor(segments, current_module, &environment.uses),
        crate::adt::registry::ConstructorLookup::Found(_)
            | crate::adt::registry::ConstructorLookup::Ambiguous
    )
}

pub(super) fn uppercase_initial(name: &str) -> String {
    let Some((_, first)) = name.char_indices().next() else {
        return String::new();
    };
    let rest = &name[first.len_utf8()..];
    first.to_ascii_uppercase().to_string() + rest
}

pub(super) fn lowercase_initial(name: &str) -> String {
    let Some((_, first)) = name.char_indices().next() else {
        return String::new();
    };
    let rest = &name[first.len_utf8()..];
    first.to_ascii_lowercase().to_string() + rest
}

pub(super) fn invalid_name_repeats_quarantined_import_alias(
    invalid: &InvalidName,
    module: &SurfaceModule,
) -> bool {
    if invalid.class != NameClass::Module
        || invalid.occurrence != NameOccurrence::PathSegment
        || invalid.segment_index != Some(0)
    {
        return false;
    }
    if module.uses.iter().any(|use_decl| {
        use_decl.span.file == invalid.span.file
            && use_decl.span.start.offset <= invalid.span.start.offset
            && invalid.span.end.offset <= use_decl.span.end.offset
    }) {
        return false;
    }
    module.uses.iter().any(|use_decl| {
        let alias = use_decl
            .name
            .rsplit("::")
            .next()
            .unwrap_or(use_decl.name.as_str());
        crate::name_recovery::use_decl_has_invalid_module_segment(module, use_decl)
            && use_decl.span.file == invalid.span.file
            && (use_decl.alias == invalid.name || alias == invalid.name)
    })
}

pub(super) fn invalid_name_is_valid_constructor_pattern(
    invalid: &InvalidName,
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> bool {
    if invalid.class != NameClass::ValueBinding || invalid.occurrence != NameOccurrence::PatternHead
    {
        return false;
    }
    let current_module = invalid.enclosing_function_span.as_ref().and_then(|span| {
        module
            .functions
            .iter()
            .find(|function| &function.span == span)
            .and_then(|function| function.module_name.as_deref())
    });
    matches!(
        environment.adts.constructor(
            std::slice::from_ref(&invalid.name),
            current_module,
            &environment.uses,
        ),
        crate::adt::registry::ConstructorLookup::Found(_)
            | crate::adt::registry::ConstructorLookup::Ambiguous
    )
}
