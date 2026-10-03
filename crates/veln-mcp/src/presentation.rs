use serde_json::{Value, json};
use veln_language_service::{
    CompletionCandidateKind, SourcePosition, completion_at, signature_help_at,
};
use veln_source::SourcePath;

use crate::check_project::{CaptureCache, capture_navigation_source};
use crate::definition::{coordinate, valid_position};
use crate::language_resources::LanguageResources;
use crate::outcome::{ToolOutcome, domain_failure};
use crate::workspace::{Selection, WorkspaceBase};

pub(crate) fn completion(
    base: &WorkspaceBase,
    selection: &Selection,
    language_resources: &mut LanguageResources,
    capture_cache: &mut CaptureCache,
    arguments: &Value,
) -> ToolOutcome {
    with_position(
        base,
        selection,
        language_resources,
        capture_cache,
        arguments,
        |snapshot, position| {
            let items = completion_at(snapshot, &position)
                .into_iter()
                .map(|candidate| {
                    let kind = match candidate.kind {
                        CompletionCandidateKind::DeclarationModifier => "modifier",
                        CompletionCandidateKind::BuiltinLocal => "builtin_local",
                    };
                    json!({"label": candidate.label, "kind": kind, "detail": candidate.detail})
                })
                .collect::<Vec<_>>();
            json!({"items": items})
        },
    )
}

pub(crate) fn signature_help(
    base: &WorkspaceBase,
    selection: &Selection,
    language_resources: &mut LanguageResources,
    capture_cache: &mut CaptureCache,
    arguments: &Value,
) -> ToolOutcome {
    with_position(
        base,
        selection,
        language_resources,
        capture_cache,
        arguments,
        |snapshot, position| {
            let signature = signature_help_at(snapshot, position).map(|help| {
                json!({
                    "label": help.label,
                    "parameters": help.parameters,
                    "activeParameter": help.active_parameter
                })
            });
            json!({"signature": signature})
        },
    )
}

fn with_position(
    base: &WorkspaceBase,
    selection: &Selection,
    language_resources: &mut LanguageResources,
    capture_cache: &mut CaptureCache,
    arguments: &Value,
    render: impl FnOnce(&veln_language_service::EffectiveProjectSnapshot, SourcePosition) -> Value,
) -> ToolOutcome {
    let source = arguments["source"]
        .as_str()
        .expect("presentation input schema requires a source");
    let line = coordinate(&arguments["line"]);
    let column = coordinate(&arguments["column"]);
    let (captured, captured_source, _) =
        match capture_navigation_source(base, selection, source, capture_cache) {
            Ok(captured) => captured,
            Err(failure) => return failure,
        };
    let source_file = captured
        .project
        .files
        .iter()
        .find(|file| file.path().as_str() == captured_source)
        .expect("presentation capture contains the requested source");
    if !valid_position(source_file.text(), line, column) {
        return domain_failure(
            "invalid_position",
            "position is outside the selected source",
            json!({"source": source, "line": arguments["line"].clone(), "column": arguments["column"].clone()}),
        );
    }
    let (
        crate::definition::Coordinate::Addressable(line),
        crate::definition::Coordinate::Addressable(column),
    ) = (line, column)
    else {
        unreachable!("validated coordinates are addressable")
    };
    let snapshot = language_resources.read_only_navigation_snapshot(
        captured.project.files,
        &captured.dependencies,
        captured.key,
    );
    ToolOutcome::Success(render(
        &snapshot,
        SourcePosition {
            source: SourcePath::new(captured_source),
            line,
            column,
        },
    ))
}
