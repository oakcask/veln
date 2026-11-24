use super::*;

#[derive(Default)]
pub(crate) struct HandlerAnalysis {
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) state_call_types: crate::semantic_model::StateCallTypes,
}

impl HandlerAnalysis {
    pub(super) fn extend(&mut self, other: Self) {
        self.diagnostics.extend(other.diagnostics);
        self.state_call_types.extend(other.state_call_types);
    }
}

pub(crate) fn check_handler_declarations(
    module: &SurfaceModule,
    environment: &TypeEnvironment,
) -> HandlerAnalysis {
    let mut analysis = HandlerAnalysis::default();
    for handler in &module.handlers {
        analysis.extend(check_handler_declaration(handler, environment));
    }
    analysis
}

fn check_handler_declaration(
    handler: &HandlerDecl,
    environment: &TypeEnvironment,
) -> HandlerAnalysis {
    let Some(signature) = handler.name.as_ref().and_then(|name| {
        match environment.handler_path_in_owner(
            std::slice::from_ref(name),
            handler.module_name.as_deref(),
            handler.package_name.as_deref(),
        ) {
            HandlerPathResolution::Found(signature) => Some(signature),
            HandlerPathResolution::PrivateCompanionTargetMismatch { .. }
            | HandlerPathResolution::QuarantinedImportTarget
            | HandlerPathResolution::Missing => None,
        }
    }) else {
        return HandlerAnalysis::default();
    };
    let effect = match environment.resolve_user_effect_path_in_owner(
        &handler.effect,
        handler.module_name.as_deref(),
        handler.package_name.as_deref(),
    ) {
        UserEffectPathResolution::Found(effect) => effect,
        UserEffectPathResolution::PrivateCompanionTargetMismatch { effect, access } => {
            return HandlerAnalysis {
                diagnostics: vec![private_companion_effect_target_diagnostic(
                    handler.node_id.display("handler"),
                    "handler_declaration",
                    &handler.effect.join("::"),
                    effect,
                    access,
                    handler.effect_span.clone(),
                )],
                ..HandlerAnalysis::default()
            };
        }
        UserEffectPathResolution::QuarantinedImportTarget => return HandlerAnalysis::default(),
        UserEffectPathResolution::Missing => {
            return HandlerAnalysis {
                diagnostics: vec![unknown_effect_diagnostic(handler, environment)],
                ..HandlerAnalysis::default()
            };
        }
    };

    let mut analysis = HandlerAnalysis {
        diagnostics: declared_effect_diagnostics(handler, environment),
        ..HandlerAnalysis::default()
    };
    analysis
        .diagnostics
        .extend(duplicate_clause_diagnostics(handler, signature, effect));
    analysis
        .diagnostics
        .extend(missing_clause_diagnostics(handler, signature, effect));
    for clause in &handler.operation_clauses {
        analysis.extend(check_clause(
            handler,
            clause,
            signature,
            effect,
            environment,
        ));
    }
    analysis
}
