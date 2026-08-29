use super::*;

pub(super) fn validate_companion_sources(project: &Project) -> Vec<Diagnostic> {
    let source_paths = project
        .files
        .iter()
        .map(|source| source.path().as_str().to_string())
        .collect::<BTreeSet<_>>();
    project
        .files
        .iter()
        .filter_map(|source| {
            let companion = classify_companion_source(source.path().as_str())?;
            if companion.chained {
                Some(chained_companion_diagnostic(source, &companion.target_path))
            } else if !source_paths.contains(&companion.target_path) {
                Some(missing_companion_target_diagnostic(
                    source,
                    &companion.target_path,
                ))
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn validate_companion_public_declarations(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(module.functions.iter().filter_map(|function| {
        public_companion_declaration(
            &function.visibility,
            &function.span,
            "public_function",
            "function",
            function.name.as_deref(),
        )
    }));
    diagnostics.extend(module.effects.iter().filter_map(|effect| {
        public_companion_declaration(
            &effect.visibility,
            &effect.span,
            "public_effect",
            "effect",
            effect.name.as_deref(),
        )
    }));
    diagnostics.extend(module.handlers.iter().filter_map(|handler| {
        public_companion_declaration(
            &handler.visibility,
            &handler.span,
            "public_handler",
            "handler",
            handler.name.as_deref(),
        )
    }));
    for ty in &module.types {
        diagnostics.extend(public_companion_declaration(
            &ty.visibility,
            &ty.span,
            "public_type",
            "type",
            ty.name.as_deref(),
        ));
        diagnostics.extend(ty.variants.iter().filter_map(|variant| {
            public_companion_declaration(
                &variant.visibility,
                &variant.span,
                "public_type_variant",
                "type variant",
                variant.name.as_deref(),
            )
        }));
    }
    diagnostics.extend(module.schemas.iter().filter_map(|schema| {
        public_companion_declaration(
            &schema.visibility,
            &schema.span,
            "public_schema",
            "schema",
            schema.name.as_deref(),
        )
    }));
    diagnostics.extend(module.aliases.iter().filter_map(|alias| {
        let (reason, declaration_kind) = alias_companion_public_reason(alias.kind);
        companion_path_for_span(&alias.span).map(|companion_path| {
            companion_public_declaration_diagnostic(
                alias.span.clone(),
                companion_path,
                reason,
                declaration_kind,
                alias.name.as_deref(),
            )
        })
    }));
    diagnostics
}

fn public_companion_declaration(
    visibility: &Visibility,
    span: &SourceSpan,
    reason: &'static str,
    declaration_kind: &'static str,
    name: Option<&str>,
) -> Option<Diagnostic> {
    if *visibility == Visibility::Public {
        companion_path_for_span(span).map(|companion_path| {
            companion_public_declaration_diagnostic(
                span.clone(),
                companion_path,
                reason,
                declaration_kind,
                name,
            )
        })
    } else {
        None
    }
}

fn alias_companion_public_reason(kind: PublicAliasKind) -> (&'static str, &'static str) {
    match kind {
        PublicAliasKind::Function => ("public_function_alias", "function alias"),
        PublicAliasKind::Type => ("public_type_alias", "type alias"),
        PublicAliasKind::Schema => ("public_schema_alias", "schema alias"),
    }
}

fn companion_path_for_span(span: &SourceSpan) -> Option<&str> {
    let path = span.file.as_str();
    classify_companion_source(path).map(|_| path)
}

fn companion_public_declaration_diagnostic(
    span: SourceSpan,
    companion_path: &str,
    reason: &'static str,
    declaration_kind: &'static str,
    declaration_name: Option<&str>,
) -> Diagnostic {
    let described_declaration = declaration_name.map_or_else(
        || format!("public {declaration_kind}"),
        |name| format!("public {declaration_kind} `{name}`"),
    );
    let mut diagnostic = Diagnostic::new(
        "module.companion_public_declaration",
        Severity::Error,
        DiagnosticKind::Module,
        format!("test companion `{companion_path}` cannot declare {described_declaration}"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("companion_public_declaration")),
            ("companion_path", JsonValue::string(companion_path)),
            ("reason", JsonValue::string(reason)),
            ("declaration_kind", JsonValue::string(declaration_kind)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string("Remove `pub`; test companion declarations are not externally visible."),
    )]));
    diagnostic
}

fn missing_companion_target_diagnostic(source: &SourceFile, target_path: &str) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.companion_missing_target",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "test companion `{}` has no matching target `{target_path}`",
            source.path().as_str()
        ),
        Some(source.span(TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("companion_target")),
            ("companion_path", JsonValue::string(source.path().as_str())),
            ("target_path", JsonValue::string(target_path)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string("Create the target source beside the companion or rename the companion."),
    )]));
    diagnostic
}

fn chained_companion_diagnostic(source: &SourceFile, target_path: &str) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.chained_companion",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "test companion `{}` cannot target another companion `{target_path}`",
            source.path().as_str()
        ),
        Some(source.span(TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("companion_target")),
            ("companion_path", JsonValue::string(source.path().as_str())),
            ("target_path", JsonValue::string(target_path)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string("Use exactly one `.test.veln` suffix for a test companion."),
    )]));
    diagnostic
}

pub(super) fn source_mod_decl_diagnostic(module: &veln_syntax::ModuleDecl) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.source_mod",
        Severity::Error,
        DiagnosticKind::Module,
        "source `mod` declarations are not supported",
        Some(module.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("module_path", JsonValue::string(module.name.clone())),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(
            "Move or rename the source file so its package-relative path derives the intended module path.",
        ),
    )]));
    diagnostic
}

pub(super) fn dotted_use_decl_diagnostic(use_decl: &veln_syntax::UseDecl) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.invalid_import_path",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "module import `{}` uses `.`; source module paths use `::`",
            use_decl.name
        ),
        Some(use_decl.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("import_path")),
            ("module_path", JsonValue::string(use_decl.name.clone())),
            ("expected_delimiter", JsonValue::string("::")),
            ("observed_delimiter", JsonValue::string(".")),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string("Rewrite the import with `::` between module path segments."),
    )]));
    diagnostic
}

pub(super) fn duplicate_derived_module_diagnostic(
    module_name: &str,
    source: &SourceFile,
    first_source: &SourceFile,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.duplicate_source_path",
        Severity::Error,
        DiagnosticKind::Module,
        format!("multiple source files derive module path `{module_name}`"),
        Some(source.span(veln_source::TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("module_path", JsonValue::string(module_name)),
            ("source_path", JsonValue::string(source.path().as_str())),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("duplicate_origin")),
        (
            "message",
            JsonValue::string(format!(
                "The first source file deriving `{module_name}` is here."
            )),
        ),
        (
            "span",
            JsonValue::object([
                ("file", JsonValue::string(first_source.path().as_str())),
                (
                    "start",
                    JsonValue::object([
                        ("line", JsonValue::Number(1)),
                        ("column", JsonValue::Number(1)),
                        ("offset", JsonValue::Number(0)),
                    ]),
                ),
                (
                    "end",
                    JsonValue::object([
                        ("line", JsonValue::Number(1)),
                        ("column", JsonValue::Number(1)),
                        ("offset", JsonValue::Number(0)),
                    ]),
                ),
            ]),
        ),
    ]));
    diagnostic
}

pub(super) fn reserved_source_module_diagnostic(
    source: &SourceFile,
    module_name: &str,
) -> Diagnostic {
    Diagnostic::new(
        "name.reserved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("module identity `{module_name}` conflicts with the standard prelude"),
        Some(source.span(TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::Null),
            ("name", JsonValue::string(module_name)),
            ("namespace", JsonValue::string("module")),
            ("reserved_for", JsonValue::string("standard_prelude")),
        ]),
    )
}
