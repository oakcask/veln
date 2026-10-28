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

fn module_with_bare_imported_values(count: usize) -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        module_values: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };
    for index in 0..count {
        let module_name = format!("values_{index}");
        let source = SourceFile::new(
            format!("values_{index}.veln"),
            format!("pub let value_{index}: Int = {index}\n"),
        );
        let module = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            module_name,
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.module_values.extend(module.module_values);
    }
    let source = SourceFile::new(
        "app.veln",
        (0..count)
            .map(|index| format!("use values_{index}\n"))
            .collect::<String>(),
    );
    let app = veln_ast::lower_surface_ast_with_module_identity(
        &parse(&source).tree,
        "app".to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    );
    merged.uses.extend(app.uses);
    merged
}

fn module_with_imported_named_annotations(count: usize) -> SurfaceModule {
    let mut merged = SurfaceModule {
        module: None,
        uses: Vec::new(),
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: Vec::new(),
        module_values: Vec::new(),
        functions: Vec::new(),
        invalid_names: Vec::new(),
    };
    for index in 0..count {
        let module_name = format!("types_{index}");
        let source = SourceFile::new(
            format!("types_{index}.veln"),
            "pub type Item\n  Only\nend\n",
        );
        let module = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&source).tree,
            module_name,
            source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.types.extend(module.types);
        let consumer_name = format!("app_{index}");
        let consumer_source = SourceFile::new(
            format!("app_{index}.veln"),
            format!("use types_{index}\nlet value_{index}: types_{index}::Item = 0\n"),
        );
        let consumer = veln_ast::lower_surface_ast_with_module_identity(
            &parse(&consumer_source).tree,
            consumer_name,
            consumer_source.span(veln_source::TextRange::new(0, 0)),
        );
        merged.module = consumer.module;
        merged.uses.extend(consumer.uses);
        merged.module_values.extend(consumer.module_values);
    }
    merged
}

fn module_with_colliding_qualified_imports(count: usize) -> SurfaceModule {
    let imports = (0..count)
        .map(|index| format!("use shared from \"package_{index}\"\n"))
        .collect::<String>();
    let owner_source = SourceFile::new("app.veln", imports);
    let mut merged = veln_ast::lower_surface_ast_with_package_module_identity(
        &parse(&owner_source).tree,
        "owner/pkg".to_string(),
        "app".to_string(),
        owner_source.span(veln_source::TextRange::new(0, 0)),
    );
    for use_decl in &mut merged.uses {
        use_decl.name = "shared".to_string();
    }
    for index in 0..count {
        let declaration_count = if index == 0 { 2 } else { 1 };
        for declaration_index in 0..declaration_count {
            let source = SourceFile::new(
                format!("package_{index}/shared_{declaration_index}.veln"),
                "pub let answer: Int = 1\n",
            );
            let mut module = veln_ast::lower_surface_ast_with_package_module_identity(
                &parse(&source).tree,
                format!("package_{index}"),
                "shared".to_string(),
                source.span(veln_source::TextRange::new(0, 0)),
            );
            for value in &mut module.module_values {
                value.module_name = Some("shared".to_string());
            }
            merged.module_values.extend(module.module_values);
        }
    }
    merged
}

fn colliding_qualified_import_work(
    count: usize,
    lookup_count: usize,
) -> crate::types::QualifiedModuleValueImportWork {
    let module = module_with_colliding_qualified_imports(count);
    crate::types::reset_qualified_module_value_import_work();
    let environment = crate::types::TypeEnvironment::from_module(&module);
    for _ in 0..lookup_count {
        assert!(matches!(
            environment.module_value_path_in_owner(
                &["shared".to_string(), "answer".to_string()],
                Some("owner/pkg::app"),
                Some("owner/pkg"),
            ),
            crate::types::ModuleValueLookup::Ambiguous
        ));
        assert!(matches!(
            environment.module_value_path_in_owner(
                &["shared".to_string(), "missing".to_string()],
                Some("owner/pkg::app"),
                Some("owner/pkg"),
            ),
            crate::types::ModuleValueLookup::Missing
        ));
    }
    crate::types::take_qualified_module_value_import_work()
}

#[test]
#[ignore = "supporting wall-clock evidence; deterministic lookup work is asserted separately"]
fn bare_imported_module_value_lookup_reports_adjacent_size_timings() {
    for count in [150, 300, 600, 1_200] {
        let module = module_with_bare_imported_values(count);
        let started = std::time::Instant::now();
        let environment = crate::types::TypeEnvironment::from_module(&module);
        for index in 0..count {
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &[format!("value_{index}")],
                    Some("app"),
                    None,
                ),
                crate::types::ModuleValueLookup::Available(_)
            ));
        }
        eprintln!(
            "bare imported module value lookup: {count} imports and references in {:?}",
            started.elapsed()
        );
    }
}

#[test]
#[ignore = "supporting wall-clock evidence; deterministic import work is asserted separately"]
fn imported_named_annotation_analysis_reports_adjacent_size_timings() {
    for count in [100, 200, 400, 800] {
        let module = module_with_imported_named_annotations(count);
        let environment = crate::types::TypeEnvironment::from_module(&module);
        let started = std::time::Instant::now();
        let diagnostics = crate::pipeline::analyze_surface_module_with_environment_for_test(
            &module,
            &environment,
        );
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic.id != "name.unresolved" || !diagnostic.message.contains("Item")
            }),
            "{diagnostics:#?}"
        );
        eprintln!(
            "imported named annotations: {count} imports and annotations in {:?}",
            started.elapsed()
        );
    }
}

#[test]
#[ignore = "supporting wall-clock evidence; deterministic collision work is asserted separately"]
fn qualified_module_value_colliding_imports_report_adjacent_size_timings() {
    for count in [128, 256, 512, 1_024] {
        let started = std::time::Instant::now();
        let work = colliding_qualified_import_work(count, count);
        eprintln!(
            "qualified module value collisions: {count} imports and references in {:?}, work={work:?}",
            started.elapsed()
        );

        let started = std::time::Instant::now();
        let control = colliding_qualified_import_work(count, 1);
        eprintln!(
            "qualified module value collision control: {count} imports and one reference in {:?}, work={control:?}",
            started.elapsed()
        );
    }
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
        assert_eq!(work.path_lookups, count * 2, "{work:#?}");
        assert_eq!(work.indexed_diagnostics, count * 2, "{work:#?}");
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
                            "Use literals, records, ADT constructors, or eligible same-module or imported module values.",
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
            SourceFile::new("values.veln", "pub let answer: String = \"imported\"\n"),
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
fn bare_and_qualified_paths_select_the_same_module_value_declaration() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        ("app", SourceFile::new("app.veln", "use values\n")),
    ]);
    let environment = crate::types::TypeEnvironment::from_module(&module);
    let bare =
        match environment.module_value_path_in_owner(&["answer".to_string()], Some("app"), None) {
            crate::types::ModuleValueLookup::Available(value) => value,
            _ => panic!("expected bare declaration"),
        };
    let qualified = match environment.module_value_path_in_owner(
        &["values".to_string(), "answer".to_string()],
        Some("app"),
        None,
    ) {
        crate::types::ModuleValueLookup::Available(value) => value,
        _ => panic!("expected qualified declaration"),
    };

    assert!(std::ptr::eq(bare, qualified));
}

#[test]
fn qualified_module_value_exact_path_no_fallback_is_stable() {
    let analyze = |reverse_sources: bool, reverse_imports: bool| {
        let alpha_exact = (
            "alpha::exact",
            SourceFile::new("alpha/exact.veln", "pub let other: Int = 1\n"),
        );
        let beta_exact = (
            "beta::exact",
            SourceFile::new("beta/exact.veln", "pub let answer: Int = 1\n"),
        );
        let imports = if reverse_imports {
            "use beta::exact\nuse alpha::exact\n"
        } else {
            "use alpha::exact\nuse beta::exact\n"
        };
        let app = (
            "app",
            SourceFile::new(
                "app.veln",
                format!("{imports}fn exact() -> Int\n  alpha::exact::answer\nend\n"),
            ),
        );
        let sources = if reverse_sources {
            vec![beta_exact, alpha_exact, app]
        } else {
            vec![alpha_exact, beta_exact, app]
        };
        analyze_surface_module(&merged_modules_with_identities(sources))
            .into_iter()
            .filter(|diagnostic| {
                matches!(diagnostic.id.as_str(), "name.ambiguous" | "name.unresolved")
            })
            .map(|diagnostic| (diagnostic.id, diagnostic.message))
            .collect::<Vec<_>>()
    };

    let expected = analyze(false, false);
    assert_eq!(expected.len(), 1, "{expected:#?}");
    assert!(expected.iter().any(|(id, message)| {
        id == "name.unresolved" && message.contains("alpha::exact::answer")
    }));
    assert_eq!(analyze(true, false), expected);
    assert_eq!(analyze(false, true), expected);
    assert_eq!(analyze(true, true), expected);
}

#[test]
fn quarantined_imports_do_not_create_qualified_module_value_ambiguity() {
    fn assert_case(
        first_import: &str,
        second_import: &str,
        qualified_value: &str,
        reverse_sources: bool,
    ) {
        let values = (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        );
        let alpha = (
            "alpha",
            SourceFile::new("alpha.veln", "pub let answer: Int = 1\n"),
        );
        let beta = (
            "beta",
            SourceFile::new("beta.veln", "pub let answer: Int = 2\n"),
        );
        let app = (
            "app",
            SourceFile::new(
                "app.veln",
                format!(
                    "{first_import}\n{second_import}\nfn read() -> Int\n  {qualified_value}\nend\n"
                ),
            ),
        );
        let sources = if reverse_sources {
            vec![app, beta, alpha, values]
        } else {
            vec![values, alpha, beta, app]
        };
        let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.duplicate")
                .count(),
            1,
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
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "name.ambiguous"),
            "{diagnostics:#?}"
        );
    }

    for reverse_sources in [false, true] {
        assert_case(
            "use values",
            "use values as config",
            "values::answer",
            reverse_sources,
        );
        assert_case(
            "use values as config",
            "use values",
            "values::answer",
            reverse_sources,
        );
        assert_case(
            "use alpha as shared",
            "use beta as shared",
            "shared::answer",
            reverse_sources,
        );
        assert_case(
            "use beta as shared",
            "use alpha as shared",
            "shared::answer",
            reverse_sources,
        );
    }
}

#[test]
fn quarantined_exact_import_blocks_same_owner_module_value_fallback() {
    let analyze = |reverse_sources: bool, reverse_imports: bool| {
        let imports = if reverse_imports {
            concat!(
                "use values as config from \"dep/pkg\"\n",
                "use values from \"dep/pkg\"\n",
            )
        } else {
            concat!(
                "use values from \"dep/pkg\"\n",
                "use values as config from \"dep/pkg\"\n",
            )
        };
        let owner_source = SourceFile::new(
            "values.veln",
            format!("{imports}let answer: Int = 1\nfn read() -> Int\n  values::answer\nend\n"),
        );
        let owner = veln_ast::lower_surface_ast_with_package_module_identity(
            &parse(&owner_source).tree,
            "owner/pkg".to_string(),
            "values".to_string(),
            owner_source.span(veln_source::TextRange::new(0, 0)),
        );
        let dependency_source = SourceFile::new(
            "vendor/dep/values.veln",
            "pub let answer: String = \"dep\"\n",
        );
        let dependency = veln_ast::lower_surface_ast_with_package_module_identity(
            &parse(&dependency_source).tree,
            "dep/pkg".to_string(),
            "values".to_string(),
            dependency_source.span(veln_source::TextRange::new(0, 0)),
        );
        let (mut merged, other) = if reverse_sources {
            (dependency, owner)
        } else {
            (owner, dependency)
        };
        merged.uses.extend(other.uses);
        merged.module_values.extend(other.module_values);
        merged.functions.extend(other.functions);

        analyze_surface_module(&merged)
            .into_iter()
            .filter(|diagnostic| {
                matches!(
                    diagnostic.id.as_str(),
                    "name.duplicate" | "name.unresolved" | "name.ambiguous" | "type.mismatch"
                )
            })
            .map(|diagnostic| (diagnostic.id, diagnostic.message))
            .collect::<Vec<_>>()
    };

    let expected = analyze(false, false);
    assert_eq!(
        expected
            .iter()
            .filter(|(id, _)| id == "name.duplicate")
            .count(),
        1,
        "{expected:#?}"
    );
    assert_eq!(
        expected
            .iter()
            .filter(|(id, message)| id == "name.unresolved" && message.contains("values::answer"))
            .count(),
        1,
        "{expected:#?}"
    );
    assert!(
        expected
            .iter()
            .all(|(id, _)| !matches!(id.as_str(), "name.ambiguous" | "type.mismatch")),
        "{expected:#?}"
    );
    assert_eq!(analyze(true, false), expected);
    assert_eq!(analyze(false, true), expected);
    assert_eq!(analyze(true, true), expected);
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
fn same_owner_function_shadows_bare_imported_module_value_in_value_and_call_positions() {
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
                    "fn answer() -> Int\n",
                    "  7\n",
                    "end\n",
                    "fn callback() -> fn() -> Int\n",
                    "  answer\n",
                    "end\n",
                    "fn call() -> Int\n",
                    "  answer()\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            !matches!(
                diagnostic.id.as_str(),
                "name.ambiguous" | "name.unresolved" | "type.not_callable" | "type.mismatch"
            )
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn imported_module_value_call_target_does_not_fall_through_to_imported_function() {
    let module = merged_modules_with_identities(vec![
        (
            "values",
            SourceFile::new("values.veln", "pub let answer: Int = 42\n"),
        ),
        (
            "functions",
            SourceFile::new("functions.veln", "pub fn answer() -> Int\n  7\nend\n"),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                "use values\nuse functions\nfn call() -> Int\n  values::answer()\nend\n",
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.not_callable")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.ambiguous"),
        "{diagnostics:#?}"
    );
}

#[test]
fn bare_imported_module_value_lookup_work_is_bounded_per_reference() {
    fn work_for(count: usize) -> usize {
        let module = module_with_bare_imported_values(count);
        let environment = crate::types::TypeEnvironment::from_module(&module);
        crate::types::reset_bare_module_value_lookup_work();
        for index in 0..count {
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &[format!("value_{index}")],
                    Some("app"),
                    None,
                ),
                crate::types::ModuleValueLookup::Available(_)
            ));
        }
        crate::types::take_bare_module_value_lookup_work()
    }

    assert_eq!(work_for(64), 64);
    assert_eq!(work_for(128), 128);
}

#[test]
fn qualified_module_value_import_work_includes_construction_and_lookup() {
    fn work_for(count: usize) -> crate::types::QualifiedModuleValueImportWork {
        let module = module_with_bare_imported_values(count);
        crate::types::reset_qualified_module_value_import_work();
        let environment = crate::types::TypeEnvironment::from_module(&module);
        for index in 0..count {
            assert!(matches!(
                environment.module_value_path_in_owner(
                    &[format!("values_{index}"), format!("value_{index}"),],
                    Some("app"),
                    None,
                ),
                crate::types::ModuleValueLookup::Available(_)
            ));
        }
        crate::types::take_qualified_module_value_import_work()
    }

    let smaller = work_for(64);
    let larger = work_for(128);
    assert!(smaller.index_build >= 64, "{smaller:?}");
    assert!(smaller.target_classification >= 64, "{smaller:?}");
    assert!(smaller.member_materialization >= 64, "{smaller:?}");
    assert_eq!(smaller.same_owner_blocking, 64);
    assert_eq!(smaller.selection, 64 * 3);
    assert_eq!(smaller.visibility, 64);
    assert!(larger.index_build <= smaller.index_build * 2 + 4);
    assert!(larger.target_classification <= smaller.target_classification * 2 + 4);
    assert!(larger.member_materialization <= smaller.member_materialization * 2 + 4);
    assert_eq!(larger.same_owner_blocking, smaller.same_owner_blocking * 2);
    assert_eq!(larger.selection, smaller.selection * 2);
    assert_eq!(larger.visibility, smaller.visibility * 2);
}

#[test]
fn colliding_qualified_module_value_import_work_is_bounded_per_reference() {
    let smaller = colliding_qualified_import_work(64, 64);
    let larger = colliding_qualified_import_work(128, 128);
    let smaller_control = colliding_qualified_import_work(64, 1);
    let larger_control = colliding_qualified_import_work(128, 1);

    assert!(
        larger.index_build <= smaller.index_build * 2 + 8,
        "{smaller:?} -> {larger:?}"
    );
    assert!(
        larger.target_classification <= smaller.target_classification * 2 + 8,
        "{smaller:?} -> {larger:?}"
    );
    assert!(
        larger.member_materialization <= smaller.member_materialization * 2 + 8,
        "{smaller:?} -> {larger:?}"
    );
    assert_eq!(larger.same_owner_blocking, smaller.same_owner_blocking * 2);
    assert_eq!(larger.selection, smaller.selection * 2);
    assert_eq!(larger.visibility, smaller.visibility * 2);

    assert_eq!(smaller_control.same_owner_blocking, 2);
    assert_eq!(larger_control.same_owner_blocking, 2);
    assert_eq!(smaller_control.selection, 4);
    assert_eq!(larger_control.selection, 4);
    assert_eq!(smaller_control.visibility, 2);
    assert_eq!(larger_control.visibility, 2);
}

#[test]
fn imported_named_annotation_lookup_work_is_adjacent_linear() {
    fn work_for(count: usize) -> (usize, usize) {
        let module = module_with_imported_named_annotations(count);
        let environment = crate::types::TypeEnvironment::from_module(&module);
        crate::adt::reset_import_path_comparisons();
        crate::adt::reset_qualified_path_work();
        let diagnostics = crate::pipeline::analyze_surface_module_with_environment_for_test(
            &module,
            &environment,
        );
        assert!(
            diagnostics.iter().all(|diagnostic| {
                diagnostic.id != "name.unresolved" || !diagnostic.message.contains("Item")
            }),
            "{diagnostics:#?}"
        );
        (
            crate::adt::import_path_comparisons(),
            crate::adt::qualified_path_work(),
        )
    }

    let work = [64, 128, 256].map(work_for);
    for (count, pair) in [64, 128, 256].into_iter().zip(work) {
        assert!(
            pair.0 >= count,
            "closedness lookups must be counted: {pair:?}"
        );
    }
    for adjacent in work.windows(2) {
        let [smaller, larger] = adjacent else {
            unreachable!();
        };
        assert!(
            larger.0 <= smaller.0 * 2 + 16,
            "import work must be adjacent-linear: {smaller:?} -> {larger:?}"
        );
        assert!(
            larger.1 <= smaller.1 * 3,
            "qualified path byte work must remain bounded: {smaller:?} -> {larger:?}"
        );
    }
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
                    "let selected: Int = answer\n",
                    "fn read() -> Int\n",
                    "  selected\n",
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
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id != "type.mismatch"
            || !diagnostic
                .span
                .as_ref()
                .is_some_and(|span| span.start.line == 3)
    }));
}

#[test]
fn invalid_imported_module_value_does_not_reserve_a_bare_exposure() {
    for reverse_sources in [false, true] {
        let alpha = (
            "alpha",
            SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
        );
        let app = (
            "app",
            SourceFile::new("app.veln", "use alpha\nfn read() -> Int\n  shared\nend\n"),
        );
        let sources = if reverse_sources {
            vec![app, alpha]
        } else {
            vec![alpha, app]
        };

        let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));

        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == "unresolved value `shared`"
            }),
            "reverse_sources={reverse_sources}: {diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.id != "name.ambiguous"),
            "reverse_sources={reverse_sources}: {diagnostics:#?}"
        );
    }
}

#[test]
fn valid_imported_value_survives_an_invalid_namesake_in_every_order() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let alpha = (
                "alpha",
                SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
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

            let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));

            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.id != "name.ambiguous"
                        && !(diagnostic.id == "name.unresolved"
                            && diagnostic.message == "unresolved value `shared`")
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
        }
    }
}

#[test]
fn module_value_initializers_observe_final_imported_availability_in_every_order() {
    for reverse_sources in [false, true] {
        for reverse_imports in [false, true] {
            let alpha = (
                "alpha",
                SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
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
                    format!(
                        "{imports}let selected: Int = shared\nfn read() -> Int\n  selected\nend\n"
                    ),
                ),
            );
            let sources = if reverse_sources {
                vec![app, beta, alpha]
            } else {
                vec![alpha, beta, app]
            };

            let diagnostics = analyze_surface_module(&merged_modules_with_identities(sources));

            assert!(
                diagnostics.iter().all(|diagnostic| {
                    diagnostic.id != "name.ambiguous"
                        && !(diagnostic.id == "name.unresolved"
                            && diagnostic.message.contains("shared"))
                        && !(diagnostic.id == "name.unresolved"
                            && diagnostic.message.contains("selected"))
                }),
                "reverse_sources={reverse_sources}, reverse_imports={reverse_imports}: {diagnostics:#?}"
            );
        }
    }

    for reference in ["shared", "alpha::shared"] {
        let module = merged_modules_with_identities(vec![
            (
                "alpha",
                SourceFile::new("alpha.veln", "pub let shared: Missing = 0\n"),
            ),
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!(
                        "use alpha\nlet selected: Int = {reference}\nfn read() -> Int\n  selected\nend\n"
                    ),
                ),
            ),
        ]);
        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "name.unresolved"
                    && diagnostic.message == format!("unresolved value `{reference}`")
            }),
            "reference={reference}: {diagnostics:#?}"
        );
    }
}

#[test]
fn cross_module_initializer_cycles_remain_outside_dependency_quarantine() {
    let module = merged_modules_with_identities(vec![
        (
            "alpha",
            SourceFile::new("alpha.veln", "use beta\npub let left: Int = right\n"),
        ),
        (
            "beta",
            SourceFile::new("beta.veln", "use alpha\npub let right: Int = left\n"),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().all(|diagnostic| {
            !matches!(
                diagnostic.id.as_str(),
                "name.unresolved" | "name.ambiguous" | "value.module_initializer_cycle"
            )
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_imported_value_does_not_collide_with_a_valid_imported_function() {
    for reverse_imports in [false, true] {
        let imports = if reverse_imports {
            "use functions\nuse values\n"
        } else {
            "use values\nuse functions\n"
        };
        let module = merged_modules_with_identities(vec![
            (
                "values",
                SourceFile::new("values.veln", "pub let shared: Missing = 0\n"),
            ),
            (
                "functions",
                SourceFile::new("functions.veln", "pub fn shared() -> Int\n  2\nend\n"),
            ),
            (
                "app",
                SourceFile::new(
                    "app.veln",
                    format!("{imports}fn read() -> Int\n  shared()\nend\n"),
                ),
            ),
        ]);

        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics.iter().all(|diagnostic| {
                !matches!(
                    diagnostic.id.as_str(),
                    "name.ambiguous" | "type.not_callable"
                ) && !(diagnostic.id == "name.unresolved" && diagnostic.message.contains("shared"))
            }),
            "reverse_imports={reverse_imports}: {diagnostics:#?}"
        );
    }
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
fn public_module_value_visibility_recurses_through_annotation_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Secret\n",
            "  pub Value\n",
            "end\n",
            "pub type State\n",
            "  Hidden\n",
            "  Other\n",
            "end\n",
            "pub type Box<T>\n",
            "  pub Box(value: T)\n",
            "end\n",
            "pub let named_generic: Box<Secret> = 0\n",
            "pub let named_function: fn(Secret) -> Secret = 0\n",
            "pub let named_record: {field: Secret} = 0\n",
            "pub let refined_generic: Box<State::Hidden> = 0\n",
            "pub let refined_function: fn(State::Hidden) -> State::Hidden = 0\n",
            "pub let refined_record: {field: State::Hidden} = 0\n",
            "pub let refined_union: State::Hidden | State::Other = 0\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.public_annotation_private")
            .count(),
        3,
        "{diagnostics:#?}"
    );
    let refinements = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.variant_refinement_private")
        .collect::<Vec<_>>();
    assert_eq!(refinements.len(), 6, "{diagnostics:#?}");
    let rendered = format!("{refinements:#?}");
    for segment in [
        "type_argument",
        "function_parameter",
        "function_result",
        "record_field",
        "union_alternative",
    ] {
        assert!(rendered.contains(segment), "missing {segment}: {rendered}");
    }
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
