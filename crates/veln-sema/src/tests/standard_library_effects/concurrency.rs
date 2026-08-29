use super::*;

#[test]
fn reports_missing_public_effect_with_call_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> ()\n",
            "  stdio::println(\"hello\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(diagnostics[0].kind, DiagnosticKind::Effect);
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `stdio`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"stdio\""));
    assert!(details.contains("\"declared_effects\":[]"));
    assert!(details.contains("\"inferred_effects\":[\"stdio\"]"));
    assert!(details.contains("\"symbol\":\"stdio::println\""));
    assert!(details.contains("\"provenance_paths\":[{\"effect\":\"stdio\""));
    assert!(details.contains("\"kind\":\"public_boundary\""));
    assert!(details.contains("\"hidden_frame_count\":0"));
    assert!(details.contains("\"omitted_path_count\":0"));
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn channel_calls_require_concurrency_effect() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(tx: Sender<String>) -> Result<(), SendError>\n",
            "  channel::send(tx, \"hello\")\n",
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
        "public function uses undeclared effect `concurrency`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"concurrency\""));
    assert!(details.contains("\"inferred_effects\":[\"concurrency\"]"));
    assert!(details.contains("\"symbol\":\"channel::send\""));
}

#[test]
fn cancellable_channel_select_many_timeout_requires_time_and_concurrency_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(receivers: List<Receiver<String>>, token: CancelToken) -> Result<Option<{index: Int, value: String}>, SelectError>\n",
            "  channel::select_many_timeout_cancellable(receivers, 10, token)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `time`"
    );
    let time_details = diagnostics[0].details.to_json();
    assert!(time_details.contains("\"effect\":\"time\""));
    assert!(time_details.contains("\"inferred_effects\":[\"time\",\"concurrency\"]"));
    assert!(time_details.contains("\"symbol\":\"channel::select_many_timeout_cancellable\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `concurrency`"
    );
    let concurrency_details = diagnostics[1].details.to_json();
    assert!(concurrency_details.contains("\"effect\":\"concurrency\""));
    assert!(concurrency_details.contains("\"inferred_effects\":[\"time\",\"concurrency\"]"));
    assert!(
        concurrency_details.contains("\"symbol\":\"channel::select_many_timeout_cancellable\"")
    );
}

#[test]
fn cancellable_channel_select_timeout_requires_time_and_concurrency_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(left: Receiver<String>, right: Receiver<String>, token: CancelToken) -> Result<Option<{index: Int, value: String}>, SelectError>\n",
            "  channel::select_timeout_cancellable(left, right, 10, token)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `time`"
    );
    let time_details = diagnostics[0].details.to_json();
    assert!(time_details.contains("\"effect\":\"time\""));
    assert!(time_details.contains("\"inferred_effects\":[\"time\",\"concurrency\"]"));
    assert!(time_details.contains("\"symbol\":\"channel::select_timeout_cancellable\""));

    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `concurrency`"
    );
    let concurrency_details = diagnostics[1].details.to_json();
    assert!(concurrency_details.contains("\"effect\":\"concurrency\""));
    assert!(concurrency_details.contains("\"inferred_effects\":[\"time\",\"concurrency\"]"));
    assert!(concurrency_details.contains("\"symbol\":\"channel::select_timeout_cancellable\""));
}

#[test]
fn task_calls_require_concurrency_effect() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn produce() -> String\n",
            "  \"hello\"\n",
            "end\n",
            "pub fn main() -> Task<String>\n",
            "  task::spawn(produce)\n",
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
        "public function uses undeclared effect `concurrency`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"concurrency\""));
    assert!(details.contains("\"symbol\":\"task::spawn\""));
}

#[test]
fn task_spawn_preserves_job_effects_at_public_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn load() -> String effects [db]\n",
            "  \"row\"\n",
            "end\n",
            "pub fn main() -> Task<String> effects [concurrency]\n",
            "  task::spawn(load)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `db`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"db\""));
    assert!(details.contains("\"inferred_effects\":[\"concurrency\",\"db\"]"));
    assert!(details.contains("\"symbol\":\"task::spawn\""));
}

#[test]
fn task_spawn_with_preserves_context_job_effects_at_public_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn serve(context: {payload: String}) -> String effects [net, db]\n",
            "  context.payload\n",
            "end\n",
            "pub fn main(payload: String) -> Task<String> effects [concurrency]\n",
            "  let context = {payload: payload}\n",
            "  task::spawn_with(serve, context)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.missing_public");
    assert_eq!(
        diagnostics[0].message,
        "public function uses undeclared effect `net`"
    );
    let net_details = diagnostics[0].details.to_json();
    assert!(net_details.contains("\"inferred_effects\":[\"concurrency\",\"net\",\"db\"]"));
    assert!(net_details.contains("\"symbol\":\"task::spawn_with\""));
    assert_eq!(diagnostics[1].id, "effect.missing_public");
    assert_eq!(
        diagnostics[1].message,
        "public function uses undeclared effect `db`"
    );
    let db_details = diagnostics[1].details.to_json();
    assert!(db_details.contains("\"inferred_effects\":[\"concurrency\",\"net\",\"db\"]"));
    assert!(db_details.contains("\"symbol\":\"task::spawn_with\""));
}
