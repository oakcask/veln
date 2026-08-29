use super::*;

#[test]
fn run_reports_missing_entry_before_jdk_execution() {
    let project = TestProject::new("run-missing-entry");
    project.write("main.veln", "pub fn main() -> ()\n  ()\nend\n");

    let output = project.run(&["missing", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &["veln: run entry `missing` was not found"],
    );
}

#[test]
fn run_rejects_wrong_entry_argument_count_before_jdk_execution() {
    let project = TestProject::new("run-entry-params");
    project.write(
        "main.veln",
        "pub fn main(value: String) -> String\n  value\nend\n",
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        concat!(
            "veln: run entry `main` expects 1 argument(s), got 0\n",
            "veln: note: pass entry arguments after `--`\n",
        )
    );
}

#[test]
fn run_rejects_unsupported_entry_parameters_before_jdk_execution() {
    let project = TestProject::new("run-entry-unsupported-param");
    project.write(
        "main.veln",
        "pub fn main(value: Vec<Int>) -> Vec<Int>\n  value\nend\n",
    );

    let output = project.run(&["main", "main.veln", "--", "1"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        concat!(
            "veln: run entry parameter `value` cannot be supplied from a command-line argument\n",
            "veln: note: supported entry argument types are String, Int, Float, and Bool\n",
        )
    );
}

#[test]
fn run_rejects_invalid_typed_entry_argument_before_jdk_execution() {
    let project = TestProject::new("run-entry-invalid-arg");
    project.write(
        "main.veln",
        "pub fn main(value: Int) -> Int\n  value\nend\n",
    );

    let output = project.run(&["main", "main.veln", "--", "not-int"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "veln: invalid Int argument for parameter `value`: `not-int`\n"
    );
}

#[test]
fn run_rejects_invalid_float_entry_argument_before_jdk_execution() {
    let project = TestProject::new("run-entry-invalid-float");
    project.write(
        "main.veln",
        "pub fn main(value: Float) -> Float\n  value\nend\n",
    );

    let output = project.run(&["main", "main.veln", "--", "not-float"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "veln: invalid Float argument for parameter `value`: `not-float`\n"
    );
}

#[test]
fn run_rejects_invalid_bool_entry_argument_before_jdk_execution() {
    let project = TestProject::new("run-entry-invalid-bool");
    project.write(
        "main.veln",
        "pub fn main(value: Bool) -> Bool\n  value\nend\n",
    );

    let output = project.run(&["main", "main.veln", "--", "yes"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "veln: invalid Bool argument for parameter `value`: `yes`\n"
    );
}

#[test]
fn run_json_reports_runtime_contract_failures_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-json-contract-failure");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> ()\n",
            "require false\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["--json", "main", "main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"schema_version\":\"veln-run-json/v0\"",
            "\"command\":\"run\"",
            "\"status\":\"failed\"",
            "\"error\":{\"kind\":\"contract\",\"message\":\"contract failure: require `false` in `main` blame caller\"",
            "\"details\":{\"kind\":\"contract\",\"phase\":\"runtime\",\"clause\":\"require\",\"predicate\":\"false\"",
            "\"function\":\"main\",\"blame\":\"caller\",\"node_id\":\"contract-",
            "\"span\":{\"file\":\"main.veln\"",
        ],
    );
}

#[test]
fn run_json_reports_success_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-json-success");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [stdio]\n",
            "  stdio::println(\"ready\")\n",
            "end\n",
        ),
    );

    let output = project.run(&["--json", "main", "main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"schema_version\":\"veln-run-json/v0\"",
            "\"command\":\"run\"",
            "\"status\":\"passed\"",
            "\"exit_code\":0",
            "\"stdout\":\"ready\\n\"",
            "\"stderr\":\"\"",
            "\"error\":null",
        ],
    );
}
