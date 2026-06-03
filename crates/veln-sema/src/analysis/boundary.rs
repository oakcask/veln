use super::*;
use crate::prelude::PRELUDE_MODULE;
use veln_ast::{PublicAliasKind, UseDecl};

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

pub(crate) fn check_declared_effect_labels(function: &Function) -> Vec<Diagnostic> {
    let Some(declared_effects) = &function.effects else {
        return Vec::new();
    };
    let boundary = declared_effect_boundary(function);
    let node_prefix = function.kind.node_prefix();

    if declared_effects.is_empty() {
        return vec![empty_declared_effect_diagnostic(
            function,
            node_prefix,
            boundary,
        )];
    }

    declared_effects
        .iter()
        .filter(|effect| !KNOWN_EFFECT_LABELS.contains(&effect.as_str()))
        .map(|effect| unknown_declared_effect_diagnostic(function, effect, node_prefix, boundary))
        .collect()
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
        Some(function.span.clone()),
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

pub(crate) fn check_public_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for alias in &module.aliases {
        if alias.name.is_none() {
            continue;
        }
        match alias.kind {
            PublicAliasKind::Function => {
                if function_target(module, &alias.target).is_none()
                    && type_target(module, &alias.target).is_some()
                {
                    diagnostics.push(alias_kind_mismatch_diagnostic(alias, "function", "type"));
                } else if function_target(module, &alias.target).is_none() {
                    diagnostics.push(unresolved_alias_diagnostic(alias, "function"));
                }
            }
            PublicAliasKind::Type => {
                if type_target(module, &alias.target).is_none()
                    && function_target(module, &alias.target).is_some()
                {
                    diagnostics.push(alias_kind_mismatch_diagnostic(alias, "type", "function"));
                } else if type_target(module, &alias.target).is_none() {
                    diagnostics.push(unresolved_alias_diagnostic(alias, "type"));
                }
            }
        }
    }
    diagnostics
}

fn function_target<'a>(
    module: &'a SurfaceModule,
    segments: &[String],
) -> Option<&'a veln_ast::Function> {
    match segments {
        [name] => module.functions.iter().find(|function| {
            function.kind == FunctionKind::Function && function.name.as_deref() == Some(name)
        }),
        [_, .., name] => {
            let module_name =
                imported_module_for_path(&module.uses, &segments[..segments.len() - 1])?;
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
) -> Option<&'a veln_ast::TypeDecl> {
    match segments {
        [name] => module
            .types
            .iter()
            .find(|type_decl| type_decl.name.as_deref() == Some(name)),
        [_, .., name] => {
            let module_name =
                imported_module_for_path(&module.uses, &segments[..segments.len() - 1])?;
            module.types.iter().find(|type_decl| {
                type_decl.name.as_deref() == Some(name)
                    && type_decl.module_name.as_deref() == Some(module_name)
            })
        }
        _ => None,
    }
}

fn imported_module_for_path<'a>(uses: &'a [UseDecl], segments: &[String]) -> Option<&'a str> {
    let module_path = segments.join("::");
    uses.iter()
        .find(|use_decl| use_decl.name == module_path || use_decl.alias == module_path)
        .map(|use_decl| use_decl.name.as_str())
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
    let mut seen = BTreeMap::<String, (String, SourceSpan)>::new();

    for use_decl in &module.uses {
        let node_id = use_decl.node_id.display("use");
        if let Some((first_node_id, first_span)) = seen.get(&use_decl.alias) {
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
            seen.insert(use_decl.alias.clone(), (node_id, use_decl.span.clone()));
        }
    }

    diagnostics
}

pub(crate) fn check_reserved_prelude_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    if let Some(header) = &module.module
        && header.name == PRELUDE_MODULE
        && !is_standard_prelude_source(&header.span)
    {
        diagnostics.push(reserved_prelude_diagnostic(
            header.node_id.display("mod"),
            header.span.clone(),
            "module",
            "module identity",
            "Choose a non-conflicting module name.",
        ));
    }

    for use_decl in &module.uses {
        if use_decl.alias == PRELUDE_MODULE {
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

fn is_standard_prelude_source(span: &SourceSpan) -> bool {
    span.file.as_str() == "stdlib/prelude.veln"
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
