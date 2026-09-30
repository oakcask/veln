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

#[test]
fn wall_clock_requires_time_effect_and_exposes_integer_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn timestamp() -> Int\n",
            "  let value: WallTime = time::wall_time()\n",
            "  value.unix_seconds + value.nanosecond\n",
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
        "public function uses undeclared effect `time`"
    );
    let details = diagnostics[0].details.to_json();
    assert!(details.contains("\"symbol\":\"time::wall_time\""));
}

#[test]
fn wall_clock_value_is_assignable_to_its_structural_record_shape() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn seconds(value: {unix_seconds: Int, nanosecond: Int}) -> Int\n",
            "  value.unix_seconds\n",
            "end\n",
            "pub fn timestamp() -> Int effects [time]\n",
            "  seconds(time::wall_time())\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.ir.is_some());
}

#[test]
fn user_defined_wall_time_does_not_gain_standard_record_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type WallTime\n",
            "  Other\n",
            "end\n",
            "pub fn timestamp(value: WallTime) -> Int\n",
            "  value.unix_seconds\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.field_missing"
                && diagnostic.message == "type `WallTime` has no field `unix_seconds`"
        }),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(lowered.ir.is_none());
}

#[test]
fn alias_of_user_defined_wall_time_remains_nominal() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type WallTime\n",
            "  Other\n",
            "end\n",
            "pub type LocalWallTime = WallTime\n",
            "pub fn timestamp(value: LocalWallTime) -> Int\n",
            "  value.unix_seconds\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.field_missing"
                && diagnostic.message == "type `LocalWallTime` has no field `unix_seconds`"
        }),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(lowered.ir.is_none());
}
