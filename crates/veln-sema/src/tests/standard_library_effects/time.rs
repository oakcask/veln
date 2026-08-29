use super::*;

#[test]
fn time_calls_require_time_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> ()\n",
            "  let deadline: Deadline = time::deadline_after_ms(10)\n",
            "  let absolute_deadline: Deadline = time::deadline_at_ms(time::monotonic_ms())\n",
            "  let token: CancelToken = time::cancel_token()\n",
            "  time::wait_until_cancellable(deadline, token)\n",
            "  time::wait_until(absolute_deadline)\n",
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
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"time\""));
    assert!(details.contains("\"inferred_effects\":[\"time\"]"));
    assert!(details.contains("\"symbol\":\"time::deadline_after_ms\""));
}

#[test]
fn cancellation_status_query_requires_time_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn token_status(token: CancelToken) -> Bool\n",
            "  time::is_cancelled(token)\n",
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
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"time\""));
    assert!(details.contains("\"inferred_effects\":[\"time\"]"));
    assert!(details.contains("\"symbol\":\"time::is_cancelled\""));
}

#[test]
fn cancellation_owner_status_query_requires_time_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn owner_status(owner: CancelOwner) -> Bool\n",
            "  time::is_cancelled_owner(owner)\n",
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
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"time\""));
    assert!(details.contains("\"inferred_effects\":[\"time\"]"));
    assert!(details.contains("\"symbol\":\"time::is_cancelled_owner\""));
}

#[test]
fn cancellation_owner_calls_require_time_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn owner_token() -> CancelToken\n",
            "  let owner: CancelOwner = time::cancel_owner()\n",
            "  let token: CancelToken = time::cancel_token_from(owner)\n",
            "  time::cancel_owned(owner)\n",
            "  token\n",
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
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"time\""));
    assert!(details.contains("\"inferred_effects\":[\"time\"]"));
    assert!(details.contains("\"symbol\":\"time::cancel_owner\""));
}

#[test]
fn monotonic_clock_requires_time_effect_with_descriptor_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn elapsed() -> Int\n",
            "  time::monotonic_ms()\n",
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
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"effect\":\"time\""));
    assert!(details.contains("\"inferred_effects\":[\"time\"]"));
    assert!(details.contains("\"symbol\":\"time::monotonic_ms\""));
}
