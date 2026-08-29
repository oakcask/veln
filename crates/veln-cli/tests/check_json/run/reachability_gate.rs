use super::*;

#[test]
fn run_blocks_reachable_holes_before_jdk_execution() {
    let project = TestProject::new("run-hole");
    project.write(
        "main.veln",
        "pub fn main() -> Result<(), AppError>\n  _\nend\n",
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Result<(), AppError>` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_function_values_before_jdk_execution() {
    let project = TestProject::new("run-function-value-hole");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> Vec<String>\n",
            "  vec_map([1], stringify)\n",
            "end\n",
            "fn stringify(value: Int) -> String\n",
            "  _\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `String` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_qualified_function_values_before_jdk_execution() {
    let project = TestProject::new("run-qualified-function-value-hole");
    project.write(
        "app/text.veln",
        concat!("pub fn stringify(value: Int) -> String\n", "  _\n", "end\n",),
    );
    project.write(
        "app/main.veln",
        concat!(
            "use app::text\n",
            "pub fn main() -> Vec<String>\n",
            "  vec_map([1], app::text::stringify)\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "app/main.veln", "app/text.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `String` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_opaque_function_value_calls_before_jdk_execution() {
    let project = TestProject::new("run-opaque-function-value-hole");
    project.write(
        "main.veln",
        concat!(
            "fn invoke(job: fn() -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool\n",
            "  _\n",
            "end\n",
            "pub fn main() -> Bool\n",
            "  invoke(ready)\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Bool` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_opaque_local_function_value_calls_before_jdk_execution() {
    let project = TestProject::new("run-opaque-local-function-value-hole");
    project.write(
        "main.veln",
        concat!(
            "fn invoke() -> Bool\n",
            "  let job: fn() -> Bool = ready\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
            "fn risky() -> Bool\n",
            "  _\n",
            "end\n",
            "pub fn main() -> Bool\n",
            "  invoke()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Bool` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_contract_helpers_before_jdk_execution() {
    let project = TestProject::new("run-contract-helper-hole");
    project.write(
        "main.veln",
        concat!(
            "fn positive(value: Int) -> Bool\n",
            "  _\n",
            "end\n",
            "pub fn main() -> output: Int\n",
            "  ensure positive(output)\n",
            "  1\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Bool` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_contract_function_values_before_jdk_execution() {
    let project = TestProject::new("run-contract-function-value-hole");
    project.write(
        "main.veln",
        concat!(
            "fn accepts(job: fn() -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "fn ready() -> Bool\n",
            "  _\n",
            "end\n",
            "pub fn main() -> ()\n",
            "  require accepts(ready)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Bool` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_blocks_holes_reachable_through_imported_calls_before_jdk_execution() {
    let project = TestProject::new("run-imported-call-hole");
    project.write(
        "app/util.veln",
        concat!("pub fn value() -> Int\n", "  _\n", "end\n",),
    );
    project.write(
        "app/main.veln",
        concat!(
            "use app::util\n",
            "pub fn main() -> Int\n",
            "  app::util::value()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "app/main.veln", "app/util.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Int` value",
            "veln: run blocked: checked program is not executable",
        ],
    );
}

#[test]
fn run_reports_parse_diagnostics_before_semantic_analysis() {
    let project = TestProject::new("run-parse-diagnostics");
    project.write("main.veln", "fn main() -> ()\n  @\nend\n");

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &["main.veln:2:3: error[parse.invalid_token]: invalid token in expression"],
    );
}

#[test]
fn run_reports_semantic_diagnostics_before_lowering() {
    let project = TestProject::new("run-semantic-diagnostics");
    project.write("main.veln", "pub fn main() -> Int\n  \"no\"\nend\n");

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &["main.veln:2:3: error[type.mismatch]: expected `Int`, but found `String`"],
    );
}

#[test]
fn run_ignores_unreachable_semantic_diagnostics() {
    let project = TestProject::new("run-unreachable-semantic-diagnostics");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
            "fn later() -> Int\n",
            "  \"no\"\n",
            "end\n",
        ),
    );

    let output = project.run_with_path(&["main", "main.veln"], "");
    let stderr = stderr(&output);

    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr,
        &["veln: `java` was not found; install a JDK to use `veln run`"],
    );
    assert!(
        !stderr.contains("type.mismatch"),
        "unreachable diagnostic should not block run: {stderr}"
    );
}

#[test]
fn run_ignores_function_shadowed_by_local_binding() {
    let project = TestProject::new("run-local-shadowed-function");
    project.write(
        "main.veln",
        concat!(
            "fn helper() -> Int\n",
            "  _\n",
            "end\n",
            "pub fn main() -> Int\n",
            "  let helper = 1\n",
            "  helper\n",
            "end\n",
        ),
    );

    let output = project.run_with_path(&["main", "main.veln"], "");
    let stderr = stderr(&output);

    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr,
        &["veln: `java` was not found; install a JDK to use `veln run`"],
    );
    assert!(
        !stderr.contains("hole.unfilled"),
        "shadowed function should not be reachable: {stderr}"
    );
}

#[test]
fn run_does_not_block_unreachable_holes_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-unreachable-hole");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [stdio]\n",
            "  stdio::println(\"ran\")\n",
            "  ()\n",
            "end\n",
            "fn later() -> ()\n",
            "  _\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "ran\n");
    assert_eq!(stderr(&output), "");
}
