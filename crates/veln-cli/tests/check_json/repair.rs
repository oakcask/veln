use super::support::*;
use veln_diagnostics::{JsonValue, parse_json_value};

const INVALID_PATH_REPAIR_SOURCE: &str = concat!(
    "fn invalid(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
    "  _value satisfy candidate => candidate.ready == order.ready\n",
    "end\n",
);

const VALID_PATH_REPAIR_SOURCE: &str = concat!(
    "fn valid(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
    "  _value satisfy candidate => candidate.ready == order.ready\n",
    "end\n",
);

#[test]
fn repair_previews_safe_candidates_without_writing() {
    let project = TestProject::new("repair-preview");
    let source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    project.write("main.veln", source);

    let output = project.repair(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "repair-1: Replace hole with `order`",
            "main.veln:2:3 -> `order`",
            "[safe_repair_candidate]",
        ],
    );
    assert_eq!(stderr(&output), "");
    assert_eq!(project.read("main.veln"), source);
}

#[test]
fn repair_json_reports_command_candidate_schema() {
    let project = TestProject::new("repair-json-preview");
    project.write(
        "main.veln",
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  _value satisfy candidate => candidate.ready == order.ready\n",
            "end\n",
        ),
    );

    let output = project.repair(&["--json", "main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"command\":\"repair\"",
            "\"mode\":\"preview\"",
            "\"status\":\"preview\"",
            "\"repair_id\":\"repair-1\"",
            "\"source_candidate_id\":\"symbol-1\"",
            "\"application_policy\":\"safe_repair_candidate\"",
            "\"application_status\":\"unapplied\"",
            "\"verification_command\":\"veln check --json main.veln\"",
            "\"summary\":{\"candidate_count\":1,\"applicable_count\":1,\"applied_count\":0",
        ],
    );
    assert_eq!(stderr(&output), "");
}

#[test]
fn repair_apply_writes_single_safe_candidate_and_verifies() {
    let project = TestProject::new("repair-apply");
    project.write(
        "main.veln",
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  _value satisfy candidate => candidate.ready == order.ready\n",
            "end\n",
        ),
    );

    let output = project.repair(&["--apply", "main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "applied repair-1 at main.veln:2:3 -> `order`",
            "verification passed",
        ],
    );
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  order\n",
            "end\n",
        )
    );
}

#[test]
fn repair_apply_accepts_source_candidate_id_and_verifies() {
    let project = TestProject::new("repair-apply-source-candidate-id");
    project.write(
        "main.veln",
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  _value satisfy candidate => candidate.ready == order.ready\n",
            "end\n",
        ),
    );

    let output = project.repair(&["--apply", "--candidate", "symbol-1", "main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "applied repair-1 at main.veln:2:3 -> `order`",
            "verification passed",
        ],
    );
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  order\n",
            "end\n",
        )
    );
}

#[test]
fn repair_apply_refuses_missing_candidate_id_without_writing() {
    let project = TestProject::new("repair-apply-missing-candidate");
    let source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    project.write("main.veln", source);

    let output = project.repair(&[
        "--json",
        "--apply",
        "--candidate",
        "saved-candidate-1",
        "main.veln",
    ]);
    let stdout = stdout(&output);

    assert!(!output.status.success(), "{stdout}");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"refused\"",
            "\"selected_candidate\":null",
            "\"candidate_count\":1",
            "\"applicable_count\":1",
            "\"applied_count\":0",
            "\"refusal_reason\":\"candidate `saved-candidate-1` was not found\"",
            "\"verification\":{\"status\":\"not_run\"",
        ],
    );
    assert_eq!(project.read("main.veln"), source);
}

#[test]
fn repair_apply_refuses_missing_confirm_id_after_selection_without_writing() {
    let project = TestProject::new("repair-apply-missing-confirm");
    let source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    project.write("main.veln", source);

    let output = project.repair(&[
        "--json",
        "--apply",
        "--candidate",
        "symbol-1",
        "--confirm",
        "missing-confirm",
        "main.veln",
    ]);
    let stdout = stdout(&output);

    assert!(!output.status.success(), "{stdout}");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"refused\"",
            "\"selected_candidate\":{\"repair_id\":\"repair-1\"",
            "\"refusal_reason\":\"confirmed candidate `missing-confirm` was not found\"",
            "\"verification\":{\"status\":\"not_run\"",
        ],
    );
    assert_eq!(project.read("main.veln"), source);
}

#[test]
fn repair_apply_consumes_saved_repair_json_candidate_input() {
    let project = TestProject::new("repair-apply-saved-json");
    project.write(
        "main.veln",
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  _value satisfy candidate => candidate.ready == order.ready\n",
            "end\n",
        ),
    );
    let preview = project.repair(&["--json", "main.veln"]);
    assert!(preview.status.success(), "{}", stderr(&preview));
    project.write("saved-candidates.json", stdout(&preview));

    let output = project.repair(&[
        "--apply",
        "--candidate",
        "symbol-1",
        "saved-candidates.json",
    ]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "applied repair-1 at main.veln:2:3 -> `order`",
            "verification passed",
        ],
    );
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  order\n",
            "end\n",
        )
    );
}

#[test]
fn repair_apply_refuses_saved_candidate_that_is_not_current() {
    let project = TestProject::new("repair-refuses-stale-saved-json");
    let original = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    let changed = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value\n",
        "end\n",
    );
    project.write("main.veln", original);
    let preview = project.repair(&["--json", "main.veln"]);
    assert!(preview.status.success(), "{}", stderr(&preview));
    project.write("saved-candidates.json", stdout(&preview));
    project.write("main.veln", changed);

    let output = project.repair(&["--json", "--apply", "saved-candidates.json"]);
    let stdout = stdout(&output);

    assert!(!output.status.success(), "{stdout}");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"refused\"",
            "\"selected_candidate\":{\"repair_id\":\"repair-1\"",
            "\"refusal_reason\":\"saved candidate is not current\"",
            "\"verification\":{\"status\":\"not_run\"",
        ],
    );
    assert_eq!(project.read("main.veln"), changed);
}

#[test]
fn repair_preview_isolates_candidates_from_invalid_source_path_identities() {
    let project = TestProject::new("repair-source-path-casing-preview");
    project.write("Bad.veln", INVALID_PATH_REPAIR_SOURCE);
    project.write("valid.veln", VALID_PATH_REPAIR_SOURCE);

    let mixed = project.repair(&["--json", "Bad.veln", "valid.veln"]);
    let mixed_stdout = stdout(&mixed);

    assert!(mixed.status.success(), "{}", stderr(&mixed));
    assert_contains_all(
        mixed_stdout,
        &[
            "\"summary\":{\"candidate_count\":1,\"applicable_count\":1,\"applied_count\":0",
            "\"repair_id\":\"repair-1\"",
            "\"span\":{\"file\":\"valid.veln\"",
            "\"replacement\":\"order\"",
        ],
    );
    assert!(!mixed_stdout.contains("\"span\":{\"file\":\"Bad.veln\""));

    let invalid_only = project.repair(&["--json", "Bad.veln"]);
    let invalid_stdout = stdout(&invalid_only);

    assert!(invalid_only.status.success(), "{}", stderr(&invalid_only));
    assert_contains_all(
        invalid_stdout,
        &[
            "\"candidates\":[]",
            "\"summary\":{\"candidate_count\":0,\"applicable_count\":0,\"applied_count\":0",
        ],
    );
}

#[test]
fn repair_preview_keeps_valid_candidate_stable_with_invalid_sibling() {
    let project = TestProject::new("repair-source-path-casing-equivalence");
    project.write("Bad.veln", INVALID_PATH_REPAIR_SOURCE);
    project.write("valid.veln", VALID_PATH_REPAIR_SOURCE);

    let mixed = project.repair(&["--json", "Bad.veln", "valid.veln"]);
    let valid_only = project.repair(&["--json", "valid.veln"]);

    assert!(mixed.status.success(), "{}", stderr(&mixed));
    assert!(valid_only.status.success(), "{}", stderr(&valid_only));
    assert_eq!(
        first_candidate_source(stdout(&mixed)),
        first_candidate_source(stdout(&valid_only))
    );
    assert_eq!(
        first_candidate_edits(stdout(&mixed)),
        first_candidate_edits(stdout(&valid_only))
    );
}

#[test]
fn repair_apply_refuses_current_candidate_from_invalid_source_path_identity() {
    let project = TestProject::new("repair-source-path-casing-current-apply");
    project.write("Bad.veln", INVALID_PATH_REPAIR_SOURCE);

    let output = project.repair(&["--json", "--apply", "Bad.veln"]);
    let stdout = stdout(&output);

    assert!(!output.status.success(), "{stdout}");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"refused\"",
            "\"selected_candidate\":null",
            "\"candidates\":[]",
            "\"refusal_reason\":\"no safe unapplied repair candidates\"",
            "\"verification\":{\"status\":\"not_run\"",
        ],
    );
    assert_eq!(project.read("Bad.veln"), INVALID_PATH_REPAIR_SOURCE);
}

#[test]
fn repair_apply_refuses_saved_candidate_from_invalid_source_path_identity() {
    let project = TestProject::new("repair-source-path-casing-saved-apply");
    project.write("Bad.veln", INVALID_PATH_REPAIR_SOURCE);
    project.write(
        "saved-candidates.json",
        &saved_command_candidate_with_edits(
            "Replace hole with `order`",
            &[("Bad.veln", 3, 64, 9, 70, "order")],
        ),
    );

    let output = project.repair(&["--json", "--apply", "Bad.veln", "saved-candidates.json"]);
    let stdout = stdout(&output);

    assert!(!output.status.success(), "{stdout}");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"refused\"",
            "\"selected_candidate\":{\"repair_id\":\"repair-1\"",
            "\"span\":{\"file\":\"Bad.veln\"",
            "\"refusal_reason\":\"saved candidate is not current\"",
            "\"verification\":{\"status\":\"not_run\"",
        ],
    );
    assert_eq!(project.read("Bad.veln"), INVALID_PATH_REPAIR_SOURCE);
}

fn first_candidate_source(stdout: &str) -> JsonValue {
    first_candidate_field(stdout, "source")
}

fn first_candidate_edits(stdout: &str) -> JsonValue {
    first_candidate_field(stdout, "edits")
}

fn first_candidate_field(stdout: &str, field: &str) -> JsonValue {
    let value = parse_json_value(stdout).expect("repair JSON should parse");
    let candidates = object_field(&value, "candidates").expect("candidates field should exist");
    let JsonValue::Array(candidates) = candidates else {
        panic!("candidates should be an array");
    };
    let first = candidates.first().expect("first candidate should exist");
    object_field(first, field)
        .unwrap_or_else(|| panic!("candidate field `{field}` should exist"))
        .clone()
}

fn object_field<'a>(value: &'a JsonValue, field: &str) -> Option<&'a JsonValue> {
    let JsonValue::Object(fields) = value else {
        return None;
    };
    fields
        .iter()
        .find_map(|(key, value)| (key == field).then_some(value))
}

fn saved_command_candidate_with_edits(
    edit_summary: &str,
    edits: &[(&str, usize, usize, usize, usize, &str)],
) -> String {
    saved_command_candidate_with_optional_verification_command(edit_summary, edits, None)
}

fn saved_command_candidate_with_optional_verification_command(
    edit_summary: &str,
    edits: &[(&str, usize, usize, usize, usize, &str)],
    verification_command: Option<&str>,
) -> String {
    let edits = edits
        .iter()
        .map(
            |(file, start_column, start_offset, end_column, end_offset, replacement)| {
                format!(
                    r#"{{"kind":"replace","span":{{"file":"{file}","start":{{"line":2,"column":{start_column},"offset":{start_offset}}},"end":{{"line":2,"column":{end_column},"offset":{end_offset}}}}},"replacement":"{replacement}"}}"#
                )
            },
        )
        .collect::<Vec<_>>()
        .join(",");
    let verification_command = verification_command
        .map(|command| format!(r#","verification_command":"{command}""#))
        .unwrap_or_default();
    format!(
        r#"{{"candidates":[{{"repair_id":"repair-7","source_candidate_id":"symbol-1","name":"order","application_policy":"safe_repair_candidate","application_status":"unapplied","edit_summary":"{edit_summary}","edits":[{edits}]{verification_command}}}]}}"#
    )
}

#[test]
fn repair_apply_writes_saved_multi_span_command_candidate_and_verifies() {
    let project = TestProject::new("repair-saved-multi-span");
    let source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    project.write("main.veln", source);
    project.write(
        "saved-candidates.json",
        &saved_command_candidate_with_edits(
            "Replace two spans",
            &[
                ("main.veln", 3, 61, 9, 67, "order"),
                ("main.veln", 9, 67, 61, 119, ""),
            ],
        ),
    );

    let output = project.repair(&["--json", "--apply", "saved-candidates.json"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{stdout}");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"applied\"",
            "\"selected_candidate\":{\"repair_id\":\"repair-1\"",
            "\"candidate_count\":1",
            "\"applied_count\":2",
            "\"verification\":{\"status\":\"passed\"",
        ],
    );
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  order\n",
            "end\n",
        )
    );
}

#[test]
fn repair_apply_writes_saved_multi_file_command_candidate_and_verifies() {
    let project = TestProject::new("repair-saved-multi-file");
    let main_source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    let helper_source = concat!(
        "fn helper(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    project.write("main.veln", main_source);
    project.write("helper.veln", helper_source);
    project.write(
        "saved-candidates.json",
        &saved_command_candidate_with_edits(
            "Replace across files",
            &[
                ("main.veln", 3, 61, 9, 67, "order"),
                ("helper.veln", 3, 63, 9, 69, "order"),
            ],
        ),
    );

    let output = project.repair(&["--json", "--apply", "saved-candidates.json"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{stdout}");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"applied\"",
            "\"selected_candidate\":{\"repair_id\":\"repair-1\"",
            "\"candidate_count\":1",
            "\"applied_count\":2",
            "\"verification\":{\"status\":\"passed\"",
        ],
    );
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  order\n",
            "end\n",
        )
    );
    assert_eq!(
        project.read("helper.veln"),
        concat!(
            "fn helper(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  order\n",
            "end\n",
        )
    );
}

#[test]
fn repair_apply_records_verification_command_without_running_it() {
    let project = TestProject::new("repair-verification-command-not-run");
    let source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    project.write("main.veln", source);
    project.write(
        "saved-candidates.json",
        &saved_command_candidate_with_optional_verification_command(
            "Replace hole with `order`",
            &[("main.veln", 3, 61, 9, 67, "order")],
            Some("touch verification-ran"),
        ),
    );

    let output = project.repair(&["--json", "--apply", "saved-candidates.json"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{stdout}");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"applied\"",
            "\"verification_command\":\"touch verification-ran\"",
            "\"verification\":{\"status\":\"passed\",\"command\":\"touch verification-ran\"",
        ],
    );
    assert!(!project.root.join("verification-ran").exists());
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  order\n",
            "end\n",
        )
    );
}

#[test]
fn repair_apply_rolls_back_saved_multi_file_candidate_when_verification_fails() {
    let project = TestProject::new("repair-saved-multi-file-rollback");
    let main_source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    let helper_source = concat!(
        "fn helper(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    project.write("main.veln", main_source);
    project.write("helper.veln", helper_source);
    project.write("bad.veln", "fn broken() -> Int\n  1\n");
    project.write(
        "saved-candidates.json",
        &saved_command_candidate_with_edits(
            "Replace across files",
            &[
                ("main.veln", 3, 61, 9, 67, "order"),
                ("helper.veln", 3, 63, 9, 69, "order"),
            ],
        ),
    );

    let output = project.repair(&["--json", "--apply", "saved-candidates.json"]);
    let stdout = stdout(&output);

    assert!(!output.status.success(), "{stdout}");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"refused\"",
            "\"refusal_reason\":\"verification failed\"",
            "\"verification\":{\"status\":\"failed\"",
            "\"id\":\"parse.expected_end\"",
        ],
    );
    assert_eq!(project.read("main.veln"), main_source);
    assert_eq!(project.read("helper.veln"), helper_source);
}

#[test]
fn repair_refuses_manual_review_candidates() {
    let project = TestProject::new("repair-refuses-manual-review");
    let source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value\n",
        "end\n",
    );
    project.write("main.veln", source);

    let output = project.repair(&["--apply", "main.veln"]);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert_contains_all(
        stdout(&output),
        &["repair refused: no safe unapplied repair candidates"],
    );
    assert_eq!(project.read("main.veln"), source);
}

#[test]
fn repair_refuses_manual_review_candidate_without_override() {
    let project = TestProject::new("repair-refuses-manual-review-confirmed");
    let source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value\n",
        "end\n",
    );
    project.write("main.veln", source);

    let output = project.repair(&["--apply", "--confirm", "symbol-1", "main.veln"]);

    assert!(!output.status.success(), "{}", stdout(&output));
    assert_contains_all(
        stdout(&output),
        &["repair refused: candidate is not safe to apply automatically"],
    );
    assert_eq!(project.read("main.veln"), source);
}

#[test]
fn repair_override_applies_manual_review_candidate_and_records_confirmation() {
    let project = TestProject::new("repair-override-manual-review");
    project.write(
        "main.veln",
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  _value\n",
            "end\n",
        ),
    );

    let output = project.repair(&[
        "--json",
        "--apply",
        "--override",
        "--confirm",
        "symbol-1",
        "main.veln",
    ]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{stdout}");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"applied\"",
            "\"confirmation\":{\"confirmed_candidate_id\":\"symbol-1\",\"repair_id\":\"repair-1\",\"source_candidate_id\":\"symbol-1\",\"override\":true}",
            "\"override\":{\"application_policy\":\"manual_review_required\",\"application_status\":\"unapplied\"",
            "\"accepted_obligations\":[\"manual_review_required\"",
            "\"verification\":{\"status\":\"passed\"",
        ],
    );
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
            "  order\n",
            "end\n",
        )
    );
}

#[test]
fn repair_rolls_back_when_verification_fails() {
    let project = TestProject::new("repair-verification-failure");
    let source = concat!(
        "fn main(order: {ready: Bool, paid: Bool}) -> {ready: Bool}\n",
        "  _value satisfy candidate => candidate.ready == order.ready\n",
        "end\n",
    );
    project.write("main.veln", source);
    project.write("bad.veln", "fn broken() -> Int\n  1\n");

    let output = project.repair(&["--json", "--apply"]);
    let stdout = stdout(&output);

    assert!(!output.status.success(), "{stdout}");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"refused\"",
            "\"refusal_reason\":\"verification failed\"",
            "\"verification\":{\"status\":\"failed\"",
            "\"id\":\"parse.expected_end\"",
        ],
    );
    assert_eq!(project.read("main.veln"), source);
}
