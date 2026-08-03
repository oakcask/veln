use super::*;
use crate::types::{
    EffectOperationSignature, EffectSignature, FunctionSignature, HandlerSignature,
    UserEffectPathResolution,
};
use std::collections::BTreeMap;
use veln_ast::{HandlerDecl, HandlerProviderDecl};

struct ResolvedProvider<'a> {
    handler: &'a HandlerDecl,
    provider: &'a HandlerProviderDecl,
    operation_name: &'a str,
    signature: &'a HandlerSignature,
    effect: &'a EffectSignature,
    operation: &'a EffectOperationSignature,
    function: &'a FunctionSignature,
}

pub(crate) fn check_handler_declarations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    module
        .handlers
        .iter()
        .flat_map(|handler| check_handler_declaration(handler, environment))
        .collect()
}

fn check_handler_declaration(
    handler: &HandlerDecl,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let Some(signature) = handler.name.as_ref().and_then(|name| {
        environment.handler_path(std::slice::from_ref(name), handler.module_name.as_deref())
    }) else {
        return Vec::new();
    };
    let effect = match environment
        .resolve_user_effect_path(&handler.effect, handler.module_name.as_deref())
    {
        UserEffectPathResolution::Found(effect) => effect,
        UserEffectPathResolution::PrivateCompanionTargetMismatch { effect, access } => {
            return vec![private_companion_effect_target_diagnostic(
                handler.node_id.display("handler"),
                "handler_declaration",
                &handler.effect.join("::"),
                effect,
                access,
                handler.effect_span.clone(),
            )];
        }
        UserEffectPathResolution::Missing => {
            return vec![unknown_effect_diagnostic(handler, environment)];
        }
    };

    let mut diagnostics = declared_effect_diagnostics(handler, environment);
    diagnostics.extend(duplicate_provider_diagnostics(handler, signature, effect));
    diagnostics.extend(missing_provider_diagnostics(handler, signature, effect));
    for provider in &handler.providers {
        diagnostics.extend(check_provider(
            handler,
            provider,
            signature,
            effect,
            environment,
        ));
    }
    diagnostics
}

fn declared_effect_diagnostics(
    handler: &HandlerDecl,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let Some(declared_effects) = &handler.effects else {
        return Vec::new();
    };
    declared_effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| {
            !effect.starts_with("...")
                && !KNOWN_EFFECT_LABELS.contains(&effect.as_str())
                && !matches!(
                    environment.resolve_user_effect_path(
                        &effect.split("::").map(str::to_string).collect::<Vec<_>>(),
                        handler.module_name.as_deref()
                    ),
                    UserEffectPathResolution::Found(_)
                )
        })
        .map(|(index, effect)| {
            let segments = effect.split("::").map(str::to_string).collect::<Vec<_>>();
            match environment.resolve_user_effect_path(&segments, handler.module_name.as_deref()) {
                UserEffectPathResolution::PrivateCompanionTargetMismatch {
                    effect: signature,
                    access,
                } => private_companion_effect_target_diagnostic(
                    handler.node_id.display("handler"),
                    "handler_declaration_effects",
                    effect,
                    signature,
                    access,
                    handler
                        .effect_spans
                        .as_ref()
                        .and_then(|spans| spans.get(index))
                        .cloned()
                        .unwrap_or_else(|| handler.span.clone()),
                ),
                UserEffectPathResolution::Found(_) | UserEffectPathResolution::Missing => {
                    unknown_declared_effect_diagnostic(handler, effect, index)
                }
            }
        })
        .collect()
}

fn unknown_declared_effect_diagnostic(
    handler: &HandlerDecl,
    effect: &str,
    effect_index: usize,
) -> Diagnostic {
    let declared_effects = handler
        .effects
        .as_ref()
        .expect("unknown effect diagnostics require a declared effects clause");
    let mut diagnostic = Diagnostic::new(
        "effect.unknown",
        Severity::Error,
        DiagnosticKind::Effect,
        format!("declared effect `{effect}` is not known"),
        handler
            .effect_spans
            .as_ref()
            .and_then(|spans| spans.get(effect_index))
            .cloned()
            .or_else(|| Some(handler.span.clone())),
        JsonValue::object([
            ("phase", JsonValue::string("effect")),
            (
                "node_id",
                JsonValue::string(handler.node_id.display("handler")),
            ),
            ("effect", JsonValue::string(effect.to_string())),
            ("boundary", JsonValue::string("handler_declaration_effects")),
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

fn unknown_effect_diagnostic(handler: &HandlerDecl, environment: &TypeEnvironment) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "handler.effect_unknown",
        Severity::Error,
        DiagnosticKind::Effect,
        format!(
            "handled effect `{}` is not known",
            handler.effect.join("::")
        ),
        Some(handler.effect_span.clone()),
        handler_details(
            handler.node_id.display("handler"),
            "handler_declaration",
            handler.name.clone().unwrap_or_default(),
            handler.effect.join("::"),
            None,
            None,
            "unknown_handled_effect",
        ),
    );
    for candidate in environment.visible_user_effects(handler.module_name.as_deref()) {
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("effect_declaration")),
            (
                "message",
                JsonValue::string(format!(
                    "Candidate effect `{}` is declared here.",
                    candidate.qualified_name
                )),
            ),
            (
                "effect",
                JsonValue::string(candidate.qualified_name.clone()),
            ),
            (
                "operations",
                JsonValue::array(
                    candidate
                        .operations
                        .iter()
                        .map(|operation| JsonValue::string(operation.name.clone())),
                ),
            ),
            ("span", span_json(&candidate.span)),
        ]));
    }
    diagnostic
}

fn duplicate_provider_diagnostics(
    handler: &HandlerDecl,
    signature: &HandlerSignature,
    effect: &EffectSignature,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut provided_operations = BTreeMap::<String, SourceSpan>::new();
    for provider in &handler.providers {
        let Some(operation_name) = &provider.operation else {
            continue;
        };
        if let Some(first_span) = provided_operations.get(operation_name) {
            let mut diagnostic = Diagnostic::new(
                "handler.duplicate_provider",
                Severity::Error,
                DiagnosticKind::Effect,
                format!(
                    "handler `{}` provides operation `{operation_name}` more than once",
                    signature.qualified_name
                ),
                Some(provider.operation_span.clone()),
                handler_details(
                    provider.node_id.display("provider"),
                    "handler_provider",
                    signature.qualified_name.clone(),
                    effect.qualified_name.clone(),
                    Some(operation_name),
                    Some(&provider.provider),
                    "duplicate_provider",
                ),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("handler_provider")),
                (
                    "message",
                    JsonValue::string(format!(
                        "The first provider for operation `{operation_name}` is here."
                    )),
                ),
                ("span", span_json(first_span)),
            ]));
            diagnostics.push(diagnostic);
        } else {
            provided_operations.insert(operation_name.clone(), provider.operation_span.clone());
        }
    }
    diagnostics
}

fn missing_provider_diagnostics(
    handler: &HandlerDecl,
    signature: &HandlerSignature,
    effect: &EffectSignature,
) -> Vec<Diagnostic> {
    effect
        .operations
        .iter()
        .filter(|operation| {
            !signature
                .providers
                .iter()
                .any(|provider| provider.operation == operation.name)
        })
        .map(|operation| {
            let mut diagnostic = Diagnostic::new(
                "handler.missing_provider",
                Severity::Error,
                DiagnosticKind::Effect,
                format!(
                    "handler `{}` does not provide operation `{}`",
                    signature.qualified_name, operation.name
                ),
                Some(handler.span.clone()),
                handler_details(
                    handler.node_id.display("handler"),
                    "handler_declaration",
                    signature.qualified_name.clone(),
                    effect.qualified_name.clone(),
                    Some(&operation.name),
                    None,
                    "missing_provider",
                ),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("effect_operation")),
                (
                    "message",
                    JsonValue::string(format!("Operation `{}` is declared here.", operation.name)),
                ),
                ("span", span_json(&operation.name_span)),
            ]));
            diagnostic
        })
        .collect()
}

fn check_provider(
    handler: &HandlerDecl,
    provider: &HandlerProviderDecl,
    signature: &HandlerSignature,
    effect: &EffectSignature,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let Some(operation_name) = &provider.operation else {
        return Vec::new();
    };
    let Some(operation) = effect
        .operations
        .iter()
        .find(|operation| operation.name == *operation_name)
    else {
        return vec![unknown_operation_diagnostic(
            provider,
            operation_name,
            signature,
            effect,
        )];
    };
    let Some(function) =
        environment.function_path(&provider.provider, handler.module_name.as_deref())
    else {
        return vec![unknown_provider_diagnostic(
            provider,
            operation_name,
            signature,
            effect,
        )];
    };

    let resolved = ResolvedProvider {
        handler,
        provider,
        operation_name,
        signature,
        effect,
        operation,
        function,
    };
    let mut diagnostics = Vec::new();
    if let Some(diagnostic) = provider_signature_diagnostic(&resolved) {
        diagnostics.push(diagnostic);
    }
    if function
        .effects
        .iter()
        .any(|effect_name| effect_name == &signature.effect)
    {
        diagnostics.push(recursive_provider_diagnostic(&resolved));
    }
    diagnostics.extend(missing_public_effect_diagnostics(&resolved));
    diagnostics
}

fn unknown_operation_diagnostic(
    provider: &HandlerProviderDecl,
    operation_name: &str,
    signature: &HandlerSignature,
    effect: &EffectSignature,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "handler.unknown_operation",
        Severity::Error,
        DiagnosticKind::Effect,
        format!(
            "handled effect `{}` has no operation `{operation_name}`",
            effect.qualified_name
        ),
        Some(provider.operation_span.clone()),
        handler_details(
            provider.node_id.display("provider"),
            "handler_provider",
            signature.qualified_name.clone(),
            effect.qualified_name.clone(),
            Some(operation_name),
            Some(&provider.provider),
            "unknown_operation",
        ),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("effect_declaration")),
        (
            "message",
            JsonValue::string(format!(
                "Effect `{}` is declared here.",
                effect.qualified_name
            )),
        ),
        ("span", span_json(&effect.span)),
    ]));
    diagnostic
}

fn unknown_provider_diagnostic(
    provider: &HandlerProviderDecl,
    operation_name: &str,
    signature: &HandlerSignature,
    effect: &EffectSignature,
) -> Diagnostic {
    Diagnostic::new(
        "handler.provider_unknown",
        Severity::Error,
        DiagnosticKind::Effect,
        format!("provider `{}` is not known", provider.provider.join("::")),
        Some(provider.provider_span.clone()),
        handler_details(
            provider.node_id.display("provider"),
            "handler_provider",
            signature.qualified_name.clone(),
            effect.qualified_name.clone(),
            Some(operation_name),
            Some(&provider.provider),
            "unknown_provider",
        ),
    )
}

fn provider_signature_diagnostic(resolved: &ResolvedProvider<'_>) -> Option<Diagnostic> {
    let mut expected_params = resolved.signature.params.clone();
    expected_params.extend(resolved.operation.params.clone());
    if resolved.function.params == expected_params
        && resolved.function.return_type == resolved.operation.return_type
    {
        return None;
    }

    let mut diagnostic = Diagnostic::new(
        "handler.provider_signature",
        Severity::Error,
        DiagnosticKind::Type,
        format!(
            "provider `{}` does not match operation `{}`",
            resolved.provider.provider.join("::"),
            resolved.operation_name
        ),
        Some(resolved.provider.provider_span.clone()),
        provider_signature_details(resolved, &expected_params),
    );
    diagnostic.related.push(provider_function_note(resolved));
    diagnostic.related.push(effect_operation_note(resolved));
    diagnostic.related.push(handler_context_note(resolved));
    Some(diagnostic)
}

fn provider_signature_details(
    resolved: &ResolvedProvider<'_>,
    expected_params: &[Type],
) -> JsonValue {
    JsonValue::object([
        ("phase", JsonValue::string("type")),
        (
            "node_id",
            JsonValue::string(resolved.provider.node_id.display("provider")),
        ),
        ("boundary", JsonValue::string("handler_provider")),
        (
            "handler",
            JsonValue::string(resolved.signature.qualified_name.clone()),
        ),
        (
            "handled_effect",
            JsonValue::string(resolved.effect.qualified_name.clone()),
        ),
        (
            "operation",
            JsonValue::string(resolved.operation_name.to_string()),
        ),
        (
            "provider",
            JsonValue::string(resolved.provider.provider.join("::")),
        ),
        ("reason", JsonValue::string("provider_signature")),
        ("context_params", debug_types(&resolved.signature.params)),
        ("operation_params", debug_types(&resolved.operation.params)),
        ("expected_params", debug_types(expected_params)),
        ("actual_params", debug_types(&resolved.function.params)),
        (
            "expected_return_type",
            JsonValue::string(format!("{:?}", resolved.operation.return_type)),
        ),
        (
            "actual_return_type",
            JsonValue::string(format!("{:?}", resolved.function.return_type)),
        ),
    ])
}

fn provider_function_note(resolved: &ResolvedProvider<'_>) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("provider_function")),
        (
            "message",
            JsonValue::string(format!(
                "Provider `{}` has this function signature.",
                resolved.provider.provider.join("::")
            )),
        ),
        ("span", span_json(&resolved.function.span)),
        ("params", debug_types(&resolved.function.params)),
        (
            "return_type",
            JsonValue::string(format!("{:?}", resolved.function.return_type)),
        ),
    ])
}

fn effect_operation_note(resolved: &ResolvedProvider<'_>) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("effect_operation")),
        (
            "message",
            JsonValue::string(format!(
                "Operation `{}` declares the required provider suffix.",
                resolved.operation_name
            )),
        ),
        ("span", span_json(&resolved.operation.name_span)),
        ("params", debug_types(&resolved.operation.params)),
        (
            "return_type",
            JsonValue::string(format!("{:?}", resolved.operation.return_type)),
        ),
    ])
}

fn handler_context_note(resolved: &ResolvedProvider<'_>) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("handler_declaration")),
        (
            "message",
            JsonValue::string(format!(
                "Handler `{}` contributes context parameters before operation parameters.",
                resolved.signature.qualified_name
            )),
        ),
        ("span", span_json(&resolved.handler.span)),
        ("context_params", debug_types(&resolved.signature.params)),
    ])
}

fn recursive_provider_diagnostic(resolved: &ResolvedProvider<'_>) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "handler.recursive_provider",
        Severity::Error,
        DiagnosticKind::Effect,
        format!(
            "provider `{}` performs handled effect `{}`",
            resolved.provider.provider.join("::"),
            resolved.signature.effect
        ),
        Some(resolved.provider.provider_span.clone()),
        handler_details(
            resolved.provider.node_id.display("provider"),
            "handler_provider",
            resolved.signature.qualified_name.clone(),
            resolved.effect.qualified_name.clone(),
            Some(resolved.operation_name),
            Some(&resolved.provider.provider),
            "recursive_provider",
        ),
    );
    diagnostic
        .related
        .push(recursive_provider_function_note(resolved));
    diagnostic
        .related
        .push(recursive_handler_context_note(resolved));
    diagnostic
}

fn recursive_provider_function_note(resolved: &ResolvedProvider<'_>) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("provider_function")),
        (
            "message",
            JsonValue::string(format!(
                "Provider `{}` retains `{}`.",
                resolved.provider.provider.join("::"),
                resolved.signature.effect
            )),
        ),
        (
            "provider",
            JsonValue::string(resolved.provider.provider.join("::")),
        ),
        (
            "effects",
            JsonValue::array(
                resolved
                    .function
                    .effects
                    .iter()
                    .cloned()
                    .map(JsonValue::string),
            ),
        ),
        ("span", span_json(&resolved.function.span)),
    ])
}

fn recursive_handler_context_note(resolved: &ResolvedProvider<'_>) -> JsonValue {
    JsonValue::object([
        ("kind", JsonValue::string("handler_declaration")),
        (
            "message",
            JsonValue::string(format!(
                "Handler `{}` handles `{}`.",
                resolved.signature.qualified_name, resolved.effect.qualified_name
            )),
        ),
        (
            "handler",
            JsonValue::string(resolved.signature.qualified_name.clone()),
        ),
        (
            "handled_effect",
            JsonValue::string(resolved.effect.qualified_name.clone()),
        ),
        ("span", span_json(&resolved.handler.effect_span)),
    ])
}

fn missing_public_effect_diagnostics(resolved: &ResolvedProvider<'_>) -> Vec<Diagnostic> {
    if resolved.handler.visibility != Visibility::Public {
        return Vec::new();
    }
    resolved
        .function
        .effects
        .iter()
        .filter(|effect_name| {
            !resolved
                .signature
                .effects
                .iter()
                .any(|declared| declared == *effect_name)
        })
        .map(|effect_name| {
            Diagnostic::new(
                "handler.missing_public_effect",
                Severity::Error,
                DiagnosticKind::Effect,
                format!(
                    "public handler `{}` uses undeclared effect `{effect_name}`",
                    resolved.signature.qualified_name
                ),
                Some(resolved.handler.span.clone()),
                handler_details(
                    resolved.handler.node_id.display("handler"),
                    "handler_declaration",
                    resolved.signature.qualified_name.clone(),
                    resolved.effect.qualified_name.clone(),
                    Some(resolved.operation_name),
                    Some(&resolved.provider.provider),
                    "missing_public_effect",
                ),
            )
        })
        .collect()
}

fn debug_types(types: &[Type]) -> JsonValue {
    JsonValue::array(types.iter().map(|ty| JsonValue::string(format!("{ty:?}"))))
}
