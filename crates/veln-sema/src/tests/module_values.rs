use super::*;

fn module_with_values(count: usize) -> SurfaceModule {
    let source = SourceFile::new(
        "main.veln",
        (0..count)
            .map(|index| format!("let value_{index}: Int = {index}\n"))
            .collect::<String>(),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

fn module_with_value_chain(count: usize) -> SurfaceModule {
    let source = SourceFile::new(
        "main.veln",
        (0..count)
            .map(|index| {
                if index == 0 {
                    "let value_0: Int = 0\n".to_string()
                } else {
                    format!("let value_{index}: Int = value_{}\n", index - 1)
                }
            })
            .collect::<String>(),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

fn module_with_unresolved_value_annotations(count: usize) -> SurfaceModule {
    let source = SourceFile::new(
        "main.veln",
        (0..count)
            .map(|index| format!("let value_{index}: Missing{index} = {index}\n"))
            .collect::<String>(),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

#[test]
#[ignore = "supporting wall-clock evidence; deterministic scaling is asserted separately"]
fn module_value_gate_reports_adjacent_size_timings() {
    for count in [2_000, 4_000, 8_000] {
        let module = module_with_values(count);
        let diagnostics = analyze_surface_module(&module);
        let started = std::time::Instant::now();
        let lowered = lower_analyzed_surface_module(&module, diagnostics);
        let elapsed = started.elapsed();

        assert!(lowered.core.is_none());
        assert!(lowered.ir.is_none());
        eprintln!("module value gate: {count} declarations in {elapsed:?}");
    }
}

#[test]
#[ignore = "supporting wall-clock evidence; deterministic semantics are asserted separately"]
fn module_value_analysis_reports_adjacent_size_timings() {
    for count in [2_000, 4_000, 8_000] {
        let module = module_with_values(count);
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        let elapsed = started.elapsed();

        assert_eq!(diagnostics.len(), count, "{diagnostics:#?}");
        eprintln!("module value analysis: {count} declarations in {elapsed:?}");
    }
}

#[test]
#[ignore = "supporting wall-clock evidence for failure-heavy annotation scaling"]
fn module_value_unresolved_annotation_analysis_reports_adjacent_size_timings() {
    for count in [2_000, 4_000, 8_000] {
        let module = module_with_unresolved_value_annotations(count);
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        let elapsed = started.elapsed();

        assert!(diagnostics.len() >= count, "{diagnostics:#?}");
        eprintln!("module value unresolved annotations: {count} declarations in {elapsed:?}");
    }
}

#[test]
#[ignore = "supporting wall-clock evidence for dependency graph scaling"]
fn module_value_dependency_analysis_reports_adjacent_size_timings() {
    for count in [2_000, 4_000, 8_000] {
        let module = module_with_value_chain(count);
        let started = std::time::Instant::now();
        let diagnostics = analyze_surface_module(&module);
        let elapsed = started.elapsed();

        assert_eq!(diagnostics.len(), count, "{diagnostics:#?}");
        eprintln!("module value dependencies: {count} declarations in {elapsed:?}");
    }
}

#[test]
fn module_value_gate_work_grows_linearly_with_declarations() {
    fn work_for(count: usize) -> crate::pipeline::ModuleValueGateWork {
        let module = module_with_values(count);
        let diagnostics = analyze_surface_module(&module);
        assert_eq!(diagnostics.len(), count, "{diagnostics:#?}");

        reset_module_value_gate_work();
        let lowered = lower_analyzed_surface_module(&module, diagnostics);
        let work = take_module_value_gate_work();

        assert!(lowered.core.is_none());
        assert!(lowered.ir.is_none());
        assert_eq!(work.indexed_values, count, "{work:#?}");
        assert_eq!(work.scanned_diagnostics, count, "{work:#?}");
        assert_eq!(work.span_index_lookups, count, "{work:#?}");
        assert_eq!(work.emitted_canonical_diagnostics, count, "{work:#?}");
        work
    }

    const N: usize = 64;
    let work_n = work_for(N);
    let work_2n = work_for(2 * N);
    assert_eq!(work_2n.total(), 2 * work_n.total());
}

#[test]
fn module_value_annotation_dedup_work_grows_linearly_with_failures() {
    fn work_for(count: usize) -> (usize, usize) {
        let module = module_with_unresolved_value_annotations(count);
        reset_module_value_annotation_dedup_work();
        let diagnostics = analyze_surface_module(&module);
        let work = take_module_value_annotation_dedup_work();

        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.unresolved")
                .count(),
            count,
            "{diagnostics:#?}"
        );
        assert_eq!(work.path_lookups, count, "{work:#?}");
        assert_eq!(work.indexed_diagnostics, count, "{work:#?}");
        (work.path_lookups, work.indexed_diagnostics)
    }

    const N: usize = 64;
    let work_n = work_for(N);
    let work_2n = work_for(2 * N);
    assert_eq!(work_2n.0, 2 * work_n.0);
    assert_eq!(work_2n.1, 2 * work_n.1);
}

#[test]
fn module_values_block_checked_core_and_typed_ir_at_the_shared_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let answer: Int = 42\n",
            "let backup: Int = 7\n",
            "let final_value: Int = 9\n",
            "fn ordinary() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    assert_eq!(module.module_values.len(), 3);
    assert_eq!(module.functions.len(), 1);

    let lowered = lower_checked_surface_module(&module);
    assert_module_value_gate_diagnostics(&lowered);

    let unrelated = Diagnostic::new(
        "caller.unrelated",
        veln_diagnostics::Severity::Warning,
        DiagnosticKind::Module,
        "caller-owned unrelated warning",
        None,
        veln_diagnostics::JsonValue::Null,
    );
    let lowered_with_incomplete_diagnostics =
        lower_analyzed_surface_module(&module, vec![unrelated.clone()]);
    assert_incomplete_diagnostics_are_completed(&lowered_with_incomplete_diagnostics);
    assert_colliding_gate_diagnostics_are_replaced(&module, unrelated);
}

fn assert_module_value_gate_diagnostics(lowered: &LoweredSurfaceModule) {
    let diagnostics = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "module.value_not_executable")
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 3, "{:#?}", lowered.diagnostics);
    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "module value declaration `answer` is not executable",
            "module value declaration `backup` is not executable",
            "module value declaration `final_value` is not executable",
        ]
    );
    let diagnostic = diagnostics[0];
    assert_eq!(
        diagnostic.message,
        "module value declaration `answer` is not executable"
    );
    assert_eq!(diagnostic.kind, DiagnosticKind::Module);
    assert_eq!(diagnostic.severity, veln_diagnostics::Severity::Error);
    assert_eq!(
        diagnostic.details,
        veln_diagnostics::JsonValue::object([
            ("phase", veln_diagnostics::JsonValue::string("module")),
            ("field", veln_diagnostics::JsonValue::string("module_value"),),
            ("name", veln_diagnostics::JsonValue::string("answer")),
        ])
    );
    assert_eq!(
        diagnostic.related,
        vec![veln_diagnostics::JsonValue::object([(
            "message",
            veln_diagnostics::JsonValue::string(
                "The declaration is semantically checked, but checked-core lowering and code generation do not support module values yet.",
            ),
        )])]
    );
    assert_diagnostic_span(diagnostic, 1, 1, 2, 1);
    assert_diagnostic_span(diagnostics[1], 2, 1, 3, 1);
    assert_diagnostic_span(diagnostics[2], 3, 1, 4, 1);
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

fn assert_incomplete_diagnostics_are_completed(lowered: &LoweredSurfaceModule) {
    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "module.value_not_executable")
            .count(),
        3
    );
    assert_eq!(lowered.diagnostics[0].id, "caller.unrelated");
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

fn assert_colliding_gate_diagnostics_are_replaced(module: &SurfaceModule, unrelated: Diagnostic) {
    let colliding_warning = |index: usize| {
        Diagnostic::new(
            "module.value_not_executable",
            veln_diagnostics::Severity::Warning,
            DiagnosticKind::Module,
            "caller-owned warning",
            Some(module.module_values[index].span.clone()),
            veln_diagnostics::JsonValue::Null,
        )
    };
    let lowered_with_colliding_warning = lower_analyzed_surface_module(
        module,
        vec![
            unrelated,
            colliding_warning(1),
            colliding_warning(1),
            colliding_warning(0),
        ],
    );
    let blockers = lowered_with_colliding_warning
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "module.value_not_executable")
        .collect::<Vec<_>>();
    assert_eq!(
        blockers.len(),
        3,
        "{:#?}",
        lowered_with_colliding_warning.diagnostics
    );
    assert_eq!(
        blockers
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "module value declaration `backup` is not executable",
            "module value declaration `answer` is not executable",
            "module value declaration `final_value` is not executable",
        ]
    );
    assert!(
        blockers
            .iter()
            .all(|diagnostic| diagnostic.severity == veln_diagnostics::Severity::Error)
    );
    assert_eq!(
        lowered_with_colliding_warning
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "caller.unrelated")
            .count(),
        1
    );
    assert!(lowered_with_colliding_warning.core.is_none());
    assert!(lowered_with_colliding_warning.ir.is_none());
}

#[test]
fn module_value_initializers_accept_finite_data_and_forward_references() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Idle\n",
            "  Active(count: Int)\n",
            "end\n",
            "let defaults: { state: State, attempts: Int } = { state: initial, attempts: -3 }\n",
            "let initial: State = Active(3)\n",
            "fn attempts() -> Int\n",
            "  defaults.attempts\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "module.value_not_executable")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id == "module.value_not_executable"
                || diagnostic.severity != veln_diagnostics::Severity::Error
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_initializers_accept_the_complete_finite_data_surface() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Idle\n",
            "  Active(count: Int)\n",
            "end\n",
            "let boolean: Bool = true\n",
            "let text: String = \"ready\"\n",
            "let nothing: () = ()\n",
            "let grouped: Bool = (boolean)\n",
            "let idle: State = Idle\n",
            "let active: State = Active(later)\n",
            "let optional: Option<Int> = Some(later)\n",
            "let outcome: Result<Int, String> = Ok(later)\n",
            "let items: List<Int> = Cons(later, Nil)\n",
            "let nested: { state: State, data: { optional: Option<Int>, items: List<Int> } } = { state: Active(later), data: { optional: Some(later), items: Cons(later, Nil) } }\n",
            "let later: Int = 3\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == veln_diagnostics::Severity::Error)
        .collect::<Vec<_>>();

    assert_eq!(errors.len(), module.module_values.len(), "{diagnostics:#?}");
    assert!(
        errors
            .iter()
            .all(|diagnostic| diagnostic.id == "module.value_not_executable"),
        "{diagnostics:#?}"
    );
}

#[test]
fn contextual_empty_dictionaries_are_not_admitted_as_empty_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Container\n",
            "  Wrap(items: Dict<Int, Int>)\n",
            "end\n",
            "pub type ContainerAlias = Container\n",
            "let direct: Dict<Int, Int> = {}\n",
            "let nested: { items: Dict<Int, Int> } = { items: {} }\n",
            "let payload: Container = Wrap({})\n",
            "let mismatched_payload: Int = Wrap({})\n",
            "let aliased: ContainerAlias = Wrap({})\n",
            "let structural: {} = {}\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let unsupported = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_unsupported")
        .collect::<Vec<_>>();

    assert_eq!(unsupported.len(), 5, "{diagnostics:#?}");
    assert_eq!(
        unsupported
            .iter()
            .map(|diagnostic| {
                let span = diagnostic.span.as_ref().expect("unsupported span");
                (span.start.line, span.end.offset - span.start.offset)
            })
            .collect::<Vec<_>>(),
        vec![(5, 2), (6, 2), (7, 2), (8, 2), (9, 2)]
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "value.module_initializer_unsupported"
                || diagnostic.span.as_ref() != Some(&module.module_values[5].initializer.span)
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn qualified_same_module_values_resolve_across_declaring_sources() {
    let module = merged_modules_with_identities(vec![
        (
            "app::config",
            SourceFile::new(
                "first.veln",
                concat!(
                    "let bare: Int = later\n",
                    "let leaf: Int = config::later\n",
                    "let complete: Int = app::config::later\n",
                ),
            ),
        ),
        (
            "app::config",
            SourceFile::new("later.veln", "let later: Int = 2\n"),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id == "module.value_not_executable"
                || diagnostic.severity != veln_diagnostics::Severity::Error
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn exact_import_precedes_same_module_leaf_value_qualification() {
    let module = merged_modules_with_identities(vec![
        (
            "app::config",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use config\n",
                    "let answer: Int = config::answer()\n",
                    "fn calls_import() -> Int\n",
                    "  config::answer()\n",
                    "end\n",
                ),
            ),
        ),
        (
            "config",
            SourceFile::new(
                "config.veln",
                concat!("pub fn answer() -> Int\n", "  1\n", "end\n"),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "value.module_initializer_unsupported")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            !matches!(
                diagnostic.id.as_str(),
                "type.not_callable" | "value.module_initializer_cycle" | "name.unresolved"
            )
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn same_module_leaf_qualification_rejects_unwritten_prefixes() {
    let module = merged_modules_with_identities(vec![
        (
            "app::config",
            SourceFile::new("first.veln", "let first: Int = bogus::config::later\n"),
        ),
        (
            "app::config",
            SourceFile::new("later.veln", "let later: Int = 2\n"),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved value `bogus::config::later`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_initializers_report_smallest_unsupported_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn compute() -> Int\n",
            "  1\n",
            "end\n",
            "let computed: Int = compute()\n",
            "let nested: { value: Int } = { value: 1 + 2 }\n",
            "let floating: Float = 1.0\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let unsupported = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_unsupported")
        .collect::<Vec<_>>();

    assert_eq!(unsupported.len(), 3, "{diagnostics:#?}");
    assert_diagnostic_span(unsupported[0], 4, 21, 4, 30);
    assert_diagnostic_span(unsupported[1], 5, 39, 5, 44);
    assert_diagnostic_span(unsupported[2], 6, 23, 6, 26);
    for (index, (diagnostic, name)) in unsupported
        .iter()
        .zip(["computed", "nested", "floating"])
        .enumerate()
    {
        let detail = |field: &str| {
            diagnostic.details.as_object().and_then(|entries| {
                entries
                    .iter()
                    .find(|(entry, _)| entry == field)
                    .and_then(|(_, value)| value.as_text())
            })
        };
        assert_eq!(detail("phase"), Some("type_check"));
        assert_eq!(detail("name"), Some(name));
        assert_eq!(
            diagnostic.related,
            vec![
                veln_diagnostics::JsonValue::object([
                    (
                        "kind",
                        veln_diagnostics::JsonValue::string("module_value_declaration"),
                    ),
                    (
                        "message",
                        veln_diagnostics::JsonValue::string(format!(
                            "Module value `{name}` is declared here."
                        )),
                    ),
                    (
                        "span",
                        crate::diagnostics::span_json(&module.module_values[index].span),
                    ),
                ]),
                veln_diagnostics::JsonValue::object([
                    (
                        "kind",
                        veln_diagnostics::JsonValue::string("permitted_initializers"),
                    ),
                    (
                        "message",
                        veln_diagnostics::JsonValue::string(
                            "Use literals, records, ADT constructors, or same-module module values.",
                        ),
                    ),
                ]),
            ]
        );
    }
}

#[test]
fn imported_module_values_follow_bare_qualified_and_alias_exposure() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use values\n",
                    "let bare: Int = answer\n",
                    "let qualified: Int = values::answer\n",
                    "fn read() -> Int\n",
                    "  values::answer\n",
                    "end\n",
                ),
            ),
        ),
        (
            "aliased",
            SourceFile::new(
                "aliased.veln",
                concat!(
                    "use values as config\n",
                    "let qualified: Int = config::answer\n",
                    "fn bare_is_hidden() -> Int\n",
                    "  answer\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved value `answer`"
        }),
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "value.module_initializer_unsupported"
                && diagnostic.id != "type.annotation_mismatch"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn reusable_standard_base_preserves_bare_prelude_module_values() {
    let standard_source = SourceFile::new("prelude.veln", "pub let answer: Int = 42\n");
    let standard = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&standard_source).tree,
        veln_stdlib::PACKAGE_NAME.to_string(),
        "prelude".to_string(),
        standard_source.span(veln_source::TextRange::new(0, 0)),
    );
    let base = crate::types::TypeEnvironment::from_module(&standard);

    let app_source = SourceFile::new("app.veln", "use prelude from \"std\"\n");
    let mut app = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&app_source).tree,
        "app".to_string(),
        app_source.span(veln_source::TextRange::new(0, 0)),
    );
    app.uses[0].name = "std::prelude".to_string();
    let environment = crate::types::TypeEnvironment::from_module_with_base_for_test(&app, &base);

    assert!(matches!(
        environment.module_value_path_in_owner(&["answer".to_string()], Some("app"), None),
        crate::types::ModuleValueLookup::Available(_)
    ));
}

#[test]
fn imported_module_value_ambiguity_includes_function_data_collisions() {
    let module = merged_modules_with_identities(vec![
        (
            "alpha",
            SourceFile::new("alpha.veln", "pub let shared: Int = 1\n"),
        ),
        (
            "beta",
            SourceFile::new("beta.veln", "pub fn shared() -> Int\n  2\nend\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use alpha\n",
                    "use beta\n",
                    "fn read() -> Int\n",
                    "  shared\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.ambiguous"),
        "{diagnostics:#?}"
    );
}

#[test]
fn imported_module_value_ambiguity_is_independent_of_source_and_import_order() {
    let analyze = |reverse_sources: bool, reverse_imports: bool| {
        let alpha = (
            "alpha",
            SourceFile::new("alpha.veln", "pub let shared: Int = 1\n"),
        );
        let beta = (
            "beta",
            SourceFile::new("beta.veln", "pub let shared: Int = 2\n"),
        );
        let imports = if reverse_imports {
            "use beta\nuse alpha\n"
        } else {
            "use alpha\nuse beta\n"
        };
        let app = (
            "app",
            SourceFile::new(
                "app.veln",
                format!("{imports}fn read() -> Int\n  shared\nend\n"),
            ),
        );
        let sources = if reverse_sources {
            vec![beta, alpha, app]
        } else {
            vec![alpha, beta, app]
        };
        analyze_surface_module(&merged_modules_with_identities(sources))
            .into_iter()
            .filter(|diagnostic| diagnostic.id == "name.ambiguous")
            .map(|diagnostic| (diagnostic.id, diagnostic.message))
            .collect::<Vec<_>>()
    };

    let expected = analyze(false, false);
    assert_eq!(expected.len(), 1, "{expected:#?}");
    assert_eq!(analyze(true, false), expected);
    assert_eq!(analyze(false, true), expected);
    assert_eq!(analyze(true, true), expected);
}

#[test]
fn invalid_local_module_value_shadows_an_imported_value() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use values\n",
                    "let answer: Missing = 0\n",
                    "fn read() -> Int\n",
                    "  answer\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn exact_companion_can_read_private_module_values_but_other_modules_cannot() {
    let module = merged_modules_with_identities(vec![
        (
            "math",
            SourceFile::new("math.veln", "let secret: Int = 42\n"),
        ),
        (
            "math.test",
            SourceFile::new(
                "math.test.veln",
                "use math\nfn read() -> Int\n  math::secret\nend\n",
            ),
        ),
        (
            "other",
            SourceFile::new(
                "other.veln",
                "use math\nfn read() -> Int\n  math::secret\nend\n",
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn public_module_value_annotations_reject_private_named_types_and_refinements() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Secret\n",
            "  pub Value\n",
            "end\n",
            "pub type State\n",
            "  pub Visible\n",
            "  Hidden\n",
            "end\n",
            "pub let named: Secret = Value\n",
            "pub let refined: State::Hidden = Hidden\n",
            "pub let hidden_initializer: State = Hidden\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.public_annotation_private")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
            .count(),
        1,
        "{diagnostics:#?}"
    );
}

#[test]
fn non_name_calls_are_rejected_and_quarantined_at_the_call_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Boxed\n",
            "  Box(value: Int)\n",
            "end\n",
            "let direct: Int = (1)()\n",
            "let nested: { value: Int } = { value: ((2))(3) }\n",
            "let payload: Boxed = Box((4)())\n",
            "let dependent: Int = direct\n",
            "let independent: Int = 1\n",
            "fn reads_direct() -> String\n",
            "  direct\n",
            "end\n",
            "fn reads_dependent() -> String\n",
            "  dependent\n",
            "end\n",
            "fn reads_independent() -> String\n",
            "  independent\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let unsupported = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_unsupported")
        .collect::<Vec<_>>();

    assert_eq!(unsupported.len(), 3, "{diagnostics:#?}");
    assert_diagnostic_span(unsupported[0], 4, 19, 4, 24);
    assert_diagnostic_span(unsupported[1], 5, 39, 5, 47);
    assert_diagnostic_span(unsupported[2], 6, 26, 6, 31);
    for (diagnostic, declaration) in unsupported.iter().zip(&module.module_values[..3]) {
        assert_eq!(
            diagnostic.related.first(),
            Some(&veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("module_value_declaration"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string(format!(
                        "Module value `{}` is declared here.",
                        declaration.name.as_deref().expect("module value name")
                    )),
                ),
                ("span", crate::diagnostics::span_json(&declaration.span),),
            ]))
        );
    }

    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();
    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        mismatches[0].span.as_ref().map(|span| span.start.line),
        Some(16)
    );
}

#[test]
fn module_value_annotations_are_checked_even_when_unused() {
    let source = SourceFile::new("main.veln", "let unused: String = 42\n");
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message.contains("expected `String`")
                && diagnostic.message.contains("found `Int`")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_annotations_must_be_closed_and_quarantine_dependents() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let open: Option<T> = None\n",
            "let dependent: Option<T> = open\n",
            "fn read() -> String\n",
            "  dependent\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let unresolved_type_parameters = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved type `T`"
        })
        .count();

    assert_eq!(unresolved_type_parameters, 2, "{diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "type.mismatch" || !diagnostic.message.contains("found `Option<T>`")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_function_annotations_validate_effect_rows_and_labels() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let unbound: fn() -> Int effects [...E] = 1\n",
            "let unknown: fn() -> Int effects [MissingAudit] = 1\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let unbound = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "effect.row_unbound")
        .unwrap_or_else(|| panic!("expected unbound effect row: {diagnostics:#?}"));
    let unbound_span = unbound.span.as_ref().expect("unbound effect row span");
    assert_eq!(
        (unbound_span.start.line, unbound_span.start.column),
        (1, 35)
    );
    assert_eq!((unbound_span.end.line, unbound_span.end.column), (1, 39));

    let unknown = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "effect.unknown")
        .unwrap_or_else(|| panic!("expected unknown effect label: {diagnostics:#?}"));
    let unknown_span = unknown.span.as_ref().expect("unknown effect label span");
    assert_eq!(
        (unknown_span.start.line, unknown_span.start.column),
        (2, 35)
    );
    assert_eq!((unknown_span.end.line, unknown_span.end.column), (2, 47));
}

#[test]
fn module_value_annotations_and_initializers_use_existing_type_name_rules() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Idle\n",
            "end\n",
            "let good: State::Idle = State::Idle\n",
            "let wrong: Option<Int>::Absent = None\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.invalid_case"
                || !diagnostic.message.contains("module name `State`")
        }),
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.variant_refinement_unknown"
                && diagnostic.message.contains("variant `Absent`")
        }),
        "{diagnostics:#?}"
    );
}

mod invariants;
