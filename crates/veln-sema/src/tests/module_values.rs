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
    assert_eq!(
        lowered_with_incomplete_diagnostics
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "module.value_not_executable")
            .count(),
        3
    );
    assert_eq!(
        lowered_with_incomplete_diagnostics.diagnostics[0].id,
        "caller.unrelated"
    );
    assert!(lowered_with_incomplete_diagnostics.core.is_none());
    assert!(lowered_with_incomplete_diagnostics.ir.is_none());

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
        &module,
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
            SourceFile::new("first.veln", "let first: Int = config::later\n"),
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
    assert!(unsupported.iter().all(|diagnostic| {
        diagnostic.related.iter().any(|related| {
            related.as_object().is_some_and(|entries| {
                entries.iter().any(|(name, value)| {
                    name == "kind" && value.as_text() == Some("permitted_initializers")
                })
            })
        })
    }));
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

#[test]
fn constructor_arity_and_unresolved_names_keep_their_existing_diagnostics() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Active(count: Int)\n",
            "end\n",
            "let wrong_arity: State = Active()\n",
            "let unknown: Int = missing()\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "core.constructor_arity_mismatch"
                && diagnostic.message == "state constructor expects 1 argument, but got 0"
        }),
        "{diagnostics:#?}"
    );
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `missing`"
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "value.module_initializer_unsupported"),
        "{diagnostics:#?}"
    );
}

#[test]
fn module_value_cycles_report_once_per_component_on_a_closing_reference() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let alpha: Int = beta\n",
            "let beta: Int = alpha\n",
            "let self_cycle: Int = self_cycle\n",
            "let independent: Int = 1\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);
    let cycles = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_cycle")
        .collect::<Vec<_>>();

    assert_eq!(cycles.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        cycles[0].span.as_ref(),
        Some(&module.module_values[1].initializer.span)
    );
    assert_eq!(
        cycles[1].span.as_ref(),
        Some(&module.module_values[2].initializer.span)
    );
    assert_eq!(
        cycles[0].details,
        veln_diagnostics::JsonValue::object([
            ("phase", veln_diagnostics::JsonValue::string("type_check")),
            ("name", veln_diagnostics::JsonValue::string("beta")),
            ("target", veln_diagnostics::JsonValue::string("alpha")),
        ])
    );
    assert_eq!(
        cycles[0].related,
        vec![
            veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("cycle_declaration"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string(
                        "Module value `alpha` participates in this initializer cycle.",
                    ),
                ),
                (
                    "span",
                    crate::diagnostics::span_json(&module.module_values[0].span),
                ),
            ]),
            veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("cycle_declaration"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string(
                        "Module value `beta` participates in this initializer cycle.",
                    ),
                ),
                (
                    "span",
                    crate::diagnostics::span_json(&module.module_values[1].span),
                ),
            ]),
            veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("cycle_reference"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string(
                        "Module value `alpha` depends on `beta` here.",
                    ),
                ),
                (
                    "span",
                    crate::diagnostics::span_json(&module.module_values[0].initializer.span),
                ),
            ]),
            veln_diagnostics::JsonValue::object([
                (
                    "kind",
                    veln_diagnostics::JsonValue::string("cycle_reference"),
                ),
                (
                    "message",
                    veln_diagnostics::JsonValue::string(
                        "Module value `beta` depends on `alpha` here.",
                    ),
                ),
                (
                    "span",
                    crate::diagnostics::span_json(&module.module_values[1].initializer.span),
                ),
            ]),
        ]
    );
}

#[test]
fn module_value_cycle_selection_is_independent_of_declaration_order() {
    fn cycle_facts(text: &str) -> (String, String, Vec<String>) {
        let source = SourceFile::new("main.veln", text);
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        let diagnostics = analyze_surface_module(&module);
        let cycle = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "value.module_initializer_cycle")
            .expect("cycle diagnostic");
        let detail = |name: &str| {
            cycle
                .details
                .as_object()
                .and_then(|entries| {
                    entries
                        .iter()
                        .find(|(field, _)| field == name)
                        .and_then(|(_, value)| value.as_text())
                })
                .expect("cycle detail")
                .to_string()
        };
        let related = cycle
            .related
            .iter()
            .filter_map(|note| {
                note.as_object().and_then(|entries| {
                    entries
                        .iter()
                        .find(|(field, _)| field == "message")
                        .and_then(|(_, value)| value.as_text())
                        .map(str::to_owned)
                })
            })
            .collect();
        (detail("name"), detail("target"), related)
    }

    let forward = cycle_facts("let alpha: Int = beta\nlet beta: Int = alpha\n");
    let reversed = cycle_facts("let beta: Int = alpha\nlet alpha: Int = beta\n");

    assert_eq!(forward, reversed);
}

#[test]
fn module_values_share_the_function_value_namespace() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn answer() -> Int\n",
            "  1\n",
            "end\n",
            "let answer: Int = 2\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.duplicate"
                && diagnostic.message == "duplicate module value declaration name `answer`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn duplicate_module_values_are_quarantined_independently_of_source_order() {
    fn assert_quarantine(first: &str, second: &str) {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "{first}{second}let dependent: Int = shared\nlet independent: Int = 1\nfn reads_dependent() -> String\n  dependent\nend\nfn reads_independent() -> String\n  independent\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);
        let mismatches = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .collect::<Vec<_>>();

        assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
        assert!(
            mismatches[0]
                .message
                .contains("expected `String`, but found `Int`"),
            "{diagnostics:#?}"
        );
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.id == "name.duplicate")
                .count(),
            1,
            "{diagnostics:#?}"
        );
    }

    assert_quarantine("let shared: Int = 1\n", "let shared: String = \"one\"\n");
    assert_quarantine("let shared: String = \"one\"\n", "let shared: Int = 1\n");
}

#[test]
fn callable_module_value_collisions_quarantine_dependents_but_keep_unrelated_facts() {
    for collision in [
        concat!("fn shared() -> Int\n", "  1\n", "end\n"),
        concat!(
            "fn original() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn shared = original\n",
        ),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "{collision}let shared: Int = 1\nlet dependent: Int = shared\nlet independent: Int = 1\nfn reads_dependent() -> String\n  dependent\nend\nfn reads_independent() -> String\n  independent\nend\n"
            ),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);
        let mismatches = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .collect::<Vec<_>>();

        assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
        assert!(
            mismatches[0]
                .message
                .contains("expected `String`, but found `Int`"),
            "{diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "name.duplicate"),
            "{diagnostics:#?}"
        );
    }
}

#[test]
fn module_value_call_targets_do_not_fall_through_to_callable_lookup() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let answer: Int = 42\n",
            "fn use_answer() -> Int\n",
            "  answer()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.not_callable")
        .expect("non-callable module value diagnostic");
    assert_eq!(diagnostic.message, "module value `answer` is not callable");
    assert_diagnostic_span(diagnostic, 3, 3, 3, 9);
    let detail = |name: &str| {
        diagnostic.details.as_object().and_then(|entries| {
            entries
                .iter()
                .find(|(field, _)| field == name)
                .and_then(|(_, value)| value.as_text())
        })
    };
    assert_eq!(detail("phase"), Some("type_check"));
    assert_eq!(detail("constraint"), Some("call_target"));
    assert_eq!(detail("actual"), Some("module_value"));
    assert_eq!(detail("name"), Some("answer"));
    assert_eq!(diagnostic.related.len(), 1);
    assert_eq!(
        diagnostic.related[0],
        veln_diagnostics::JsonValue::object([
            (
                "kind",
                veln_diagnostics::JsonValue::string("module_value_declaration"),
            ),
            (
                "message",
                veln_diagnostics::JsonValue::string("Module value `answer` is declared here."),
            ),
            (
                "span",
                crate::diagnostics::span_json(&module.module_values[0].span),
            ),
        ])
    );
}

#[test]
fn complete_dependency_discovery_finds_hidden_cycles_and_respects_local_shadowing() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn keep(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "let operator_left: Int = operator_right + 1\n",
            "let operator_right: Int = operator_left\n",
            "let call_left: Int = keep(call_right)\n",
            "let call_right: Int = call_left\n",
            "let shadowed: Int = 0\n",
            "let target: Int = shadowed\n",
            "fn scoped() -> Int\n",
            "  begin\n",
            "    let target: Int = 1\n",
            "    target\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut module = lower_surface_ast(&parsed.tree);
    let scoped_initializer = module
        .functions
        .iter()
        .find(|function| function.name.as_deref() == Some("scoped"))
        .and_then(|function| function.body.first())
        .and_then(|line| match &line.kind {
            veln_ast::BodyLineKind::Expr { expr } => Some(expr.clone()),
            _ => None,
        })
        .expect("scoped begin expression");
    module.module_values[4].initializer = scoped_initializer;

    let diagnostics = analyze_surface_module(&module);
    let cycles = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "value.module_initializer_cycle")
        .collect::<Vec<_>>();

    assert_eq!(cycles.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        cycles
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "module value reference `call_left` closes an initializer cycle",
            "module value reference `operator_left` closes an initializer cycle",
        ]
    );
    assert!(
        cycles
            .iter()
            .all(|diagnostic| !diagnostic.message.contains("target")),
        "{diagnostics:#?}"
    );
}

#[test]
fn unsupported_acyclic_dependencies_are_quarantined_transitively() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let base: Int = 1\n",
            "let invalid: Int = base + 1\n",
            "let dependent: Int = invalid\n",
            "let independent: Int = 1\n",
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
    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();

    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        mismatches[0].span.as_ref().map(|span| span.start.line),
        Some(9)
    );
}

#[test]
fn invalid_module_value_dependencies_are_withheld_without_hiding_independent_facts() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn compute() -> Int\n",
            "  1\n",
            "end\n",
            "let invalid: Int = compute()\n",
            "let dependent: Int = invalid\n",
            "let independent: Int = 1\n",
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
    let mismatches = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "type.mismatch")
        .collect::<Vec<_>>();

    assert_eq!(mismatches.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        mismatches[0].span.as_ref().map(|span| span.start.line),
        Some(11)
    );
}
