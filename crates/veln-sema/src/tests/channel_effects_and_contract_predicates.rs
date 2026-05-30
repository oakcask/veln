use super::*;

#[test]
fn channel_recv_checks_receiver_against_expected_option_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(rx: Receiver(Int)) -> Option(String) effects [concurrency]\n",
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
        "expected `Receiver(String)`, but found `Receiver(Int)`"
    );
}

#[test]
fn channel_select_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver(String), right: Receiver(String)) -> Option({index: Int, value: String}) effects [concurrency]\n",
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
            "pub fn main(left: Receiver(String), right: Receiver(Int)) -> Option({index: Int, value: String}) effects [concurrency]\n",
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
        "expected `Receiver(String)`, but found `Receiver(Int)`"
    );
}

#[test]
fn channel_select_priority_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver(String), right: Receiver(String)) -> Option({index: Int, value: String}) effects [concurrency]\n",
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
fn channel_select_timeout_preserves_receiver_item_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver(String), right: Receiver(String)) -> Option({index: Int, value: String}) effects [concurrency]\n",
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
            "pub fn main(left: Receiver(String), right: Receiver(String)) -> Result(Option({index: Int, value: String}), SelectError) effects [concurrency]\n",
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
fn channel_select_timeout_result_requires_integer_timeout() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver(String), right: Receiver(String)) -> Result(Option({index: Int, value: String}), SelectError) effects [concurrency]\n",
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
            "pub fn main(left: Receiver(String), right: Receiver(String)) -> Option({index: Int, value: String}) effects [concurrency]\n",
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
            "pub fn main(rx: Receiver(String)) -> () effects [concurrency]\n",
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
        "expected `Sender(unknown)`, but found `Receiver(String)`"
    );
}

#[test]
fn infers_transitive_private_helper_effects_from_body() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn say(text: String) -> ()\n",
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
            "fn say(text: String) -> ()\n",
            "  stdio::println(text)\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let console = lower_surface_ast(&parse(&console_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
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
            "fn positive(value: Int) -> Bool\n",
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
            "fn positive(value: Int) -> Bool\n",
            "  value > 0\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let rules = lower_surface_ast(&parse(&rules_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
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
