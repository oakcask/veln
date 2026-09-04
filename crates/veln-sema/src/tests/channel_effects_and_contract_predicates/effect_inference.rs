use super::*;

#[test]
fn infers_transitive_private_helper_effects_from_body() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn say(text: String) -> () effects [stdio]\n",
            "  stdio::println(text)\n",
            "end\n",
            "fn greet(text: String) -> ()\n",
            "  say(text)\n",
            "end\n",
            "pub fn main() -> ()\n",
            "  greet(\"hello\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"inferred_effects\":[\"stdio\"]"));
    assert!(details.contains("\"symbol\":\"greet\""));
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn infers_import_alias_call_effects_from_function_body() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.console\n",
            "pub fn main() -> ()\n",
            "  console::say(\"hello\")\n",
            "end\n",
        ),
    );
    let console_source = SourceFile::new(
        "console.veln",
        concat!(
            "mod app.console\n",
            "pub fn say(text: String) -> () effects [stdio]\n",
            "  stdio::println(text)\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let console = lower_surface_ast(&parse(&console_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: main.types.into_iter().chain(console.types).collect(),
        functions: main
            .functions
            .into_iter()
            .chain(console.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"inferred_effects\":[\"stdio\"]"));
    assert!(details.contains("\"symbol\":\"console::say\""));
}

#[test]
fn function_typed_value_calls_infer_declared_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(callback: fn(String) -> () effects [stdio]) -> ()\n",
            "  callback(\"hello\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"symbol\":\"callback\""));
}

#[test]
fn bounded_effect_inference_preserves_private_propagation_paths() {
    struct Case {
        name: &'static str,
        sources: Vec<SourceFile>,
        expected_symbol: &'static str,
        expected_effects: &'static str,
        expect_handler_work: bool,
    }

    let cases = vec![
        Case {
            name: "private helper chain",
            sources: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "fn terminal() -> ()\n",
                    "  stdio::println(\"terminal\")\n",
                    "end\n",
                    "\n",
                    "fn middle() -> ()\n",
                    "  terminal()\n",
                    "end\n",
                    "\n",
                    "pub fn main() -> ()\n",
                    "  middle()\n",
                    "end\n",
                ),
            )],
            expected_symbol: "middle",
            expected_effects: "\"inferred_effects\":[\"stdio\"]",
            expect_handler_work: false,
        },
        Case {
            name: "private handler retained effects",
            sources: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "effect Ask\n",
                    "  value() -> Int\n",
                    "end\n",
                    "\n",
                    "fn provide(offset: Int) -> Int\n",
                    "  stdio::println(\"provider\")\n",
                    "  offset + 1\n",
                    "end\n",
                    "\n",
                    "handler ask(offset: Int) handles Ask\n",
                    "  value() => provide(offset)\n",
                    "end\n",
                    "\n",
                    "fn compute() -> Int effects [Ask]\n",
                    "  perform Ask::value()\n",
                    "end\n",
                    "\n",
                    "pub fn main() -> Int\n",
                    "  handle compute() with ask(1)\n",
                    "end\n",
                ),
            )],
            expected_symbol: "ask",
            expected_effects: "\"inferred_effects\":[\"stdio\"]",
            expect_handler_work: true,
        },
        Case {
            name: "cyclic dependency graph",
            sources: vec![SourceFile::new(
                "main.veln",
                concat!(
                    "fn left() -> ()\n",
                    "  right()\n",
                    "end\n",
                    "\n",
                    "fn right() -> ()\n",
                    "  left()\n",
                    "  stdio::println(\"cycle\")\n",
                    "end\n",
                    "\n",
                    "pub fn main() -> ()\n",
                    "  left()\n",
                    "end\n",
                ),
            )],
            expected_symbol: "left",
            expected_effects: "\"inferred_effects\":[\"stdio\"]",
            expect_handler_work: false,
        },
    ];

    for case in cases {
        crate::types::effect_inference_counters::reset();
        let module = merged_modules(case.sources);

        let diagnostics = analyze_surface_module(&module);
        let counters = crate::types::effect_inference_counters::snapshot();

        assert_eq!(diagnostics.len(), 1, "{}: {diagnostics:#?}", case.name);
        assert_eq!(diagnostics[0].id, "effect.missing_public", "{}", case.name);
        let details = diagnostics[0].details.to_json();
        assert!(
            details.contains(case.expected_symbol),
            "{}: {details}",
            case.name
        );
        assert!(
            details.contains(case.expected_effects),
            "{}: {details}",
            case.name
        );
        assert!(counters.dependency_discovery_scans > 0, "{counters:#?}");
        assert!(counters.function_body_collections > 0, "{counters:#?}");
        assert_eq!(
            counters.handler_operation_clause_evaluations > 0,
            case.expect_handler_work,
            "{}: {counters:#?}",
            case.name
        );
    }
}

#[test]
fn effect_inference_updates_shared_function_and_handler_dependents() {
    crate::types::effect_inference_counters::reset();
    let module = merged_modules(vec![SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "fn terminal() -> Int\n",
            "  stdio::println(\"terminal\")\n",
            "  1\n",
            "end\n",
            "\n",
            "fn direct() -> Int\n",
            "  terminal()\n",
            "end\n",
            "\n",
            "handler ask() handles Ask\n",
            "  value() => terminal()\n",
            "end\n",
            "\n",
            "fn compute() -> Int effects [Ask]\n",
            "  perform Ask::value()\n",
            "end\n",
            "\n",
            "pub fn main() -> Int\n",
            "  direct()\n",
            "  handle compute() with ask()\n",
            "end\n",
        ),
    )]);

    let diagnostics = analyze_surface_module(&module);
    let counters = crate::types::effect_inference_counters::snapshot();

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    let details = diagnostics[0].details.to_json();
    assert!(
        details.contains("\"inferred_effects\":[\"stdio\"]"),
        "{details}"
    );
    assert!(
        counters.handler_operation_clause_evaluations > 0,
        "{counters:#?}"
    );
    assert!(counters.changed_reevaluations > 0, "{counters:#?}");
}

#[test]
fn bounded_effect_inference_preserves_stable_order_in_multi_effect_cycle() {
    crate::types::effect_inference_counters::reset();
    let module = merged_modules(vec![SourceFile::new(
        "main.veln",
        concat!(
            "fn z() -> Int\n",
            "  a()\n",
            "  stdio::println(\"z\")\n",
            "  1\n",
            "end\n",
            "\n",
            "fn a() -> Int\n",
            "  z()\n",
            "  time::monotonic_ms()\n",
            "end\n",
            "\n",
            "pub fn main() -> Int\n",
            "  z()\n",
            "end\n",
        ),
    )]);

    let diagnostics = analyze_surface_module(&module);
    let counters = crate::types::effect_inference_counters::snapshot();

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"symbol\":\"main\""), "{details}");
    assert!(
        details.contains("\"inferred_effects\":[\"stdio\",\"time\"]"),
        "{details}"
    );
    assert!(counters.changed_reevaluations > 0, "{counters:#?}");
}

#[test]
fn bounded_effect_inference_work_grows_linearly_for_unrelated_annotated_modules() {
    fn fixture(unrelated_count: usize) -> SurfaceModule {
        let mut sources = vec![SourceFile::new(
            "base.veln",
            concat!(
                "mod base\n",
                "effect Ask\n",
                "  value() -> Int\n",
                "end\n",
                "\n",
                "fn terminal() -> ()\n",
                "  stdio::println(\"terminal\")\n",
                "end\n",
                "\n",
                "fn middle() -> ()\n",
                "  terminal()\n",
                "end\n",
                "\n",
                "fn provide(offset: Int) -> Int\n",
                "  middle()\n",
                "  offset + 1\n",
                "end\n",
                "\n",
                "handler ask(offset: Int) handles Ask\n",
                "  value() => provide(offset)\n",
                "end\n",
                "\n",
                "fn compute() -> Int effects [Ask]\n",
                "  perform Ask::value()\n",
                "end\n",
                "\n",
                "pub fn main() -> Int\n",
                "  handle compute() with ask(1)\n",
                "end\n",
            ),
        )];
        for module_index in 0..unrelated_count {
            sources.push(SourceFile::new(
                format!("annotated_{module_index}.veln"),
                format!(
                    "mod annotated_{module_index}\n\
                     fn helper_{module_index}(value: Int) -> Int\n  value\nend\n"
                ),
            ));
        }
        merged_modules(sources)
    }

    fn work_count(unrelated_count: usize) -> (usize, usize) {
        crate::types::effect_inference_counters::reset();
        let module = fixture(unrelated_count);
        let diagnostics = analyze_surface_module(&module);
        let counters = crate::types::effect_inference_counters::snapshot();
        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
        assert_eq!(diagnostics[0].id, "effect.missing_public");
        let total = counters.dependency_discovery_scans
            + counters.function_body_collections
            + counters.handler_operation_clause_evaluations
            + counters.changed_reevaluations;
        (total, counters.changed_reevaluations)
    }

    const N: usize = 6;
    let (work_0, reevaluations_0) = work_count(0);
    let (work_n, reevaluations_n) = work_count(N);
    let (work_2n, reevaluations_2n) = work_count(2 * N);

    assert!(
        work_2n - work_n <= work_n - work_0,
        "W(0)={work_0}, W(N)={work_n}, W(2N)={work_2n}"
    );
    assert_eq!(
        reevaluations_n, reevaluations_0,
        "unrelated annotated modules should not add base dependency reevaluations"
    );
    assert_eq!(
        reevaluations_2n, reevaluations_0,
        "unrelated annotated modules should not add base dependency reevaluations"
    );
}

#[test]
fn effect_provenance_reports_omitted_equivalent_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> ()\n",
            "  stdio::print(\"one\")\n",
            "  stdio::println(\"two\")\n",
            "  stdio::eprint(\"three\")\n",
            "  stdio::eprintln(\"four\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"provenance_truncated\":true"));
    assert!(details.contains("\"omitted_path_count\":1"));
}
