use crate::adt::type_operations as adt;
use crate::diagnostics::{module_details, span_json};
pub(super) use crate::name_recovery::normal_imported_use_for_path;
use crate::name_recovery::{resolved_import_module_name, use_decl_is_recovery_candidate};
use crate::semantic_model::Type;
use crate::standard_names::PRELUDE_MODULE;
use crate::type_syntax::parse_type_annotation;
use std::collections::BTreeMap;
use veln_ast::{Function, FunctionKind, SurfaceModule, UseDecl};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_source::SourceSpan;

pub(super) fn function_target<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a veln_ast::Function> {
    match segments {
        [name] => module.functions.iter().find(|function| {
            function.kind == FunctionKind::Function
                && function.name.as_deref() == Some(name)
                && function.name.as_deref().is_some_and(valid_function_name)
        }),
        [_, .., name] => {
            let module_name = normal_imported_module_for_path(
                module,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            module.functions.iter().find(|function| {
                function.kind == FunctionKind::Function
                    && function.name.as_deref() == Some(name)
                    && function.name.as_deref().is_some_and(valid_function_name)
                    && function.module_name.as_deref() == Some(module_name.as_str())
            })
        }
        _ => None,
    }
}

pub(super) fn type_target<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a veln_ast::TypeDecl> {
    match segments {
        [name] => module.types.iter().find(|type_decl| {
            type_decl.name.as_deref() == Some(name)
                && type_decl.name.as_deref().is_some_and(valid_type_name)
        }),
        [_, .., name] => {
            let module_name = normal_imported_module_for_path(
                module,
                &segments[..segments.len() - 1],
                current_module,
            )?;
            module.types.iter().find(|type_decl| {
                type_decl.name.as_deref() == Some(name)
                    && type_decl.name.as_deref().is_some_and(valid_type_name)
                    && type_decl.module_name.as_deref() == Some(module_name.as_str())
            })
        }
        _ => None,
    }
}

pub(super) fn valid_function_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
}

pub(super) fn valid_type_name(name: &str) -> bool {
    name.as_bytes().first().is_some_and(u8::is_ascii_uppercase)
}

pub(super) fn normal_imported_module_for_path(
    module: &SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<String> {
    normal_imported_use_for_path(module, segments, current_module)
        .map(|use_decl| resolved_import_module_name(use_decl, current_module))
}

pub(super) fn quarantined_imported_use_for_path<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
    current_module: Option<&str>,
) -> Option<&'a UseDecl> {
    let module_path = segments.join("::");
    module.uses.iter().find(|use_decl| {
        use_decl_is_recovery_candidate(module, use_decl)
            && use_decl.module_name.as_deref() == current_module
            && (use_decl.name == module_path || use_decl.alias == module_path)
    })
}

pub(super) fn unresolved_alias_diagnostic(
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

pub(super) fn private_alias_diagnostic(alias: &veln_ast::PublicAlias) -> Diagnostic {
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

pub(super) fn alias_kind_mismatch_diagnostic(
    alias: &veln_ast::PublicAlias,
    expected_kind: &'static str,
    actual_kind: &'static str,
) -> Diagnostic {
    let actual_kind_message = match actual_kind {
        "module_value" => "module value",
        other => other,
    };
    Diagnostic::new(
        "name.kind_mismatch",
        Severity::Error,
        DiagnosticKind::Name,
        format!(
            "public alias target `{}` is a {actual_kind_message}, not a {expected_kind}",
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
    let mut previous = Vec::<&UseDecl>::new();

    for use_decl in module
        .uses
        .iter()
        .filter(|use_decl| use_decl.origin == veln_ast::UseOrigin::Source && !use_decl.recovered)
    {
        let duplicate_target = previous.iter().copied().find(|first| {
            first.module_name == use_decl.module_name
                && first.written_name == use_decl.written_name
                && first.package == use_decl.package
        });
        if let Some(first) = duplicate_target {
            diagnostics.push(duplicate_name_diagnostic(
                &use_decl.written_name,
                "module",
                "import",
                use_decl.node_id.display("use"),
                import_target_span(use_decl),
                first.node_id.display("use"),
                &import_target_span(first),
            ));
        }
        if let Some(first) = previous.iter().copied().find(|first| {
            first.module_name == use_decl.module_name
                && first.alias == use_decl.alias
                && (first.has_explicit_alias() || use_decl.has_explicit_alias())
                && duplicate_target.is_none_or(|target| !std::ptr::eq(*first, target))
        }) {
            diagnostics.push(duplicate_name_diagnostic(
                &use_decl.alias,
                "module",
                "import alias",
                use_decl.node_id.display("use"),
                import_qualifier_span(use_decl),
                first.node_id.display("use"),
                &import_qualifier_span(first),
            ));
        }
        previous.push(use_decl);
    }

    diagnostics
}

fn import_target_span(use_decl: &UseDecl) -> SourceSpan {
    let Some(first) = use_decl.name_spans.first() else {
        return use_decl.span.clone();
    };
    let mut span = first.clone();
    if let Some(last) = use_decl.name_spans.last() {
        span.end = last.end;
    }
    span
}

fn import_qualifier_span(use_decl: &UseDecl) -> SourceSpan {
    use_decl
        .alias_span
        .clone()
        .or_else(|| use_decl.name_spans.last().cloned())
        .unwrap_or_else(|| use_decl.span.clone())
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
            && (use_decl.has_explicit_alias()
                || !use_decl
                    .module_name
                    .as_deref()
                    .is_some_and(|module_name| module_name.starts_with("std::")))
        {
            diagnostics.push(reserved_prelude_diagnostic(
                use_decl.node_id.display("use"),
                import_qualifier_span(use_decl),
                "module",
                "import alias",
                "Choose a non-conflicting import alias.",
            ));
        }
    }

    diagnostics
}

pub(super) fn is_toolchain_standard_prelude(span: &SourceSpan) -> bool {
    span.file.as_str() == "prelude.veln"
}

pub(super) fn reserved_prelude_diagnostic(
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

pub(in crate::analysis) fn duplicate_name_diagnostic(
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

    if let Some(diagnostic) = test_parameter_diagnostic(function, &node_id) {
        diagnostics.push(diagnostic);
    }

    if let Some(diagnostic) = test_boundary_return_diagnostic(function, &node_id) {
        diagnostics.push(diagnostic);
    }

    diagnostics
}

fn test_parameter_diagnostic(function: &Function, node_id: &str) -> Option<Diagnostic> {
    let param = function.params.first()?;
    let mut diagnostic = Diagnostic::new(
        "test.parameters",
        Severity::Error,
        DiagnosticKind::Type,
        "test declaration has parameters",
        Some(param.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("test")),
            ("node_id", JsonValue::string(node_id)),
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
    Some(diagnostic)
}

fn test_boundary_return_diagnostic(function: &Function, node_id: &str) -> Option<Diagnostic> {
    let Some(return_type) = function.return_type.as_deref() else {
        return Some(test_return_diagnostic(
            function,
            node_id,
            "test declaration has no return type annotation".to_string(),
            "missing".to_string(),
        ));
    };
    let Ok(ty) = parse_type_annotation(return_type) else {
        return None;
    };
    (!is_allowed_test_return(&ty)).then(|| {
        test_return_diagnostic(
            function,
            node_id,
            format!("test declaration returns `{}`", ty.render()),
            ty.render(),
        )
    })
}

pub(super) fn is_allowed_test_return(ty: &Type) -> bool {
    ty == &Type::unit() || adt::result_parts(ty).is_some_and(|(value, _)| value == &Type::unit())
}

pub(in crate::analysis) fn type_contains_unknown(ty: &Type) -> bool {
    match ty {
        Type::Unknown => true,
        Type::Named { args, .. } | Type::VariantRefinement { args, .. } => {
            args.iter().any(type_contains_unknown)
        }
        Type::Record(fields) => fields.iter().any(|(_, ty)| type_contains_unknown(ty)),
        Type::Function {
            params,
            return_type,
            ..
        } => params.iter().any(type_contains_unknown) || type_contains_unknown(return_type),
    }
}

pub(super) fn test_return_diagnostic(
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
