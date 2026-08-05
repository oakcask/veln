use super::*;
use crate::types::{
    EffectOperationSignature, EffectSignature, HandlerSignature, UserEffectPathResolution,
    synthetic_handler_clause_function_name,
};
use std::collections::{BTreeMap, BTreeSet};
use veln_ast::{HandlerDecl, HandlerOperationClauseDecl};

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
        match environment.handler_path(std::slice::from_ref(name), handler.module_name.as_deref()) {
            HandlerPathResolution::Found(signature) => Some(signature),
            HandlerPathResolution::PrivateCompanionTargetMismatch { .. }
            | HandlerPathResolution::Missing => None,
        }
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
    diagnostics.extend(duplicate_clause_diagnostics(handler, signature, effect));
    diagnostics.extend(missing_clause_diagnostics(handler, signature, effect));
    for clause in &handler.operation_clauses {
        diagnostics.extend(check_clause(
            handler,
            clause,
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
        ("span", span_json(&handler.span)),
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

fn duplicate_clause_diagnostics(
    handler: &HandlerDecl,
    signature: &HandlerSignature,
    effect: &EffectSignature,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::<String, SourceSpan>::new();
    for clause in &handler.operation_clauses {
        let Some(operation_name) = &clause.operation else {
            continue;
        };
        if let Some(first_span) = seen.get(operation_name) {
            let mut diagnostic = Diagnostic::new(
                "handler.duplicate_operation_clause",
                Severity::Error,
                DiagnosticKind::Effect,
                format!(
                    "handler `{}` declares operation `{operation_name}` more than once",
                    signature.qualified_name
                ),
                Some(clause.operation_span.clone()),
                clause_details(
                    clause.node_id.display("clause"),
                    signature,
                    effect,
                    Some(operation_name),
                    "duplicate_operation_clause",
                ),
            );
            diagnostic.related.push(JsonValue::object([
                ("kind", JsonValue::string("handler_operation_clause")),
                (
                    "message",
                    JsonValue::string(format!(
                        "The first clause for operation `{operation_name}` is here."
                    )),
                ),
                ("span", span_json(first_span)),
            ]));
            diagnostics.push(diagnostic);
        } else {
            seen.insert(operation_name.clone(), clause.operation_span.clone());
        }
    }
    diagnostics
}

fn missing_clause_diagnostics(
    handler: &HandlerDecl,
    signature: &HandlerSignature,
    effect: &EffectSignature,
) -> Vec<Diagnostic> {
    effect
        .operations
        .iter()
        .filter(|operation| {
            !signature
                .operation_clauses
                .iter()
                .any(|clause| clause.operation == operation.name)
        })
        .map(|operation| {
            let mut diagnostic = Diagnostic::new(
                "handler.missing_operation_clause",
                Severity::Error,
                DiagnosticKind::Effect,
                format!(
                    "handler `{}` does not declare operation `{}`",
                    signature.qualified_name, operation.name
                ),
                Some(handler.span.clone()),
                clause_details(
                    handler.node_id.display("handler"),
                    signature,
                    effect,
                    Some(&operation.name),
                    "missing_operation_clause",
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

fn check_clause(
    handler: &HandlerDecl,
    clause: &HandlerOperationClauseDecl,
    signature: &HandlerSignature,
    effect: &EffectSignature,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let Some(operation_name) = &clause.operation else {
        return Vec::new();
    };
    let Some(operation) = effect
        .operations
        .iter()
        .find(|operation| operation.name == *operation_name)
    else {
        return vec![unknown_operation_diagnostic(
            clause,
            operation_name,
            signature,
            effect,
        )];
    };

    let mut diagnostics = clause_binding_diagnostics(clause, operation, signature, effect);
    diagnostics.extend(clause_body_diagnostics(
        handler,
        clause,
        operation,
        signature,
        effect,
        environment,
    ));
    diagnostics
}

fn clause_binding_diagnostics(
    clause: &HandlerOperationClauseDecl,
    operation: &EffectOperationSignature,
    signature: &HandlerSignature,
    effect: &EffectSignature,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    if clause.params.len() != operation.params.len() {
        diagnostics.push(Diagnostic::new(
            "handler.operation_clause_arity",
            Severity::Error,
            DiagnosticKind::Type,
            format!(
                "operation `{}` expects {} parameter bindings, but clause declares {}",
                operation.name,
                operation.params.len(),
                clause.params.len()
            ),
            Some(clause.span.clone()),
            clause_details(
                clause.node_id.display("clause"),
                signature,
                effect,
                Some(&operation.name),
                "operation_clause_arity",
            ),
        ));
    }
    let mut seen = BTreeSet::new();
    for param in &clause.params {
        if !seen.insert(param.name.clone()) {
            diagnostics.push(Diagnostic::new(
                "handler.duplicate_operation_binding",
                Severity::Error,
                DiagnosticKind::Name,
                format!("operation clause repeats binding `{}`", param.name),
                Some(param.span.clone()),
                clause_details(
                    param.node_id.display("param"),
                    signature,
                    effect,
                    Some(&operation.name),
                    "duplicate_operation_binding",
                ),
            ));
        }
    }
    diagnostics
}

fn clause_body_diagnostics(
    handler: &HandlerDecl,
    clause: &HandlerOperationClauseDecl,
    operation: &EffectOperationSignature,
    signature: &HandlerSignature,
    effect: &EffectSignature,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    let synthetic = synthetic_clause_function(handler, clause, operation);
    let mut checker = FunctionChecker::new(&synthetic, environment);
    for (index, param) in handler.params.iter().enumerate() {
        checker.bindings.push(Binding::new(
            param.name.clone(),
            signature
                .params
                .get(index)
                .cloned()
                .unwrap_or(Type::Unknown),
        ));
        checker.local_names.insert(
            param.name.clone(),
            (param.node_id.display("param"), param.span.clone()),
        );
    }
    for (index, param) in clause.params.iter().enumerate() {
        checker.bindings.push(Binding::new(
            param.name.clone(),
            operation
                .params
                .get(index)
                .cloned()
                .unwrap_or(Type::Unknown),
        ));
        checker.local_names.insert(
            param.name.clone(),
            (param.node_id.display("param"), param.span.clone()),
        );
    }
    let expected = ExpectedType {
        ty: operation.return_type.clone(),
        source: ExpectedTypeSource::DeclaredReturn,
        origin_node_id: operation.node_id,
        origin_span: Some(operation.name_span.clone()),
        origin_message: "Operation result type is declared here.",
    };
    let actual = checker.infer_expr(&clause.body, Some(&expected));
    checker.check_assignable(
        &clause.body,
        &operation.return_type,
        &actual,
        &expected,
        "handler_operation_result",
    );
    let mut diagnostics = checker.diagnostics;
    if checker
        .inferred_effects
        .iter()
        .any(|effect_use| effect_use.effect == signature.effect)
    {
        diagnostics.push(recursive_clause_diagnostic(clause, signature, effect));
    }
    if handler.visibility == Visibility::Public {
        diagnostics.extend(
            checker
                .inferred_effects
                .iter()
                .filter(|effect_use| {
                    effect_use.effect != signature.effect
                        && !signature
                            .effects
                            .iter()
                            .any(|declared| declared == &effect_use.effect)
                })
                .map(|effect_use| {
                    Diagnostic::new(
                        "handler.missing_public_effect",
                        Severity::Error,
                        DiagnosticKind::Effect,
                        format!(
                            "public handler `{}` uses undeclared effect `{}`",
                            signature.qualified_name, effect_use.effect
                        ),
                        Some(clause.body.span.clone()),
                        clause_details(
                            clause.node_id.display("clause"),
                            signature,
                            effect,
                            Some(&operation.name),
                            "missing_public_effect",
                        ),
                    )
                }),
        );
    }
    diagnostics
}

fn synthetic_clause_function(
    handler: &HandlerDecl,
    clause: &HandlerOperationClauseDecl,
    operation: &EffectOperationSignature,
) -> Function {
    Function {
        node_id: clause.node_id,
        module_name: handler.module_name.clone(),
        kind: FunctionKind::Function,
        visibility: Visibility::Private,
        name: Some(synthetic_clause_function_name(handler, clause)),
        effect_binder: None,
        params: Vec::new(),
        return_binding: None,
        return_type: Some(operation.return_type.render()),
        return_type_span: Some(operation.name_span.clone()),
        effects: None,
        effect_spans: None,
        contracts: Vec::new(),
        body: Vec::new(),
        span: clause.span.clone(),
    }
}

pub(crate) fn synthetic_clause_function_name(
    handler: &HandlerDecl,
    clause: &HandlerOperationClauseDecl,
) -> String {
    synthetic_handler_clause_function_name(
        handler.name.as_deref().unwrap_or("missing"),
        clause.operation.as_deref().unwrap_or("missing"),
    )
}

fn unknown_operation_diagnostic(
    clause: &HandlerOperationClauseDecl,
    operation_name: &str,
    signature: &HandlerSignature,
    effect: &EffectSignature,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "handler.unknown_operation",
        Severity::Error,
        DiagnosticKind::Effect,
        format!(
            "handled effect `{}` has no operation clause `{operation_name}`",
            effect.qualified_name
        ),
        Some(clause.operation_span.clone()),
        clause_details(
            clause.node_id.display("clause"),
            signature,
            effect,
            Some(operation_name),
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

fn recursive_clause_diagnostic(
    clause: &HandlerOperationClauseDecl,
    signature: &HandlerSignature,
    effect: &EffectSignature,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "handler.recursive_operation_clause",
        Severity::Error,
        DiagnosticKind::Effect,
        format!(
            "operation clause `{}` performs handled effect `{}`",
            clause.operation.as_deref().unwrap_or("<missing>"),
            signature.effect
        ),
        Some(clause.body.span.clone()),
        clause_details(
            clause.node_id.display("clause"),
            signature,
            effect,
            clause.operation.as_deref(),
            "recursive_operation_clause",
        ),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("handler_declaration")),
        (
            "message",
            JsonValue::string(format!(
                "Handler `{}` handles `{}`.",
                signature.qualified_name, effect.qualified_name
            )),
        ),
        (
            "handler",
            JsonValue::string(signature.qualified_name.clone()),
        ),
        (
            "handled_effect",
            JsonValue::string(effect.qualified_name.clone()),
        ),
        ("span", span_json(&effect.span)),
    ]));
    diagnostic
}

fn clause_details(
    node_id: String,
    signature: &HandlerSignature,
    effect: &EffectSignature,
    operation: Option<&str>,
    reason: &'static str,
) -> JsonValue {
    handler_details(
        node_id,
        "handler_operation_clause",
        signature.qualified_name.clone(),
        effect.qualified_name.clone(),
        operation,
        reason,
    )
}
