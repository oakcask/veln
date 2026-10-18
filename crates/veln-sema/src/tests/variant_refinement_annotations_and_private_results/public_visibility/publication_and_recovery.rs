use super::*;

fn assert_related_span(diagnostic: &Diagnostic, kind: &str, expected: &veln_source::SourceSpan) {
    let related = diagnostic
        .related
        .iter()
        .find(|related| {
            related.as_object().is_some_and(|fields| {
                fields.iter().any(|(name, value)| {
                    name == "kind" && value.as_text().is_some_and(|value| value == kind)
                })
            })
        })
        .unwrap_or_else(|| panic!("missing related note `{kind}`: {diagnostic:#?}"));
    let span = related
        .as_object()
        .and_then(|fields| fields.iter().find(|(name, _)| name == "span"))
        .map(|(_, value)| value)
        .expect("related note span");
    assert_eq!(span, &veln_diagnostics::source_span_to_json(expected));
}

fn related_kinds_and_spans(
    diagnostic: &Diagnostic,
) -> Vec<(String, String, veln_diagnostics::JsonValue)> {
    diagnostic
        .related
        .iter()
        .map(|related| {
            let fields = related.as_object().expect("related note object");
            let kind = fields
                .iter()
                .find(|(name, _)| name == "kind")
                .and_then(|(_, value)| value.as_text())
                .expect("related note kind")
                .to_string();
            let message = fields
                .iter()
                .find(|(name, _)| name == "message")
                .and_then(|(_, value)| value.as_text())
                .expect("related note message")
                .to_string();
            let span = fields
                .iter()
                .find(|(name, _)| name == "span")
                .map(|(_, value)| value.clone())
                .expect("related note span");
            (kind, message, span)
        })
        .collect()
}

fn assert_embedded_prelude_diagnostic_details(diagnostics: &[Diagnostic]) {
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.variant_refinement_private",
            "type.variant_refinement_private",
            "type.variant_refinement_private",
            "type.variant_refinement_private",
            "type.variant_refinement_private",
            "type.variant_refinement_private",
        ],
        "{diagnostics:#?}"
    );

    let expected_written = [
        "Byte::Byte",
        "prelude::Byte::Byte",
        "Local::Byte",
        "Local::Byte",
        "Imported::Byte",
        "Facade::Byte",
    ];
    for (diagnostic, written_type) in diagnostics.iter().zip(expected_written) {
        let json = veln_diagnostics::diagnostic_to_json(diagnostic).to_json();
        assert!(
            json.contains(&format!("\"written_type\":\"{written_type}\"")),
            "{json}"
        );
        assert!(
            json.contains("\"base_type\":\"std::bytes::Byte\""),
            "{json}"
        );
    }
}

fn assert_embedded_prelude_direct_related(
    diagnostic: &Diagnostic,
    application: &veln_ast::SurfaceModule,
    prelude_alias_span: &veln_source::SourceSpan,
    terminal_span: &veln_source::SourceSpan,
) {
    assert_eq!(
        related_kinds_and_spans(diagnostic),
        vec![
            (
                "type_alias_declaration".to_string(),
                "Public type alias `Byte` is declared here.".to_string(),
                veln_diagnostics::source_span_to_json(prelude_alias_span),
            ),
            (
                "private_type_declaration".to_string(),
                "Private type `std::bytes::Byte` is declared here.".to_string(),
                veln_diagnostics::source_span_to_json(terminal_span),
            ),
            (
                "public_parameter_boundary".to_string(),
                "Public parameter `value` of function `direct` exposes this refinement."
                    .to_string(),
                veln_diagnostics::source_span_to_json(&application.functions[0].params[0].span),
            ),
        ]
    );
    assert_eq!(
        diagnostic.span.as_ref(),
        Some(&application.functions[0].params[0].ty_paths[0].segment_spans[0])
    );
}

fn assert_embedded_prelude_imported_related(
    diagnostic: &Diagnostic,
    application: &veln_ast::SurfaceModule,
    prelude_alias_span: &veln_source::SourceSpan,
    terminal_span: &veln_source::SourceSpan,
) {
    assert_eq!(
        related_kinds_and_spans(diagnostic),
        vec![
            (
                "type_alias_declaration".to_string(),
                "Public type alias `app::Facade` is declared here.".to_string(),
                veln_diagnostics::source_span_to_json(&application.aliases[2].span),
            ),
            (
                "type_alias_declaration".to_string(),
                "Public type alias `app::Imported` is declared here.".to_string(),
                veln_diagnostics::source_span_to_json(&application.aliases[1].span),
            ),
            (
                "type_alias_declaration".to_string(),
                "Public type alias `Byte` is declared here.".to_string(),
                veln_diagnostics::source_span_to_json(prelude_alias_span),
            ),
            (
                "private_type_declaration".to_string(),
                "Private type `std::bytes::Byte` is declared here.".to_string(),
                veln_diagnostics::source_span_to_json(terminal_span),
            ),
            (
                "public_result_boundary".to_string(),
                "Public result of function `imported` exposes this refinement.".to_string(),
                veln_diagnostics::source_span_to_json(
                    application.functions[2]
                        .return_type_span
                        .as_ref()
                        .expect("imported result span"),
                ),
            ),
        ]
    );
    assert_eq!(
        diagnostic.span.as_ref(),
        Some(&application.functions[2].return_type_paths[0].segment_spans[0])
    );
}

fn embedded_prelude_visibility_project(
    standard: &veln_ast::SurfaceModule,
) -> (veln_ast::SurfaceModule, veln_ast::SurfaceModule) {
    let source = SourceFile::new(
        "src/main.veln",
        concat!(
            "mod app\n",
            "pub type Local = Byte\n",
            "pub type Imported = prelude::Byte\n",
            "pub type Facade = Imported\n",
            "pub fn direct(value: Byte::Byte) -> prelude::Byte::Byte\n",
            "  value\n",
            "end\n",
            "pub fn direct_alias = direct\n",
            "pub fn local(value: Local::Byte) -> Local::Byte\n",
            "  value\n",
            "end\n",
            "pub fn local_alias = local\n",
            "pub fn imported(value: Imported::Byte) -> Facade::Byte\n",
            "  value\n",
            "end\n",
            "pub fn imported_alias = imported\n",
        ),
    );
    let mut application = lower_surface_ast(&parse(&source).tree);
    application
        .uses
        .push(veln_ast::UseDecl::implicit_standard_prelude(
            "app".to_string(),
            source.span(veln_source::TextRange::new(0, 0)),
        ));
    let mut project = standard.clone();
    project.module = application.module.clone();
    project.uses.extend(application.uses.clone());
    project.aliases.extend(application.aliases.clone());
    project.types.extend(application.types.clone());
    project.functions.extend(application.functions.clone());
    (application, project)
}

fn assert_embedded_prelude_functions_are_unpublished(environment: &TypeEnvironment) {
    for name in [
        "direct",
        "direct_alias",
        "local",
        "local_alias",
        "imported",
        "imported_alias",
    ] {
        assert!(!environment.function_is_published_for_test(name), "{name}");
    }
}

fn assert_visibility_collision_is_quarantined(module: &veln_ast::SurfaceModule) {
    let environment = TypeEnvironment::from_module(module);
    assert!(!environment.function_is_published_for_test("expose"));

    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            "mod consumer\nuse main\n",
        ))
        .tree,
    );
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &environment);
    assert!(
        consumer_environment
            .function_path(
                &["main".to_string(), "expose".to_string()],
                Some("consumer"),
            )
            .is_none()
    );
    assert!(
        consumer_environment
            .unqualified_function_import_candidates("expose", Some("consumer"))
            .is_empty()
    );

    let lowered = lower_checked_surface_module(module);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn mixed_annotation_errors_retain_independent_root_visibility_failures() {
    let diagnostics = diagnostics_for(concat!(
        "type Secret<A>\n",
        "  pub Hidden(A)\n",
        "end\n",
        "pub type State<A>\n",
        "  Hidden(A)\n",
        "  pub Ready(A)\n",
        "end\n",
        "pub type Public<A>\n",
        "  pub Open(A)\n",
        "end\n",
        "pub fn private_base_same(value: Secret<Option>::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn private_variant_same(value: State<Option>::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn private_variant_sibling(value: State<Int>::Hidden | State<Option>::Ready) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn private_variant_result() -> State<Option>::Ready | State<Int>::Hidden\n",
        "  State::Hidden(1)\n",
        "end\n",
        "pub fn public_only(value: Public<Option>::Open | Public<Int>::Open) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let observed = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "type.invalid_annotation"
                || diagnostic.id == "type.variant_refinement_private"
        })
        .map(|diagnostic| {
            (
                diagnostic.id.as_str(),
                veln_diagnostics::diagnostic_to_json(diagnostic).to_json(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        observed.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [
            "type.invalid_annotation",
            "type.variant_refinement_private",
            "type.invalid_annotation",
            "type.variant_refinement_private",
            "type.invalid_annotation",
            "type.variant_refinement_private",
            "type.invalid_annotation",
            "type.variant_refinement_private",
            "type.invalid_annotation",
        ],
        "{diagnostics:#?}"
    );
    assert!(observed[1].1.contains("\"base_type\":\"Secret\""));
    assert!(observed[3].1.contains("\"variant\":\"Hidden\""));
    assert!(observed[3].1.contains("\"boundary\":\"public_parameter\""));
    assert!(observed[5].1.contains("\"variant\":\"Hidden\""));
    assert!(observed[5].1.contains("\"boundary\":\"public_parameter\""));
    assert!(observed[7].1.contains("\"variant\":\"Hidden\""));
    assert!(observed[7].1.contains("\"boundary\":\"public_result\""));
}

#[test]
fn malformed_record_annotations_keep_independent_visibility_checks() {
    let diagnostics = diagnostics_for(concat!(
        "mod main\n",
        "pub type Arg\n",
        "  pub Value\n",
        "end\n",
        "pub type State<A>\n",
        "  Hidden(A)\n",
        "  pub Ready(A)\n",
        "end\n",
        "pub type Box<A>\n",
        "  pub Boxed(A)\n",
        "end\n",
        "pub type Alias = State\n",
        "pub fn record(value: {state: State<Option>::Hidden}) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn generic(value: Box<State<main::Arg>::Hidden>) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn alias(value: Alias<Int>::Hidden | State<Option>::Ready) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "type.invalid_annotation"),
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
            .count(),
        3,
        "{diagnostics:#?}"
    );
}

#[test]
fn alias_qualified_roots_report_outer_alias_and_ordered_declaration_path() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "type Secret\n",
            "  pub Hidden\n",
            "end\n",
            "pub type State\n",
            "  Hidden\n",
            "  pub Ready\n",
            "end\n",
            "pub type SecretAlias = Secret\n",
            "pub type Inner = State\n",
            "pub type Outer = Inner\n",
            "pub fn expose(value: SecretAlias::Hidden) -> Outer::Hidden\n",
            "  State::Hidden\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    let parameter = veln_diagnostics::diagnostic_to_json(&diagnostics[0]).to_json();
    assert!(parameter.contains("\"written_type\":\"SecretAlias::Hidden\""));
    assert!(parameter.contains("\"base_type\":\"main::Secret\""));
    assert!(parameter.contains("\"kind\":\"type_alias_declaration\""));
    let result = veln_diagnostics::diagnostic_to_json(&diagnostics[1]).to_json();
    assert!(result.contains("\"written_type\":\"Outer::Hidden\""));
    let outer = result.find("main::Outer").expect("outer alias note");
    let inner = result.find("main::Inner").expect("inner alias note");
    let terminal = result
        .find("private_variant_declaration")
        .expect("terminal declaration note");
    let boundary = result
        .find("public_result_boundary")
        .expect("public boundary note");
    assert!(
        outer < inner && inner < terminal && terminal < boundary,
        "{result}"
    );
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.column, 22);
    assert_eq!(diagnostics[1].span.as_ref().unwrap().start.column, 46);
}

#[test]
fn distinct_alias_paths_to_one_private_variant_remain_distinct() {
    let diagnostics = visibility_diagnostics(concat!(
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub type First = State\n",
        "pub type Second = State\n",
        "pub fn expose(value: First::Hidden | Second::Hidden | First::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    let written = diagnostics
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert!(written[0].contains("\"written_type\":\"First::Hidden\""));
    assert!(written[1].contains("\"written_type\":\"Second::Hidden\""));
}

#[test]
fn imported_and_locally_extended_alias_paths_reach_the_terminal_declaration() {
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "shared.veln",
            concat!(
                "mod shared\n",
                "pub type State\n",
                "  Hidden\n",
                "end\n",
                "pub type Inner = State\n",
                "pub type Outer = Inner\n",
            ),
        ))
        .tree,
    );
    let base = TypeEnvironment::from_module(&dependency);
    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            concat!(
                "mod consumer\n",
                "use shared\n",
                "pub type Facade = shared::Outer\n",
                "pub fn imported(value: shared::Outer::Hidden) -> Facade::Hidden\n",
                "  value\n",
                "end\n",
            ),
        ))
        .tree,
    );

    let diagnostics = analyze_surface_module_with_base_for_test(&consumer, &base)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    let expected_terminal =
        veln_diagnostics::source_span_to_json(&dependency.types[0].variants[0].span);
    assert_eq!(
        related_kinds_and_spans(&diagnostics[0]),
        vec![
            (
                "type_alias_declaration".to_string(),
                "Public type alias `shared::Outer` is declared here.".to_string(),
                veln_diagnostics::source_span_to_json(&dependency.aliases[1].span),
            ),
            (
                "type_alias_declaration".to_string(),
                "Public type alias `shared::Inner` is declared here.".to_string(),
                veln_diagnostics::source_span_to_json(&dependency.aliases[0].span),
            ),
            (
                "private_variant_declaration".to_string(),
                "Private variant `shared::State::Hidden` is declared here.".to_string(),
                expected_terminal.clone(),
            ),
            (
                "public_parameter_boundary".to_string(),
                "Public parameter `value` of function `imported` exposes this refinement."
                    .to_string(),
                veln_diagnostics::source_span_to_json(&consumer.functions[0].params[0].span),
            ),
        ]
    );
    assert_eq!(
        related_kinds_and_spans(&diagnostics[1]),
        vec![
            (
                "type_alias_declaration".to_string(),
                "Public type alias `consumer::Facade` is declared here.".to_string(),
                veln_diagnostics::source_span_to_json(&consumer.aliases[0].span),
            ),
            (
                "type_alias_declaration".to_string(),
                "Public type alias `shared::Outer` is declared here.".to_string(),
                veln_diagnostics::source_span_to_json(&dependency.aliases[1].span),
            ),
            (
                "type_alias_declaration".to_string(),
                "Public type alias `shared::Inner` is declared here.".to_string(),
                veln_diagnostics::source_span_to_json(&dependency.aliases[0].span),
            ),
            (
                "private_variant_declaration".to_string(),
                "Private variant `shared::State::Hidden` is declared here.".to_string(),
                expected_terminal,
            ),
            (
                "public_result_boundary".to_string(),
                "Public result of function `imported` exposes this refinement.".to_string(),
                veln_diagnostics::source_span_to_json(
                    consumer.functions[0]
                        .return_type_span
                        .as_ref()
                        .expect("result type span"),
                ),
            ),
        ]
    );
    assert_eq!(
        diagnostics[0].span.as_ref(),
        Some(&consumer.functions[0].params[0].ty_paths[0].segment_spans[1])
    );
    assert_eq!(
        diagnostics[1].span.as_ref(),
        Some(&consumer.functions[0].return_type_paths[0].segment_spans[0])
    );
    let imported = veln_diagnostics::diagnostic_to_json(&diagnostics[0]).to_json();
    let extended = veln_diagnostics::diagnostic_to_json(&diagnostics[1]).to_json();
    assert!(imported.contains("\"written_type\":\"shared::Outer::Hidden\""));
    assert!(extended.contains("\"written_type\":\"Facade::Hidden\""));
}

#[test]
fn embedded_prelude_aliases_retain_private_terminal_visibility() {
    let _guard = crate::tests::standard_reuse::standard_reuse_test_lock_for_sibling();
    let standard = crate::types::embedded_standard_surface_module();
    let reusable = prepare_current_reusable_standard_surface_module_environment(&standard);
    let prelude_byte_alias = standard
        .aliases
        .iter()
        .find(|alias| {
            alias.module_name.as_deref() == Some("std::prelude")
                && alias.name.as_deref() == Some("Byte")
        })
        .expect("embedded prelude Byte alias");
    let terminal_byte = standard
        .types
        .iter()
        .find(|declaration| {
            declaration.module_name.as_deref() == Some("std::bytes")
                && declaration.name.as_deref() == Some("Byte")
        })
        .expect("embedded private Byte declaration");

    let (application, project) = embedded_prelude_visibility_project(&standard);

    let diagnostics = check_project_surface_module_with_standard_environment(&project, &reusable)
        .0
        .into_iter()
        .filter(|diagnostic| {
            diagnostic
                .span
                .as_ref()
                .is_some_and(|span| span.file.as_str() == "src/main.veln")
        })
        .collect::<Vec<_>>();
    assert_embedded_prelude_diagnostic_details(&diagnostics);
    assert_embedded_prelude_direct_related(
        &diagnostics[0],
        &application,
        &prelude_byte_alias.span,
        &terminal_byte.span,
    );
    assert_embedded_prelude_imported_related(
        &diagnostics[5],
        &application,
        &prelude_byte_alias.span,
        &terminal_byte.span,
    );

    let environment = TypeEnvironment::from_module_with_standard(&project, &reusable);
    assert_embedded_prelude_functions_are_unpublished(&environment);
}

#[test]
fn cyclic_and_ambiguous_aliases_do_not_manufacture_visibility_diagnostics() {
    let diagnostics = diagnostics_for(concat!(
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub type CycleA = CycleB\n",
        "pub type CycleB = CycleA\n",
        "pub type Choice = State\n",
        "pub type Choice = State\n",
        "pub type Outer = Choice\n",
        "pub fn cycle(value: CycleA::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn ambiguous(value: Choice::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn transitively_ambiguous(value: Outer::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.duplicate")
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.variant_refinement_private"),
        "{diagnostics:#?}"
    );
}

#[test]
fn exact_companion_access_does_not_publish_an_alias_hidden_target_signature() {
    let target = lower_surface_ast(
        &parse(&SourceFile::new(
            "math.veln",
            concat!(
                "mod math\n",
                "pub type State\n",
                "  Hidden\n",
                "end\n",
                "pub type Alias = State\n",
                "pub fn exposed(value: Alias::Hidden) -> Alias::Hidden\n",
                "  value\n",
                "end\n",
                "pub fn exposed_alias = exposed\n",
            ),
        ))
        .tree,
    );
    let companion = lower_surface_ast(
        &parse(&SourceFile::new(
            "math.test.veln",
            concat!(
                "mod math__test_companion\n",
                "use math\n",
                "test can_use_private_variant() -> ()\n",
                "  let value: math::State = math::State::Hidden\n",
                "  ()\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let module = veln_ast::SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: target.aliases,
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: target.types,
        functions: companion
            .functions
            .into_iter()
            .chain(target.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "type.variant_refinement_private",
            "type.variant_refinement_private"
        ],
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic
            .span
            .as_ref()
            .is_some_and(|span| span.file.as_str() == "math.veln")
    }));
    let environment = TypeEnvironment::from_module(&module);
    assert!(!environment.function_is_published_for_test("exposed"));
    assert!(environment.function("exposed_alias").is_none());
}

#[test]
fn inherited_private_declaration_provenance_is_retained() {
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "dependency.veln",
            concat!(
                "mod shared\n",
                "type Secret\n",
                "  pub Hidden\n",
                "end\n",
                "pub type State\n",
                "  Hidden\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let base = TypeEnvironment::from_module(&dependency);
    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            concat!(
                "mod shared\n",
                "pub fn private_base(value: Secret::Hidden) -> ()\n",
                "  ()\n",
                "end\n",
                "pub fn private_variant(value: State::Hidden) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        ))
        .tree,
    );

    let diagnostics = analyze_surface_module_with_base_for_test(&consumer, &base)
        .into_iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    let observed = diagnostics
        .iter()
        .map(|diagnostic| veln_diagnostics::diagnostic_to_json(diagnostic).to_json())
        .collect::<Vec<_>>();
    assert!(
        observed[0].contains(concat!(
            "\"kind\":\"private_type_declaration\"",
            ",\"message\":\"Private type `shared::Secret` is declared here.\"",
            ",\"span\":{\"file\":\"dependency.veln\",\"start\":{\"line\":2,\"column\":1"
        )),
        "{}",
        observed[0]
    );
    assert!(
        observed[0].contains(concat!(
            "\"kind\":\"public_parameter_boundary\"",
            ",\"message\":\"Public parameter `value` of function `private_base` exposes this refinement.\"",
            ",\"span\":{\"file\":\"consumer.veln\",\"start\":{\"line\":2,\"column\":21"
        )),
        "{}",
        observed[0]
    );
    assert!(
        observed[1].contains(concat!(
            "\"kind\":\"private_variant_declaration\"",
            ",\"message\":\"Private variant `shared::State::Hidden` is declared here.\"",
            ",\"span\":{\"file\":\"dependency.veln\",\"start\":{\"line\":6,\"column\":3"
        )),
        "{}",
        observed[1]
    );
    assert!(
        observed[1].contains(concat!(
            "\"kind\":\"public_parameter_boundary\"",
            ",\"message\":\"Public parameter `value` of function `private_variant` exposes this refinement.\"",
            ",\"span\":{\"file\":\"consumer.veln\",\"start\":{\"line\":5,\"column\":24"
        )),
        "{}",
        observed[1]
    );
}

#[test]
fn record_and_named_argument_refinements_are_checked() {
    let source = concat!(
        "pub type State\n",
        "  Hidden\n",
        "end\n",
        "pub type Box<A>\n",
        "  pub Boxed(A)\n",
        "end\n",
        "pub type Alias = State\n",
        "pub fn alias(value: Alias::Hidden) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn nested(value: {state: State::Hidden}) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn named_nested(value: Box<State::Hidden>) -> ()\n",
        "  ()\n",
        "end\n",
        "pub fn record_named_nested(value: {item: Box<State::Hidden>}) -> ()\n",
        "  ()\n",
        "end\n",
        "pub type Envelope\n",
        "  pub Root(value: Option< State::Hidden >::Some)\n",
        "end\n",
        "pub fn root_refinement(value: Option<State::Hidden>::Some) -> ()\n",
        "  ()\n",
        "end\n",
    );
    let diagnostics = diagnostics_for(source);
    let visibility = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();

    assert_eq!(visibility.len(), 6, "{diagnostics:#?}");
    for (diagnostic, written) in [
        (visibility[4], "Option< State::Hidden >::Some"),
        (visibility[5], "Option<State::Hidden>::Some"),
    ] {
        let start = source.find(written).expect("written root refinement");
        let span = diagnostic.span.as_ref().expect("visibility span");
        assert_eq!(span.start.offset, start, "{diagnostic:#?}");
        assert_eq!(span.end.offset, start + written.len(), "{diagnostic:#?}");
        let json = veln_diagnostics::diagnostic_to_json(diagnostic).to_json();
        assert!(
            json.contains("\"written_type\":\"State::Hidden\""),
            "{json}"
        );
        assert!(
            json.contains("\"kind\":\"type_argument\",\"index\":0"),
            "{json}"
        );
    }
}

#[test]
fn alias_collisions_classify_the_descriptor_selected_by_arity() {
    for (declaration, expected_detail) in [
        (
            concat!("type State<A>\n", "  pub Hidden(A)\n", "end\n",),
            "\"base_type\":\"main::State\"",
        ),
        (
            concat!("pub type State<A>\n", "  Hidden(A)\n", "end\n",),
            "\"variant\":\"Hidden\"",
        ),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "mod main\n\
                 pub type Public\n  pub Hidden\nend\n\
                 pub type State = Public\n\
                 {declaration}\
                 pub fn exposed(value: State<Int>::Hidden) -> ()\n  ()\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        let diagnostics = analyze_surface_module(&module);
        let visibility = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
            .collect::<Vec<_>>();
        assert_eq!(visibility.len(), 1, "{diagnostics:#?}");
        let json = veln_diagnostics::diagnostic_to_json(visibility[0]).to_json();
        assert!(json.contains(expected_detail), "{json}");

        let environment = TypeEnvironment::from_module(&module);
        assert!(!environment.function_is_published_for_test("exposed"));
        let consumer = lower_surface_ast(
            &parse(&SourceFile::new(
                "consumer.veln",
                "mod consumer\nuse main\n",
            ))
            .tree,
        );
        let consumer_environment =
            TypeEnvironment::from_module_with_base_for_test(&consumer, &environment);
        assert!(
            consumer_environment
                .function_path(
                    &["main".to_string(), "exposed".to_string()],
                    Some("consumer"),
                )
                .is_none()
        );
        assert!(
            consumer_environment
                .unqualified_function_import_candidates("exposed", Some("consumer"))
                .is_empty()
        );

        let lowered = lower_checked_surface_module(&module);
        assert!(lowered.core.is_none());
        assert!(lowered.ir.is_none());
    }

    let control = SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "pub type Public\n",
            "  pub Open\n",
            "end\n",
            "pub type State = Public\n",
            "pub type State<A>\n",
            "  Hidden(A)\n",
            "end\n",
            "pub fn accepted(value: State::Open) -> State::Open\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&control);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "type.variant_refinement_private"),
        "{diagnostics:#?}"
    );
    assert!(TypeEnvironment::from_module(&module).function_is_published_for_test("accepted"));
}

#[test]
fn private_base_related_span_uses_the_descriptor_selected_by_arity() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "type Secret\n",
            "  pub A\n",
            "end\n",
            "type Secret<T>\n",
            "  pub B(T)\n",
            "end\n",
            "pub fn expose(value: Secret::A) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    let visibility = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(visibility.len(), 1, "{diagnostics:#?}");
    assert_related_span(
        visibility[0],
        "private_type_declaration",
        &module.types[0].span,
    );
    assert_visibility_collision_is_quarantined(&module);
}

#[test]
fn private_variant_related_span_uses_the_descriptor_selected_by_arity() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "pub type State\n",
            "  Hidden\n",
            "end\n",
            "pub type State<T>\n",
            "  Hidden(T)\n",
            "end\n",
            "pub fn expose(value: State::Hidden) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    let visibility = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(visibility.len(), 1, "{diagnostics:#?}");
    assert_related_span(
        visibility[0],
        "private_variant_declaration",
        &module.types[0].variants[0].span,
    );
    assert_visibility_collision_is_quarantined(&module);
}

#[test]
fn visibility_failure_quarantines_the_signature_and_blocks_lowering() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type State\n",
            "  Hidden\n",
            "end\n",
            "pub fn exposed(value: State::Hidden, sibling: Int) -> State::Hidden\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let signature = environment.function("exposed").expect("function signature");
    assert_eq!(signature.params, [Type::Unknown, Type::Unknown]);
    assert_eq!(signature.return_type, Type::Unknown);
    assert!(!environment.function_is_published_for_test("exposed"));

    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered.core.is_none(),
        "invalid declaration must not be checked"
    );
    assert!(
        lowered.ir.is_none(),
        "invalid declaration must not produce IR"
    );
}

#[test]
fn record_visibility_failure_quarantines_aliases_imports_and_checked_output() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "pub type State\n",
            "  Hidden\n",
            "end\n",
            "pub type Alias = State\n",
            "pub fn exposed(value: {callback: fn() -> {inside: State::Hidden} effects [stdio, net], nested: {state: Alias::Hidden}}, sibling: Int) -> () effects [stdio, net]\n",
            "  ()\n",
            "end\n",
            "pub fn exposed_alias = exposed\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let signature = environment.function("exposed").expect("recovery signature");
    assert_eq!(signature.params, [Type::Unknown, Type::Unknown]);
    assert_eq!(signature.return_type, Type::Unknown);
    assert!(!environment.function_is_published_for_test("exposed"));
    assert!(environment.function("exposed_alias").is_none());

    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            "mod consumer\nuse main\n",
        ))
        .tree,
    );
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &environment);
    for name in ["exposed", "exposed_alias"] {
        assert!(
            consumer_environment
                .function_path(&["main".to_string(), name.to_string()], Some("consumer"))
                .is_none(),
            "{name} must not be inherited"
        );
        assert!(
            consumer_environment
                .unqualified_function_import_candidates(name, Some("consumer"))
                .is_empty(),
            "{name} must not be imported"
        );
    }

    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn record_prefix_collision_quarantines_function_and_transitive_aliases() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "pub type State\n",
            "  Hidden\n",
            "end\n",
            "pub fn exposed(value: {callback: fn() -> State::Hidden<Int>, direct: State::Hidden}) -> ()\n",
            "  ()\n",
            "end\n",
            "pub fn direct_alias = exposed\n",
            "pub fn transitive_alias = direct_alias\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    assert!(!environment.function_is_published_for_test("exposed"));
    assert!(environment.function("direct_alias").is_none());
    assert!(environment.function("transitive_alias").is_none());

    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            "mod consumer\nuse main\n",
        ))
        .tree,
    );
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &environment);
    for name in ["exposed", "direct_alias", "transitive_alias"] {
        assert!(
            consumer_environment
                .function_path(&["main".to_string(), name.to_string()], Some("consumer"))
                .is_none(),
            "{name} must not be inherited"
        );
        assert!(
            consumer_environment
                .unqualified_function_import_candidates(name, Some("consumer"))
                .is_empty(),
            "{name} must not be imported"
        );
    }
}

#[test]
fn recovered_visibility_failure_quarantines_aliases_and_checked_output() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "pub type State\n",
            "  Hidden\n",
            "end\n",
            "pub fn exposed(value: state::Hidden) -> state::Hidden\n",
            "  Hidden\n",
            "end\n",
            "pub fn exposed_alias = exposed\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let signature = environment.function("exposed").expect("recovery signature");
    assert_eq!(signature.params, [Type::Unknown]);
    assert_eq!(signature.return_type, Type::Unknown);
    assert!(!environment.function_is_published_for_test("exposed"));
    assert!(environment.function("exposed_alias").is_none());

    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            "mod consumer\nuse main\n",
        ))
        .tree,
    );
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &environment);
    for name in ["exposed", "exposed_alias"] {
        assert!(
            consumer_environment
                .function_path(&["main".to_string(), name.to_string()], Some("consumer"))
                .is_none(),
            "{name} must not be inherited"
        );
        assert!(
            consumer_environment
                .unqualified_function_import_candidates(name, Some("consumer"))
                .is_empty(),
            "{name} must not be an import candidate"
        );
    }

    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered.core.is_none(),
        "invalid declaration must not be checked"
    );
    assert!(
        lowered.ir.is_none(),
        "invalid declaration must not produce IR"
    );
}

#[test]
fn mixed_visibility_failure_quarantines_callable_facts_aliases_and_lowering() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "pub type State<A>\n",
            "  Hidden(A)\n",
            "  pub Ready(A)\n",
            "end\n",
            "pub fn exposed(value: State<Int>::Hidden | State<Option>::Ready, sibling: Int) -> State<Option>::Ready | State<Int>::Hidden\n",
            "  value\n",
            "end\n",
            "pub fn exposed_alias = exposed\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);
    let signature = environment.function("exposed").expect("recovery signature");
    assert_eq!(signature.params, [Type::Unknown, Type::Unknown]);
    assert_eq!(signature.return_type, Type::Unknown);
    assert!(!environment.function_is_published_for_test("exposed"));
    assert!(environment.function("exposed_alias").is_none());

    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "consumer.veln",
            "mod consumer\nuse main\n",
        ))
        .tree,
    );
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &environment);
    for name in ["exposed", "exposed_alias"] {
        assert!(
            consumer_environment
                .function_path(&["main".to_string(), name.to_string()], Some("consumer"))
                .is_none(),
            "{name} must not be inherited"
        );
        assert!(
            consumer_environment
                .unqualified_function_import_candidates(name, Some("consumer"))
                .is_empty(),
            "{name} must not be an import candidate"
        );
    }

    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered.core.is_none(),
        "invalid declaration must not be checked"
    );
    assert!(
        lowered.ir.is_none(),
        "invalid declaration must not produce IR"
    );
}

#[test]
fn visibility_failure_is_not_published_to_a_consumer_environment() {
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "dep.veln",
            concat!(
                "mod dep\n",
                "pub type State\n",
                "  Hidden\n",
                "end\n",
                "pub fn exposed(value: State::Hidden) -> State::Hidden\n",
                "  value\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let dependency_environment = TypeEnvironment::from_module(&dependency);
    assert!(!dependency_environment.function_is_published_for_test("exposed"));

    let consumer =
        lower_surface_ast(&parse(&SourceFile::new("main.veln", "mod main\nuse dep\n")).tree);
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &dependency_environment);
    assert!(consumer_environment.function("exposed").is_none());
    assert!(
        consumer_environment
            .function_path(&["dep".to_string(), "exposed".to_string()], Some("main"))
            .is_none()
    );
    assert!(
        consumer_environment
            .unqualified_function_import_candidates("exposed", Some("main"))
            .is_empty()
    );
}

#[test]
fn visibility_failure_is_not_republished_through_function_aliases() {
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "dep.veln",
            concat!(
                "mod dep\n",
                "pub type State\n",
                "  Hidden\n",
                "  pub Ready\n",
                "end\n",
                "pub type Alias = State\n",
                "pub fn exposed(value: Alias::Hidden) -> Alias::Hidden\n",
                "  value\n",
                "end\n",
                "pub fn first_alias = exposed\n",
                "pub fn second_alias = exposed\n",
                "pub fn accepted(value: Alias::Ready) -> Alias::Ready\n",
                "  value\n",
                "end\n",
                "pub fn accepted_alias = accepted\n",
            ),
        ))
        .tree,
    );
    let dependency_environment = TypeEnvironment::from_module(&dependency);
    assert_visibility_alias_dependency(&dependency_environment);
    assert_visibility_alias_consumer_outcomes(&dependency_environment);

    let bridge = lower_surface_ast(
        &parse(&SourceFile::new(
            "bridge.veln",
            concat!(
                "mod bridge\n",
                "use dep\n",
                "pub fn bridged_invalid = dep::first_alias\n",
                "pub fn bridged_valid = dep::accepted_alias\n",
            ),
        ))
        .tree,
    );
    let bridge_environment =
        TypeEnvironment::from_module_with_base_for_test(&bridge, &dependency_environment);
    assert!(bridge_environment.function("bridged_invalid").is_none());
    assert!(bridge_environment.function_is_published_for_test("bridged_valid"));

    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "main.veln",
            "mod main\nuse dep\nuse bridge\n",
        ))
        .tree,
    );
    let consumer_environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &bridge_environment);
    assert_visibility_alias_publication_boundary(&consumer_environment);
}

fn assert_visibility_alias_consumer_outcomes(dependency_environment: &TypeEnvironment) {
    let invalid_consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "invalid_consumer.veln",
            concat!(
                "mod invalid_consumer\n",
                "use dep\n",
                "fn consume(value: Int) -> Int\n",
                "  dep::first_alias(value)\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let (invalid_diagnostics, invalid_lowered) =
        check_project_surface_module_with_base_for_test(&invalid_consumer, dependency_environment);
    assert!(!invalid_diagnostics.is_empty());
    assert!(
        invalid_lowered.core.is_none(),
        "an unpublished alias must not recover checked core"
    );
    assert!(
        invalid_lowered.ir.is_none(),
        "an unpublished alias must not recover IR"
    );

    let valid_consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "valid_consumer.veln",
            concat!(
                "mod valid_consumer\n",
                "use dep\n",
                "fn consume(value: State::Ready) -> State::Ready\n",
                "  dep::accepted_alias(value)\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let (valid_diagnostics, valid_lowered) =
        check_project_surface_module_with_base_for_test(&valid_consumer, dependency_environment);
    assert!(valid_diagnostics.is_empty(), "{valid_diagnostics:#?}");
    assert!(valid_lowered.core.is_some());
    assert!(valid_lowered.ir.is_some());
}

fn assert_visibility_alias_dependency(dependency_environment: &TypeEnvironment) {
    let exposed = dependency_environment
        .function("exposed")
        .expect("same-module recovery signature");
    assert_eq!(exposed.params, [Type::Unknown]);
    assert_eq!(exposed.return_type, Type::Unknown);
    assert!(!dependency_environment.function_is_published_for_test("exposed"));
    assert!(dependency_environment.function("first_alias").is_none());
    assert!(dependency_environment.function("second_alias").is_none());
    assert!(dependency_environment.function_is_published_for_test("accepted_alias"));
}

fn assert_visibility_alias_publication_boundary(consumer_environment: &TypeEnvironment) {
    for (module, name) in [
        ("dep", "exposed"),
        ("dep", "first_alias"),
        ("dep", "second_alias"),
        ("bridge", "bridged_invalid"),
    ] {
        assert!(
            consumer_environment
                .function_path(&[module.to_string(), name.to_string()], Some("main"))
                .is_none(),
            "{module}::{name} must remain unpublished"
        );
        assert!(
            consumer_environment
                .unqualified_function_import_candidates(name, Some("main"))
                .is_empty(),
            "{name} must not be an import candidate"
        );
    }
    assert!(
        consumer_environment
            .function_path(
                &["bridge".to_string(), "bridged_valid".to_string()],
                Some("main"),
            )
            .is_some(),
        "an alias chain to a valid public function must remain published"
    );
}

#[test]
fn unpublished_local_function_keeps_alias_target_shadowing() {
    let imported = lower_surface_ast(
        &parse(&SourceFile::new(
            "imported.veln",
            concat!(
                "mod imported\n",
                "pub fn exposed(value: Int) -> Int\n",
                "  value\n",
                "end\n",
            ),
        ))
        .tree,
    );
    let imported_environment = TypeEnvironment::from_module(&imported);

    let application = lower_surface_ast(
        &parse(&SourceFile::new(
            "application.veln",
            concat!(
                "mod application\n",
                "use imported\n",
                "pub type State\n",
                "  Hidden\n",
                "end\n",
                "pub fn exposed(value: State::Hidden) -> State::Hidden\n",
                "  value\n",
                "end\n",
                "pub fn alias = exposed\n",
            ),
        ))
        .tree,
    );
    let application_environment =
        TypeEnvironment::from_module_with_base_for_test(&application, &imported_environment);

    let exposed = application_environment
        .function("exposed")
        .expect("local recovery signature must continue to shadow the imported function");
    assert_eq!(exposed.module_name.as_deref(), Some("application"));
    assert_eq!(exposed.params, [Type::Unknown]);
    assert!(
        application_environment
            .function_path(
                &["imported".to_string(), "exposed".to_string()],
                Some("application"),
            )
            .is_some(),
        "the imported namesake must remain available when explicitly qualified"
    );
    assert!(
        application_environment.function("alias").is_none(),
        "an unpublished local target must not be skipped in favor of an imported namesake"
    );
}
