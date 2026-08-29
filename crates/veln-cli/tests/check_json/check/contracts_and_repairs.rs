use super::*;

#[test]
fn check_json_reports_contract_validation_diagnostics() {
    let project = TestProject::new("contract-diagnostics");
    project.write(
        "main.veln",
        concat!(
            "pub fn main(ready: Bool) -> ()\n",
            "require stdio::println(\"no\")\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"contract.unsupported_construct\"",
            "\"kind\":\"contract\"",
            "\"details\":{\"phase\":\"contract\"",
            "\"clause\":\"require\"",
            "\"predicate_text\":\"stdio::println(\\\"no\\\")\"",
            "\"validation_status\":\"invalid\"",
            "\"obligation_status\":\"failed_static\"",
            "\"reason\":\"effectful_operation\"",
            "\"runtime_required\":false",
        ],
    );
}

#[test]
fn check_json_keeps_satisfy_predicate_parse_errors_as_parse_kind() {
    let project = TestProject::new("satisfy-predicate-parse-kind");
    project.write(
        "main.veln",
        concat!(
            "pub fn choose() -> Int\n",
            "  _value satisfy candidate => candidate |> valid\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"parse.satisfy_predicate\"",
            "\"kind\":\"parse\"",
            "\"message\":\"pipeline syntax is not allowed in a contract predicate\"",
            "\"details\":{\"phase\":\"parse\"",
            "\"parser_context\":\"satisfy_predicate\"",
            "\"unexpected\":{\"kind\":\"|>\",\"text\":\"|>\"}",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"parse\":1}}",
        ],
    );
}

#[test]
fn check_json_reports_contract_type_mismatch_with_type_context() {
    let project = TestProject::new("contract-type-mismatch-json");
    project.write(
        "main.veln",
        concat!(
            "pub fn main(value: Int) -> ()\n",
            "require value\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"contract.type_mismatch\"",
            "\"kind\":\"contract\"",
            "\"message\":\"contract predicate is not `Bool`\"",
            "\"details\":{\"phase\":\"contract\"",
            "\"clause\":\"require\"",
            "\"predicate_text\":\"value\"",
            "\"validation_status\":\"invalid\"",
            "\"obligation_status\":\"failed_static\"",
            "\"reason\":\"non_boolean_predicate\"",
            "\"runtime_required\":false",
            "\"referenced_bindings\":[{\"name\":\"value\",\"kind\":\"local\"}]",
            "\"id\":\"type.mismatch\"",
            "\"kind\":\"type\"",
            "\"message\":\"expected `Bool`, but found `Int`\"",
            "\"expected_type\":\"Bool\"",
            "\"actual_type\":\"Int\"",
            "\"constraint\":\"contract_predicate\"",
            "\"summary\":{\"diagnostic_count\":2,\"by_severity\":{\"error\":2},\"by_kind\":{\"contract\":1,\"type\":1}}",
        ],
    );
}

#[test]
fn check_human_reports_contract_missing_record_field() {
    let project = TestProject::new("contract-missing-record-field");
    project.write(
        "main.veln",
        concat!(
            "pub fn main(value: {total: Int}) -> output: {total: Int}\n",
            "ensure output.missing == value.total\n",
            "  value\n",
            "end\n",
        ),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(
        stdout(&output),
        "main.veln:2:1: error[contract.field_missing]: contract field `missing` is not present on `{total: Int}`\n",
    );
}

#[test]
fn check_json_reports_contract_missing_call_result_field_details() {
    let project = TestProject::new("contract-missing-call-result-field-json");
    project.write(
        "main.veln",
        concat!(
            "fn summary(value: Int) -> {total: Int}\n",
            "  {total: value}\n",
            "end\n",
            "pub fn main(value: Int) -> Int\n",
            "require summary(value).missing == 1\n",
            "  value\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"contract.field_missing\"",
            "\"severity\":\"error\"",
            "\"kind\":\"contract\"",
            "\"message\":\"contract field `missing` is not present on `{total: Int}`\"",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":5,\"column\":1",
            "\"details\":{\"phase\":\"contract\"",
            "\"clause\":\"require\"",
            "\"predicate_text\":\"summary(value).missing == 1\"",
            "\"validation_status\":\"invalid\"",
            "\"obligation_status\":\"failed_static\"",
            "\"reason\":\"missing_field\"",
            "\"runtime_required\":false",
            "\"referenced_bindings\":[{\"name\":\"value\",\"kind\":\"local\"}]",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"contract\":1}}",
        ],
    );
}

#[test]
fn check_human_reports_contract_missing_call_result_field() {
    let project = TestProject::new("contract-missing-call-result-field");
    project.write(
        "main.veln",
        concat!(
            "fn summary(value: Int) -> {total: Int}\n",
            "  {total: value}\n",
            "end\n",
            "pub fn main(value: Int) -> Int\n",
            "require summary(value).missing == 1\n",
            "  value\n",
            "end\n",
        ),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(
        stdout(&output),
        "main.veln:5:1: error[contract.field_missing]: contract field `missing` is not present on `{total: Int}`\n",
    );
}

#[test]
fn check_json_reports_hole_constraints_from_contracts_and_satisfy() {
    let project = TestProject::new("hole-constraints");
    project.write(
        "main.veln",
        concat!(
            "pub fn default_port(max: Int) -> Int\n",
            "require max > 0\n",
            "  _port satisfy candidate => candidate > 0 and candidate <= max\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"partial\"",
            "\"id\":\"hole.unfilled\"",
            "\"expected_type\":\"Int\"",
            "\"constraints\":[{\"kind\":\"contract\",\"clause\":\"require\",\"text\":\"max > 0\"",
            "{\"kind\":\"satisfy\",\"text\":\"candidate > 0 and candidate <= max\",\"candidate_binding\":\"candidate\"",
            "\"repair_status\":\"statically_satisfied\"",
            "\"related\":[{\"kind\":\"expected_type_origin\"",
            "\"kind\":\"constraint_origin\"",
        ],
    );
}

#[test]
fn check_human_reports_satisfy_candidate_context() {
    let project = TestProject::new("satisfy-candidate-context");
    project.write(
        "main.veln",
        concat!(
            "fn default_port(max: Int) -> Int\n",
            "  _port satisfy max => true\n",
            "end\n",
        ),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &[
            "main.veln:2:17: error[hole.satisfy_candidate_shadow]: satisfy candidate `max` shadows a visible binding",
            "  note: main.veln:1:17: Visible binding with this name is here.",
            "main.veln:2:17: error[hole.satisfy_candidate_unused]: satisfy predicate does not reference candidate `max`",
            "  note: main.veln:2:9: The predicate for this satisfy clause is here.",
        ],
    );
}

#[test]
fn check_json_reports_assignable_safe_satisfy_candidate_reason() {
    let project = TestProject::new("assignable-satisfy-candidate-reason");
    project.write(
        "main.veln",
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  _value satisfy candidate => candidate.ready == order.ready\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"hole.unfilled\"",
            "\"severity\":\"hint\"",
            "\"candidate_status\":\"query_only\"",
            "\"candidate_id\":\"symbol-1\",\"name\":\"order\"",
            "\"type\":\"{ready: Bool, paid: Bool}\"",
            "\"reason\":\"satisfy_equality_match\"",
            "\"application_policy\":\"safe_repair_candidate\"",
            "\"edits\":[{\"kind\":\"replace\"",
            "\"replacement\":\"order\"",
            "\"target\":{\"node_id\":\"hole-",
            "\"edit_summary\":\"Replace hole with `order`\"",
            "\"evidence\":[{\"kind\":\"type\",\"status\":\"passed\"",
            "\"known_limits\":[\"edit is advisory and unapplied\"",
            "\"blocking_obligations\":[\"verification.not_run\"]",
            "\"verification_hint\":{\"command\":\"veln check --json main.veln\"",
            "\"application_status\":\"unapplied\"",
            "\"satisfy_status\":\"statically_satisfied\"",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"hint\":1},\"by_kind\":{\"hole\":1}}",
        ],
    );
}

#[test]
fn check_json_leaves_safe_repair_candidate_unapplied() {
    let project = TestProject::new("safe-repair-candidate-unapplied");
    let source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    project.write("main.veln", source);

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"application_policy\":\"safe_repair_candidate\"",
            "\"replacement\":\"order\"",
            "\"application_status\":\"unapplied\"",
            "\"verification_hint\":{\"command\":\"veln check --json main.veln\"",
        ],
    );
    assert_eq!(project.read("main.veln"), source);
}
#[test]
fn check_rejects_repair_options_without_applying_candidate_edits() {
    let project = TestProject::new("check-repair-options");
    let source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    project.write("main.veln", source);

    let repair_output = project.veln(&["check"], &["--repair", "main.veln"]);
    let apply_output = project.veln(&["check"], &["--apply", "main.veln"]);

    assert_eq!(repair_output.status.code(), Some(2));
    assert_eq!(stdout(&repair_output), "");
    assert_eq!(
        stderr(&repair_output),
        "veln: unknown check flag `--repair`\n"
    );
    assert_eq!(project.read("main.veln"), source);

    assert_eq!(apply_output.status.code(), Some(2));
    assert_eq!(stdout(&apply_output), "");
    assert_eq!(
        stderr(&apply_output),
        "veln: unknown check flag `--apply`\n"
    );
    assert_eq!(project.read("main.veln"), source);
}

#[test]
fn check_json_keeps_safe_satisfy_candidate_after_manual_candidate_bound() {
    let project = TestProject::new("safe-satisfy-candidate-bound");
    project.write(
        "main.veln",
        concat!(
            "fn main(target: Int, a: Int, b: Int, c: Int, d: Int, e: Int) -> Int\n",
            "  require target > 0\n",
            "  _value satisfy candidate => candidate > 0\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"hole.unfilled\"",
            "\"candidate_id\":\"symbol-6\",\"name\":\"target\"",
            "\"type\":\"Int\",\"rank\":6,\"reason\":\"satisfy_require_match\"",
            "\"application_policy\":\"safe_repair_candidate\"",
            "\"replacement\":\"target\"",
            "\"satisfy_status\":\"statically_satisfied\"",
            "\"repair_status\":\"statically_satisfied\"",
        ],
    );
    assert!(
        !stdout.contains("\"candidate_id\":\"symbol-6\",\"name\":\"target\",\"type\":\"Int\",\"rank\":6,\"reason\":\"exact_type_match\""),
        "{stdout}"
    );
}

#[test]
fn check_json_reports_malformed_satisfy_clause() {
    let project = TestProject::new("malformed-satisfy");
    project.write(
        "main.veln",
        concat!(
            "fn main() -> ()\n",
            "  _first satisfy => candidate > 0\n",
            "  _second satisfy candidate candidate > 0\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"parse.satisfy_candidate\"",
            "\"message\":\"satisfy clause is missing a candidate binding\"",
            "\"expected\":[\"candidate binding\"]",
            "\"recovery\":{\"strategy\":\"insert_token\",\"anchor\":\"=>\",\"dropped_token_count\":0}",
            "\"id\":\"parse.satisfy_arrow\"",
            "\"message\":\"satisfy clause is missing `=>`\"",
            "\"expected\":[\"=>\"]",
        ],
    );
}
