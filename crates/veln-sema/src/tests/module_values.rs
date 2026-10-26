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
                "The declaration is available to parsing and formatting, but semantic analysis and code generation do not support module values yet.",
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
