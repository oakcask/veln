use super::*;

#[test]
fn check_json_reports_checked_core_call_arity_blockers() {
    let project = TestProject::new("check-json-core-call-arity");
    project.write(
        "main.veln",
        concat!(
            "fn add(left: Int, right: Int) -> Int\n",
            "  left + right\n",
            "end\n",
            "fn make_result() -> Result<Int, AppError>\n",
            "  Ok()\n",
            "end\n",
            "fn make_option() -> Option<Int>\n",
            "  Some(1, 2)\n",
            "end\n",
            "pub fn main() -> Int\n",
            "  add(1)\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"core.call_arity_mismatch\"",
            "\"severity\":\"error\"",
            "\"kind\":\"type\"",
            "\"message\":\"call expects 2 argument(s), but got 1\"",
            "\"details\":{\"phase\":\"core_lowering\"",
            "\"reason\":\"call_arity_mismatch\"",
            "\"id\":\"core.result_constructor_arity_mismatch\"",
            "\"message\":\"result constructor expects 1 argument, but got 0\"",
            "\"reason\":\"result_constructor_arity_mismatch\"",
            "\"id\":\"core.option_constructor_arity_mismatch\"",
            "\"message\":\"option constructor expects 1 argument, but got 2\"",
            "\"reason\":\"option_constructor_arity_mismatch\"",
            "\"id\":\"core.missing_expression\"",
            "\"message\":\"expression is missing\"",
            "\"reason\":\"missing_constructor_argument\"",
            "\"expected_type\":\"Int\"",
            "\"expected_argument_count\":2",
            "\"actual_argument_count\":1",
            "\"expected_argument_count\":1",
            "\"actual_argument_count\":0",
            "\"actual_argument_count\":2",
            "\"summary\":{\"diagnostic_count\":4,\"by_severity\":{\"error\":4},\"by_kind\":{\"type\":4}}",
        ],
    );
}

#[test]
fn check_json_reports_checked_core_missing_expression_blocker() {
    let project = TestProject::new("check-json-core-missing-expression");
    project.write(
        "main.veln",
        concat!("pub fn main() -> Int\n", "  1 +\n", "end\n"),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"core.missing_expression\"",
            "\"severity\":\"error\"",
            "\"kind\":\"type\"",
            "\"message\":\"expression is missing\"",
            "\"details\":{\"phase\":\"core_lowering\"",
            "\"reason\":\"missing_expression\"",
            "\"expected_type\":\"Int\"",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"type\":1}}",
        ],
    );
}

#[test]
fn check_json_accepts_executable_concurrency_runtime_calls() {
    let project = TestProject::new("check-json-core-concurrency-runtime");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> String effects [concurrency]\n",
            "  let pair: {tx: Sender<String>, rx: Receiver<String>} = channel::bounded(1)\n",
            "  let _ = channel::send(pair.tx, \"hello\")\n",
            "  match channel::recv(pair.rx)\n",
            "    Some(value) => value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"ok\"",
            "\"diagnostics\":[]",
            "\"summary\":{\"diagnostic_count\":0,\"by_severity\":{},\"by_kind\":{}}",
        ],
    );
}

#[test]
fn check_human_reports_checked_core_call_arity_blocker() {
    let project = TestProject::new("check-human-core-call-arity");
    project.write(
        "main.veln",
        concat!(
            "fn add(left: Int, right: Int) -> Int\n",
            "  left + right\n",
            "end\n",
            "pub fn main() -> Int\n",
            "  add(1)\n",
            "end\n",
        ),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &["main.veln:5:3: error[core.call_arity_mismatch]: call expects 2 argument(s), but got 1"],
    );
}

#[test]
fn check_human_reports_checked_core_missing_expression_blocker() {
    let project = TestProject::new("check-human-core-missing-expression");
    project.write(
        "main.veln",
        concat!("pub fn main() -> Int\n", "  1 +\n", "end\n"),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &["main.veln:4:1: error[core.missing_expression]: expression is missing"],
    );
}

#[test]
fn check_human_accepts_executable_concurrency_runtime_calls() {
    let project = TestProject::new("check-human-core-concurrency-runtime");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> String effects [concurrency]\n",
            "  let pair: {tx: Sender<String>, rx: Receiver<String>} = channel::bounded(1)\n",
            "  let _ = channel::send(pair.tx, \"hello\")\n",
            "  match channel::recv(pair.rx)\n",
            "    Some(value) => value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(stdout(&output), "ok\n");
}

#[test]
fn check_human_reports_duplicate_pattern_binding_origin() {
    let project = TestProject::new("check-human-duplicate-pattern-binding");
    project.write(
        "main.veln",
        concat!(
            "fn main(input: {left: Int, right: Int}) -> Int\n",
            "  match input\n",
            "    {left: value, right: value} => value\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &[
            "main.veln:3:26: error[name.duplicate]: duplicate pattern binding name `value`",
            "  note: main.veln:3:12: First pattern binding with this name is here.",
        ],
    );
}

#[test]
fn check_human_reports_refutable_let_pattern_hint() {
    let project = TestProject::new("check-human-refutable-let-pattern");
    project.write(
        "main.veln",
        concat!(
            "fn main(value: Int) -> ()\n",
            "  let 1 = value\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &[
            "main.veln:2:7: error[pattern.refutable_let]: refutable let pattern is not supported",
            "  note: main.veln:2:7: Use a binding, wildcard, record pattern, or constructor pattern in a let statement.",
        ],
    );
}
