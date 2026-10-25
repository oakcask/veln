use crate::adt::type_operations as adt;
use crate::diagnostics::{module_details, span_json};
pub(super) use crate::name_recovery::normal_imported_use_for_path;
use crate::name_recovery::{
    ImportCollisionIndex, resolved_import_module_name, use_decl_has_invalid_explicit_alias,
    use_decl_has_invalid_module_segment,
};
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
    let collisions = ImportCollisionIndex::new(module);
    let mut candidates = module.uses.iter().filter(|use_decl| {
        let recoverable_name = if use_decl.explicit_alias {
            !use_decl_has_invalid_module_segment(module, use_decl)
                && use_decl_has_invalid_explicit_alias(module, use_decl)
                && use_decl.alias == module_path
        } else {
            use_decl_has_invalid_module_segment(module, use_decl)
                && (use_decl.written_name == module_path || use_decl.alias == module_path)
        };
        use_decl.origin == veln_ast::UseOrigin::Source
            && !use_decl.syntax_recovered
            && use_decl.alias != PRELUDE_MODULE
            && use_decl.module_name.as_deref() == current_module
            && !collisions.contains_target(use_decl)
            && !collisions.contains_qualifier(use_decl)
            && recoverable_name
    });
    let candidate = candidates.next()?;
    candidates.next().is_none().then_some(candidate)
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

fn source_uses_in_diagnostic_order(module: &SurfaceModule) -> Vec<&UseDecl> {
    let mut source_uses = module
        .uses
        .iter()
        .filter(|use_decl| use_decl.origin == veln_ast::UseOrigin::Source)
        .collect::<Vec<_>>();
    source_uses.sort_by(|left, right| {
        left.span
            .file
            .as_str()
            .cmp(right.span.file.as_str())
            .then(left.span.start.offset.cmp(&right.span.start.offset))
            .then(left.span.end.offset.cmp(&right.span.end.offset))
    });
    source_uses
}

type ImportOrigin = (String, SourceSpan, SourceSpan);

#[derive(Default)]
struct DuplicateUseOrigins {
    targets: BTreeMap<(Option<String>, String, Option<String>), ImportOrigin>,
    qualifiers: BTreeMap<(Option<String>, String), ImportOrigin>,
}

impl DuplicateUseOrigins {
    fn check(&mut self, diagnostics: &mut Vec<Diagnostic>, use_decl: &UseDecl) {
        let node_id = use_decl.node_id.display("use");
        let target_origin = self.check_target(diagnostics, use_decl, &node_id);
        self.check_qualifier(diagnostics, use_decl, node_id, target_origin.as_ref());
    }

    fn check_target(
        &mut self,
        diagnostics: &mut Vec<Diagnostic>,
        use_decl: &UseDecl,
        node_id: &str,
    ) -> Option<ImportOrigin> {
        let target_key = (
            use_decl.module_name.clone(),
            use_decl.written_name.clone(),
            use_decl.package.clone(),
        );
        let target_origin = self.targets.get(&target_key).cloned();
        if let Some((first_node_id, first_span, _)) = &target_origin {
            diagnostics.push(duplicate_name_diagnostic(
                &use_decl.written_name,
                "module",
                "import path",
                node_id.to_string(),
                use_decl.target_span(),
                first_node_id.clone(),
                first_span,
            ));
        } else {
            self.targets.insert(
                target_key,
                (
                    node_id.to_string(),
                    use_decl.target_span(),
                    use_decl.span.clone(),
                ),
            );
        }
        target_origin
    }

    fn check_qualifier(
        &mut self,
        diagnostics: &mut Vec<Diagnostic>,
        use_decl: &UseDecl,
        node_id: String,
        target_origin: Option<&ImportOrigin>,
    ) {
        let has_qualifier = !use_decl.syntax_recovered
            && crate::name_recovery::use_decl_has_import_qualifier(use_decl);
        let qualifier_key = (use_decl.module_name.clone(), use_decl.alias.clone());
        let qualifier_origin = has_qualifier
            .then(|| self.qualifiers.get(&qualifier_key).cloned())
            .flatten();
        let same_origin = match (target_origin, &qualifier_origin) {
            (Some((_, _, target_decl_span)), Some((_, _, qualifier_decl_span))) => {
                target_decl_span == qualifier_decl_span
            }
            _ => false,
        };
        if let Some((first_node_id, first_span, _)) = &qualifier_origin
            && !same_origin
        {
            diagnostics.push(duplicate_name_diagnostic(
                &use_decl.alias,
                "module",
                "import alias",
                node_id,
                use_decl.qualifier_span(),
                first_node_id.clone(),
                first_span,
            ));
        } else if has_qualifier && qualifier_origin.is_none() {
            self.qualifiers.insert(
                qualifier_key,
                (node_id, use_decl.qualifier_span(), use_decl.span.clone()),
            );
        }
    }
}

pub(crate) fn check_duplicate_use_aliases(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut origins = DuplicateUseOrigins::default();

    for use_decl in source_uses_in_diagnostic_order(module) {
        origins.check(&mut diagnostics, use_decl);
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

    for use_decl in module.uses.iter().filter(|use_decl| {
        use_decl.origin == veln_ast::UseOrigin::Source && !use_decl.syntax_recovered
    }) {
        if use_decl.alias == PRELUDE_MODULE
            && (use_decl.explicit_alias
                || !use_decl
                    .module_name
                    .as_deref()
                    .is_some_and(|module_name| module_name.starts_with("std::")))
        {
            diagnostics.push(reserved_prelude_diagnostic(
                use_decl.node_id.display("use"),
                use_decl.qualifier_span(),
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
