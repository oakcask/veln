use super::*;

pub(super) fn check_function_type_parameter_shadowing(
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Vec<Diagnostic> {
    function
        .type_binders
        .iter()
        .enumerate()
        .filter_map(|(ordinal, binder)| {
            function_type_parameter_shadow_diagnostic(ordinal, binder, function, environment)
        })
        .collect()
}

pub(super) fn check_function_type_parameter_signature(
    function: &veln_ast::Function,
) -> Vec<Diagnostic> {
    if function.visibility == Visibility::Public || function.type_binders.is_empty() {
        return Vec::new();
    }

    let mut diagnostics = function
        .params
        .iter()
        .filter(|param| {
            param
                .ty
                .as_deref()
                .is_none_or(|annotation| param.is_variadic && annotation.is_empty())
        })
        .map(|param| {
            Diagnostic::new(
                "type.generic_signature_missing",
                Severity::Error,
                DiagnosticKind::Type,
                format!("generic parameter `{}` has no type annotation", param.name),
                Some(param.span.clone()),
                crate::diagnostics::type_details(
                    param.node_id.display("param"),
                    "explicit",
                    "missing",
                    "declared_parameter",
                    "source",
                    "assignable",
                    [function.node_id.display("fn")],
                ),
            )
        })
        .collect::<Vec<_>>();
    if function.return_type.is_none() {
        diagnostics.push(Diagnostic::new(
            "type.generic_signature_missing",
            Severity::Error,
            DiagnosticKind::Type,
            "generic function has no return type annotation",
            Some(function.span.clone()),
            crate::diagnostics::type_details(
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

fn function_type_parameter_shadow_diagnostic(
    ordinal: usize,
    binder: &veln_ast::TypeBinder,
    function: &veln_ast::Function,
    environment: &TypeEnvironment,
) -> Option<Diagnostic> {
    let builtin = crate::type_syntax::compiler_known_type_annotation_arity(&binder.name).is_some()
        || crate::source_less_lookup::is_builtin_adt_type(&binder.name);
    let descriptor = environment
        .adts
        .unique_descriptor_for_type_path_any_arity_in_owner(
            &binder.name,
            function.module_name.as_deref(),
            function.package_name.as_deref(),
            &environment.uses,
        )
        .or_else(|| {
            environment
                .adts
                .consistent_imported_type_path_descriptor_any_arity(
                    &binder.name,
                    function.module_name.as_deref(),
                    function.package_name.as_deref(),
                    &environment.uses,
                )
        })
        .or_else(|| {
            builtin.then(|| {
                environment.adts.local_source_descriptor_with_declaration(
                    &binder.name,
                    function.module_name.as_deref(),
                    function.package_name.as_deref(),
                )
            })?
        });
    if !builtin && descriptor.is_none() {
        return None;
    }

    let mut diagnostic = type_parameter_shadow_diagnostic(ordinal, binder, builtin);
    if let Some(descriptor) = descriptor {
        append_shadowed_type_declaration(&mut diagnostic, binder, descriptor, environment);
    }
    Some(diagnostic)
}

fn type_parameter_shadow_diagnostic(
    ordinal: usize,
    binder: &veln_ast::TypeBinder,
    builtin: bool,
) -> Diagnostic {
    Diagnostic::new(
        "name.type_parameter_shadow",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "function type parameter `{}` shadows a visible type",
            binder.name
        ),
        Some(binder.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("name_resolution")),
            ("boundary", JsonValue::string("function_type_parameter")),
            ("name", JsonValue::string(binder.name.clone())),
            ("ordinal", JsonValue::Number(ordinal as i64)),
            (
                "shadowed_kind",
                JsonValue::string(if builtin {
                    "builtin_type"
                } else {
                    "visible_type"
                }),
            ),
        ]),
    )
}

fn append_shadowed_type_declaration(
    diagnostic: &mut Diagnostic,
    binder: &veln_ast::TypeBinder,
    descriptor: &crate::adt::descriptors::AdtDescriptor,
    environment: &TypeEnvironment,
) {
    let related = environment
        .adts
        .type_alias_exposure_path(descriptor)
        .and_then(|path| path.declarations().next().map(|decl| decl.span.clone()))
        .or_else(|| environment.adts.declaration_span(descriptor).cloned());
    if let Some(span) = related {
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("shadowed_type_declaration")),
            (
                "message",
                JsonValue::string(format!("Visible type `{}` is declared here.", binder.name)),
            ),
            ("span", crate::diagnostics::span_json(&span)),
        ]));
    }
}
