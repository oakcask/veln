use super::*;

#[test]
fn channel_recv_checks_receiver_against_expected_option_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(rx: Receiver<Int>) -> Option<String> effects [concurrency]\n",
            "  channel::recv(rx)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `Receiver<String>`, but found `Receiver<Int>`"
    );
}

#[test]
fn channel_select_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select(left, right)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select return");
    };
    assert_eq!(
        expr.ty,
        CoreType::option(CoreType::Record(vec![
            ("index".to_string(), CoreType::int()),
            ("value".to_string(), CoreType::string()),
        ]))
    );
}

#[test]
fn channel_select_checks_both_receivers_against_same_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<Int>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select(left, right)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `Receiver<String>`, but found `Receiver<Int>`"
    );
}

#[test]
fn channel_select_priority_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_priority(left, right)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select priority return");
    };
    assert_eq!(
        expr.ty,
        CoreType::option(CoreType::Record(vec![
            ("index".to_string(), CoreType::int()),
            ("value".to_string(), CoreType::string()),
        ]))
    );
}

#[test]
fn channel_select_many_priority_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_many_priority(receivers)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select many priority return");
    };
    assert_eq!(
        expr.ty,
        CoreType::option(CoreType::Record(vec![
            ("index".to_string(), CoreType::int()),
            ("value".to_string(), CoreType::string()),
        ]))
    );
}

#[test]
fn channel_select_many_priority_checks_receiver_list_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<Int>>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_many_priority(receivers)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `List<Receiver<String>>`, but found `List<Receiver<Int>>`"
    );
}

#[test]
fn channel_select_many_timeout_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_many_timeout(receivers, 10)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select many timeout return");
    };
    assert_eq!(
        expr.ty,
        CoreType::option(CoreType::Record(vec![
            ("index".to_string(), CoreType::int()),
            ("value".to_string(), CoreType::string()),
        ]))
    );
}

#[test]
fn channel_select_many_timeout_requires_integer_timeout() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_many_timeout(receivers, \"soon\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
}

#[test]
fn channel_select_many_timeout_result_reports_interrupts_with_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [concurrency]\n",
            "  channel::select_many_timeout_result(receivers, 10)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select many timeout result return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(
            CoreType::option(CoreType::Record(vec![
                ("index".to_string(), CoreType::int()),
                ("value".to_string(), CoreType::string()),
            ])),
            CoreType::named("SelectError", Vec::new())
        )
    );
}

#[test]
fn channel_select_many_timeout_result_requires_integer_timeout() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [concurrency]\n",
            "  channel::select_many_timeout_result(receivers, \"soon\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
}

#[test]
fn channel_select_many_timeout_cancellable_reports_cancellation_with_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>, token: CancelToken) -> Result<Option<{index: Int, value: String}>, SelectError> effects [time, concurrency]\n",
            "  channel::select_many_timeout_cancellable(receivers, 10, token)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected cancellable select many timeout return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(
            CoreType::option(CoreType::Record(vec![
                ("index".to_string(), CoreType::int()),
                ("value".to_string(), CoreType::string()),
            ])),
            CoreType::named("SelectError", Vec::new())
        )
    );
}

#[test]
fn channel_select_many_timeout_cancellable_requires_cancel_token() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [time, concurrency]\n",
            "  channel::select_many_timeout_cancellable(receivers, 10, \"stop\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `CancelToken`, but found `String`"
    );
}

#[test]
fn channel_select_timeout_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_timeout(left, right, 10)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select timeout return");
    };
    assert_eq!(
        expr.ty,
        CoreType::option(CoreType::Record(vec![
            ("index".to_string(), CoreType::int()),
            ("value".to_string(), CoreType::string()),
        ]))
    );
}

#[test]
fn channel_select_result_reports_interrupts_with_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [concurrency]\n",
            "  channel::select_result(left, right)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected select result return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(
            CoreType::option(CoreType::Record(vec![
                ("index".to_string(), CoreType::int()),
                ("value".to_string(), CoreType::string()),
            ])),
            CoreType::named("SelectError", Vec::new())
        )
    );
}

#[test]
fn channel_select_timeout_cancellable_reports_interrupts_with_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>, token: CancelToken) -> Result<Option<{index: Int, value: String}>, SelectError> effects [time, concurrency]\n",
            "  channel::select_timeout_cancellable(left, right, 10, token)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert_eq!(lowered.diagnostics.len(), 0, "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = &core.functions[0];
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("expected cancellable select timeout return");
    };
    assert_eq!(
        expr.ty,
        CoreType::result(
            CoreType::option(CoreType::Record(vec![
                ("index".to_string(), CoreType::int()),
                ("value".to_string(), CoreType::string()),
            ])),
            CoreType::named("SelectError", Vec::new())
        )
    );
}

#[test]
fn channel_select_timeout_cancellable_requires_cancel_token() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [time, concurrency]\n",
            "  channel::select_timeout_cancellable(left, right, 10, \"stop\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `CancelToken`, but found `String`"
    );
}

#[test]
fn channel_select_timeout_result_requires_integer_timeout() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Result<Option<{index: Int, value: String}>, SelectError> effects [concurrency]\n",
            "  channel::select_timeout_result(left, right, \"soon\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
}

#[test]
fn channel_select_timeout_requires_integer_timeout() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>) -> Option<{index: Int, value: String}> effects [concurrency]\n",
            "  channel::select_timeout(left, right, \"soon\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
}

#[test]
fn channel_close_requires_sender_handle() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(rx: Receiver<String>) -> () effects [concurrency]\n",
            "  channel::close(rx)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `Sender<unknown>`, but found `Receiver<String>`"
    );
}

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
        codecs: Vec::new(),
        types: main.types.into_iter().chain(console.types).collect(),
        functions: main
            .functions
            .into_iter()
            .chain(console.functions)
            .collect(),
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
                    "  value = provide\n",
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
            counters.handler_provider_evaluations > 0,
            case.expect_handler_work,
            "{}: {counters:#?}",
            case.name
        );
    }
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
                "  value = provide\n",
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
            + counters.handler_provider_evaluations
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

#[test]
fn reports_non_boolean_contract_predicate() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Int) -> ()\n",
            "require value\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "contract.type_mismatch"
            && diagnostic.kind == DiagnosticKind::Contract
            && diagnostic.message == "contract predicate is not `Bool`"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"non_boolean_predicate\"")
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.mismatch"
            && diagnostic.kind == DiagnosticKind::Type
            && diagnostic.message == "expected `Bool`, but found `Int`"
    }));
}

#[test]
fn ensure_can_reference_explicit_result_binding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> output: Int\n",
            "ensure output == value\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_field_access_resolves_record_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {total: Int}) -> output: {total: Int}\n",
            "ensure output.total == value.total\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_boolean_field_access_is_a_boolean_predicate() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}) -> output: {ready: Bool}\n",
            "require value.ready\n",
            "ensure output.ready\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_pure_call_result_field_access() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn summary(value: Int) -> {total: Int, ready: Bool}\n",
            "  {total: value, ready: true}\n",
            "end\n",
            "pub fn identity(value: Int) -> output: Int\n",
            "require summary(value).ready\n",
            "ensure summary(output).total >= value\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_pure_boolean_function_calls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn positive(value: Int) -> Bool\n",
            "  value > 0\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
            "require positive(value)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_qualified_pure_function_calls() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.rules\n",
            "pub fn identity(value: Int) -> Int\n",
            "require rules::positive(value)\n",
            "  value\n",
            "end\n",
        ),
    );
    let rules_source = SourceFile::new(
        "rules.veln",
        concat!(
            "mod app.rules\n",
            "pub fn positive(value: Int) -> Bool\n",
            "  value > 0\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let rules = lower_surface_ast(&parse(&rules_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: main.types.into_iter().chain(rules.types).collect(),
        functions: main.functions.into_iter().chain(rules.functions).collect(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_pure_function_calls_inside_comparisons() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn same(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
            "require same(value) > 0\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_nested_pure_function_call_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn same(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "fn positive(value: Int) -> Bool\n",
            "  value > 0\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
            "require positive(same(value))\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_function_value_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "fn accepts(job: fn() -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
            "require accepts(ready)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}
