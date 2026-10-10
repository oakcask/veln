use super::*;

#[test]
fn check_json_reports_public_function_boundary_errors() {
    let project = TestProject::new("public-boundary");
    project.write("main.veln", "pub fn main(value)\n  value\nend\n");

    let output = project.check_json(&["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        concat!(
            "{\"schema_version\":1,",
            "\"tool\":{\"name\":\"veln\",\"version\":\"0.1.0\"},",
            "\"status\":\"error\",",
            "\"diagnostics\":[{",
            "\"id\":\"type.public_signature_missing\",",
            "\"severity\":\"error\",",
            "\"kind\":\"type\",",
            "\"message\":\"public parameter `value` has no type annotation\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":1,\"column\":13,\"offset\":12},\"end\":{\"line\":1,\"column\":18,\"offset\":17}},",
            "\"details\":{\"phase\":\"type\",\"node_id\":\"param-2\",\"expected_type\":\"explicit\",\"actual_type\":\"missing\",",
            "\"expected_type_source\":\"declared_parameter\",\"actual_type_source\":\"source\",",
            "\"constraint\":\"assignable\",\"origin_node_ids\":[\"fn-1\"]},",
            "\"related\":[]},{",
            "\"id\":\"type.public_signature_missing\",",
            "\"severity\":\"error\",",
            "\"kind\":\"type\",",
            "\"message\":\"public function has no return type annotation\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":1,\"column\":1,\"offset\":0},\"end\":{\"line\":4,\"column\":1,\"offset\":31}},",
            "\"details\":{\"phase\":\"type\",\"node_id\":\"fn-1\",\"expected_type\":\"explicit\",\"actual_type\":\"missing\",",
            "\"expected_type_source\":\"declared_return\",\"actual_type_source\":\"source\",",
            "\"constraint\":\"return_value\",\"origin_node_ids\":[\"fn-1\"]},",
            "\"related\":[]}],",
            "\"summary\":{\"diagnostic_count\":2,\"by_severity\":{\"error\":2},\"by_kind\":{\"type\":2}}}\n"
        )
    );
}

#[test]
fn check_json_reports_empty_effects_declaration() {
    let project = TestProject::new("empty-effects-declaration");
    project.write("main.veln", "pub fn main() -> () effects []\n  ()\nend\n");

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"effect.empty_declaration\"",
            "\"kind\":\"effect\"",
            "\"message\":\"empty effects list is not allowed on a function declaration\"",
            "\"boundary\":\"public_function\"",
            "\"declared_effects\":[]",
            "\"related\":[{\"kind\":\"repair_hint\",\"message\":\"Remove the clause when the inferred effect set is empty.\"}",
            "{\"kind\":\"repair_hint\",\"message\":\"Replace the empty list with non-empty effect labels when the body performs effects.\"}]",
        ],
    );
}

#[test]
fn check_json_reports_hole_with_return_expected_type() {
    let project = TestProject::new("hole-return");
    project.write(
        "main.veln",
        "pub fn main() -> Result<(), AppError>\n  _\nend\n",
    );

    let output = project.check_json(&["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        concat!(
            "{\"schema_version\":1,",
            "\"tool\":{\"name\":\"veln\",\"version\":\"0.1.0\"},",
            "\"status\":\"partial\",",
            "\"diagnostics\":[{",
            "\"id\":\"hole.unfilled\",",
            "\"severity\":\"hint\",",
            "\"kind\":\"hole\",",
            "\"message\":\"hole requires a `Result<(), AppError>` value\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":2,\"column\":3,\"offset\":40},\"end\":{\"line\":2,\"column\":4,\"offset\":41}},",
            "\"details\":{\"phase\":\"hole\",\"node_id\":\"hole-3\",\"label\":null,",
            "\"expected_type\":\"Result<(), AppError>\",\"expected_type_source\":\"declared\",",
            "\"constraints\":[],\"local_bindings\":[],",
            "\"candidate_queries\":[{\"kind\":\"symbol\",",
            "\"candidate_status\":\"query_only\",",
            "\"application_policy\":\"manual_review_required\",",
            "\"query\":\"fn() -> Result<(), AppError>\"}]},",
            "\"related\":[{\"kind\":\"expected_type_origin\",\"message\":\"Return type declared here.\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":1,\"column\":1,\"offset\":0},\"end\":{\"line\":4,\"column\":1,\"offset\":46}}}]}],",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"hint\":1},\"by_kind\":{\"hole\":1}}}\n"
        )
    );
}

#[test]
fn check_json_keeps_sema_for_other_files_when_one_file_has_parse_errors() {
    let project = TestProject::new("parse-and-sema");
    project.write("a_parse.veln", "fn broken() -> ()\n  @\nend\n");
    project.write("b_type.veln", "pub fn main() -> Int\n  \"no\"\nend\n");

    let output = project.check_json(&[]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"parse.invalid_token\"",
            "\"file\":\"a_parse.veln\"",
            "\"id\":\"type.mismatch\"",
            "\"file\":\"b_type.veln\"",
            "\"summary\":{\"diagnostic_count\":2,\"by_severity\":{\"error\":2},\"by_kind\":{\"parse\":1,\"type\":1}}",
        ],
    );
}

#[test]
fn check_json_resolves_imported_calls_across_selected_files() {
    let project = TestProject::new("check-shared-project-analysis");
    project.write("app/util.veln", "pub fn value() -> Int\n  1\nend\n");
    project.write(
        "app/main.veln",
        concat!(
            "use app::util\n",
            "pub fn main() -> Int\n",
            "  app::util::value()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&[]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_contains_all(
        stdout(&output),
        &[
            "\"diagnostics\":[]",
            "\"summary\":{\"diagnostic_count\":0,\"by_severity\":{},\"by_kind\":{}}",
        ],
    );
}

#[test]
fn check_json_reports_return_type_mismatch() {
    let project = TestProject::new("return-mismatch");
    project.write("main.veln", "pub fn main() -> Int\n  \"no\"\nend\n");

    let output = project.check_json(&["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout(&output),
        &[
            "\"id\":\"type.mismatch\"",
            "\"kind\":\"type\"",
            "\"message\":\"expected `Int`, but found `String`\"",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":2,\"column\":3,\"offset\":23},\"end\":{\"line\":2,\"column\":7,\"offset\":27}}",
            "\"details\":{\"phase\":\"type\",\"node_id\":\"expr-3\",\"expected_type\":\"Int\",\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",\"actual_type_source\":\"inferred_expression\",\"constraint\":\"return_value\"",
        ],
    );
}

#[test]
fn check_json_reports_match_exhaustiveness_details() {
    let project = TestProject::new("match-exhaustiveness-json");
    project.write(
        "main.veln",
        concat!(
            "fn main(value: Result<Int, String>) -> String\n",
            "  match value\n",
            "    Err(error) => error\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout(&output),
        &[
            "\"id\":\"type.match_non_exhaustive\"",
            "\"message\":\"match is missing case Ok(_)\"",
            "\"scrutinee_type\":\"Result<Int, String>\"",
            "\"missing_case\":\"Ok(_)\"",
            "\"constraint\":\"match_exhaustiveness\"",
            "\"kind\":\"scrutinee_type\"",
            "\"kind\":\"covered_case\"",
        ],
    );
}

#[test]
fn check_json_deduplicates_repeated_explicit_inputs() {
    let project = TestProject::new("dedupe-explicit-inputs");
    project.write("main.veln", "pub fn main() -> Int\n  \"no\"\nend\n");

    let output = project.check_json(&["main.veln", "main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"type.mismatch\"",
            "\"message\":\"expected `Int`, but found `String`\"",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"type\":1}}",
        ],
    );
    assert_eq!(stdout.matches("\"id\":\"type.mismatch\"").count(), 1);
}

#[test]
fn check_json_deduplicates_overlapping_directory_and_file_inputs() {
    let project = TestProject::new("dedupe-overlapping-directory-file-inputs");
    project.write("src/main.veln", "pub fn main() -> Int\n  \"no\"\nend\n");
    project.write(
        "src/target/generated.veln",
        "fn generated() -> Int\n  1\nend\n",
    );
    project.write("src/.git/hooks/hook.veln", "fn broken() -> ()\n  @\nend\n");

    let output = project.check_json(&["src", "src/main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"type.mismatch\"",
            "\"message\":\"expected `Int`, but found `String`\"",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"type\":1}}",
        ],
    );
    assert_eq!(stdout.matches("\"id\":\"type.mismatch\"").count(), 1);
}

#[test]
fn check_json_reports_implicit_unit_return_type_mismatch() {
    let project = TestProject::new("implicit-unit-return-mismatch");
    project.write("main.veln", "pub fn main() -> Int\n  let value = 1\nend\n");

    let output = project.check_json(&["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout(&output),
        &[
            "\"id\":\"type.mismatch\"",
            "\"kind\":\"type\"",
            "\"message\":\"expected `Int`, but found `()`\"",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":1,\"column\":1,\"offset\":0},\"end\":{\"line\":4,\"column\":1,\"offset\":41}}",
            "\"details\":{\"phase\":\"type\",\"node_id\":\"fn-1\",\"expected_type\":\"Int\",\"actual_type\":\"()\",\"expected_type_source\":\"declared_return\",\"actual_type_source\":\"implicit_unit\",\"constraint\":\"return_value\",\"origin_node_ids\":[\"fn-1\",\"fn-1\"]}",
        ],
    );
}

#[test]
fn check_human_reports_missing_record_field_with_base_note() {
    let project = TestProject::new("field-missing-human");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> Int\n",
            "  let payload: {count: Int} = {count: 1}\n",
            "  payload.name\n",
            "end\n",
        ),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(
        stdout(&output),
        concat!(
            "main.veln:3:11: error[type.field_missing]: type `{count: Int}` has no field `name`\n",
            "  note: main.veln:3:3: Field access base has type `{count: Int}`.\n",
        ),
    );
}

#[test]
fn check_json_reports_unresolved_name_and_call_target() {
    let project = TestProject::new("name-diagnostics");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> ()\n",
            "  missing_value\n",
            "  missing_call()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"name.unresolved\"",
            "\"severity\":\"error\"",
            "\"kind\":\"name\"",
            "\"symbol\":\"missing_value\"",
            "\"namespace\":\"value\"",
            "\"symbol\":\"missing_call\"",
            "\"namespace\":\"call_target\"",
            "\"resolution_status\":\"unresolved\"",
            "\"summary\":{\"diagnostic_count\":2,\"by_severity\":{\"error\":2},\"by_kind\":{\"name\":2}}",
        ],
    );
}

#[test]
fn check_json_reports_missing_public_stdio_effect_with_provenance() {
    let project = TestProject::new("effect-provenance");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> ()\n",
            "  stdio::println(\"hello\")\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"effect.missing_public\"",
            "\"kind\":\"effect\"",
            "\"message\":\"public function uses undeclared effect `stdio`\"",
            "\"details\":{\"phase\":\"effect\",\"node_id\":\"fn-1\",\"effect\":\"stdio\",",
            "\"declared_effects\":[],\"inferred_effects\":[\"stdio\"]",
            "\"provenance\":[{\"node_id\":\"call-3\",\"kind\":\"direct_call\",\"symbol\":\"stdio::println\"}]",
            "\"related\":[{\"kind\":\"effect_provenance\"",
        ],
    );
}

#[test]
fn check_json_reports_unknown_effect_label_details() {
    let project = TestProject::new("effect-json-unknown-label");
    project.write(
        "main.veln",
        "pub fn main() -> () effects [telepathy]\n  ()\nend\n",
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"effect.unknown\"",
            "\"kind\":\"effect\"",
            "\"message\":\"declared effect `telepathy` is not known\"",
            "\"details\":{\"phase\":\"effect\",\"node_id\":\"fn-1\",\"effect\":\"telepathy\",",
            "\"boundary\":\"public_function\"",
            "\"declared_effects\":[\"telepathy\"]",
            "\"known_effects\":[\"stdio\",\"fs\",\"net\",\"db\",\"time\",\"random\",\"process\",\"concurrency\"]",
            "\"related\":[{\"kind\":\"repair_hint\",\"message\":\"Use a known effect label or remove the declaration.\"}]",
        ],
    );
}

#[test]
fn check_human_reports_missing_public_effect_cause() {
    let project = TestProject::new("effect-human-provenance");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> ()\n",
            "  stdio::println(\"hello\")\n",
            "end\n",
        ),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(
        stdout(&output),
        concat!(
            "main.veln:1:1: error[effect.missing_public]: public function uses undeclared effect `stdio`\n",
            "  note: main.veln:2:3: Call to `stdio::println` requires this effect.\n",
        ),
    );
}

#[test]
fn check_human_reports_empty_effects_repair_hints_as_notes() {
    let project = TestProject::new("effect-human-empty-declaration");
    project.write("main.veln", "pub fn main() -> () effects []\n  ()\nend\n");

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(
        stdout(&output),
        concat!(
            "main.veln:1:1: error[effect.empty_declaration]: empty effects list is not allowed on a function declaration\n",
            "  note: Remove the clause when the inferred effect set is empty.\n",
            "  note: Replace the empty list with non-empty effect labels when the body performs effects.\n",
        ),
    );
}

#[test]
fn check_human_reports_unknown_effect_label_hint_as_note() {
    let project = TestProject::new("effect-human-unknown-label");
    project.write(
        "main.veln",
        "pub fn main() -> () effects [telepathy]\n  ()\nend\n",
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(
        stdout(&output),
        concat!(
            "main.veln:1:30: error[effect.unknown]: declared effect `telepathy` is not known\n",
            "  note: Use a known effect label or remove the declaration.\n",
        ),
    );
}
