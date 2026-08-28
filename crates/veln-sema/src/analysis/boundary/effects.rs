use super::super::private_companion_effect_target_diagnostic;
use crate::effects::KNOWN_EFFECT_LABELS;
use crate::semantic_model::Type;
use crate::type_syntax::parse_type_annotation;
use crate::types::{TypeEnvironment, signatures::UserEffectPathResolution};
use veln_ast::{Function, FunctionKind, Visibility};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

pub(crate) fn check_declared_effect_labels(
    function: &Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut diagnostics = check_function_type_effect_labels(function, environment);
    let Some(declared_effects) = &function.effects else {
        return diagnostics;
    };
    let boundary = declared_effect_boundary(function);
    let node_prefix = function.kind.node_prefix();

    if declared_effects.is_empty() {
        diagnostics.push(empty_declared_effect_diagnostic(
            function,
            node_prefix,
            boundary,
        ));
        return diagnostics;
    }

    diagnostics.extend(check_effect_row_entries(
        function,
        declared_effects,
        function.effect_spans.as_deref(),
        boundary,
    ));
    diagnostics.extend(check_unknown_declared_effects(
        function,
        declared_effects,
        environment,
        node_prefix,
        boundary,
    ));
    diagnostics
}

fn check_unknown_declared_effects(
    function: &Function,
    declared_effects: &[String],
    environment: &TypeEnvironment,
    node_prefix: &'static str,
    boundary: &'static str,
) -> Vec<Diagnostic> {
    declared_effects
        .iter()
        .enumerate()
        .filter_map(|(index, effect)| {
            unknown_declared_effect_diagnostic_for(
                function,
                environment,
                effect,
                index,
                node_prefix,
                boundary,
            )
        })
        .collect()
}

fn unknown_declared_effect_diagnostic_for(
    function: &Function,
    environment: &TypeEnvironment,
    effect: &str,
    index: usize,
    node_prefix: &'static str,
    boundary: &'static str,
) -> Option<Diagnostic> {
    if effect_row_name(effect).is_some() || KNOWN_EFFECT_LABELS.contains(&effect) {
        return None;
    }
    let segments = effect.split("::").map(str::to_string).collect::<Vec<_>>();
    match environment.resolve_user_effect_path(&segments, function.module_name.as_deref()) {
        UserEffectPathResolution::PrivateCompanionTargetMismatch {
            effect: signature,
            access,
        } => Some(private_companion_effect_target_diagnostic(
            function.node_id.display(node_prefix),
            boundary,
            effect,
            signature,
            access,
            declared_effect_span(function, index),
        )),
        UserEffectPathResolution::Missing => Some(unknown_declared_effect_diagnostic(
            function,
            effect,
            index,
            node_prefix,
            boundary,
        )),
        UserEffectPathResolution::Found(_) | UserEffectPathResolution::QuarantinedImportTarget => {
            None
        }
    }
}

fn declared_effect_span(function: &Function, index: usize) -> SourceSpan {
    function
        .effect_spans
        .as_ref()
        .and_then(|spans| spans.get(index))
        .cloned()
        .unwrap_or_else(|| function.span.clone())
}

fn check_effect_row_entries(
    function: &Function,
    effects: &[String],
    spans: Option<&[SourceSpan]>,
    boundary: &'static str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut row_seen = false;
    for (index, effect) in effects.iter().enumerate() {
        let Some(row) = effect_row_name(effect) else {
            continue;
        };
        let span = spans.and_then(|spans| spans.get(index)).cloned();
        if row_seen {
            diagnostics.push(effect_row_diagnostic(
                function,
                "effect.row_multiple",
                "effect set has more than one row tail".to_string(),
                row,
                span.clone(),
                boundary,
            ));
        }
        if index + 1 != effects.len() {
            diagnostics.push(effect_row_diagnostic(
                function,
                "effect.row_non_final",
                "effect row tail must be the final effect".to_string(),
                row,
                span.clone(),
                boundary,
            ));
        }
        if function
            .effect_binder
            .as_ref()
            .is_none_or(|binder| binder.name != row)
        {
            diagnostics.push(effect_row_diagnostic(
                function,
                "effect.row_unbound",
                format!("effect row variable `{row}` is not bound"),
                row,
                span,
                boundary,
            ));
        }
        row_seen = true;
    }
    diagnostics
}

fn effect_row_name(effect: &str) -> Option<&str> {
    effect.strip_prefix("...")
}

fn effect_row_diagnostic(
    function: &Function,
    id: &'static str,
    message: String,
    row: &str,
    span: Option<SourceSpan>,
    boundary: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Effect,
        message,
        span.or_else(|| Some(function.span.clone())),
        JsonValue::object([
            ("phase", JsonValue::string("effect")),
            (
                "node_id",
                JsonValue::string(function.node_id.display(function.kind.node_prefix())),
            ),
            ("row", JsonValue::string(row.to_string())),
            ("boundary", JsonValue::string(boundary)),
        ]),
    )
}

fn check_function_type_effect_labels(
    function: &Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for param in &function.params {
        let (Some(ty), Some(span)) = (&param.ty, &param.ty_span) else {
            continue;
        };
        diagnostics.extend(check_type_effect_labels(
            ty,
            span,
            function,
            environment,
            "parameter_type",
        ));
    }
    if let (Some(ty), Some(span)) = (&function.return_type, &function.return_type_span) {
        diagnostics.extend(check_type_effect_labels(
            ty,
            span,
            function,
            environment,
            "return_type",
        ));
    }
    diagnostics
}

fn check_type_effect_labels(
    annotation: &str,
    annotation_span: &SourceSpan,
    function: &Function,
    environment: &TypeEnvironment,
    boundary: &'static str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(check_annotation_effect_rows(
        annotation,
        annotation_span,
        function,
        boundary,
    ));
    let Ok(ty) = parse_type_annotation(annotation) else {
        return diagnostics;
    };
    collect_unknown_type_effects(
        &ty,
        annotation,
        annotation_span,
        function,
        environment,
        boundary,
        &mut diagnostics,
    );
    diagnostics
}

fn check_annotation_effect_rows(
    annotation: &str,
    annotation_span: &SourceSpan,
    function: &Function,
    boundary: &'static str,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut search_from = 0;
    while let Some(relative_effects) = annotation[search_from..].find("effects") {
        let effects_offset = search_from + relative_effects;
        let Some(open_relative) = annotation[effects_offset..].find('[') else {
            break;
        };
        let open = effects_offset + open_relative;
        let Some(close_relative) = annotation[open..].find(']') else {
            break;
        };
        let close = open + close_relative;
        let entries = annotation[open + 1..close]
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>();
        let mut row_seen = false;
        for (index, entry) in entries.iter().enumerate() {
            let Some(row) = entry.strip_prefix("...") else {
                continue;
            };
            if row_seen {
                diagnostics.push(effect_row_diagnostic(
                    function,
                    "effect.row_multiple",
                    "effect set has more than one row tail".to_string(),
                    row,
                    type_effect_span(annotation, annotation_span, entry),
                    boundary,
                ));
            }
            if index + 1 != entries.len() {
                diagnostics.push(effect_row_diagnostic(
                    function,
                    "effect.row_non_final",
                    "effect row tail must be the final effect".to_string(),
                    row,
                    type_effect_span(annotation, annotation_span, entry),
                    boundary,
                ));
            }
            if function
                .effect_binder
                .as_ref()
                .is_none_or(|binder| binder.name != row)
            {
                diagnostics.push(effect_row_diagnostic(
                    function,
                    "effect.row_unbound",
                    format!("effect row variable `{row}` is not bound"),
                    row,
                    type_effect_span(annotation, annotation_span, entry),
                    boundary,
                ));
            }
            row_seen = true;
        }
        search_from = close + 1;
    }
    diagnostics
}

fn collect_unknown_type_effects(
    ty: &Type,
    annotation: &str,
    annotation_span: &SourceSpan,
    function: &Function,
    environment: &TypeEnvironment,
    boundary: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match ty {
        Type::Function {
            params,
            variadic,
            return_type,
            effects,
        } => {
            for effect in effects {
                if effect_row_name(effect).is_some() {
                    continue;
                }
                if KNOWN_EFFECT_LABELS.contains(&effect.as_str()) {
                    continue;
                }
                let segments = effect.split("::").map(str::to_string).collect::<Vec<_>>();
                match environment
                    .resolve_user_effect_path(&segments, function.module_name.as_deref())
                {
                    UserEffectPathResolution::Found(_) => {}
                    UserEffectPathResolution::QuarantinedImportTarget => {}
                    UserEffectPathResolution::PrivateCompanionTargetMismatch {
                        effect: signature,
                        access,
                    } => diagnostics.push(private_companion_effect_target_diagnostic(
                        function.node_id.display(function.kind.node_prefix()),
                        boundary,
                        effect,
                        signature,
                        access,
                        type_effect_span(annotation, annotation_span, effect)
                            .unwrap_or_else(|| annotation_span.clone()),
                    )),
                    UserEffectPathResolution::Missing => {
                        diagnostics.push(unknown_type_effect_diagnostic(
                            function,
                            effect,
                            annotation,
                            annotation_span,
                            boundary,
                        ))
                    }
                }
            }
            for param in params {
                collect_unknown_type_effects(
                    param,
                    annotation,
                    annotation_span,
                    function,
                    environment,
                    boundary,
                    diagnostics,
                );
            }
            if let Some(variadic) = variadic {
                collect_unknown_type_effects(
                    variadic,
                    annotation,
                    annotation_span,
                    function,
                    environment,
                    boundary,
                    diagnostics,
                );
            }
            collect_unknown_type_effects(
                return_type,
                annotation,
                annotation_span,
                function,
                environment,
                boundary,
                diagnostics,
            );
        }
        Type::Named { args, .. } => {
            for arg in args {
                collect_unknown_type_effects(
                    arg,
                    annotation,
                    annotation_span,
                    function,
                    environment,
                    boundary,
                    diagnostics,
                );
            }
        }
        Type::Record(fields) => {
            for (_, field) in fields {
                collect_unknown_type_effects(
                    field,
                    annotation,
                    annotation_span,
                    function,
                    environment,
                    boundary,
                    diagnostics,
                );
            }
        }
        Type::Unknown => {}
    }
}

fn unknown_type_effect_diagnostic(
    function: &Function,
    effect: &str,
    annotation: &str,
    annotation_span: &SourceSpan,
    boundary: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "effect.unknown",
        Severity::Error,
        DiagnosticKind::Effect,
        format!("function type effect `{effect}` is not known"),
        type_effect_span(annotation, annotation_span, effect),
        JsonValue::object([
            ("phase", JsonValue::string("effect")),
            (
                "node_id",
                JsonValue::string(function.node_id.display(function.kind.node_prefix())),
            ),
            ("effect", JsonValue::string(effect.to_string())),
            ("boundary", JsonValue::string(boundary)),
            (
                "known_effects",
                JsonValue::array(KNOWN_EFFECT_LABELS.iter().copied().map(JsonValue::string)),
            ),
        ]),
    )
}

fn type_effect_span(
    annotation: &str,
    annotation_span: &SourceSpan,
    effect: &str,
) -> Option<SourceSpan> {
    let offset = effect_offset_in_effect_clause(annotation, effect)?;
    let prefix_columns = annotation[..offset].chars().count();
    let effect_columns = effect.chars().count();
    Some(SourceSpan {
        file: annotation_span.file.clone(),
        start: veln_source::LineCol {
            line: annotation_span.start.line,
            column: annotation_span.start.column + prefix_columns,
            offset: annotation_span.start.offset + offset,
        },
        end: veln_source::LineCol {
            line: annotation_span.start.line,
            column: annotation_span.start.column + prefix_columns + effect_columns,
            offset: annotation_span.start.offset + offset + effect.len(),
        },
    })
}

fn effect_offset_in_effect_clause(annotation: &str, effect: &str) -> Option<usize> {
    let mut search_from = 0;
    while let Some(relative_effects) = annotation[search_from..].find("effects") {
        let effects_offset = search_from + relative_effects;
        let clause = &annotation[effects_offset..];
        let Some(open_relative) = clause.find('[') else {
            break;
        };
        let open = effects_offset + open_relative;
        let Some(close_relative) = annotation[open..].find(']') else {
            break;
        };
        let close = open + close_relative;
        if let Some(relative_effect) = annotation[open + 1..close].find(effect) {
            return Some(open + 1 + relative_effect);
        }
        search_from = close + 1;
    }
    None
}

fn declared_effect_boundary(function: &Function) -> &'static str {
    match function.kind {
        FunctionKind::Test => "test_declaration",
        FunctionKind::Function if function.visibility == Visibility::Public => "public_function",
        FunctionKind::Function => "private_function",
    }
}

fn empty_declared_effect_diagnostic(
    function: &Function,
    node_prefix: &'static str,
    boundary: &'static str,
) -> Diagnostic {
    let subject = match function.kind {
        FunctionKind::Test => "test declaration",
        FunctionKind::Function => "function declaration",
    };
    let mut diagnostic = Diagnostic::new(
        "effect.empty_declaration",
        Severity::Error,
        DiagnosticKind::Effect,
        format!("empty effects list is not allowed on a {subject}"),
        Some(function.span.clone()),
        crate::diagnostics::effect_details(function.node_id.display(node_prefix), boundary),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string("Remove the clause when the inferred effect set is empty."),
        ),
    ]));
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string(
                "Replace the empty list with non-empty effect labels when the body performs effects.",
            ),
        ),
    ]));
    diagnostic
}

fn unknown_declared_effect_diagnostic(
    function: &Function,
    effect: &str,
    effect_index: usize,
    node_prefix: &'static str,
    boundary: &'static str,
) -> Diagnostic {
    let declared_effects = function
        .effects
        .as_ref()
        .expect("unknown effect diagnostics require a declared effects clause");
    let mut diagnostic = Diagnostic::new(
        "effect.unknown",
        Severity::Error,
        DiagnosticKind::Effect,
        format!("declared effect `{effect}` is not known"),
        function
            .effect_spans
            .as_ref()
            .and_then(|spans| spans.get(effect_index))
            .cloned()
            .or_else(|| Some(function.span.clone())),
        unknown_declared_effect_details(function, effect, node_prefix, boundary, declared_effects),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string("Use a known effect label or remove the declaration."),
        ),
    ]));
    diagnostic
}

fn unknown_declared_effect_details(
    function: &Function,
    effect: &str,
    node_prefix: &'static str,
    boundary: &'static str,
    declared_effects: &[String],
) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("effect")),
        (
            "node_id",
            JsonValue::string(function.node_id.display(node_prefix)),
        ),
        ("effect", JsonValue::string(effect.to_string())),
        ("boundary", JsonValue::string(boundary)),
        (
            "declared_effects",
            JsonValue::array(declared_effects.iter().cloned().map(JsonValue::string)),
        ),
        (
            "known_effects",
            JsonValue::array(KNOWN_EFFECT_LABELS.iter().copied().map(JsonValue::string)),
        ),
    ])
}
