use std::collections::{BTreeMap, HashMap, HashSet};

use veln_ast::{
    InvalidName, NameClass, NameOccurrence, QualifiedPathSegment, QualifiedPathSegmentEvidence,
    SurfaceModule,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};

use crate::prelude::{qualified_prelude_builtin_signature_with_input, qualified_prelude_signature};
use crate::types::TypeEnvironment;

type RecoveredQualifiedSegmentPush = fn(
    &[String],
    &[veln_source::SourceSpan],
    Option<&str>,
    &veln_source::SourceSpan,
    &TypeEnvironment,
    &mut Vec<InvalidName>,
);

mod occurrence_index;
mod recovered_segments;
mod recovered_traversal;
mod role_resolution;
mod valid_segments;

use occurrence_index::{
    EnclosingFunctionSpanIndex, QualifiedPathOccurrenceIndex, classified_invalid_path_segment,
    name_satisfies_class,
};
use recovered_segments::*;
use recovered_traversal::*;
use role_resolution::*;
use valid_segments::valid_qualified_path_segments;
pub use valid_segments::{
    CanonicalTypeAnnotationIdentity, CanonicalTypeAnnotationWork,
    VariantRefinementClassificationWork, canonical_type_annotation_identities_with_context,
    canonical_type_annotation_identities_with_context_and_work,
    classified_project_qualified_path_segments,
    classified_project_qualified_path_segments_with_context,
    classified_project_qualified_path_segments_with_context_and_work,
};

pub(super) fn check_invalid_name_casing(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut invalid_names = classified_invalid_names(module, environment);
    radix_sort_by_span(&mut invalid_names, |invalid| &invalid.span);
    invalid_names.dedup_by(|left, right| {
        left.class == right.class
            && left.occurrence == right.occurrence
            && left.span.file == right.span.file
            && left.span.start.offset == right.span.start.offset
            && left.span.end.offset == right.span.end.offset
    });
    invalid_names.iter().map(invalid_name_diagnostic).collect()
}

fn classified_invalid_names(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<InvalidName> {
    let classified_segments = classified_qualified_path_segments(module, environment);
    let mut enclosing_functions = EnclosingFunctionSpanIndex::new(module);
    let mut invalid_names = module
        .invalid_names
        .iter()
        .inspect(|_| crate::pipeline::record_variant_refinement_casing_work(1))
        .filter(|invalid| invalid.occurrence != NameOccurrence::PathSegment)
        .filter(|invalid| !invalid_name_is_valid_constructor_pattern(invalid, module, environment))
        .cloned()
        .collect::<Vec<_>>();
    invalid_names.extend(
        classified_segments
            .into_iter()
            .filter(|segment| !name_satisfies_class(&segment.name, segment.role))
            .map(|segment| {
                let enclosing_function_span = enclosing_functions.for_ordered_segment(&segment);
                InvalidName {
                    name: segment.name,
                    class: segment.role,
                    occurrence: segment.occurrence,
                    span: segment.span,
                    enclosing_function_span,
                    segment_index: Some(segment.segment_index),
                }
            }),
    );
    invalid_names
}

fn classified_qualified_path_segments(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<QualifiedPathSegment> {
    classified_qualified_path_segments_with_refinements(module, environment, true, false)
}

fn classified_qualified_path_segments_for_navigation(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<QualifiedPathSegment> {
    classified_qualified_path_segments_with_refinements(module, environment, true, true)
}

fn classified_qualified_path_segments_with_refinements(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
    include_variant_refinements: bool,
    require_valid_variant_refinements: bool,
) -> Vec<QualifiedPathSegment> {
    let mut segments = valid_qualified_path_segments(
        module,
        environment,
        include_variant_refinements,
        require_valid_variant_refinements,
    );
    segments.extend(recovered_qualified_type_segments(module, environment));
    segments.extend(recovered_qualified_module_segments(module, environment));
    segments.extend(recovered_qualified_function_segments(module, environment));
    let resolved_locations = segments
        .iter()
        .inspect(|_| crate::pipeline::record_variant_refinement_casing_work(1))
        .filter(|segment| segment.evidence != QualifiedPathSegmentEvidence::Syntax)
        .map(|segment| {
            (
                segment.span.file.as_str().to_string(),
                segment.span.start.offset,
                segment.span.end.offset,
                segment.segment_index,
            )
        })
        .collect::<HashSet<_>>();
    segments.retain(|segment| {
        crate::pipeline::record_variant_refinement_casing_work(1);
        segment.evidence != QualifiedPathSegmentEvidence::Syntax
            || !resolved_locations.contains(&(
                segment.span.file.as_str().to_string(),
                segment.span.start.offset,
                segment.span.end.offset,
                segment.segment_index,
            ))
    });
    let mut classified_locations = HashMap::<String, HashSet<(usize, usize, usize)>>::new();
    for segment in &segments {
        crate::pipeline::record_variant_refinement_casing_work(1);
        classified_locations
            .entry(segment.span.file.as_str().to_string())
            .or_default()
            .insert((
                segment.span.start.offset,
                segment.span.end.offset,
                segment.segment_index,
            ));
    }
    let occurrence_index = QualifiedPathOccurrenceIndex::new(module);
    segments.extend(
        module
            .invalid_names
            .iter()
            .inspect(|_| crate::pipeline::record_variant_refinement_casing_work(1))
            .filter(|invalid| invalid.occurrence == NameOccurrence::PathSegment)
            .filter(|invalid| {
                !invalid_path_segment_is_already_classified(invalid, &classified_locations)
            })
            .filter_map(|invalid| {
                classified_invalid_path_segment(invalid, &occurrence_index, module, environment)
            }),
    );
    radix_sort_by_span(&mut segments, |segment| &segment.span);
    segments.dedup_by(|left, right| {
        left.role == right.role
            && left.occurrence == right.occurrence
            && left.span.file == right.span.file
            && left.span.start.offset == right.span.start.offset
            && left.span.end.offset == right.span.end.offset
    });
    segments
}

fn invalid_path_segment_is_already_classified(
    invalid: &InvalidName,
    classified_locations: &HashMap<String, HashSet<(usize, usize, usize)>>,
) -> bool {
    let Some(segment_index) = invalid.segment_index else {
        return false;
    };
    #[cfg(test)]
    invalid_path_classification_counters::record_index_lookup();
    classified_locations
        .get(invalid.span.file.as_str())
        .is_some_and(|locations| {
            locations.contains(&(
                invalid.span.start.offset,
                invalid.span.end.offset,
                segment_index,
            ))
        })
}

fn radix_sort_by_span<T>(items: &mut Vec<T>, span: fn(&T) -> &veln_source::SourceSpan) {
    if items.len() < 2 {
        return;
    }
    crate::pipeline::record_variant_refinement_casing_work(items.len() * 2);
    let mut indices = (0..items.len()).collect::<Vec<_>>();
    let mut scratch = vec![0; items.len()];
    for use_end in [true, false] {
        for shift in (0..usize::BITS).step_by(8) {
            let mut counts = [0usize; 256];
            crate::pipeline::record_variant_refinement_casing_work(counts.len());
            for &index in &indices {
                crate::pipeline::record_variant_refinement_casing_work(1);
                let source_span = span(&items[index]);
                let key = if use_end {
                    source_span.end.offset
                } else {
                    source_span.start.offset
                };
                counts[(key >> shift) as u8 as usize] += 1;
            }
            let mut next = 0;
            for count in &mut counts {
                crate::pipeline::record_variant_refinement_casing_work(1);
                let start = next;
                next += *count;
                *count = start;
            }
            for &index in &indices {
                crate::pipeline::record_variant_refinement_casing_work(1);
                let source_span = span(&items[index]);
                let key = if use_end {
                    source_span.end.offset
                } else {
                    source_span.start.offset
                };
                let bucket = (key >> shift) as u8 as usize;
                scratch[counts[bucket]] = index;
                counts[bucket] += 1;
            }
            std::mem::swap(&mut indices, &mut scratch);
        }
    }
    let mut original = std::mem::take(items)
        .into_iter()
        .map(Some)
        .collect::<Vec<_>>();
    crate::pipeline::record_variant_refinement_casing_work(original.len() * 2);
    items.extend(
        indices
            .into_iter()
            .map(|index| original[index].take().expect("radix index is unique")),
    );
}

fn invalid_name_diagnostic(invalid: &InvalidName) -> Diagnostic {
    let subject = match invalid.class {
        NameClass::Type => "type name",
        NameClass::Constructor => "constructor name",
        NameClass::Module => "module name",
        NameClass::Function => "function name",
        NameClass::ValueBinding => "binding name",
    };
    let subject = if invalid.occurrence == NameOccurrence::AliasTarget {
        match invalid.class {
            NameClass::Type => "type alias target",
            NameClass::Function => "function alias target",
            NameClass::Constructor | NameClass::Module | NameClass::ValueBinding => subject,
        }
    } else {
        subject
    };
    let required_letter = match invalid.class {
        NameClass::Type | NameClass::Constructor => "uppercase",
        NameClass::Module | NameClass::Function | NameClass::ValueBinding => "lowercase",
    };
    let observed_initial = invalid.name.as_bytes().first().map_or("other", |initial| {
        if initial.is_ascii_uppercase() {
            "ascii_uppercase"
        } else if initial.is_ascii_lowercase() {
            "ascii_lowercase"
        } else if *initial == b'_' {
            "underscore"
        } else {
            "other"
        }
    });
    let mut details = vec![
        ("phase", JsonValue::string("name")),
        ("origin", JsonValue::string("source")),
        ("occurrence", JsonValue::string(invalid.occurrence.as_str())),
        ("name", JsonValue::string(invalid.name.clone())),
        ("name_class", JsonValue::string(invalid.class.as_str())),
        (
            "required_initial",
            JsonValue::string(invalid.class.required_initial()),
        ),
        ("observed_initial", JsonValue::string(observed_initial)),
    ];
    if let Some(index) = invalid.segment_index {
        details.push(("segment_index", JsonValue::Number(index as i64)));
    }
    Diagnostic::new(
        "name.invalid_case",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "{subject} `{}` must start with an ASCII {required_letter} letter",
            invalid.name
        ),
        Some(invalid.span.clone()),
        JsonValue::object(details),
    )
}

#[cfg(test)]
mod invalid_path_classification_counters {
    use std::cell::Cell;

    thread_local! {
        static INDEX_LOOKUPS: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn reset() {
        INDEX_LOOKUPS.set(0);
    }

    pub(super) fn record_index_lookup() {
        INDEX_LOOKUPS.set(INDEX_LOOKUPS.get() + 1);
    }

    pub(super) fn index_lookups() -> usize {
        INDEX_LOOKUPS.get()
    }
}

#[cfg(test)]
mod classification_tests;
