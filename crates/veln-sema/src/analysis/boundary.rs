use super::*;
use crate::adt::AdtRegistry;
use crate::schema::dispatch::{
    SchemaDispatchCase, SchemaDispatchCasePayload, SchemaDispatchSpec,
    closed_dispatch_schema_primitive, extension_dispatch_schema_primitive,
    lowercase_schema_primitive_nested_payloads,
    schema_dispatch_payload_accepts_lowercase_primitive,
};
use crate::schema::primitives::{
    ByteViewLengthExpr, LowercaseSchemaPrimitive, LowercaseSchemaPrimitiveError,
    SchemaRepeatPayload, SchemaRepeatSpec, byte_view_multiple_constraint,
    byte_view_schema_primitive, exact_width_schema_primitive,
    exact_width_schema_primitive_bit_width, lowercase_reserved_bits_schema_primitive,
    lowercase_schema_primitive, repeat_schema_primitive, reserved_bits_schema_primitive,
    schema_field_reference_type, schema_length_expression_references,
    schema_payload_name_last_segment, schema_payload_name_path,
    schema_repeat_payload_accepts_lowercase_primitive,
};
use crate::schema::reserved_layout::{
    schema_field_uses_generalized_reserved_byte_prefix,
    schema_payload_has_generalized_reserved_byte_prefix, supported_encode_reserved_bits,
};
use crate::standard_names::PRELUDE_MODULE;
use crate::types::schema_types::{
    binary_schema_anonymous_record_decode_type,
    format_neutral_schema_encode_field_is_source_adt_candidate,
    format_neutral_schema_encode_field_type_for_schema,
    format_neutral_schema_field_type_for_schema,
    recursive_dispatch_decode_only_payload_case_is_eligible,
    recursive_dispatch_payload_case_is_eligible, recursive_dispatch_payload_is_eligible,
    schema_decode_step_function_name, schema_decode_value_type, schema_dispatch_payload_schema,
    schema_encode_function_name, schema_encode_value_type, schema_field_target,
    schema_field_uses_existing_grammar, schema_has_eligible_recursive_dispatch_payload,
    schema_has_recursive_dispatch_payload, schema_recursive_dispatch_helper_payload_type,
    schema_recursive_dispatch_payload_type,
};
use crate::types::signatures::UserEffectPathResolution;
use std::collections::{BTreeMap, BTreeSet};
use veln_ast::{PublicAliasKind, SchemaDecl, SchemaField, SchemaValidationClause, UseDecl};
use veln_literals::parse_integer_literal;
use veln_project::classify_companion_source;

pub(crate) fn check_public_function_boundary(function: &Function) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for param in &function.params {
        if param.ty.is_none() {
            diagnostics.push(Diagnostic::new(
                "type.public_signature_missing",
                Severity::Error,
                DiagnosticKind::Type,
                format!("public parameter `{}` has no type annotation", param.name),
                Some(param.span.clone()),
                type_details(
                    param.node_id.display("param"),
                    "explicit",
                    "missing",
                    "declared_parameter",
                    "source",
                    "assignable",
                    [function.node_id.display("fn")],
                ),
            ));
        }
    }

    if function.return_type.is_none() {
        diagnostics.push(Diagnostic::new(
            "type.public_signature_missing",
            Severity::Error,
            DiagnosticKind::Type,
            "public function has no return type annotation",
            Some(function.span.clone()),
            type_details(
                function.node_id.display("fn"),
                "explicit",
                "missing",
                "declared_return",
                "source",
                "return_value",
                [function.node_id.display("fn")],
            ),
        ));
    }

    diagnostics
}

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

    diagnostics.extend(
        declared_effects
            .iter()
            .enumerate()
            .filter(|(_, effect)| {
                if effect_row_name(effect).is_some() {
                    return false;
                }
                let segments = effect.split("::").map(str::to_string).collect::<Vec<_>>();
                !KNOWN_EFFECT_LABELS.contains(&effect.as_str())
                    && matches!(
                        environment
                            .resolve_user_effect_path(&segments, function.module_name.as_deref()),
                        UserEffectPathResolution::Missing
                            | UserEffectPathResolution::PrivateCompanionTargetMismatch { .. }
                    )
            })
            .map(|(index, effect)| {
                let segments = effect.split("::").map(str::to_string).collect::<Vec<_>>();
                match environment
                    .resolve_user_effect_path(&segments, function.module_name.as_deref())
                {
                    UserEffectPathResolution::PrivateCompanionTargetMismatch {
                        effect: signature,
                        access,
                    } => private_companion_effect_target_diagnostic(
                        function.node_id.display(node_prefix),
                        boundary,
                        effect,
                        signature,
                        access,
                        function
                            .effect_spans
                            .as_ref()
                            .and_then(|spans| spans.get(index))
                            .cloned()
                            .unwrap_or_else(|| function.span.clone()),
                    ),
                    UserEffectPathResolution::Found(_) | UserEffectPathResolution::Missing => {
                        unknown_declared_effect_diagnostic(
                            function,
                            effect,
                            index,
                            node_prefix,
                            boundary,
                        )
                    }
                }
            }),
    );
    diagnostics
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
        effect_details(function.node_id.display(node_prefix), boundary),
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
        ]),
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

pub(crate) fn check_duplicate_function_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<(Option<String>, String), (String, SourceSpan)>::new();

    for function in &module.functions {
        let Some(name) = &function.name else {
            continue;
        };
        let key = (function.module_name.clone(), name.clone());
        let node_id = function.node_id.display(function.kind.node_prefix());
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "function",
                "function declaration",
                node_id,
                function.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, function.span.clone()));
        }
    }
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Function)
    {
        let Some(name) = &alias.name else {
            continue;
        };
        let key = (alias.module_name.clone(), name.clone());
        let node_id = alias.node_id.display("alias");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "function",
                "function alias",
                node_id,
                alias.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, alias.span.clone()));
        }
    }

    diagnostics
}

pub(crate) fn check_duplicate_type_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<(Option<String>, String), (String, SourceSpan)>::new();

    for type_decl in &module.types {
        let Some(name) = &type_decl.name else {
            continue;
        };
        let key = (type_decl.module_name.clone(), name.clone());
        let node_id = type_decl.node_id.display("type");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "type",
                "type declaration",
                node_id,
                type_decl.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, type_decl.span.clone()));
        }
    }
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Type)
    {
        let Some(name) = &alias.name else {
            continue;
        };
        let key = (alias.module_name.clone(), name.clone());
        let node_id = alias.node_id.display("alias");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "type",
                "type alias",
                node_id,
                alias.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, alias.span.clone()));
        }
    }

    diagnostics
}

pub(crate) fn check_duplicate_effect_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<(Option<String>, String), (String, SourceSpan)>::new();

    for effect in &module.effects {
        let Some(name) = &effect.name else {
            continue;
        };
        let key = (effect.module_name.clone(), name.clone());
        let node_id = effect.node_id.display("effect");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "effect",
                "effect declaration",
                node_id,
                effect.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, effect.span.clone()));
        }

        let mut operations = BTreeMap::<String, (String, SourceSpan)>::new();
        for operation in &effect.operations {
            let Some(operation_name) = &operation.name else {
                continue;
            };
            let operation_node_id = operation.node_id.display("operation");
            if let Some((first_node_id, first_span)) = operations.get(operation_name) {
                diagnostics.push(duplicate_name_diagnostic(
                    operation_name,
                    "operation",
                    "effect operation declaration",
                    operation_node_id,
                    operation.name_span.clone(),
                    first_node_id.clone(),
                    first_span,
                ));
            } else {
                operations.insert(
                    operation_name.clone(),
                    (operation_node_id, operation.name_span.clone()),
                );
            }
        }
    }

    diagnostics
}

pub(crate) fn check_duplicate_schema_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<(Option<String>, String), (String, SourceSpan)>::new();

    for schema in &module.schemas {
        let Some(name) = &schema.name else {
            continue;
        };
        let key = (schema.module_name.clone(), name.clone());
        let node_id = schema.node_id.display("schema");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "schema",
                "schema declaration",
                node_id,
                schema.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, schema.span.clone()));
        }
    }
    for alias in module
        .aliases
        .iter()
        .filter(|alias| alias.kind == PublicAliasKind::Schema)
    {
        let Some(name) = &alias.name else {
            continue;
        };
        let key = (alias.module_name.clone(), name.clone());
        let node_id = alias.node_id.display("alias");
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                name,
                "schema",
                "schema alias",
                node_id,
                alias.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, alias.span.clone()));
        }
    }

    diagnostics
}

#[derive(Clone, Copy, Debug)]
enum SchemaAliasCheckResolution {
    Resolved,
    Private,
    WrongKind(&'static str),
    Cyclic,
    Unresolved,
}

fn codec_schema_wrong_kind(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
) -> Option<&'static str> {
    if module.functions.iter().any(|function| {
        function.kind == FunctionKind::Function
            && function.name.as_deref() == Some(name)
            && function.module_name.as_deref() == module_name
    }) {
        return Some("function");
    }
    if module.types.iter().any(|type_decl| {
        type_decl.name.as_deref() == Some(name) && type_decl.module_name.as_deref() == module_name
    }) {
        return Some("type");
    }
    if module.codecs.iter().any(|codec| {
        codec.name.as_deref() == Some(name) && codec.module_name.as_deref() == module_name
    }) {
        return Some("codec");
    }
    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.name.as_deref() == Some(name) && alias.module_name.as_deref() == module_name
    }) {
        return match alias.kind {
            PublicAliasKind::Function => Some("function"),
            PublicAliasKind::Type => Some("type"),
            PublicAliasKind::Schema => None,
        };
    }
    None
}

pub(crate) fn check_public_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut schema_alias_cache = BTreeMap::new();
    for alias in &module.aliases {
        if alias.name.is_none() {
            continue;
        }
        match alias.kind {
            PublicAliasKind::Function => {
                if function_target(module, &alias.target, alias.module_name.as_deref()).is_none()
                    && type_target(module, &alias.target, alias.module_name.as_deref()).is_some()
                {
                    diagnostics.push(alias_kind_mismatch_diagnostic(alias, "function", "type"));
                } else if function_target(module, &alias.target, alias.module_name.as_deref())
                    .is_none()
                {
                    diagnostics.push(unresolved_alias_diagnostic(alias, "function"));
                }
            }
            PublicAliasKind::Type => {
                if type_target(module, &alias.target, alias.module_name.as_deref()).is_none()
                    && function_target(module, &alias.target, alias.module_name.as_deref())
                        .is_some()
                {
                    diagnostics.push(alias_kind_mismatch_diagnostic(alias, "type", "function"));
                } else if type_target(module, &alias.target, alias.module_name.as_deref()).is_none()
                {
                    diagnostics.push(unresolved_alias_diagnostic(alias, "type"));
                }
            }
            PublicAliasKind::Schema => {
                match resolve_schema_alias_check_reference(
                    module,
                    &alias.target,
                    alias.module_name.as_deref(),
                    false,
                    &mut Vec::new(),
                    &mut schema_alias_cache,
                ) {
                    SchemaAliasCheckResolution::Resolved => {}
                    SchemaAliasCheckResolution::Private => {
                        diagnostics.push(private_alias_diagnostic(alias));
                    }
                    SchemaAliasCheckResolution::WrongKind(actual_kind) => {
                        diagnostics.push(alias_kind_mismatch_diagnostic(
                            alias,
                            "schema",
                            actual_kind,
                        ));
                    }
                    SchemaAliasCheckResolution::Cyclic => {
                        diagnostics.push(unresolved_alias_diagnostic(alias, "schema"));
                    }
                    SchemaAliasCheckResolution::Unresolved => {
                        diagnostics.push(unresolved_alias_diagnostic(alias, "schema"));
                    }
                }
            }
        }
    }
    diagnostics
}

fn resolve_schema_alias_check_reference(
    module: &SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
    allow_private_local_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
    cache: &mut BTreeMap<(Option<String>, String), SchemaAliasCheckResolution>,
) -> SchemaAliasCheckResolution {
    match segments {
        [name] => resolve_schema_alias_check_in_module(
            module,
            current_module,
            name,
            allow_private_local_schema,
            visited_aliases,
            cache,
        ),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            ) else {
                return SchemaAliasCheckResolution::Unresolved;
            };
            resolve_schema_alias_check_in_module(
                module,
                Some(&use_decl.name),
                name,
                false,
                visited_aliases,
                cache,
            )
        }
        _ => SchemaAliasCheckResolution::Unresolved,
    }
}

fn resolve_schema_alias_check_in_module(
    module: &SurfaceModule,
    module_name: Option<&str>,
    name: &str,
    allow_private_schema: bool,
    visited_aliases: &mut Vec<(Option<String>, String)>,
    cache: &mut BTreeMap<(Option<String>, String), SchemaAliasCheckResolution>,
) -> SchemaAliasCheckResolution {
    if let Some(schema) = module.schemas.iter().find(|schema| {
        schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == module_name
    }) {
        return if allow_private_schema || schema.visibility == Visibility::Public {
            SchemaAliasCheckResolution::Resolved
        } else {
            SchemaAliasCheckResolution::Private
        };
    }

    if let Some(alias) = module.aliases.iter().find(|alias| {
        alias.kind == PublicAliasKind::Schema
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    }) {
        return resolve_schema_alias_check_target(module, alias, visited_aliases, cache);
    }

    codec_schema_wrong_kind(module, module_name, name).map_or(
        SchemaAliasCheckResolution::Unresolved,
        SchemaAliasCheckResolution::WrongKind,
    )
}

fn resolve_schema_alias_check_target(
    module: &SurfaceModule,
    alias: &veln_ast::PublicAlias,
    visited_aliases: &mut Vec<(Option<String>, String)>,
    cache: &mut BTreeMap<(Option<String>, String), SchemaAliasCheckResolution>,
) -> SchemaAliasCheckResolution {
    let Some(name) = &alias.name else {
        return SchemaAliasCheckResolution::Unresolved;
    };
    let key = (alias.module_name.clone(), name.clone());
    if let Some(resolution) = cache.get(&key) {
        return *resolution;
    }
    if visited_aliases.contains(&key) {
        return SchemaAliasCheckResolution::Cyclic;
    }

    visited_aliases.push(key.clone());
    let resolution = resolve_schema_alias_check_reference(
        module,
        &alias.target,
        alias.module_name.as_deref(),
        false,
        visited_aliases,
        cache,
    );
    visited_aliases.pop();
    cache.insert(key, resolution);
    resolution
}

pub(crate) fn check_schema_type_references(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for function in &module.functions {
        let current_module = function.module_name.as_deref();
        for param in &function.params {
            if let Some(annotation) = &param.ty {
                push_schema_type_reference_diagnostics(
                    module,
                    current_module,
                    annotation,
                    param.node_id.display("param"),
                    param.span.clone(),
                    "parameter_type",
                    &mut diagnostics,
                );
            }
        }
        if let Some(return_type) = &function.return_type {
            push_schema_type_reference_diagnostics(
                module,
                current_module,
                return_type,
                function.node_id.display(function.kind.node_prefix()),
                function.span.clone(),
                "return_type",
                &mut diagnostics,
            );
        }
        for line in &function.body {
            let BodyLineKind::Let {
                annotation: Some(annotation),
                ..
            } = &line.kind
            else {
                continue;
            };
            push_schema_type_reference_diagnostics(
                module,
                current_module,
                annotation,
                line.node_id.display("let"),
                line.span.clone(),
                "local_annotation",
                &mut diagnostics,
            );
        }
    }

    for type_decl in &module.types {
        let current_module = type_decl.module_name.as_deref();
        for variant in &type_decl.variants {
            for field in &variant.fields {
                push_schema_type_reference_diagnostics(
                    module,
                    current_module,
                    &field.ty,
                    field.node_id.display("field"),
                    field.span.clone(),
                    "type_variant_field",
                    &mut diagnostics,
                );
            }
        }
    }

    diagnostics
}

pub(crate) fn check_schema_field_primitives(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for schema in &module.schemas {
        let format_name = schema.format.as_ref().map(|format| format.name.as_str());
        let mut decoded_fields = BTreeMap::<String, Type>::new();
        let mut field_bindings = BTreeSet::new();
        let adts = AdtRegistry::from_module(module);
        for field in &schema.fields {
            if !field_bindings.insert(field.name.clone()) {
                diagnostics.push(schema_composition_duplicate_binding_diagnostic(
                    schema, field,
                ));
                continue;
            }
            if check_schema_field_composition(module, schema, field, &mut diagnostics) {
                continue;
            }
            if check_direct_schema_primitive(
                schema,
                field,
                format_name,
                &mut decoded_fields,
                &mut diagnostics,
            ) {
                continue;
            }
            if check_nested_lowercase_schema_primitives(
                schema,
                field,
                format_name,
                &mut diagnostics,
            ) {
                continue;
            }
            if format_name == Some("binary")
                && check_binary_schema_field(
                    module,
                    schema,
                    field,
                    &mut decoded_fields,
                    &mut diagnostics,
                )
            {
                continue;
            }
            if format_name.is_none() {
                check_format_neutral_schema_field(
                    module,
                    schema,
                    field,
                    &adts,
                    &mut decoded_fields,
                    &mut diagnostics,
                );
                continue;
            }
            check_schema_non_byte_view_multiple(schema, field, &mut diagnostics);
            if schema_payload_name_path(&field.ty).is_some() {
                diagnostics.push(schema_composition_reference_diagnostic(
                    module,
                    schema,
                    field,
                    unresolved_schema_composition_reason(module, schema, &field.ty),
                ));
            }
        }
        for (index, validation) in schema.validations.iter().enumerate() {
            if index > 0 {
                diagnostics.push(schema_validation_duplicate_diagnostic(schema, validation));
                continue;
            }
            check_schema_validation_clause(schema, validation, &decoded_fields, &mut diagnostics);
        }
    }

    diagnostics
}

fn check_schema_field_composition(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if let Some(reason) = schema_composition_reference_blocker(module, schema, field) {
        diagnostics.push(schema_composition_reference_diagnostic(
            module, schema, field, reason,
        ));
        return true;
    }
    if schema_field_uses_existing_grammar_at_boundary(schema, &field.ty) {
        return false;
    }
    let Some(target) = schema_field_target(module, schema, &field.ty) else {
        return false;
    };

    let decode_eligible = schema_decode_value_type(module, target).is_some();
    let encode_eligible = schema_encode_value_type(module, target).is_some();
    if !decode_eligible {
        diagnostics.push(schema_composition_reference_diagnostic(
            module,
            schema,
            field,
            "decode_ineligible_target",
        ));
    }
    if !encode_eligible {
        diagnostics.push(schema_composition_reference_diagnostic(
            module,
            schema,
            field,
            "encode_ineligible_target",
        ));
    }
    !decode_eligible
}

fn check_direct_schema_primitive(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if let Some(reserved) = lowercase_reserved_bits_schema_primitive(&field.ty) {
        check_lowercase_reserved_bits(schema, field, format_name, reserved, diagnostics);
        return true;
    }
    if let Some(primitive) = lowercase_schema_primitive(&field.ty) {
        check_lowercase_integer_primitive(
            schema,
            field,
            format_name,
            primitive,
            decoded_fields,
            diagnostics,
        );
        return true;
    }
    if let Some(primitive) = exact_width_binary_primitive_name(&field.ty) {
        check_exact_width_primitive(
            schema,
            field,
            format_name,
            primitive,
            decoded_fields,
            diagnostics,
        );
        return true;
    }
    if let Some(primitive) = reserved_bits_primitive(&field.ty) {
        check_reserved_bits(schema, field, format_name, primitive, diagnostics);
        return true;
    }
    false
}

fn check_lowercase_reserved_bits(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    reserved: Result<(i64, i64), LowercaseSchemaPrimitiveError>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match (format_name, reserved) {
        (Some("binary"), Ok(reserved)) => {
            check_schema_non_byte_view_multiple(schema, field, diagnostics);
            check_reserved_bits_encode_shape(schema, field, reserved, diagnostics);
        }
        (Some("binary"), Err(reason)) | (_, Err(reason)) => {
            diagnostics.push(lowercase_schema_primitive_diagnostic(
                &field.ty,
                Some(schema),
                Some(field),
                field.node_id.display("schema-field"),
                field.span.clone(),
                reason,
            ));
        }
        (_, Ok(_)) => diagnostics.push(lowercase_schema_primitive_position_diagnostic(
            &field.ty,
            Some(schema),
            Some(field),
            field.node_id.display("schema-field"),
            field.span.clone(),
            "non_binary_format",
        )),
    }
}

fn check_lowercase_integer_primitive(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    primitive: Result<LowercaseSchemaPrimitive, LowercaseSchemaPrimitiveError>,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match (format_name, primitive) {
        (Some("binary"), Ok(_)) => {
            check_schema_non_byte_view_multiple(schema, field, diagnostics);
            record_decoded_schema_field(schema, field, Type::int(), decoded_fields, diagnostics);
        }
        (Some("binary"), Err(reason)) | (_, Err(reason)) => {
            diagnostics.push(lowercase_schema_primitive_diagnostic(
                &field.ty,
                Some(schema),
                Some(field),
                field.node_id.display("schema-field"),
                field.span.clone(),
                reason,
            ));
        }
        (_, Ok(_)) => diagnostics.push(lowercase_schema_primitive_position_diagnostic(
            &field.ty,
            Some(schema),
            Some(field),
            field.node_id.display("schema-field"),
            field.span.clone(),
            "non_binary_format",
        )),
    }
}

fn check_exact_width_primitive(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    primitive: &str,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if format_name != Some("binary") {
        diagnostics.push(exact_width_schema_primitive_diagnostic(
            primitive,
            Some(schema),
            Some(field),
            field.node_id.display("schema-field"),
            field.span.clone(),
            "non_binary_format",
        ));
        return;
    }
    check_schema_non_byte_view_multiple(schema, field, diagnostics);
    record_decoded_schema_field(schema, field, Type::int(), decoded_fields, diagnostics);
}

fn check_reserved_bits(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    primitive: Result<(i64, i64), ReservedBitsArgumentReason>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if format_name != Some("binary") {
        diagnostics.push(reserved_bits_format_diagnostic(schema, field));
        return;
    }
    match primitive {
        Err(reason) => diagnostics.push(reserved_bits_argument_diagnostic(schema, field, reason)),
        Ok(reserved) => {
            check_schema_non_byte_view_multiple(schema, field, diagnostics);
            check_reserved_bits_encode_shape(schema, field, reserved, diagnostics);
        }
    }
}

fn check_reserved_bits_encode_shape(
    schema: &SchemaDecl,
    field: &SchemaField,
    reserved: (i64, i64),
    diagnostics: &mut Vec<Diagnostic>,
) {
    let field_index = schema
        .fields
        .iter()
        .position(|schema_field| schema_field.node_id == field.node_id);
    if field_index
        .and_then(|index| supported_encode_reserved_bits(&schema.fields, index, reserved))
        .is_none()
    {
        diagnostics.push(reserved_bits_encode_shape_diagnostic(
            schema,
            field,
            field_index,
            reserved,
        ));
    }
}

fn check_nested_lowercase_schema_primitives(
    schema: &SchemaDecl,
    field: &SchemaField,
    format_name: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut pushed_diagnostic = false;
    for (primitive, reason) in lowercase_schema_primitive_nested_payloads(&field.ty) {
        let supported_dispatch = reason == "dispatch_payload"
            && schema_dispatch_payload_accepts_lowercase_primitive(primitive);
        let supported_repeat = reason == "repeat_payload"
            && schema_repeat_payload_accepts_lowercase_primitive(primitive);
        if format_name == Some("binary") && (supported_dispatch || supported_repeat) {
            continue;
        }
        let reason = if format_name == Some("binary") {
            reason
        } else {
            "non_binary_format"
        };
        diagnostics.push(lowercase_schema_primitive_position_diagnostic(
            primitive,
            Some(schema),
            Some(field),
            field.node_id.display("schema-field"),
            field.span.clone(),
            reason,
        ));
        pushed_diagnostic = true;
    }
    pushed_diagnostic
}

fn check_binary_schema_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
        if check_schema_byte_view_reference(
            schema,
            field,
            &length_expr,
            decoded_fields,
            diagnostics,
        ) && check_schema_byte_view_multiple(schema, field, decoded_fields, diagnostics)
        {
            decoded_fields.insert(field.name.clone(), Type::named("ByteView", Vec::new()));
        }
        return true;
    }
    if let Some(repeat) = repeat_schema_primitive(&field.ty) {
        check_schema_non_byte_view_multiple(schema, field, diagnostics);
        if let Some(field_ty) =
            check_schema_repeat_field(module, schema, field, &repeat, decoded_fields, diagnostics)
        {
            record_decoded_schema_field(schema, field, field_ty, decoded_fields, diagnostics);
        }
        return true;
    }
    if let Some(field_ty) = binary_composed_schema_field_type(module, schema, field) {
        check_schema_non_byte_view_multiple(schema, field, diagnostics);
        record_decoded_schema_field(schema, field, field_ty, decoded_fields, diagnostics);
        return true;
    }
    if let Some(field_ty) = binary_schema_anonymous_record_decode_type(&field.ty) {
        check_schema_non_byte_view_multiple(schema, field, diagnostics);
        record_decoded_schema_field(schema, field, field_ty, decoded_fields, diagnostics);
        return true;
    }
    let Some(dispatch) = closed_dispatch_schema_primitive(&field.ty)
        .or_else(|| extension_dispatch_schema_primitive(&field.ty))
    else {
        return false;
    };
    check_schema_non_byte_view_multiple(schema, field, diagnostics);
    if let Some(field_ty) = check_schema_dispatch_field(
        module,
        schema,
        field,
        &dispatch,
        decoded_fields,
        diagnostics,
    ) {
        record_decoded_schema_field(schema, field, field_ty, decoded_fields, diagnostics);
    }
    true
}

fn binary_composed_schema_field_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
) -> Option<Type> {
    let payload_schema = schema_field_target(module, schema, &field.ty)?;
    (payload_schema
        .format
        .as_ref()
        .map(|format| format.name.as_str())
        == Some("binary"))
    .then(|| schema_decode_value_type(module, payload_schema))
    .flatten()
}

fn check_format_neutral_schema_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    adts: &AdtRegistry,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let decode_field_type =
        format_neutral_schema_field_type_for_schema(module, schema, adts, &field.ty);
    if let Some(field_ty) = decode_field_type.clone() {
        record_decoded_schema_field(schema, field, field_ty, decoded_fields, diagnostics);
    } else if schema_payload_name_path(&field.ty).is_some()
        && !schema_field_has_ordinary_type_target(module, schema, &field.ty)
    {
        diagnostics.push(schema_composition_reference_diagnostic(
            module,
            schema,
            field,
            unresolved_schema_composition_reason(module, schema, &field.ty),
        ));
    } else {
        diagnostics.push(format_neutral_schema_helper_diagnostic(schema, field));
    }

    let encode_unsupported =
        format_neutral_schema_encode_field_type_for_schema(module, schema, adts, &field.ty)
            .is_none();
    let direct_source_adt_candidate =
        format_neutral_schema_encode_field_is_source_adt_candidate(&field.ty);
    let ordinary_or_non_path = schema_payload_name_path(&field.ty).is_none()
        || schema_field_has_ordinary_type_target(module, schema, &field.ty);
    if encode_unsupported
        && direct_source_adt_candidate
        && decode_field_type.is_none()
        && ordinary_or_non_path
    {
        diagnostics.push(format_neutral_schema_encode_helper_diagnostic(
            schema.name.as_deref().unwrap_or("<missing>"),
            &schema.span,
            field,
        ));
    }
}

fn schema_composition_duplicate_binding_diagnostic(
    schema: &SchemaDecl,
    field: &SchemaField,
) -> Diagnostic {
    Diagnostic::new(
        "schema.composition_duplicate_binding",
        Severity::Error,
        DiagnosticKind::Name,
        format!("duplicate schema field binding `{}`", field.name),
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            (
                "schema",
                JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
            ),
            ("binding", JsonValue::string(field.name.clone())),
            ("reason", JsonValue::string("duplicate_binding")),
        ]),
    )
}

fn unresolved_schema_composition_reason(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    text: &str,
) -> &'static str {
    let Some(path) = schema_payload_name_path(text) else {
        return "missing_schema";
    };
    match resolve_schema_alias_check_reference(
        module,
        &path,
        schema.module_name.as_deref(),
        true,
        &mut Vec::new(),
        &mut BTreeMap::new(),
    ) {
        SchemaAliasCheckResolution::Private => "private_schema",
        SchemaAliasCheckResolution::WrongKind(_) => "wrong_kind",
        SchemaAliasCheckResolution::Cyclic => "cyclic_composition",
        SchemaAliasCheckResolution::Resolved | SchemaAliasCheckResolution::Unresolved => {
            "missing_schema"
        }
    }
}

fn schema_composition_reference_blocker(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
) -> Option<&'static str> {
    if schema_field_uses_existing_grammar_at_boundary(schema, &field.ty) {
        return None;
    }
    let target = schema_field_target(module, schema, &field.ty)?;
    if schema_field_has_ordinary_type_target(module, schema, &field.ty) {
        return Some("ambiguous_type_and_schema");
    }
    let containing_format = schema.format.as_ref().map(|format| format.name.as_str());
    let target_format = target.format.as_ref().map(|format| format.name.as_str());
    if containing_format != target_format {
        return Some("format_incompatible");
    }
    schema_composition_reaches(module, target, schema, &mut Vec::new())
        .then_some("cyclic_composition")
}

fn schema_field_uses_existing_grammar_at_boundary(schema: &SchemaDecl, text: &str) -> bool {
    schema_field_uses_existing_grammar(schema, text)
        || (schema.format.as_ref().map(|format| format.name.as_str()) == Some("binary")
            && (exact_width_binary_primitive_name(text).is_some()
                || reserved_bits_primitive(text).is_some()))
}

fn schema_field_has_ordinary_type_target(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    text: &str,
) -> bool {
    let Some(path) = schema_payload_name_path(text) else {
        return false;
    };
    let (module_name, name, imported) = match path.as_slice() {
        [name] => (schema.module_name.as_deref(), name.as_str(), false),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &path[..path.len() - 1],
                schema.module_name.as_deref(),
            ) else {
                return false;
            };
            (Some(use_decl.name.as_str()), name.as_str(), true)
        }
        _ => return false,
    };
    module.types.iter().any(|ty| {
        ty.name.as_deref() == Some(name)
            && ty.module_name.as_deref() == module_name
            && (!imported || ty.visibility == Visibility::Public)
    }) || module.aliases.iter().any(|alias| {
        alias.kind == PublicAliasKind::Type
            && alias.name.as_deref() == Some(name)
            && alias.module_name.as_deref() == module_name
    })
}

fn schema_composition_reaches(
    module: &SurfaceModule,
    current: &SchemaDecl,
    target: &SchemaDecl,
    visited: &mut Vec<NodeId>,
) -> bool {
    if current.node_id == target.node_id {
        return true;
    }
    if visited.contains(&current.node_id) {
        return false;
    }
    visited.push(current.node_id);
    let reaches = current.fields.iter().any(|field| {
        schema_field_target(module, current, &field.ty)
            .is_some_and(|next| schema_composition_reaches(module, next, target, visited))
    });
    visited.pop();
    reaches
}

fn schema_composition_reference_diagnostic(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    reason: &'static str,
) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    let fact = match reason {
        "ambiguous_type_and_schema" => "resolves as both an ordinary type and a schema",
        "format_incompatible" => "does not use the containing schema's format",
        "cyclic_composition" => "creates a schema composition cycle",
        "private_schema" => "resolves to a private imported schema",
        "wrong_kind" => "resolves in a non-schema namespace",
        "decode_ineligible_target" => "does not expose an eligible decode boundary",
        "encode_ineligible_target" => "does not expose an eligible encode boundary",
        _ => "does not resolve to a visible schema or supported binary field family",
    };
    let companion_target = if reason == "private_schema" {
        companion_schema_access_target(module, schema)
    } else {
        None
    };
    let mut details = vec![
        ("phase", JsonValue::string("schema")),
        (
            "node_id",
            JsonValue::string(field.node_id.display("schema-field")),
        ),
        ("schema", JsonValue::string(schema_name)),
        ("binding", JsonValue::string(field.name.clone())),
        ("target", JsonValue::string(field.ty.clone())),
        ("reason", JsonValue::string(reason)),
    ];
    if let Some(target_module) = companion_target.as_deref() {
        if let Some(current_module) = schema.module_name.as_deref() {
            details.push(("companion_module", JsonValue::string(current_module)));
        }
        details.push(("companion_target_module", JsonValue::string(target_module)));
    }
    let mut diagnostic = Diagnostic::new(
        "schema.composition_reference",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "schema field `{}` cannot compose `{}` because it {fact}",
            field.name, field.ty
        ),
        Some(field.span.clone()),
        JsonValue::object(details),
    );
    if let Some(target_module) = companion_target {
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("companion_target")),
            (
                "message",
                JsonValue::string(format!(
                    "This test companion may access private schemas only from target module `{target_module}`."
                )),
            ),
            ("target_module", JsonValue::string(target_module)),
        ]));
    }
    diagnostic
}

fn format_neutral_schema_helper_diagnostic(schema: &SchemaDecl, field: &SchemaField) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    let supported = "recursive format-neutral visible shape made from scalar leaves, anonymous record fields, Option<T>, List<T>, Vec<T>, Dict<String, T>, Result<recursive visible shape, recursive visible shape>, or same-module or public imported source ADTs whose constructor payloads are recursive visible shapes";
    let boundary_message = format!(
        "Generated format-neutral decode helpers for schema `{schema_name}` accept recursive visible shapes made from scalar leaves, anonymous record fields, Option<T>, List<T>, Vec<T>, Dict<String, T>, Result<Ok, Err> when both payloads are recursive visible shapes, and same-module or public imported source ADTs whose constructor payloads are recursive visible shapes."
    );
    let mut diagnostic = Diagnostic::new(
        "schema.format_neutral_decode_helper",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "format-neutral schema field `{}` cannot expose a generated decode helper because `{}` is not a {supported}",
            field.name, field.ty,
        ),
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            ("schema", JsonValue::string(schema_name)),
            ("field", JsonValue::string(field.name.clone())),
            ("field_type", JsonValue::string(field.ty.clone())),
            (
                "reason",
                JsonValue::string("unsupported_format_neutral_field_type"),
            ),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("schema_helper_boundary")),
        ("span", span_json(&schema.span)),
        ("message", JsonValue::string(boundary_message)),
    ]));
    diagnostic
}

pub(in crate::analysis) fn format_neutral_schema_encode_helper_diagnostic(
    schema_name: &str,
    schema_span: &SourceSpan,
    field: &SchemaField,
) -> Diagnostic {
    let supported = "recursive format-neutral visible shape";
    let boundary_message = format!(
        "Generated format-neutral encode helpers for schema `{schema_name}` accept recursive visible shapes made from Int, Bool, Float, and String leaves, anonymous records, Option<T>, List<T>, Vec<T>, Dict<String, T>, Result<Ok, Err>, and eligible same-module or public imported source ADTs when every recursively visited child or constructor payload is also eligible."
    );
    let mut diagnostic = Diagnostic::new(
        "schema.format_neutral_encode_helper",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "format-neutral schema field `{}` cannot expose a generated encode helper because `{}` is not a {supported}",
            field.name, field.ty,
        ),
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            ("schema", JsonValue::string(schema_name)),
            ("field", JsonValue::string(field.name.clone())),
            ("field_type", JsonValue::string(field.ty.clone())),
            (
                "reason",
                JsonValue::string("unsupported_format_neutral_encode_field_type"),
            ),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("schema_helper_boundary")),
        ("span", span_json(schema_span)),
        ("message", JsonValue::string(boundary_message)),
    ]));
    diagnostic
}

fn check_schema_validation_clause(
    schema: &SchemaDecl,
    validation: &SchemaValidationClause,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for reference in schema_validation_references(&validation.predicate) {
        let Some(ty) = schema_field_reference_type(decoded_fields, &reference) else {
            diagnostics.push(schema_validation_reference_diagnostic(
                schema,
                validation,
                &reference,
                "unknown_field_reference",
                format!("schema validation reference `{reference}` is not a decoded schema field"),
                [],
            ));
            continue;
        };
        if ty != &Type::int() {
            diagnostics.push(schema_validation_reference_diagnostic(
                schema,
                validation,
                &reference,
                "incompatible_field_reference",
                format!(
                    "schema validation reference `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            ));
        }
    }
}

fn record_decoded_schema_field(
    schema: &SchemaDecl,
    field: &SchemaField,
    field_ty: Type,
    decoded_fields: &mut BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    check_schema_field_predicate_references(schema, field, &field_ty, decoded_fields, diagnostics);
    decoded_fields.insert(field.name.clone(), field_ty);
}

fn check_schema_field_predicate_references(
    schema: &SchemaDecl,
    field: &SchemaField,
    field_ty: &Type,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(where_clause) = &field.where_clause else {
        return;
    };
    let mut visible_fields = decoded_fields.clone();
    visible_fields.insert(field.name.clone(), field_ty.clone());
    for reference in schema_validation_references(&where_clause.predicate) {
        let Some(ty) = schema_field_reference_type(&visible_fields, &reference) else {
            let reason = if schema_field_declared_after(schema, field, &reference) {
                "forward_field_reference"
            } else {
                "unknown_field_reference"
            };
            diagnostics.push(schema_field_predicate_reference_diagnostic(
                schema,
                field,
                &reference,
                reason,
                format!(
                    "schema field predicate reference `{reference}` must name the field being checked or an earlier decoded schema field"
                ),
                [],
            ));
            continue;
        };
        if reference.contains('.') && ty != &Type::int() {
            diagnostics.push(schema_field_predicate_reference_diagnostic(
                schema,
                field,
                &reference,
                "incompatible_field_reference",
                format!(
                    "schema field predicate reference `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            ));
        }
    }
}

fn schema_validation_references(predicate: &str) -> Vec<String> {
    let mut references = BTreeSet::new();
    let mut chars = predicate.char_indices().peekable();
    while let Some((start, ch)) = chars.next() {
        if !(ch.is_ascii_alphabetic() || ch == '_') {
            continue;
        }
        let mut end = start + ch.len_utf8();
        while let Some((index, next)) = chars.peek().copied() {
            if next.is_ascii_alphanumeric() || next == '_' || next == '.' {
                chars.next();
                end = index + next.len_utf8();
            } else {
                break;
            }
        }
        let ident = &predicate[start..end];
        if !matches!(ident, "true" | "false" | "and" | "or" | "not") {
            references.insert(ident.to_string());
        }
    }
    references.into_iter().collect()
}

fn check_schema_repeat_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    repeat: &SchemaRepeatSpec,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if !check_schema_repeat_references(
        schema,
        field,
        decoded_fields,
        &repeat.count_field,
        diagnostics,
    ) {
        return None;
    }
    let element_ty = match &repeat.payload {
        SchemaRepeatPayload::Primitive { .. } => Type::int(),
        SchemaRepeatPayload::ReservedBits { .. } => return None,
        SchemaRepeatPayload::ByteView { length_field } => {
            if !check_schema_repeat_byte_view_reference(
                schema,
                field,
                length_field,
                decoded_fields,
                diagnostics,
            ) {
                return None;
            }
            Type::named("ByteView", Vec::new())
        }
        SchemaRepeatPayload::Schema { schema_name } => {
            let payload_schema = resolve_schema_repeat_payload_schema(
                module,
                schema,
                field,
                schema_name,
                diagnostics,
            )?;
            if schema_payload_has_generalized_reserved_byte_prefix(payload_schema) {
                diagnostics.push(schema_repeat_payload_diagnostic(
                    schema,
                    field,
                    schema_name,
                    "incompatible_payload_schema",
                    format!(
                        "repeat payload schema `{}` uses a reserved-byte-prefix layout outside repeat payload helpers",
                        schema_payload_name_last_segment(schema_name)
                    ),
                    [],
                ));
                return None;
            }
            schema_decode_value_type(module, payload_schema).or_else(|| {
                diagnostics.push(schema_repeat_payload_diagnostic(
                    schema,
                    field,
                    schema_name,
                    "incompatible_payload_schema",
                    format!(
                        "repeat payload schema `{}` is not a supported decoded binary schema",
                        schema_payload_name_last_segment(schema_name)
                    ),
                    [],
                ));
                None
            })?
        }
    };
    Some(Type::named("List", vec![element_ty]))
}

fn check_schema_repeat_byte_view_reference(
    schema: &SchemaDecl,
    field: &SchemaField,
    length_expr: &str,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(references) = schema_length_expression_references(length_expr) else {
        return false;
    };
    let mut valid = true;
    for reference in references {
        let Some(ty) = schema_field_reference_type(decoded_fields, reference) else {
            let reason = if schema_field_declared_after(schema, field, reference) {
                "forward_field_reference"
            } else {
                "unknown_field_reference"
            };
            let mut diagnostic = schema_byte_view_reference_diagnostic(
                schema,
                field,
                reference,
                reason,
                format!(
                    "repeat ByteView length operand `{reference}` must be an earlier decoded `Int` field"
                ),
                [],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "length",
            );
            diagnostics.push(diagnostic);
            valid = false;
            continue;
        };
        if ty != &Type::int() {
            let mut diagnostic = schema_byte_view_reference_diagnostic(
                schema,
                field,
                reference,
                "incompatible_field_reference",
                format!(
                    "repeat ByteView length operand `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "length",
            );
            diagnostics.push(diagnostic);
            valid = false;
        }
    }
    valid
}

fn check_schema_byte_view_reference(
    schema: &SchemaDecl,
    field: &SchemaField,
    length_expr: &ByteViewLengthExpr,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let mut valid = true;
    for reference in length_expr.references() {
        let Some(ty) = schema_field_reference_type(decoded_fields, reference) else {
            let reason = if schema_field_declared_after(schema, field, reference) {
                "forward_field_reference"
            } else {
                "unknown_field_reference"
            };
            let mut diagnostic = schema_byte_view_reference_diagnostic(
                schema,
                field,
                reference,
                reason,
                format!(
                    "ByteView length operand `{reference}` must be an earlier decoded `Int` field"
                ),
                [],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "length",
            );
            diagnostics.push(diagnostic);
            valid = false;
            continue;
        };
        if ty != &Type::int() {
            let mut diagnostic = schema_byte_view_reference_diagnostic(
                schema,
                field,
                reference,
                "incompatible_field_reference",
                format!(
                    "ByteView length operand `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "length",
            );
            diagnostics.push(diagnostic);
            valid = false;
        }
    }
    valid
}

fn check_schema_byte_view_multiple(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(where_clause) = &field.where_clause else {
        return true;
    };
    let Some(constraint) = byte_view_multiple_constraint(&where_clause.predicate) else {
        diagnostics.push(schema_byte_view_multiple_diagnostic(
            schema,
            field,
            &where_clause.predicate,
            "unsupported_multiple_predicate",
            "ByteView field validation must use `payload_count multiple of <field-or-positive-integer>`".to_string(),
            [],
        ));
        return false;
    };
    let Some(reference) = constraint.reference() else {
        return true;
    };
    let Some(ty) = schema_field_reference_type(decoded_fields, reference) else {
        let reason = if schema_field_declared_after(schema, field, reference) {
            "forward_field_reference"
        } else {
            "unknown_field_reference"
        };
        let mut diagnostic = schema_byte_view_multiple_diagnostic(
            schema,
            field,
            reference,
            reason,
            format!(
                "ByteView multiple operand `{reference}` must be an earlier decoded `Int` field"
            ),
            [],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, "multiple");
        diagnostics.push(diagnostic);
        return false;
    };
    if ty != &Type::int() {
        let mut diagnostic = schema_byte_view_multiple_diagnostic(
            schema,
            field,
            reference,
            "incompatible_field_reference",
            format!(
                "ByteView multiple operand `{reference}` decodes as `{}`, not `Int`",
                ty.render()
            ),
            [("actual", JsonValue::string(ty.render()))],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, "multiple");
        diagnostics.push(diagnostic);
        return false;
    }
    true
}

fn check_schema_non_byte_view_multiple(
    schema: &SchemaDecl,
    field: &SchemaField,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(where_clause) = &field.where_clause else {
        return;
    };
    if !where_clause
        .predicate
        .trim()
        .starts_with("payload_count multiple of ")
    {
        return;
    }
    diagnostics.push(schema_byte_view_multiple_diagnostic(
        schema,
        field,
        &where_clause.predicate,
        "invalid_field_kind",
        "ByteView multiple validation can only be used on length-bounded `ByteView` fields"
            .to_string(),
        [],
    ));
}

fn check_schema_repeat_references(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    count_expr: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(references) = schema_length_expression_references(count_expr) else {
        return false;
    };
    let label = if references.len() == 1 {
        "repeat count field"
    } else {
        "repeat count operand"
    };
    let mut valid = true;
    for reference in references {
        let Some(ty) = schema_field_reference_type(decoded_fields, reference) else {
            let reason = if schema_field_declared_after(schema, field, reference) {
                "forward_field_reference"
            } else {
                "unknown_field_reference"
            };
            let mut diagnostic = schema_repeat_reference_diagnostic(
                schema,
                field,
                reference,
                reason,
                format!("{label} `{reference}` must be an earlier decoded `Int` field"),
                [],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "count",
            );
            diagnostics.push(diagnostic);
            valid = false;
            continue;
        };
        if ty != &Type::int() {
            let mut diagnostic = schema_repeat_reference_diagnostic(
                schema,
                field,
                reference,
                "incompatible_field_reference",
                format!(
                    "{label} `{reference}` decodes as `{}`, not `Int`",
                    ty.render()
                ),
                [("actual", JsonValue::string(ty.render()))],
            );
            add_compatible_prior_int_field_related(
                &mut diagnostic,
                schema,
                decoded_fields,
                "count",
            );
            diagnostics.push(diagnostic);
            valid = false;
        }
    }
    valid
}

fn resolve_schema_repeat_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    payload_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let Some(segments) = schema_payload_name_path(payload_name) else {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            payload_name,
            "invalid_payload_name",
            format!("repeat payload schema `{payload_name}` is not a valid schema path"),
            [],
        ));
        return None;
    };
    match segments.as_slice() {
        [name] => {
            resolve_local_schema_repeat_payload_schema(module, schema, field, name, diagnostics)
        }
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                schema.module_name.as_deref(),
            ) else {
                diagnostics.push(schema_repeat_payload_diagnostic(
                    schema,
                    field,
                    payload_name,
                    "unknown_import",
                    format!("repeat payload schema `{payload_name}` is not declared"),
                    [],
                ));
                return None;
            };
            resolve_imported_schema_repeat_payload_schema(
                module,
                schema,
                field,
                use_decl,
                payload_name,
                name,
                diagnostics,
            )
        }
        _ => None,
    }
}

fn resolve_local_schema_repeat_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let current_index = module
        .schemas
        .iter()
        .position(|candidate| candidate.node_id == schema.node_id)?;
    if let Some((index, candidate)) = module.schemas.iter().enumerate().find(|(_, candidate)| {
        candidate.name.as_deref() == Some(name)
            && candidate.module_name.as_deref() == schema.module_name.as_deref()
    }) {
        if index == current_index {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                name,
                "self_payload_schema",
                format!("repeat payload schema `{name}` cannot reference itself"),
                [],
            ));
            return None;
        }
        if index > current_index {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                name,
                "forward_payload_schema",
                format!(
                    "repeat payload schema `{name}` must be declared before schema `{}`",
                    schema.name.as_deref().unwrap_or("<missing>")
                ),
                [],
            ));
            return None;
        }
        if candidate.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                name,
                "non_binary_payload_schema",
                format!("repeat payload schema `{name}` must use `format binary`"),
                [],
            ));
            return None;
        }
        return Some(candidate);
    }
    if let Some(kind) = codec_schema_wrong_kind(module, schema.module_name.as_deref(), name) {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            name,
            "non_schema_payload",
            format!("repeat payload `{name}` resolves to a {kind}, not a schema"),
            [("resolved_kind", JsonValue::string(kind))],
        ));
    } else {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            name,
            "unknown_payload_schema",
            format!("repeat payload schema `{name}` is not declared"),
            [],
        ));
    }
    None
}

fn resolve_imported_schema_repeat_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    use_decl: &UseDecl,
    payload_name: &str,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let target_module = Some(use_decl.name.as_str());
    if let Some(candidate) = module.schemas.iter().find(|candidate| {
        candidate.name.as_deref() == Some(name) && candidate.module_name.as_deref() == target_module
    }) {
        if candidate.visibility != Visibility::Public
            && !companion_private_schema_access_allowed(module, schema, use_decl)
        {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                payload_name,
                "private_imported_payload_schema",
                format!("imported repeat payload schema `{payload_name}` is private"),
                [],
            ));
            return None;
        }
        if candidate.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
            diagnostics.push(schema_repeat_payload_diagnostic(
                schema,
                field,
                payload_name,
                "non_binary_payload_schema",
                format!("repeat payload schema `{payload_name}` must use `format binary`"),
                [],
            ));
            return None;
        }
        return Some(candidate);
    }
    if let Some(kind) = codec_schema_wrong_kind(module, target_module, name) {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            payload_name,
            "non_schema_payload",
            format!("repeat payload `{payload_name}` resolves to a {kind}, not a schema"),
            [("resolved_kind", JsonValue::string(kind))],
        ));
    } else {
        diagnostics.push(schema_repeat_payload_diagnostic(
            schema,
            field,
            payload_name,
            "unknown_payload_schema",
            format!("repeat payload schema `{payload_name}` is not declared"),
            [],
        ));
    }
    None
}

fn companion_private_schema_access_allowed(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    use_decl: &UseDecl,
) -> bool {
    use_decl.package.is_none()
        && schema.module_name.as_deref().is_some_and(|current_module| {
            companion_access_targets(module)
                .get(current_module)
                .is_some_and(|allowed| allowed == use_decl.name.as_str())
        })
}

fn companion_schema_access_target(module: &SurfaceModule, schema: &SchemaDecl) -> Option<String> {
    let current_module = schema.module_name.as_deref()?;
    companion_access_targets(module)
        .get(current_module)
        .cloned()
}

fn companion_access_targets(module: &SurfaceModule) -> BTreeMap<String, String> {
    module
        .functions
        .iter()
        .filter_map(|function| {
            companion_access_target(function.span.file.as_str(), function.module_name.as_deref())
        })
        .chain(module.schemas.iter().filter_map(|schema| {
            companion_access_target(schema.span.file.as_str(), schema.module_name.as_deref())
        }))
        .collect()
}

fn companion_access_target(path: &str, module_name: Option<&str>) -> Option<(String, String)> {
    let companion = classify_companion_source(path)?;
    let companion_module = module_name?.to_string();
    let target_module = companion
        .target_path
        .strip_suffix(".veln")?
        .replace('/', "::");
    Some((companion_module, target_module))
}

fn schema_repeat_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string("count")));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.repeat_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn schema_byte_view_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string("length")));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.byte_view_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn schema_byte_view_multiple_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string("multiple")));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.byte_view_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn schema_validation_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    validation: &SchemaValidationClause,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = vec![
        (
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>").to_string()),
        ),
        ("reason", JsonValue::string(reason)),
        ("reference", JsonValue::string(reference.to_string())),
    ];
    fields.extend(extra);
    Diagnostic::new(
        "schema.validation_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(validation.span.clone()),
        JsonValue::object(fields),
    )
}

fn schema_field_predicate_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string("predicate")));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.field_predicate_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn schema_validation_duplicate_diagnostic(
    schema: &SchemaDecl,
    validation: &SchemaValidationClause,
) -> Diagnostic {
    Diagnostic::new(
        "schema.validation_duplicate",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "schema `{}` can declare only one schema-level validation",
            schema.name.as_deref().unwrap_or("<missing>")
        ),
        Some(validation.span.clone()),
        JsonValue::object([
            (
                "schema",
                JsonValue::string(schema.name.as_deref().unwrap_or("<missing>").to_string()),
            ),
            ("reason", JsonValue::string("duplicate_validation")),
        ]),
    )
}

fn schema_repeat_payload_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    payload_name: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("payload", JsonValue::string(payload_name.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.repeat_payload",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn check_schema_dispatch_field(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    let mut valid =
        check_schema_dispatch_references(schema, field, dispatch, decoded_fields, diagnostics);
    let payload_types =
        collect_schema_dispatch_payload_types(module, schema, field, dispatch, diagnostics);
    valid &= !payload_types.mixed && !payload_types.resolution_failed;

    let recursive_dispatch_payload =
        schema_dispatch_has_recursive_payload(module, schema, field, dispatch);
    reconcile_schema_dispatch_payload_types(
        SchemaDispatchFieldContext {
            module,
            schema,
            field,
            dispatch,
        },
        &payload_types,
        recursive_dispatch_payload,
        &mut valid,
        diagnostics,
    )?;

    if !valid {
        return None;
    }
    let payload_ty = if recursive_dispatch_payload {
        schema_recursive_dispatch_helper_payload_type(module, schema, dispatch)?
    } else {
        payload_types.expected?
    };
    if dispatch.preserves_unknown {
        Some(Type::named("SchemaDispatchPayload", vec![payload_ty]))
    } else {
        Some(payload_ty)
    }
}

fn check_schema_dispatch_references(
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    decoded_fields: &BTreeMap<String, Type>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let tag_valid = check_schema_dispatch_reference(
        schema,
        field,
        decoded_fields,
        &dispatch.tag_field,
        "tag",
        diagnostics,
    );
    let length_valid = dispatch.length_field.as_ref().is_none_or(|length_field| {
        check_schema_dispatch_reference(
            schema,
            field,
            decoded_fields,
            length_field,
            "length",
            diagnostics,
        )
    });
    tag_valid && length_valid
}

struct SchemaDispatchPayloadTypes {
    expected: Option<Type>,
    mixed: bool,
    resolution_failed: bool,
}

#[derive(Clone, Copy)]
struct SchemaDispatchFieldContext<'a> {
    module: &'a SurfaceModule,
    schema: &'a SchemaDecl,
    field: &'a SchemaField,
    dispatch: &'a SchemaDispatchSpec,
}

fn collect_schema_dispatch_payload_types(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    diagnostics: &mut Vec<Diagnostic>,
) -> SchemaDispatchPayloadTypes {
    let mut result = SchemaDispatchPayloadTypes {
        expected: None,
        mixed: false,
        resolution_failed: false,
    };
    for case in &dispatch.cases {
        let Some(payload_ty) = resolve_schema_dispatch_case_payload_type(
            module,
            schema,
            field,
            dispatch,
            case,
            diagnostics,
        ) else {
            result.resolution_failed = true;
            continue;
        };
        if let Some(expected) = &result.expected {
            result.mixed |= expected != &payload_ty;
        } else {
            result.expected = Some(payload_ty);
        }
    }
    result
}

fn resolve_schema_dispatch_case_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    case: &SchemaDispatchCase,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
        SchemaDispatchCasePayload::ReservedBits { .. } => Some(Type::unit()),
        SchemaDispatchCasePayload::Schema { schema_name } => resolve_schema_dispatch_named_payload(
            module,
            schema,
            field,
            dispatch,
            case.tag,
            schema_name,
            diagnostics,
        ),
    }
}

fn resolve_schema_dispatch_named_payload(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    tag: i64,
    schema_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if schema.name.as_deref() == Some(schema_name) {
        return resolve_self_schema_dispatch_payload(
            module,
            schema,
            field,
            dispatch,
            tag,
            schema_name,
            diagnostics,
        );
    }
    let payload_schema = resolve_schema_dispatch_payload_schema(
        module,
        schema,
        field,
        tag,
        schema_name,
        diagnostics,
    )?;
    resolve_external_schema_dispatch_payload(
        SchemaDispatchFieldContext {
            module,
            schema,
            field,
            dispatch,
        },
        tag,
        schema_name,
        payload_schema,
        diagnostics,
    )
}

fn resolve_self_schema_dispatch_payload(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    tag: i64,
    schema_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if !recursive_dispatch_payload_is_eligible(schema, field, dispatch, schema_name) {
        push_recursive_dispatch_payload_blocker(
            schema,
            field,
            dispatch,
            tag,
            schema_name,
            schema,
            diagnostics,
        );
        return None;
    }
    schema_recursive_dispatch_payload_type(module, schema).or_else(|| {
        diagnostics.push(incompatible_schema_dispatch_payload_diagnostic(
            module,
            schema,
            field,
            tag,
            schema_name,
            schema,
            SchemaHelperAvailability {
                decode: false,
                encode: false,
            },
        ));
        None
    })
}

fn resolve_external_schema_dispatch_payload(
    context: SchemaDispatchFieldContext<'_>,
    tag: i64,
    schema_name: &str,
    payload_schema: &SchemaDecl,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if schema_has_recursive_dispatch_payload(payload_schema)
        && !(recursive_dispatch_payload_case_is_eligible(
            context.module,
            context.schema,
            context.field,
            context.dispatch,
            schema_name,
        ) || recursive_dispatch_decode_only_payload_case_is_eligible(
            context.module,
            context.schema,
            context.dispatch,
            schema_name,
        ))
    {
        push_recursive_dispatch_payload_blocker(
            context.schema,
            context.field,
            context.dispatch,
            tag,
            schema_name,
            payload_schema,
            diagnostics,
        );
        return None;
    }
    schema_dispatch_payload_helper_type(
        context.module,
        context.schema,
        context.field,
        tag,
        schema_name,
        payload_schema,
        diagnostics,
    )
}

fn push_recursive_dispatch_payload_blocker(
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    tag: i64,
    schema_name: &str,
    payload_schema: &SchemaDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let blocker =
        recursive_dispatch_payload_blocker(schema, field, dispatch, schema_name, payload_schema);
    diagnostics.push(schema_dispatch_payload_diagnostic(
        schema,
        field,
        tag,
        schema_name,
        blocker.reason,
        blocker.message,
        [(
            "recursive_helper_fact",
            JsonValue::string(blocker.fact.to_string()),
        )],
    ));
}

fn schema_dispatch_has_recursive_payload(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
) -> bool {
    dispatch.cases.iter().any(|case| {
        matches!(
            &case.payload,
            SchemaDispatchCasePayload::Schema { schema_name }
                    if recursive_dispatch_payload_case_is_eligible(
                        module,
                        schema,
                        field,
                        dispatch,
                        schema_name,
                    ) || recursive_dispatch_decode_only_payload_case_is_eligible(
                        module,
                        schema,
                        dispatch,
                        schema_name,
                    )
        )
    })
}

fn reconcile_schema_dispatch_payload_types(
    context: SchemaDispatchFieldContext<'_>,
    payload_types: &SchemaDispatchPayloadTypes,
    recursive_dispatch_payload: bool,
    valid: &mut bool,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<()> {
    if !payload_types.mixed {
        return Some(());
    }
    if recursive_dispatch_payload {
        *valid = !payload_types.resolution_failed;
        return Some(());
    }
    if payload_types.resolution_failed {
        *valid = false;
        return Some(());
    }
    let expected = payload_types.expected.as_ref()?;
    let Some((case, payload_ty)) = context.dispatch.cases.iter().find_map(|case| {
        let payload_ty =
            schema_dispatch_case_known_payload_type(context.module, context.schema, case)?;
        (&payload_ty != expected).then_some((case, payload_ty))
    }) else {
        return Some(());
    };
    diagnostics.push(schema_dispatch_payload_diagnostic(
        context.schema,
        context.field,
        case.tag,
        schema_dispatch_case_payload_name(&case.payload),
        "incompatible_payload_type",
        format!(
            "dispatch payload case `{}` decodes as `{}`, but earlier cases decode as `{}`",
            case.tag,
            payload_ty.render(),
            expected.render()
        ),
        [
            ("expected", JsonValue::string(expected.render())),
            ("actual", JsonValue::string(payload_ty.render())),
        ],
    ));
    Some(())
}

fn schema_dispatch_case_known_payload_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    case: &SchemaDispatchCase,
) -> Option<Type> {
    match &case.payload {
        SchemaDispatchCasePayload::Primitive { .. } => Some(Type::int()),
        SchemaDispatchCasePayload::ReservedBits { .. } => Some(Type::unit()),
        SchemaDispatchCasePayload::Schema { schema_name }
            if schema.name.as_deref() == Some(schema_name.as_str()) =>
        {
            schema_recursive_dispatch_payload_type(module, schema)
        }
        SchemaDispatchCasePayload::Schema { schema_name } => {
            schema_dispatch_payload_schema(module, schema, schema_name)
                .and_then(|payload_schema| schema_decode_value_type(module, payload_schema))
        }
    }
}

struct RecursiveDispatchPayloadBlocker {
    reason: &'static str,
    fact: &'static str,
    message: String,
}

fn recursive_dispatch_payload_blocker(
    _parent_schema: &SchemaDecl,
    field: &SchemaField,
    dispatch: &SchemaDispatchSpec,
    schema_name: &str,
    payload_schema: &SchemaDecl,
) -> RecursiveDispatchPayloadBlocker {
    if dispatch.length_field.is_none() {
        return RecursiveDispatchPayloadBlocker {
            reason: "recursive_payload_missing_length_bound",
            fact: "recursive dispatch payloads require a length-bounded parent dispatch field",
            message: format!(
                "dispatch payload schema `{schema_name}` requires parent dispatch field `{}` to include a length field",
                field.name
            ),
        };
    }
    if !dispatch
        .cases
        .iter()
        .any(|case| matches!(case.payload, SchemaDispatchCasePayload::Primitive { .. }))
    {
        return RecursiveDispatchPayloadBlocker {
            reason: "recursive_payload_missing_primitive_base_case",
            fact: "recursive dispatch parents require a non-recursive primitive base case",
            message: format!(
                "dispatch payload schema `{schema_name}` requires parent dispatch field `{}` to include a non-recursive primitive case",
                field.name
            ),
        };
    }
    if !schema_has_eligible_recursive_dispatch_payload(payload_schema) {
        return RecursiveDispatchPayloadBlocker {
            reason: "recursive_payload_missing_bounded_helper",
            fact: "recursive dispatch payload schemas must expose a bounded recursive helper",
            message: format!(
                "dispatch payload schema `{schema_name}` does not expose a bounded recursive helper"
            ),
        };
    }
    RecursiveDispatchPayloadBlocker {
        reason: "recursive_payload_ineligible_parent",
        fact: "recursive dispatch payloads require a length-bounded parent with recursive helper support and a non-recursive base case",
        message: format!(
            "dispatch payload schema `{schema_name}` does not satisfy recursive dispatch helper requirements"
        ),
    }
}

fn check_schema_dispatch_reference(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    reference: &str,
    role: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let Some(ty) = schema_field_reference_type(decoded_fields, reference) else {
        let reason = if schema_field_declared_after(schema, field, reference) {
            "forward_field_reference"
        } else {
            "unknown_field_reference"
        };
        let mut diagnostic = schema_dispatch_reference_diagnostic(
            schema,
            field,
            reference,
            role,
            reason,
            format!("dispatch {role} field `{reference}` must be an earlier decoded `Int` field"),
            [],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, role);
        diagnostics.push(diagnostic);
        return false;
    };
    if ty != &Type::int() {
        let mut diagnostic = schema_dispatch_reference_diagnostic(
            schema,
            field,
            reference,
            role,
            "incompatible_field_reference",
            format!(
                "dispatch {role} field `{reference}` decodes as `{}`, not `Int`",
                ty.render()
            ),
            [("actual", JsonValue::string(ty.render()))],
        );
        add_compatible_prior_int_field_related(&mut diagnostic, schema, decoded_fields, role);
        diagnostics.push(diagnostic);
        return false;
    }
    true
}

fn resolve_schema_dispatch_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let context = SchemaDispatchPayloadContext { schema, field, tag };
    let Some(segments) = schema_payload_name_path(payload_name) else {
        diagnostics.push(schema_dispatch_payload_diagnostic(
            schema,
            field,
            tag,
            payload_name,
            "invalid_payload_name",
            format!("dispatch payload schema `{payload_name}` is not a valid schema path"),
            [],
        ));
        return None;
    };
    match segments.as_slice() {
        [name] => resolve_local_schema_dispatch_payload_schema(
            module,
            schema,
            field,
            tag,
            name,
            diagnostics,
        ),
        [_, .., name] => {
            let Some(use_decl) = imported_use_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                schema.module_name.as_deref(),
            ) else {
                diagnostics.push(schema_dispatch_payload_diagnostic(
                    schema,
                    field,
                    tag,
                    payload_name,
                    "unknown_import",
                    format!("dispatch payload schema `{payload_name}` is not declared"),
                    [],
                ));
                return None;
            };
            resolve_imported_schema_dispatch_payload_schema(
                module,
                context,
                use_decl,
                payload_name,
                name,
                diagnostics,
            )
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct SchemaDispatchPayloadContext<'a> {
    schema: &'a SchemaDecl,
    field: &'a SchemaField,
    tag: i64,
}

fn resolve_local_schema_dispatch_payload_schema<'a>(
    module: &'a SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let current_index = module
        .schemas
        .iter()
        .position(|candidate| candidate.node_id == schema.node_id)?;
    if let Some((index, candidate)) = module.schemas.iter().enumerate().find(|(_, candidate)| {
        candidate.name.as_deref() == Some(name)
            && candidate.module_name.as_deref() == schema.module_name.as_deref()
    }) {
        if index == current_index {
            diagnostics.push(schema_dispatch_payload_diagnostic(
                schema,
                field,
                tag,
                name,
                "self_payload_schema",
                format!("dispatch payload schema `{name}` cannot reference itself"),
                [],
            ));
            return None;
        }
        if index > current_index {
            diagnostics.push(schema_dispatch_payload_diagnostic(
                schema,
                field,
                tag,
                name,
                "forward_payload_schema",
                format!(
                    "dispatch payload schema `{name}` must be declared before schema `{}`",
                    schema.name.as_deref().unwrap_or("<missing>")
                ),
                [],
            ));
            return None;
        }
        if candidate.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
            diagnostics.push(schema_dispatch_payload_diagnostic(
                schema,
                field,
                tag,
                name,
                "non_binary_payload_schema",
                format!("dispatch payload schema `{name}` must use `format binary`"),
                [],
            ));
            return None;
        }
        return Some(candidate);
    }
    if let Some(kind) = codec_schema_wrong_kind(module, schema.module_name.as_deref(), name) {
        diagnostics.push(schema_dispatch_payload_diagnostic(
            schema,
            field,
            tag,
            name,
            "non_schema_payload",
            format!("dispatch payload `{name}` resolves to a {kind}, not a schema"),
            [("resolved_kind", JsonValue::string(kind))],
        ));
    } else {
        diagnostics.push(schema_dispatch_payload_diagnostic(
            schema,
            field,
            tag,
            name,
            "unknown_payload_schema",
            format!("dispatch payload schema `{name}` is not declared"),
            [],
        ));
    }
    None
}

fn resolve_imported_schema_dispatch_payload_schema<'a>(
    module: &'a SurfaceModule,
    context: SchemaDispatchPayloadContext<'_>,
    use_decl: &UseDecl,
    payload_name: &str,
    name: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a SchemaDecl> {
    let target_module = Some(use_decl.name.as_str());
    if let Some(candidate) = module.schemas.iter().find(|candidate| {
        candidate.name.as_deref() == Some(name) && candidate.module_name.as_deref() == target_module
    }) {
        if candidate.visibility != Visibility::Public
            && !companion_private_schema_access_allowed(module, context.schema, use_decl)
        {
            diagnostics.push(schema_dispatch_payload_diagnostic(
                context.schema,
                context.field,
                context.tag,
                payload_name,
                "private_imported_payload_schema",
                format!("imported dispatch payload schema `{payload_name}` is private"),
                [],
            ));
            return None;
        }
        if candidate.format.as_ref().map(|format| format.name.as_str()) != Some("binary") {
            diagnostics.push(schema_dispatch_payload_diagnostic(
                context.schema,
                context.field,
                context.tag,
                payload_name,
                "non_binary_payload_schema",
                format!("dispatch payload schema `{payload_name}` must use `format binary`"),
                [],
            ));
            return None;
        }
        return Some(candidate);
    }
    if let Some(kind) = codec_schema_wrong_kind(module, target_module, name) {
        diagnostics.push(schema_dispatch_payload_diagnostic(
            context.schema,
            context.field,
            context.tag,
            payload_name,
            "non_schema_payload",
            format!("dispatch payload `{payload_name}` resolves to a {kind}, not a schema"),
            [("resolved_kind", JsonValue::string(kind))],
        ));
    } else {
        diagnostics.push(schema_dispatch_payload_diagnostic(
            context.schema,
            context.field,
            context.tag,
            payload_name,
            "unknown_payload_schema",
            format!("dispatch payload schema `{payload_name}` is not declared"),
            [],
        ));
    }
    None
}

fn schema_dispatch_case_payload_name(payload: &SchemaDispatchCasePayload) -> &str {
    match payload {
        SchemaDispatchCasePayload::Primitive { .. } => "<primitive>",
        SchemaDispatchCasePayload::ReservedBits { .. } => "<reserved>",
        SchemaDispatchCasePayload::Schema { schema_name } => schema_name,
    }
}

fn schema_field_declared_after(schema: &SchemaDecl, field: &SchemaField, reference: &str) -> bool {
    let reference = reference.split('.').next().unwrap_or(reference);
    let current_index = schema
        .fields
        .iter()
        .position(|candidate| candidate.node_id == field.node_id);
    let reference_index = schema
        .fields
        .iter()
        .position(|candidate| candidate.name == reference);
    matches!((current_index, reference_index), (Some(current), Some(reference)) if reference > current)
}

fn add_compatible_prior_int_field_related(
    diagnostic: &mut Diagnostic,
    schema: &SchemaDecl,
    decoded_fields: &BTreeMap<String, Type>,
    role: &str,
) {
    let expected_type = Type::int();
    let int_field = |field: &&SchemaField| decoded_fields.get(&field.name) == Some(&expected_type);
    let Some(candidate_field) = schema
        .fields
        .iter()
        .find(|field| int_field(field) && field.name.contains(role))
        .or_else(|| schema.fields.iter().find(int_field))
    else {
        return;
    };
    let candidate_name = &candidate_field.name;
    diagnostic.related.push(JsonValue::object([
        ("span", span_json(&candidate_field.span)),
        (
            "message",
            JsonValue::string(format!(
                "Compatible earlier {role} field `{candidate_name}` is declared here."
            )),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    (
                        "name",
                        JsonValue::string(
                            schema.name.as_deref().unwrap_or("<missing>").to_string(),
                        ),
                    ),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string(candidate_name.clone())),
                ]),
            ]),
        ),
    ]));
}

fn schema_dispatch_reference_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    reference: &str,
    role: &'static str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("role", JsonValue::string(role)));
    fields.push(("reference", JsonValue::string(reference.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.dispatch_reference",
        Severity::Error,
        DiagnosticKind::Name,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn schema_dispatch_payload_diagnostic<const N: usize>(
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    reason: &'static str,
    message: String,
    extra: [(&'static str, JsonValue); N],
) -> Diagnostic {
    let mut fields = schema_dispatch_details(schema, field, reason);
    fields.push(("case_tag", JsonValue::Number(tag)));
    fields.push(("payload", JsonValue::string(payload_name.to_string())));
    fields.extend(extra);
    Diagnostic::new(
        "schema.dispatch_payload",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(field.span.clone()),
        JsonValue::object(fields),
    )
}

fn incompatible_schema_dispatch_payload_diagnostic(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    payload_schema: &SchemaDecl,
    helper_availability: SchemaHelperAvailability,
) -> Diagnostic {
    let payload_schema_name = schema_payload_name_last_segment(payload_name);
    let decode_helper = schema_decode_step_function_name(payload_schema_name);
    let encode_helper = schema_encode_function_name(payload_schema_name);
    let unsupported_blocker =
        unsupported_dispatch_payload_helper_blocker(module, payload_schema, helper_availability);
    let mut diagnostic = schema_dispatch_payload_diagnostic(
        schema,
        field,
        tag,
        payload_name,
        "incompatible_payload_schema",
        format!(
            "dispatch payload schema `{payload_schema_name}` is outside the generated binary schema helper slice"
        ),
        [
            (
                "expected_decode_helper",
                JsonValue::string(decode_helper.clone()),
            ),
            (
                "decode_helper_boundary",
                JsonValue::string("generated_binary_schema_decode_step"),
            ),
            (
                "expected_encode_helper",
                JsonValue::string(encode_helper.clone()),
            ),
            (
                "encode_helper_boundary",
                JsonValue::string("generated_binary_schema_encode"),
            ),
        ],
    );
    diagnostic.details =
        add_dispatch_payload_helper_unavailable_details(diagnostic.details, helper_availability);
    if let Some(unsupported_blocker) = &unsupported_blocker {
        diagnostic.details = add_dispatch_payload_unsupported_blocker_details(
            diagnostic.details,
            unsupported_blocker,
        );
    }
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("schema_declaration")),
        (
            "message",
            JsonValue::string(dispatch_payload_schema_declaration_message(
                payload_schema_name,
                &decode_helper,
                &encode_helper,
                helper_availability,
            )),
        ),
        ("span", span_json(&payload_schema.span)),
    ]));
    if let Some(unsupported_blocker) = unsupported_blocker {
        diagnostic
            .related
            .push(dispatch_payload_unsupported_blocker_related(
                &unsupported_blocker,
            ));
    }
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("helper_boundary")),
        (
            "message",
            JsonValue::string(dispatch_payload_helper_boundary_message(
                &decode_helper,
                &encode_helper,
                helper_availability,
            )),
        ),
    ]));
    diagnostic
}

#[derive(Clone, Copy)]
struct SchemaHelperAvailability {
    decode: bool,
    encode: bool,
}

fn schema_dispatch_payload_helper_type(
    module: &SurfaceModule,
    schema: &SchemaDecl,
    field: &SchemaField,
    tag: i64,
    payload_name: &str,
    payload_schema: &SchemaDecl,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if schema_payload_has_generalized_reserved_byte_prefix(payload_schema) {
        diagnostics.push(incompatible_schema_dispatch_payload_diagnostic(
            module,
            schema,
            field,
            tag,
            payload_name,
            payload_schema,
            SchemaHelperAvailability {
                decode: false,
                encode: false,
            },
        ));
        return None;
    }
    let decode_type = schema_decode_value_type(module, payload_schema);
    match decode_type {
        Some(payload_ty) => Some(payload_ty),
        None => {
            let encode_type = schema_encode_value_type(module, payload_schema);
            diagnostics.push(incompatible_schema_dispatch_payload_diagnostic(
                module,
                schema,
                field,
                tag,
                payload_name,
                payload_schema,
                SchemaHelperAvailability {
                    decode: false,
                    encode: encode_type.is_some(),
                },
            ));
            None
        }
    }
}

fn dispatch_payload_schema_declaration_message(
    payload_schema_name: &str,
    decode_helper: &str,
    encode_helper: &str,
    helper_availability: SchemaHelperAvailability,
) -> String {
    match (helper_availability.decode, helper_availability.encode) {
        (false, false) => format!(
            "Schema `{payload_schema_name}` is declared here and does not expose the generated `{decode_helper}` helper required for dispatch payload decoding."
        ),
        (false, true) => format!(
            "Schema `{payload_schema_name}` is declared here and does not expose the generated `{decode_helper}` helper required for dispatch payload decoding."
        ),
        (true, false) => format!(
            "Schema `{payload_schema_name}` is declared here and does not expose the generated `{encode_helper}` helper required for dispatch payload encoding."
        ),
        (true, true) => format!(
            "Schema `{payload_schema_name}` is declared here and exposes the generated dispatch payload helpers."
        ),
    }
}

fn dispatch_payload_helper_boundary_message(
    decode_helper: &str,
    encode_helper: &str,
    helper_availability: SchemaHelperAvailability,
) -> String {
    match (helper_availability.decode, helper_availability.encode) {
        (false, false) | (true, true) => format!(
            "Dispatch payload schemas must expose generated decode helpers before parent decode helpers can use them, and generated encode helpers before parent encode helpers can use them; expected `{decode_helper}` and `{encode_helper}`."
        ),
        (false, true) => format!(
            "Dispatch payload schemas must expose generated decode helpers before parent decode helpers can use them; expected `{decode_helper}`."
        ),
        (true, false) => format!(
            "Dispatch payload schemas must expose generated encode helpers before parent encode helpers can use them; expected `{encode_helper}`."
        ),
    }
}

struct UnsupportedDispatchPayloadHelperField<'a> {
    schema_name: String,
    field: &'a SchemaField,
    field_path_display: String,
    layout_fact: String,
    reason: &'static str,
}

enum UnsupportedDispatchPayloadHelperBlocker<'a> {
    Field(UnsupportedDispatchPayloadHelperField<'a>),
}

fn unsupported_dispatch_payload_helper_blocker<'a>(
    _module: &'a SurfaceModule,
    schema: &'a SchemaDecl,
    _helper_availability: SchemaHelperAvailability,
) -> Option<UnsupportedDispatchPayloadHelperBlocker<'a>> {
    unsupported_dispatch_payload_helper_field(schema)
        .map(UnsupportedDispatchPayloadHelperBlocker::Field)
}

fn unsupported_dispatch_payload_helper_field(
    schema: &SchemaDecl,
) -> Option<UnsupportedDispatchPayloadHelperField<'_>> {
    let schema_name = schema.name.clone().unwrap_or_default();
    let mut decoded_fields = BTreeMap::<String, Type>::new();
    for (index, field) in schema.fields.iter().enumerate() {
        if let Some(reserved) = reserved_bits_schema_primitive(&field.ty) {
            if schema_field_uses_generalized_reserved_byte_prefix(&schema.fields, index, reserved) {
                return Some(UnsupportedDispatchPayloadHelperField {
                    field,
                    field_path_display: format!("{schema_name}.{}", field.name),
                    layout_fact: format!(
                        "`ReservedBits({}, {})` uses the general direct byte-prefix rule, which is outside dispatch and repeat payload helpers",
                        reserved.0, reserved.1
                    ),
                    reason: "unsupported_reserved_bits_layout",
                    schema_name,
                });
            }
            if supported_encode_reserved_bits(&schema.fields, index, reserved).is_none() {
                let layout =
                    reserved_bits_unsupported_layout_context(schema, Some(index), reserved.0);
                return Some(UnsupportedDispatchPayloadHelperField {
                    field,
                    field_path_display: format!("{schema_name}.{}", field.name),
                    layout_fact: format!(
                        "`ReservedBits({}, {})` is outside the supported `{}` layout: {}",
                        reserved.0,
                        reserved.1,
                        layout.supported_layout_family,
                        layout.human_supported_note
                    ),
                    reason: "unsupported_reserved_bits_layout",
                    schema_name,
                });
            }
            continue;
        }
        if let Some(width) = exact_width_schema_primitive(&field.ty) {
            decoded_fields.insert(field.name.clone(), Type::int());
            if width == 0 {
                return None;
            }
            continue;
        }
        if let Some(length_expr) = byte_view_schema_primitive(&field.ty) {
            if let Some(reference) = length_expr.references().into_iter().find(|reference| {
                schema_field_reference_type(&decoded_fields, reference) != Some(&Type::int())
            }) {
                let reference_fact =
                    byte_view_ineligible_length_fact(schema, field, &decoded_fields, reference);
                return Some(UnsupportedDispatchPayloadHelperField {
                    field,
                    field_path_display: format!("{schema_name}.{}", field.name),
                    layout_fact: format!(
                        "`ByteView({})` requires {reference_fact}",
                        length_expr.render()
                    ),
                    reason: "ineligible_byte_view_length_reference",
                    schema_name,
                });
            }
            decoded_fields.insert(field.name.clone(), Type::named("ByteView", Vec::new()));
            continue;
        }
    }
    None
}

fn byte_view_ineligible_length_fact(
    schema: &SchemaDecl,
    field: &SchemaField,
    decoded_fields: &BTreeMap<String, Type>,
    reference: &str,
) -> String {
    if let Some(actual) = schema_field_reference_type(decoded_fields, reference) {
        format!(
            "length reference `{reference}` to decode as `Int`; it decodes as `{}`",
            actual.render()
        )
    } else if schema_field_declared_after(schema, field, reference) {
        format!(
            "length reference `{reference}` to be declared before field `{}`",
            field.name
        )
    } else {
        format!("length reference `{reference}` to name an earlier decoded `Int` field")
    }
}

fn add_dispatch_payload_helper_unavailable_details(
    details: JsonValue,
    helper_availability: SchemaHelperAvailability,
) -> JsonValue {
    let JsonValue::Object(mut fields) = details else {
        return details;
    };
    fields.push((
        "unavailable_helper_directions".to_string(),
        JsonValue::array(dispatch_payload_unavailable_helper_directions(
            helper_availability,
        )),
    ));
    JsonValue::Object(fields)
}

fn dispatch_payload_unavailable_helper_directions(
    helper_availability: SchemaHelperAvailability,
) -> Vec<JsonValue> {
    let mut directions = Vec::new();
    if !helper_availability.decode {
        directions.push(JsonValue::string("decode"));
    }
    if !helper_availability.encode {
        directions.push(JsonValue::string("encode"));
    }
    directions
}

fn add_dispatch_payload_unsupported_blocker_details(
    details: JsonValue,
    unsupported: &UnsupportedDispatchPayloadHelperBlocker<'_>,
) -> JsonValue {
    match unsupported {
        UnsupportedDispatchPayloadHelperBlocker::Field(field) => {
            add_dispatch_payload_unsupported_field_details(details, field)
        }
    }
}

fn add_dispatch_payload_unsupported_field_details(
    details: JsonValue,
    unsupported: &UnsupportedDispatchPayloadHelperField<'_>,
) -> JsonValue {
    let JsonValue::Object(mut fields) = details else {
        return details;
    };
    fields.push((
        "unsupported_nested_schema".to_string(),
        JsonValue::string(unsupported.schema_name.clone()),
    ));
    fields.push((
        "unsupported_nested_field".to_string(),
        JsonValue::string(unsupported.field.name.clone()),
    ));
    fields.push((
        "unsupported_nested_field_path".to_string(),
        dispatch_payload_unsupported_field_path(unsupported),
    ));
    fields.push((
        "unsupported_nested_layout_reason".to_string(),
        JsonValue::string(unsupported.reason),
    ));
    fields.push((
        "unsupported_nested_layout_fact".to_string(),
        JsonValue::string(unsupported.layout_fact.clone()),
    ));
    JsonValue::Object(fields)
}

fn dispatch_payload_unsupported_blocker_related(
    unsupported: &UnsupportedDispatchPayloadHelperBlocker<'_>,
) -> JsonValue {
    match unsupported {
        UnsupportedDispatchPayloadHelperBlocker::Field(field) => {
            dispatch_payload_unsupported_field_related(field)
        }
    }
}

fn dispatch_payload_unsupported_field_related(
    unsupported: &UnsupportedDispatchPayloadHelperField<'_>,
) -> JsonValue {
    let layout_fact = unsupported.layout_fact.trim_end_matches('.');
    JsonValue::object([
        ("kind", JsonValue::string("unsupported_nested_field")),
        ("span", span_json(&unsupported.field.span)),
        (
            "field_path",
            dispatch_payload_unsupported_field_path(unsupported),
        ),
        (
            "message",
            JsonValue::string(format!(
                "Nested dispatch payload field `{}` prevents generated decode and encode helpers: {}.",
                unsupported.field_path_display, layout_fact
            )),
        ),
    ])
}

fn dispatch_payload_unsupported_field_path(
    unsupported: &UnsupportedDispatchPayloadHelperField<'_>,
) -> JsonValue {
    JsonValue::array([
        JsonValue::object([
            ("kind", JsonValue::string("schema")),
            ("name", JsonValue::string(unsupported.schema_name.clone())),
        ]),
        JsonValue::object([
            ("kind", JsonValue::string("field")),
            ("name", JsonValue::string(unsupported.field.name.clone())),
        ]),
    ])
}

fn schema_dispatch_details(
    schema: &SchemaDecl,
    field: &SchemaField,
    reason: &'static str,
) -> Vec<(&'static str, JsonValue)> {
    vec![
        ("phase", JsonValue::string("schema")),
        (
            "node_id",
            JsonValue::string(field.node_id.display("schema-field")),
        ),
        (
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ),
        ("field", JsonValue::string(field.name.clone())),
        ("field_path", schema_dispatch_field_path(schema, field)),
        ("reason", JsonValue::string(reason)),
    ]
}

fn schema_dispatch_field_path(schema: &SchemaDecl, field: &SchemaField) -> JsonValue {
    JsonValue::array([
        JsonValue::object([
            ("kind", JsonValue::string("schema")),
            (
                "name",
                JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
            ),
        ]),
        JsonValue::object([
            ("kind", JsonValue::string("field")),
            ("name", JsonValue::string(field.name.clone())),
        ]),
    ])
}

fn push_schema_type_reference_diagnostics(
    module: &SurfaceModule,
    current_module: Option<&str>,
    annotation: &str,
    node_id: String,
    span: SourceSpan,
    use_kind: &'static str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Ok(ty) = parse_type_annotation(annotation) else {
        return;
    };
    let mut schemas = Vec::new();
    collect_schema_type_references(module, current_module, &ty, &mut schemas);
    for schema in schemas {
        diagnostics.push(schema_type_reference_diagnostic(
            schema,
            node_id.clone(),
            span.clone(),
            use_kind,
        ));
    }
    let mut primitives = Vec::new();
    collect_exact_width_schema_primitive_references(&ty, &mut primitives);
    for primitive in primitives {
        diagnostics.push(exact_width_schema_primitive_diagnostic(
            primitive,
            None,
            None,
            node_id.clone(),
            span.clone(),
            use_kind,
        ));
    }
    let mut lowercase_primitives = Vec::new();
    collect_lowercase_schema_primitive_references(&ty, &mut lowercase_primitives);
    for primitive in lowercase_primitives {
        diagnostics.push(lowercase_schema_primitive_position_diagnostic(
            primitive,
            None,
            None,
            node_id.clone(),
            span.clone(),
            use_kind,
        ));
    }
}

fn collect_schema_type_references<'a>(
    module: &'a SurfaceModule,
    current_module: Option<&str>,
    ty: &Type,
    schemas: &mut Vec<&'a SchemaDecl>,
) {
    match ty {
        Type::Named { name, args } => {
            if let Some(schema) = schema_for_type_name(module, current_module, name) {
                schemas.push(schema);
            }
            for arg in args {
                collect_schema_type_references(module, current_module, arg, schemas);
            }
        }
        Type::Record(fields) => {
            for (_, field_ty) in fields {
                collect_schema_type_references(module, current_module, field_ty, schemas);
            }
        }
        Type::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_schema_type_references(module, current_module, param, schemas);
            }
            collect_schema_type_references(module, current_module, return_type, schemas);
        }
        Type::Unknown => {}
    }
}

fn collect_exact_width_schema_primitive_references<'a>(
    ty: &'a Type,
    primitives: &mut Vec<&'a str>,
) {
    match ty {
        Type::Named { name, args } => {
            if let Some(primitive) = exact_width_binary_primitive_name(name) {
                primitives.push(primitive);
            }
            for arg in args {
                collect_exact_width_schema_primitive_references(arg, primitives);
            }
        }
        Type::Record(fields) => {
            for (_, field_ty) in fields {
                collect_exact_width_schema_primitive_references(field_ty, primitives);
            }
        }
        Type::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_exact_width_schema_primitive_references(param, primitives);
            }
            collect_exact_width_schema_primitive_references(return_type, primitives);
        }
        Type::Unknown => {}
    }
}

fn collect_lowercase_schema_primitive_references<'a>(ty: &'a Type, primitives: &mut Vec<&'a str>) {
    match ty {
        Type::Named { name, args } => {
            if lowercase_schema_primitive(name).is_some() {
                primitives.push(name);
            }
            for arg in args {
                collect_lowercase_schema_primitive_references(arg, primitives);
            }
        }
        Type::Record(fields) => {
            for (_, field_ty) in fields {
                collect_lowercase_schema_primitive_references(field_ty, primitives);
            }
        }
        Type::Function {
            params,
            return_type,
            ..
        } => {
            for param in params {
                collect_lowercase_schema_primitive_references(param, primitives);
            }
            collect_lowercase_schema_primitive_references(return_type, primitives);
        }
        Type::Unknown => {}
    }
}

pub(in crate::analysis) fn lowercase_schema_primitive_diagnostic(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: LowercaseSchemaPrimitiveError,
) -> Diagnostic {
    let reason_text = match reason {
        LowercaseSchemaPrimitiveError::MissingWidth => "missing_width",
        LowercaseSchemaPrimitiveError::UnknownEndian => "unknown_endian",
        LowercaseSchemaPrimitiveError::MissingEndian => "missing_endian",
        LowercaseSchemaPrimitiveError::RedundantEndian => "redundant_endian",
        LowercaseSchemaPrimitiveError::UnsupportedWidth => "unsupported_width",
        LowercaseSchemaPrimitiveError::ReservesValue => "reserves_value",
    };
    let message = match reason {
        LowercaseSchemaPrimitiveError::MissingWidth => {
            format!("binary schema primitive `{primitive}` must specify a width")
        }
        LowercaseSchemaPrimitiveError::UnknownEndian => {
            format!(
                "binary schema primitive `{primitive}` must end with `be` or `le` when it specifies byte order"
            )
        }
        LowercaseSchemaPrimitiveError::MissingEndian => {
            format!("binary schema primitive `{primitive}` requires byte order suffix `be` or `le`")
        }
        LowercaseSchemaPrimitiveError::RedundantEndian => {
            format!("binary schema primitive `{primitive}` must not specify byte order")
        }
        LowercaseSchemaPrimitiveError::UnsupportedWidth => {
            format!("binary schema primitive `{primitive}` uses an unsupported width")
        }
        LowercaseSchemaPrimitiveError::ReservesValue => {
            format!(
                "binary schema primitive `{primitive}` requires `reserves` value to be a literal non-negative integer"
            )
        }
    };
    lowercase_schema_primitive_diagnostic_with_message(
        primitive,
        schema,
        field,
        node_id,
        span,
        reason_text,
        message,
    )
}

pub(in crate::analysis) fn lowercase_schema_primitive_position_diagnostic(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: &'static str,
) -> Diagnostic {
    let message = match reason {
        "repeat_payload" => format!(
            "binary schema primitive `{primitive}` is not yet supported in `Repeat` payload positions"
        ),
        "dispatch_payload" => format!(
            "binary schema primitive `{primitive}` is not yet supported in dispatch payload positions"
        ),
        _ => format!(
            "binary schema primitive `{primitive}` can only be used in a `format binary` schema field"
        ),
    };
    lowercase_schema_primitive_diagnostic_with_message(
        primitive, schema, field, node_id, span, reason, message,
    )
}

fn lowercase_schema_primitive_diagnostic_with_message(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: &'static str,
    message: String,
) -> Diagnostic {
    let mut details = vec![
        ("phase", JsonValue::string("schema")),
        ("node_id", JsonValue::string(node_id)),
        ("primitive", JsonValue::string(primitive.to_string())),
        ("reason", JsonValue::string(reason)),
    ];
    if let Some(schema) = schema {
        details.push((
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ));
    }
    if let Some(field) = field {
        details.push(("field", JsonValue::string(field.name.clone())));
    }
    Diagnostic::new(
        "schema.lowercase_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(span),
        JsonValue::object(details),
    )
}

fn schema_for_type_name<'a>(
    module: &'a SurfaceModule,
    current_module: Option<&str>,
    name: &str,
) -> Option<&'a SchemaDecl> {
    let segments = name.split("::").map(str::to_string).collect::<Vec<_>>();
    match segments.as_slice() {
        [name] => module.schemas.iter().find(|schema| {
            schema.name.as_deref() == Some(name) && schema.module_name.as_deref() == current_module
        }),
        [_, .., name] => {
            let module_name = imported_module_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            module.schemas.iter().find(|schema| {
                schema.name.as_deref() == Some(name)
                    && schema.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
}

fn schema_type_reference_diagnostic(
    schema: &SchemaDecl,
    node_id: String,
    span: SourceSpan,
    use_kind: &'static str,
) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    Diagnostic::new(
        "type.schema_reference",
        Severity::Error,
        DiagnosticKind::Type,
        format!("schema `{schema_name}` cannot be used as an ordinary type"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("type")),
            ("node_id", JsonValue::string(node_id)),
            ("schema", JsonValue::string(schema_name)),
            ("use_kind", JsonValue::string(use_kind)),
        ]),
    )
}

pub(in crate::analysis) fn exact_width_binary_primitive_name(name: &str) -> Option<&'static str> {
    match name {
        "UInt1" => Some("UInt1"),
        "UInt2" => Some("UInt2"),
        "UInt3" => Some("UInt3"),
        "UInt4" => Some("UInt4"),
        "UInt5" => Some("UInt5"),
        "UInt6" => Some("UInt6"),
        "UInt7" => Some("UInt7"),
        "UInt8" => Some("UInt8"),
        "UInt16be" => Some("UInt16be"),
        "UInt16le" => Some("UInt16le"),
        "UInt24be" => Some("UInt24be"),
        "UInt24le" => Some("UInt24le"),
        "UInt31be" => Some("UInt31be"),
        "UInt31le" => Some("UInt31le"),
        "UInt32be" => Some("UInt32be"),
        "UInt32le" => Some("UInt32le"),
        "UInt40be" => Some("UInt40be"),
        "UInt40le" => Some("UInt40le"),
        "UInt48be" => Some("UInt48be"),
        "UInt48le" => Some("UInt48le"),
        "UInt56be" => Some("UInt56be"),
        "UInt56le" => Some("UInt56le"),
        "UInt64be" => Some("UInt64be"),
        "UInt64le" => Some("UInt64le"),
        _ => None,
    }
}

pub(in crate::analysis) fn exact_width_schema_primitive_diagnostic(
    primitive: &str,
    schema: Option<&SchemaDecl>,
    field: Option<&SchemaField>,
    node_id: String,
    span: SourceSpan,
    reason: &'static str,
) -> Diagnostic {
    let mut details = vec![
        ("phase", JsonValue::string("schema")),
        ("node_id", JsonValue::string(node_id)),
        ("primitive", JsonValue::string(primitive.to_string())),
        ("reason", JsonValue::string(reason)),
    ];
    if let Some(schema) = schema {
        details.push((
            "schema",
            JsonValue::string(schema.name.as_deref().unwrap_or("<missing>")),
        ));
    }
    if let Some(field) = field {
        details.push(("field", JsonValue::string(field.name.clone())));
    }
    Diagnostic::new(
        "schema.exact_width_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "binary schema primitive `{primitive}` can only be used in a `format binary` schema field"
        ),
        Some(span),
        JsonValue::object(details),
    )
}

fn reserved_bits_primitive(ty: &str) -> Option<Result<(i64, i64), ReservedBitsArgumentReason>> {
    let rest = ty.strip_prefix("ReservedBits")?;
    if rest
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return Some(Err(ReservedBitsArgumentReason::Arity));
    }
    if !rest.starts_with('(') {
        return None;
    }
    if !rest.ends_with(')') {
        return Some(Err(ReservedBitsArgumentReason::Arity));
    }
    let inner = rest[1..rest.len() - 1].trim();
    let args = inner
        .split(',')
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .collect::<Vec<_>>();
    if args.len() != 2 {
        return Some(Err(ReservedBitsArgumentReason::Arity));
    }
    let Ok(width) = parse_reserved_bits_integer(args[0]) else {
        return Some(Err(ReservedBitsArgumentReason::Literal));
    };
    let Ok(value) = parse_reserved_bits_integer(args[1]) else {
        return Some(Err(ReservedBitsArgumentReason::Literal));
    };
    Some(Ok((width, value)))
}

fn parse_reserved_bits_integer(text: &str) -> Result<i64, ()> {
    parse_integer_literal(text)
        .map(|literal| literal.value)
        .map_err(|_| ())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReservedBitsArgumentReason {
    Arity,
    Literal,
}

fn reserved_bits_format_diagnostic(schema: &SchemaDecl, field: &SchemaField) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    Diagnostic::new(
        "schema.reserved_bits_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        "`ReservedBits` can only be used in a `format binary` schema field",
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            ("schema", JsonValue::string(schema_name)),
            ("field", JsonValue::string(field.name.clone())),
            ("primitive", JsonValue::string("ReservedBits")),
            ("reason", JsonValue::string("non_binary_format")),
        ]),
    )
}

fn reserved_bits_argument_diagnostic(
    schema: &SchemaDecl,
    field: &SchemaField,
    reason: ReservedBitsArgumentReason,
) -> Diagnostic {
    let schema_name = schema.name.as_deref().unwrap_or("<missing>");
    let reason_text = match reason {
        ReservedBitsArgumentReason::Arity => "argument_count",
        ReservedBitsArgumentReason::Literal => "non_literal_argument",
    };
    let message = match reason {
        ReservedBitsArgumentReason::Arity => {
            "`ReservedBits` requires width and value integer arguments"
        }
        ReservedBitsArgumentReason::Literal => {
            "`ReservedBits` arguments must be literal non-negative integers"
        }
    };
    Diagnostic::new(
        "schema.reserved_bits_primitive",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(field.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("schema")),
            (
                "node_id",
                JsonValue::string(field.node_id.display("schema-field")),
            ),
            ("schema", JsonValue::string(schema_name)),
            ("field", JsonValue::string(field.name.clone())),
            ("primitive", JsonValue::string("ReservedBits")),
            ("reason", JsonValue::string(reason_text)),
        ]),
    )
}

fn reserved_bits_encode_shape_diagnostic(
    schema: &SchemaDecl,
    field: &SchemaField,
    field_index: Option<usize>,
    reserved: (i64, i64),
) -> Diagnostic {
    let layout = reserved_bits_unsupported_layout_context(schema, field_index, reserved.0);
    let mut details = vec![
        (
            "schema",
            JsonValue::string(schema.name.clone().unwrap_or_default()),
        ),
        ("field", JsonValue::string(field.name.clone())),
        ("primitive", JsonValue::string("ReservedBits")),
        ("bit_width", JsonValue::Number(reserved.0)),
        ("expected_value", JsonValue::Number(reserved.1)),
        ("reason", JsonValue::string("unsupported_encode_shape")),
        (
            "supported_layout_family",
            JsonValue::string(layout.supported_layout_family),
        ),
    ];
    if let Some(previous_width) = layout.previous_visible_bit_width {
        details.push((
            "previous_visible_bit_width",
            JsonValue::Number(i64::from(previous_width)),
        ));
    }
    if let Some(next_width) = layout.next_visible_bit_width {
        details.push((
            "next_visible_bit_width",
            JsonValue::Number(i64::from(next_width)),
        ));
    }

    let mut diagnostic = Diagnostic::new(
        "schema.reserved_bits_encode",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "`ReservedBits({}, {})` is outside the supported binary schema field layouts",
            reserved.0, reserved.1
        ),
        Some(field.span.clone()),
        JsonValue::object(details),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(format!(
            "Schema `{}` field `{}` declares ReservedBits({}, {}).",
            schema.name.clone().unwrap_or_default(),
            field.name,
            reserved.0,
            reserved.1
        )),
    )]));
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(layout.human_adjacent_note),
    )]));
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(format!(
            "Supported layout family: {}; {}",
            layout.supported_layout_family, layout.human_supported_note
        )),
    )]));
    diagnostic
}

struct ReservedBitsUnsupportedLayoutContext {
    supported_layout_family: &'static str,
    previous_visible_bit_width: Option<u8>,
    next_visible_bit_width: Option<u8>,
    human_adjacent_note: String,
    human_supported_note: &'static str,
}

fn reserved_bits_unsupported_layout_context(
    schema: &SchemaDecl,
    field_index: Option<usize>,
    bit_width: i64,
) -> ReservedBitsUnsupportedLayoutContext {
    let previous_field = field_index
        .and_then(|index| index.checked_sub(1))
        .and_then(|index| schema.fields.get(index));
    let previous_previous_field = field_index
        .and_then(|index| index.checked_sub(2))
        .and_then(|index| schema.fields.get(index));
    let next_field = field_index.and_then(|index| schema.fields.get(index + 1));
    let previous_previous_visible_bit_width =
        previous_previous_field.and_then(|field| exact_width_schema_primitive_bit_width(&field.ty));
    let previous_visible_bit_width =
        previous_field.and_then(|field| exact_width_schema_primitive_bit_width(&field.ty));
    let next_visible_bit_width =
        next_field.and_then(|field| exact_width_schema_primitive_bit_width(&field.ty));

    let (supported_layout_family, human_supported_note) = reserved_bits_supported_layout_family(
        bit_width,
        previous_previous_visible_bit_width,
        previous_visible_bit_width,
        next_visible_bit_width,
    );
    ReservedBitsUnsupportedLayoutContext {
        supported_layout_family,
        previous_visible_bit_width,
        next_visible_bit_width,
        human_adjacent_note: reserved_bits_adjacent_width_note(
            previous_field,
            previous_visible_bit_width,
            next_field,
            next_visible_bit_width,
        ),
        human_supported_note,
    }
}

fn reserved_bits_supported_layout_family(
    bit_width: i64,
    previous_previous_visible_bit_width: Option<u8>,
    previous_visible_bit_width: Option<u8>,
    next_visible_bit_width: Option<u8>,
) -> (&'static str, &'static str) {
    if previous_visible_bit_width.is_some() && next_visible_bit_width.is_some() {
        return (
            "middle_reserved_bits",
            "visible and reserved widths must complete one supported big-endian storage unit.",
        );
    }
    if previous_visible_bit_width.is_some()
        && suffix_packed_reserved_storage_bit_width(bit_width).is_some()
    {
        if previous_visible_bit_width == Some(8)
            && previous_previous_visible_bit_width
                .is_some_and(|width| i64::from(width) + 8 + bit_width == 16)
            && (1..=7).contains(&bit_width)
        {
            return (
                "suffix_reserved_group",
                "two visible widths plus the reserved width must complete the same two-byte big-endian storage unit.",
            );
        }
        return (
            "packed_reserved_suffix",
            "the previous visible width plus the reserved width must complete one supported big-endian storage unit.",
        );
    }
    if next_visible_bit_width.is_some() && packed_reserved_storage_bit_width(bit_width).is_some() {
        return (
            "packed_reserved_prefix",
            "the reserved width plus the next visible width must complete one supported big-endian storage unit.",
        );
    }
    if bit_width > 0 && bit_width <= 32 && bit_width % 8 == 0 {
        return (
            "byte_aligned_reserved_bits",
            "byte-aligned reserved fields are supported up to four bytes when the value fits the width.",
        );
    }
    (
        "bit_packed_reserved_group",
        "a bit-packed group must contain at least one visible field and complete one supported big-endian storage unit.",
    )
}

fn packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    if (1..=7).contains(&bit_width) {
        Some(8)
    } else if (9..=15).contains(&bit_width) {
        Some(16)
    } else if (17..=23).contains(&bit_width) {
        Some(24)
    } else if (25..=31).contains(&bit_width) {
        Some(32)
    } else {
        None
    }
}

fn suffix_packed_reserved_storage_bit_width(bit_width: i64) -> Option<i64> {
    packed_reserved_storage_bit_width(bit_width).or_else(|| {
        if (33..=39).contains(&bit_width) {
            Some(40)
        } else if (41..=47).contains(&bit_width) {
            Some(48)
        } else {
            None
        }
    })
}

fn reserved_bits_adjacent_width_note(
    previous_field: Option<&SchemaField>,
    previous_visible_bit_width: Option<u8>,
    next_field: Option<&SchemaField>,
    next_visible_bit_width: Option<u8>,
) -> String {
    let mut parts = Vec::new();
    if let (Some(field), Some(width)) = (previous_field, previous_visible_bit_width) {
        parts.push(format!("previous `{}` is {} bit(s)", field.name, width));
    }
    if let (Some(field), Some(width)) = (next_field, next_visible_bit_width) {
        parts.push(format!("next `{}` is {} bit(s)", field.name, width));
    }
    if parts.is_empty() {
        "No adjacent visible exact-width field participates in this unsupported layout.".to_string()
    } else {
        format!("Adjacent visible field widths: {}.", parts.join("; "))
    }
}

fn function_target<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a veln_ast::Function> {
    match segments {
        [name] => module.functions.iter().find(|function| {
            function.kind == FunctionKind::Function && function.name.as_deref() == Some(name)
        }),
        [_, .., name] => {
            let module_name = imported_module_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            module.functions.iter().find(|function| {
                function.kind == FunctionKind::Function
                    && function.name.as_deref() == Some(name)
                    && function.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
}

fn type_target<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a veln_ast::TypeDecl> {
    match segments {
        [name] => module
            .types
            .iter()
            .find(|type_decl| type_decl.name.as_deref() == Some(name)),
        [_, .., name] => {
            let module_name = imported_module_for_path(
                &module.uses,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            module.types.iter().find(|type_decl| {
                type_decl.name.as_deref() == Some(name)
                    && type_decl.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
}

fn imported_module_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a str> {
    imported_use_for_path(uses, segments, current_module).map(|use_decl| use_decl.name.as_str())
}

fn imported_use_for_path<'a>(
    uses: &'a [UseDecl],
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    uses.iter().find(|use_decl| {
        use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
}

fn unresolved_alias_diagnostic(
    alias: &veln_ast::PublicAlias,
    expected_kind: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "name.unresolved",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "unresolved {expected_kind} alias target `{}`",
            alias.target.join("::")
        ),
        Some(alias.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(alias.node_id.display("alias"))),
            ("expected_kind", JsonValue::string(expected_kind)),
            ("target", JsonValue::string(alias.target.join("::"))),
        ]),
    )
}

fn private_alias_diagnostic(alias: &veln_ast::PublicAlias) -> Diagnostic {
    Diagnostic::new(
        "name.visibility",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "schema alias target `{}` is private",
            alias.target.join("::")
        ),
        Some(alias.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(alias.node_id.display("alias"))),
            ("expected_kind", JsonValue::string("schema")),
            ("target", JsonValue::string(alias.target.join("::"))),
            ("reason", JsonValue::string("private")),
        ]),
    )
}

fn alias_kind_mismatch_diagnostic(
    alias: &veln_ast::PublicAlias,
    expected_kind: &'static str,
    actual_kind: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "name.kind_mismatch",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "public alias target `{}` is a {actual_kind}, not a {expected_kind}",
            alias.target.join("::")
        ),
        Some(alias.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(alias.node_id.display("alias"))),
            ("expected_kind", JsonValue::string(expected_kind)),
            ("actual_kind", JsonValue::string(actual_kind)),
            ("target", JsonValue::string(alias.target.join("::"))),
        ]),
    )
}

pub(crate) fn check_duplicate_use_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<(Option<String>, String), (String, SourceSpan)>::new();

    for use_decl in module
        .uses
        .iter()
        .filter(|use_decl| use_decl.origin == veln_ast::UseOrigin::Source)
    {
        let node_id = use_decl.node_id.display("use");
        let key = (use_decl.module_name.clone(), use_decl.alias.clone());
        if let Some((first_node_id, first_span)) = seen.get(&key) {
            diagnostics.push(duplicate_name_diagnostic(
                &use_decl.alias,
                "module",
                "import alias",
                node_id,
                use_decl.span.clone(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            seen.insert(key, (node_id, use_decl.span.clone()));
        }
    }

    diagnostics
}

pub(crate) fn check_reserved_prelude_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if let Some(header) = &module.module
        && header.name == PRELUDE_MODULE
        && !is_toolchain_standard_prelude(&header.span)
    {
        diagnostics.push(reserved_prelude_diagnostic(
            header.node_id.display("mod"),
            header.span.clone(),
            "module",
            "module identity",
            "Choose a non-conflicting module name.",
        ));
    }

    for use_decl in module
        .uses
        .iter()
        .filter(|use_decl| use_decl.origin == veln_ast::UseOrigin::Source)
    {
        if use_decl.alias == PRELUDE_MODULE
            && !use_decl
                .module_name
                .as_deref()
                .is_some_and(|module_name| module_name.starts_with("std::"))
        {
            diagnostics.push(reserved_prelude_diagnostic(
                use_decl.node_id.display("use"),
                use_decl.span.clone(),
                "module",
                "import alias",
                "Choose a non-conflicting import path.",
            ));
        }
    }

    diagnostics
}

fn is_toolchain_standard_prelude(span: &SourceSpan) -> bool {
    span.file.as_str() == "prelude.veln"
}

fn reserved_prelude_diagnostic(
    node_id: String,
    span: SourceSpan,
    namespace: &'static str,
    declaration_kind: &'static str,
    hint: &'static str,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "name.reserved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("{declaration_kind} `prelude` conflicts with the standard prelude"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(node_id)),
            ("name", JsonValue::string(PRELUDE_MODULE)),
            ("namespace", JsonValue::string(namespace)),
            ("reserved_for", JsonValue::string("standard_prelude")),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        ("message", JsonValue::string(hint)),
    ]));
    diagnostic
}

pub(crate) fn check_duplicate_constructor_names(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen =
        BTreeMap::<(Option<String>, Option<String>, String), (String, SourceSpan)>::new();

    for type_decl in &module.types {
        for variant in &type_decl.variants {
            let Some(name) = &variant.name else {
                continue;
            };
            let key = (
                type_decl.module_name.clone(),
                type_decl.name.clone(),
                name.clone(),
            );
            let node_id = variant.node_id.display("variant");
            if let Some((first_node_id, first_span)) = seen.get(&key) {
                diagnostics.push(duplicate_name_diagnostic(
                    name,
                    "constructor",
                    "constructor declaration",
                    node_id,
                    variant.span.clone(),
                    first_node_id.clone(),
                    first_span,
                ));
            } else {
                seen.insert(key, (node_id, variant.span.clone()));
            }
        }
    }

    diagnostics
}

pub(crate) fn check_module_boundary(module: &SurfaceModule) -> Vec<Diagnostic> {
    if module.module.is_some() || module.uses.is_empty() {
        return Vec::new();
    }

    let first_use = &module.uses[0];
    let mut diagnostic = Diagnostic::new(
        "module.missing_identity",
        Severity::Error,
        DiagnosticKind::Module,
        "module import requires a module identity",
        Some(first_use.span.clone()),
        module_details(
            first_use.node_id.display("use"),
            "module_identity",
            "source",
            "missing",
        ),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string("Add a `mod` declaration before `use` declarations."),
        ),
    ]));
    vec![diagnostic]
}

pub(super) fn duplicate_name_diagnostic(
    name: &str,
    namespace: &'static str,
    declaration_kind: &'static str,
    node_id: String,
    span: SourceSpan,
    first_node_id: String,
    first_span: &SourceSpan,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "name.duplicate",
        Severity::Error,
        DiagnosticKind::Name,
        format!("duplicate {declaration_kind} name `{name}`"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::string(node_id)),
            ("name", JsonValue::string(name)),
            ("namespace", JsonValue::string(namespace)),
            ("first_node_id", JsonValue::string(first_node_id)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("duplicate_origin")),
        (
            "message",
            JsonValue::string(format!("First {declaration_kind} with this name is here.")),
        ),
        ("span", span_json(first_span)),
    ]));
    diagnostic
}

pub(crate) fn check_test_declaration_boundary(function: &Function) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let node_id = function.node_id.display(function.kind.node_prefix());

    if let Some(param) = function.params.first() {
        let mut diagnostic = Diagnostic::new(
            "test.parameters",
            Severity::Error,
            DiagnosticKind::Type,
            "test declaration has parameters",
            Some(param.span.clone()),
            JsonValue::object([
                ("phase", JsonValue::string("test")),
                ("node_id", JsonValue::string(node_id.clone())),
                ("expected_parameters", JsonValue::Number(0)),
                (
                    "actual_parameters",
                    JsonValue::Number(function.params.len() as i64),
                ),
            ]),
        );
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("test_shape")),
            (
                "message",
                JsonValue::string("A test declaration uses an empty parameter list."),
            ),
            ("span", span_json(&function.span)),
        ]));
        diagnostics.push(diagnostic);
    }

    match function.return_type.as_deref() {
        Some(return_type) => {
            if let Ok(ty) = parse_type_annotation(return_type)
                && !is_allowed_test_return(&ty)
            {
                diagnostics.push(test_return_diagnostic(
                    function,
                    &node_id,
                    format!("test declaration returns `{}`", ty.render()),
                    ty.render(),
                ));
            }
        }
        None => diagnostics.push(test_return_diagnostic(
            function,
            &node_id,
            "test declaration has no return type annotation".to_string(),
            "missing".to_string(),
        )),
    }

    diagnostics
}

fn is_allowed_test_return(ty: &Type) -> bool {
    ty == &Type::unit() || adt::result_parts(ty).is_some_and(|(value, _)| value == &Type::unit())
}

pub(super) fn type_contains_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } => args.iter().any(type_contains_unknown),
        Type::Record(fields) => fields.iter().any(|(_, ty)| type_contains_unknown(ty)),
        Type::Function {
            params,
            return_type,
            ..
        } => params.iter().any(type_contains_unknown) || type_contains_unknown(return_type),
    }
}

fn test_return_diagnostic(
    function: &Function,
    node_id: &str,
    message: String,
    actual_type: String,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "test.return_type",
        Severity::Error,
        DiagnosticKind::Type,
        message,
        Some(function.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("test")),
            ("node_id", JsonValue::string(node_id)),
            ("expected_type", JsonValue::string("() or Result<(), E>")),
            ("actual_type", JsonValue::string(actual_type)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("test_shape")),
        (
            "message",
            JsonValue::string("A test declaration returns `()` or `Result<(), E>`."),
        ),
        ("span", span_json(&function.span)),
    ]));
    diagnostic
}
