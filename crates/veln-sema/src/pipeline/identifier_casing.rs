use std::collections::{BTreeMap, BTreeSet};

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
    QualifiedPathOccurrenceIndex, classified_invalid_path_segment,
    enclosing_function_span_for_segment, name_satisfies_class,
};
use recovered_segments::*;
use recovered_traversal::*;
use role_resolution::*;
use valid_segments::valid_qualified_path_segments;
pub use valid_segments::{
    classified_project_qualified_path_segments,
    classified_project_qualified_path_segments_with_context,
};

pub(super) fn check_invalid_name_casing(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut invalid_names = classified_invalid_names(module, environment);
    invalid_names.sort_by_key(|invalid| (invalid.span.start.offset, invalid.span.end.offset));
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
    let mut invalid_names = module
        .invalid_names
        .iter()
        .filter(|invalid| invalid.occurrence != NameOccurrence::PathSegment)
        .filter(|invalid| !invalid_name_is_valid_constructor_pattern(invalid, module, environment))
        .cloned()
        .collect::<Vec<_>>();
    invalid_names.extend(
        classified_segments
            .into_iter()
            .filter(|segment| !name_satisfies_class(&segment.name, segment.role))
            .map(|segment| {
                let enclosing_function_span = enclosing_function_span_for_segment(module, &segment);
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
    let mut segments = valid_qualified_path_segments(module, environment);
    segments.extend(recovered_qualified_type_segments(module, environment));
    segments.extend(recovered_qualified_module_segments(module, environment));
    segments.extend(recovered_qualified_function_segments(module, environment));
    let classified_keys = segments
        .iter()
        .map(classified_segment_key)
        .collect::<BTreeSet<_>>();
    let occurrence_index = QualifiedPathOccurrenceIndex::new(module);
    segments.extend(
        module
            .invalid_names
            .iter()
            .filter(|invalid| invalid.occurrence == NameOccurrence::PathSegment)
            .filter(|invalid| {
                !invalid_path_segment_is_already_classified(invalid, &classified_keys)
            })
            .filter_map(|invalid| {
                classified_invalid_path_segment(invalid, &occurrence_index, module, environment)
            }),
    );
    segments.sort_by_key(|segment| (segment.span.start.offset, segment.span.end.offset));
    segments.dedup_by(|left, right| {
        left.role == right.role
            && left.occurrence == right.occurrence
            && left.span.file == right.span.file
            && left.span.start.offset == right.span.start.offset
            && left.span.end.offset == right.span.end.offset
    });
    segments
}

fn classified_segment_key(
    segment: &QualifiedPathSegment,
) -> (String, usize, usize, usize, &'static str) {
    (
        segment.span.file.as_str().to_string(),
        segment.span.start.offset,
        segment.span.end.offset,
        segment.segment_index,
        segment.role.as_str(),
    )
}

fn invalid_path_segment_is_already_classified(
    invalid: &InvalidName,
    classified_keys: &BTreeSet<(String, usize, usize, usize, &'static str)>,
) -> bool {
    let Some(segment_index) = invalid.segment_index else {
        return false;
    };
    classified_keys.contains(&(
        invalid.span.file.as_str().to_string(),
        invalid.span.start.offset,
        invalid.span.end.offset,
        segment_index,
        invalid.class.as_str(),
    ))
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
