use super::*;

#[test]
fn duplicate_use_aliases_are_static_errors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use platform.io\n",
            "use platform.io\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].message, "duplicate import alias name `io`");
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"namespace\":\"module\"")
    );
}

#[test]
fn duplicate_use_aliases_are_scoped_to_declaring_module() {
    let first_source = SourceFile::new(
        "first.veln",
        concat!(
            "mod first\n",
            "use shared\n",
            "fn first_value() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let second_source = SourceFile::new(
        "second.veln",
        concat!(
            "mod second\n",
            "use shared\n",
            "fn second_value() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let first = lower_surface_ast(&parse(&first_source).tree);
    let second = lower_surface_ast(&parse(&second_source).tree);
    let module = SurfaceModule {
        module: first.module,
        uses: [first.uses, second.uses].concat(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: Vec::new(),
        functions: [first.functions, second.functions].concat(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.duplicate"),
        "{diagnostics:#?}"
    );
}

#[test]
fn public_function_alias_rejects_type_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "type Document\n",
            "  pub Text(String)\n",
            "end\n",
            "pub fn parse = Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message == "public alias target `Document` is a type, not a function"
    }));
}

#[test]
fn public_type_alias_rejects_function_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "fn parse() -> Int\n",
            "  1\n",
            "end\n",
            "pub type Document = parse\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.kind_mismatch"
            && diagnostic.message == "public alias target `parse` is a function, not a type"
    }));
}

#[test]
fn public_alias_rejects_unresolved_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "pub fn parse = impl::parse\n",
            "pub type Document = impl::Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved function alias target `impl::parse`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved type alias target `impl::Document`"
    }));
}

#[test]
fn public_type_alias_resolves_nested_import_path_target() {
    let api_source = SourceFile::new(
        "api.veln",
        concat!(
            "use http2::core::detail\n",
            "pub type DocumentAlias = http2::core::detail::Document\n",
        ),
    );
    let detail_source = SourceFile::new(
        "detail.veln",
        concat!("pub type Document\n", "  pub Text(String)\n", "end\n",),
    );
    let api = lower_surface_ast_with_module_identity(
        &parse(&api_source).tree,
        "std::http2::core".to_string(),
        api_source.span(TextRange::new(0, 0)),
    );
    let detail = lower_surface_ast_with_module_identity(
        &parse(&detail_source).tree,
        "std::http2::core::detail".to_string(),
        detail_source.span(TextRange::new(0, 0)),
    );
    let module = SurfaceModule {
        module: api.module,
        uses: [api.uses, detail.uses].concat(),
        aliases: [api.aliases, detail.aliases].concat(),
        effects: [api.effects, detail.effects].concat(),
        handlers: [api.handlers, detail.handlers].concat(),
        types: [api.types, detail.types].concat(),
        schemas: [api.schemas, detail.schemas].concat(),
        functions: [api.functions, detail.functions].concat(),
        invalid_names: [api.invalid_names, detail.invalid_names].concat(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved"
                || diagnostic.message
                    != "unresolved type alias target `http2::core::detail::Document`"
        }),
        "{diagnostics:#?}"
    );
    assert!(
        TypeEnvironment::from_module(&module)
            .adts
            .descriptors()
            .iter()
            .any(|descriptor| {
                descriptor.module_name.as_deref() == Some("std::http2::core")
                    && descriptor.type_name == "DocumentAlias"
            })
    );
}

#[test]
fn public_alias_target_leaf_casing_reports_before_independent_target_failures() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "type Document\n",
            "  pub Text(String)\n",
            "end\n",
            "fn parse() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn wrongKind = Document\n",
            "pub type WrongKind = parse\n",
            "pub fn missing = Missing\n",
            "pub type MissingType = missing_type\n",
            "pub schema Packet = schema_impl::packet\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        alias_target_observations(&diagnostics),
        vec![
            (
                "name.invalid_case",
                "function alias target `Document` must start with an ASCII lowercase letter"
            ),
            (
                "name.invalid_case",
                "type alias target `parse` must start with an ASCII uppercase letter"
            ),
            (
                "name.invalid_case",
                "function alias target `Missing` must start with an ASCII lowercase letter"
            ),
            (
                "name.invalid_case",
                "type alias target `missing_type` must start with an ASCII uppercase letter"
            ),
            (
                "name.kind_mismatch",
                "public alias target `Document` is a type, not a function"
            ),
            (
                "name.kind_mismatch",
                "public alias target `parse` is a function, not a type"
            ),
            (
                "name.unresolved",
                "unresolved function alias target `Missing`"
            ),
            (
                "name.unresolved",
                "unresolved type alias target `missing_type`"
            ),
            (
                "name.unresolved",
                "unresolved schema alias target `schema_impl::packet`"
            ),
        ],
        "{diagnostics:#?}"
    );
    assert_first_alias_target_invalid_case(&diagnostics);
    assert_invalid_alias_targets_are_quarantined(&module, &diagnostics);
}

fn alias_target_observations(diagnostics: &[Diagnostic]) -> Vec<(&str, &str)> {
    diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.invalid_case"
                || diagnostic.id == "name.kind_mismatch"
                || diagnostic.id == "name.unresolved"
        })
        .map(|diagnostic| (diagnostic.id.as_str(), diagnostic.message.as_str()))
        .collect()
}

fn assert_first_alias_target_invalid_case(diagnostics: &[Diagnostic]) {
    let invalid = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.invalid_case")
        .expect("invalid target diagnostic");
    let span = invalid.span.as_ref().expect("invalid target span");
    assert_eq!(
        (span.start.line, span.start.column, span.end.column),
        (8, 20, 28)
    );
    let details = invalid.details.to_json();
    assert!(details.contains("\"occurrence\":\"alias_target\""));
    assert!(details.contains("\"name\":\"Document\""));
    assert!(details.contains("\"name_class\":\"function\""));
    assert!(details.contains("\"required_initial\":\"ascii_lowercase\""));
    assert!(details.contains("\"observed_initial\":\"ascii_uppercase\""));
}

fn assert_invalid_alias_targets_are_quarantined(
    surface: &SurfaceModule,
    diagnostics: &[Diagnostic],
) {
    assert!(
        !diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.invalid_case"
                && diagnostic.message.contains("schema alias target")
        }),
        "{diagnostics:#?}"
    );
    let environment = TypeEnvironment::from_module(surface);
    assert!(environment.function("wrongKind").is_none());
    assert_eq!(
        environment
            .schema_reference_error(&["WrongKind".to_string()], Some("spec.api"))
            .kind,
        SchemaReferenceErrorKind::Unresolved
    );
    assert!(
        environment
            .adts
            .descriptors()
            .iter()
            .all(|descriptor| descriptor.type_name != "WrongKind")
    );
}

#[test]
fn public_schema_alias_with_invalid_target_leaf_does_not_enter_schema_namespace() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "schema Packet\n",
            "  format binary\n",
            "  byte: UInt8\n",
            "end\n",
            "pub schema Alias = Packet\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut module = lower_surface_ast(&parsed.tree);
    let target_span = module.aliases[0].target_spans[0].clone();
    module.invalid_names.push(InvalidName {
        name: "Packet".to_string(),
        class: NameClass::Function,
        occurrence: NameOccurrence::AliasTarget,
        span: target_span,
        enclosing_function_span: None,
        segment_index: None,
    });

    let environment = TypeEnvironment::from_module(&module);

    assert!(
        environment
            .schema_decode_step_signature(&["Packet".to_string()], Some("spec.api"))
            .is_some()
    );
    assert!(
        environment
            .schema_decode_step_signature(&["Alias".to_string()], Some("spec.api"))
            .is_none()
    );
}

#[test]
fn public_alias_names_share_member_namespaces() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "fn parse() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn parse = parse\n",
            "type Document\n",
            "  pub Text(String)\n",
            "end\n",
            "pub type Document = Document\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate function alias name `parse`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate type alias name `Document`"
    }));
}

#[test]
fn public_schema_alias_names_share_schema_namespace() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "pub schema Packet\n",
            "  format binary\n",
            "  length: UInt8\n",
            "end\n",
            "pub schema Packet = Packet\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.duplicate"
            && diagnostic.message == "duplicate schema alias name `Packet`"
    }));
}
