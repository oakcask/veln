use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

struct TestProject {
    root: PathBuf,
}

impl TestProject {
    fn new(name: &str) -> Self {
        let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veln-cli-check-json-{name}-{}-{nanos}-{id}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("test project directory should be created");
        Self { root }
    }

    fn write(&self, path: &str, text: &str) {
        let path = self.root.join(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("fixture parent should be created");
        }
        fs::write(path, text).expect("fixture should be written");
    }

    fn check_json(&self, args: &[&str]) -> Output {
        self.veln(&["check", "--json"], args)
    }

    fn fmt(&self, args: &[&str]) -> Output {
        self.veln(&["fmt"], args)
    }

    fn assert_fmt_idempotent(&self, args: &[&str], expected_files: &[(&str, &str)]) {
        let output = self.fmt(args);

        assert!(output.status.success(), "{}", stderr(&output));
        assert_eq!(stdout(&output), "");
        self.assert_files(expected_files);

        let second_output = self.fmt(args);

        assert!(second_output.status.success(), "{}", stderr(&second_output));
        assert_eq!(stdout(&second_output), "");
        self.assert_files(expected_files);
    }

    fn assert_files(&self, expected_files: &[(&str, &str)]) {
        for (path, expected) in expected_files {
            assert_eq!(self.read(path), *expected);
        }
    }

    fn run(&self, args: &[&str]) -> Output {
        self.veln(&["run"], args)
    }

    fn test(&self, args: &[&str]) -> Output {
        self.veln(&["test"], args)
    }

    fn repair(&self, args: &[&str]) -> Output {
        self.veln(&["repair"], args)
    }

    fn run_with_path(&self, args: &[&str], path: &str) -> Output {
        self.veln_with_path("run", args, path)
    }

    fn veln_with_path(&self, subcommand: &str, args: &[&str], path: &str) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_veln"));
        command.current_dir(&self.root);
        command.env("PATH", path);
        command.arg(subcommand);
        for arg in args {
            command.arg(arg);
        }
        command.output().expect("veln should run")
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.root.join(path)).expect("fixture should be read")
    }

    fn veln(&self, command_args: &[&str], args: &[&str]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_veln"));
        command.current_dir(&self.root);
        for arg in command_args {
            command.arg(arg);
        }
        for arg in args {
            command.arg(arg);
        }
        command.output().expect("veln should run")
    }
}

impl Drop for TestProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn repo_file(path: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("crate should live under repository root")
        .join(path)
        .to_string_lossy()
        .into_owned()
}

#[test]
fn explain_reports_diagnostic_help() {
    let project = TestProject::new("cli-explain");

    let output = project.veln(&["explain"], &["hole.unfilled"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("hole.unfilled: unfilled typed hole"));
    assert!(stdout(&output).contains("Meaning:"));
    assert!(stdout(&output).contains("Repair:"));
    assert_eq!(stderr(&output), "");
}

#[test]
fn explain_lists_known_diagnostics() {
    let project = TestProject::new("cli-explain-list");

    let output = project.veln(&["explain"], &["--list"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains("hole.unfilled - unfilled typed hole"));
    assert!(
        stdout(&output)
            .contains("parse.contract_predicate - unsupported contract predicate syntax")
    );
    assert!(stdout(&output).contains("parse.satisfy_candidate - missing satisfy candidate"));
    assert!(stdout(&output).contains("parse.satisfy_arrow - missing satisfy arrow"));
    assert!(stdout(&output).contains("hole.satisfy_candidate_unused - unused satisfy candidate"));
    assert_eq!(stderr(&output), "");
}

#[test]
fn explain_list_takes_precedence_over_diagnostic_id() {
    let project = TestProject::new("cli-explain-list-with-id");

    let output = project.veln(&["explain"], &["--list", "hole.unfilled"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout.contains("hole.unfilled - unfilled typed hole"));
    assert!(!stdout.contains("Meaning:"));
    assert_eq!(stderr(&output), "");
}

#[test]
fn explain_reports_missing_and_unknown_diagnostic_ids() {
    let project = TestProject::new("cli-explain-errors");

    let missing = project.veln(&["explain"], &[]);
    let unknown = project.veln(&["explain"], &["unknown.id"]);

    assert_eq!(missing.status.code(), Some(2));
    assert_eq!(
        stderr(&missing),
        "veln: explain requires a diagnostic id or --list\n"
    );
    assert_eq!(stdout(&missing), "");
    assert_eq!(unknown.status.code(), Some(2));
    assert_eq!(
        stderr(&unknown),
        "veln: no explanation for diagnostic `unknown.id`\n"
    );
    assert_eq!(stdout(&unknown), "");
}

#[test]
fn cli_prints_version() {
    let project = TestProject::new("cli-version");

    let output = project.veln(&[], &["--version"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "veln 0.1.0\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn cli_reports_parser_errors_before_project_discovery() {
    let project = TestProject::new("cli-parser-errors");

    let unknown_command = project.veln(&[], &["wat"]);
    let unknown_doc_flag = project.veln(&["doc"], &["--wat"]);
    let unknown_repair_flag = project.veln(&["repair"], &["--wat"]);
    let unknown_check_flag = project.veln(&["check"], &["--wat"]);
    let unknown_run_flag = project.veln(&["run"], &["--wat"]);
    let unknown_test_flag = project.veln(&["test"], &["--wat"]);
    let unknown_explain_flag = project.veln(&["explain"], &["--wat"]);
    let unexpected_explain_argument = project.veln(&["explain"], &["hole.unfilled", "extra"]);
    let missing_run_entry = project.veln(&["run"], &[]);

    assert_eq!(unknown_command.status.code(), Some(2));
    assert_eq!(stdout(&unknown_command), "");
    assert_eq!(stderr(&unknown_command), "veln: unknown command `wat`\n");

    assert_eq!(unknown_doc_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_doc_flag), "");
    assert_eq!(
        stderr(&unknown_doc_flag),
        "veln: unknown doc flag `--wat`\n"
    );

    assert_eq!(unknown_repair_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_repair_flag), "");
    assert_eq!(
        stderr(&unknown_repair_flag),
        "veln: unknown repair flag `--wat`\n"
    );

    assert_eq!(unknown_check_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_check_flag), "");
    assert_eq!(
        stderr(&unknown_check_flag),
        "veln: unknown check flag `--wat`\n"
    );

    assert_eq!(unknown_run_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_run_flag), "");
    assert_eq!(
        stderr(&unknown_run_flag),
        "veln: unknown run flag `--wat`\n"
    );

    assert_eq!(unknown_test_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_test_flag), "");
    assert_eq!(
        stderr(&unknown_test_flag),
        "veln: unknown test flag `--wat`\n"
    );

    assert_eq!(unknown_explain_flag.status.code(), Some(2));
    assert_eq!(stdout(&unknown_explain_flag), "");
    assert_eq!(
        stderr(&unknown_explain_flag),
        "veln: unknown explain flag `--wat`\n"
    );

    assert_eq!(unexpected_explain_argument.status.code(), Some(2));
    assert_eq!(stdout(&unexpected_explain_argument), "");
    assert_eq!(
        stderr(&unexpected_explain_argument),
        "veln: unexpected explain argument `extra`\n"
    );

    assert_eq!(missing_run_entry.status.code(), Some(2));
    assert_eq!(stdout(&missing_run_entry), "");
    assert_eq!(
        stderr(&missing_run_entry),
        "veln: run requires an entry function name\n"
    );
}

#[test]
fn check_json_accepts_valid_input() {
    let project = TestProject::new("valid");
    project.write(
        "main.veln",
        "pub fn main() -> Result((), AppError) effects [stdio]\n  Ok(())\nend\n",
    );

    let output = project.check_json(&["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        concat!(
            "{\"schema_version\":1,",
            "\"tool\":{\"name\":\"veln\",\"version\":\"0.1.0\"},",
            "\"status\":\"ok\",",
            "\"diagnostics\":[],",
            "\"summary\":{\"diagnostic_count\":0,\"by_severity\":{},\"by_kind\":{}}}\n"
        )
    );
}

#[test]
fn check_human_prints_ok_for_valid_input() {
    let project = TestProject::new("check-human-ok");
    project.write("main.veln", "pub fn main() -> ()\n  ()\nend\n");

    let output = project.veln(&["check"], &["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "ok\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn check_human_reports_diagnostics_to_stdout() {
    let project = TestProject::new("check-human-diagnostics");
    project.write("main.veln", "pub fn main() -> Int\n  \"no\"\nend\n");

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &["main.veln:2:3: error[type.mismatch]: expected `Int`, but found `String`"],
    );
}

#[test]
fn check_human_reports_method_call_repair_note() {
    let project = TestProject::new("check-human-method-call");
    project.write(
        "main.veln",
        "pub fn main(value: String) -> Int\n  value.len()\nend\n",
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &[
            "main.veln:2:9: error[type.method_call]: method call syntax is not supported",
            "  note: main.veln:2:3: Use a named function call with the receiver as an explicit argument.",
        ],
    );
}

#[test]
fn check_human_reports_match_exhaustiveness_context() {
    let project = TestProject::new("check-human-match-exhaustiveness");
    project.write(
        "main.veln",
        concat!(
            "fn main(value: Option(Int)) -> String\n",
            "  match value\n",
            "    Some(count) => \"some\"\n",
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
            "main.veln:2:3: error[type.match_non_exhaustive]: match is missing case None",
            "  note: main.veln:2:9: Scrutinee has type `Option(Int)`.",
            "  note: main.veln:3:5: This arm covers Some(_).",
        ],
    );
}

#[test]
fn check_human_reports_missing_module_identity_for_imports() {
    let project = TestProject::new("check-human-module-identity");
    project.write(
        "main.veln",
        concat!("use platform.io\n", "fn main() -> ()\n", "  ()\n", "end\n",),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &[
            "main.veln:1:1: error[module.missing_identity]: module import requires a module identity",
            "  note: Add a `mod` declaration before `use` declarations.",
        ],
    );
}

#[test]
fn check_json_reports_missing_module_identity_for_imports() {
    let project = TestProject::new("check-json-module-identity");
    project.write(
        "main.veln",
        concat!("use platform.io\n", "fn main() -> ()\n", "  ()\n", "end\n",),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"module.missing_identity\"",
            "\"kind\":\"module\"",
            "\"message\":\"module import requires a module identity\"",
            "\"details\":{\"phase\":\"module\",\"node_id\":\"use-1\",\"field\":\"module_identity\",\"expected_owner\":\"source\",\"observed_owner\":\"missing\"}",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"module\":1}}",
        ],
    );
}

#[test]
fn check_human_reports_manifest_module_name_drift() {
    let project = TestProject::new("check-human-manifest-name-drift");
    project.write("veln.toml", "[modules]\n\"main.veln\" = \"app.manifest\"\n");
    project.write(
        "main.veln",
        concat!(
            "mod app.source\n",
            "pub fn main() -> ()\n",
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
            "veln.toml:2:16: error[module.metadata_drift]: manifest module name `app.manifest` does not match source module `app.source`",
            "  note: main.veln:1:1: The source `mod` declaration owns the compiler-visible module name.",
            "  note: Update the manifest entry or remove the duplicated module name.",
        ],
    );
}

#[test]
fn check_human_reports_manifest_module_without_source_owner() {
    let project = TestProject::new("check-human-manifest-without-source-owner");
    project.write("veln.toml", "[modules]\n\"main.veln\" = \"app.manifest\"\n");
    project.write(
        "main.veln",
        concat!("pub fn main() -> ()\n", "  ()\n", "end\n"),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &[
            "veln.toml:2:16: error[module.metadata_drift]: manifest module name `app.manifest` has no source `mod` owner",
            "  note: Add a `mod` declaration to the source file or remove the manifest module name.",
        ],
    );
}

#[test]
fn check_human_accepts_matching_manifest_module_name() {
    let project = TestProject::new("check-human-matching-manifest-module");
    project.write("veln.toml", "[modules]\n\"main.veln\" = \"app.main\"\n");
    project.write(
        "main.veln",
        concat!("mod app.main\n", "pub fn main() -> ()\n", "  ()\n", "end\n",),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "ok\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn check_json_reports_manifest_module_name_drift() {
    let project = TestProject::new("check-json-manifest-name-drift");
    project.write("veln.toml", "[modules]\n\"main.veln\" = \"app.manifest\"\n");
    project.write(
        "main.veln",
        concat!(
            "mod app.source\n",
            "pub fn main() -> ()\n",
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
            "\"id\":\"module.metadata_drift\"",
            "\"kind\":\"module\"",
            "\"message\":\"manifest module name `app.manifest` does not match source module `app.source`\"",
            "\"span\":{\"file\":\"veln.toml\",\"start\":{\"line\":2,\"column\":16,\"offset\":25},\"end\":{\"line\":2,\"column\":28,\"offset\":37}}",
            "\"details\":{\"phase\":\"module\",\"field\":\"module_identity\",\"canonical_owner\":\"source\",\"derived_owner\":\"manifest\",\"expected_value\":\"app.source\",\"observed_value\":\"app.manifest\",\"manifest_path\":\"veln.toml\",\"source_path\":\"main.veln\"}",
            "\"related\":[{\"kind\":\"canonical_owner\",\"message\":\"The source `mod` declaration owns the compiler-visible module name.\"",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":1,\"column\":1,\"offset\":0},\"end\":{\"line\":2,\"column\":1,\"offset\":15}}}",
            "{\"message\":\"Update the manifest entry or remove the duplicated module name.\"}]",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"module\":1}}",
        ],
    );
}

#[test]
fn check_json_reports_manifest_module_without_source_owner() {
    let project = TestProject::new("check-json-manifest-without-source-owner");
    project.write("veln.toml", "[modules]\n\"main.veln\" = \"app.manifest\"\n");
    project.write(
        "main.veln",
        concat!("pub fn main() -> ()\n", "  ()\n", "end\n"),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"module.metadata_drift\"",
            "\"kind\":\"module\"",
            "\"message\":\"manifest module name `app.manifest` has no source `mod` owner\"",
            "\"span\":{\"file\":\"veln.toml\",\"start\":{\"line\":2,\"column\":16,\"offset\":25},\"end\":{\"line\":2,\"column\":28,\"offset\":37}}",
            "\"details\":{\"phase\":\"module\",\"field\":\"module_identity\",\"canonical_owner\":\"source\",\"derived_owner\":\"manifest\",\"observed_value\":\"app.manifest\",\"manifest_path\":\"veln.toml\",\"source_path\":\"main.veln\"}",
            "\"related\":[{\"message\":\"Add a `mod` declaration to the source file or remove the manifest module name.\"}]",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"module\":1}}",
        ],
    );
}

#[test]
fn check_json_reports_checked_core_call_arity_blockers() {
    let project = TestProject::new("check-json-core-call-arity");
    project.write(
        "main.veln",
        concat!(
            "fn add(left: Int, right: Int) -> Int\n",
            "  left + right\n",
            "end\n",
            "fn make_result() -> Result(Int, AppError)\n",
            "  Ok()\n",
            "end\n",
            "fn make_option() -> Option(Int)\n",
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
            "  let pair: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
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
            "  let pair: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
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
            "fn main(value: Option(Int)) -> ()\n",
            "  let Some(amount) = value\n",
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
            "  note: main.veln:2:7: Use a binding, wildcard, or record pattern in a let statement.",
        ],
    );
}

#[test]
fn fmt_formats_supported_golden_and_is_idempotent() {
    let project = TestProject::new("fmt-golden");
    project.write(
        "main.veln",
        concat!(
            "mod app\n",
            "use stdio\n",
            "pub   fn   main ( name : String ) -> Result ( () , AppError ) effects [ stdio ]\n",
            " require name != \"\"\n",
            " let payload : { message : String, values : Vec(Int) } = { message : name , values : [ 1 , 2 , add ( 3 , 4 ) ] }\n",
            " stdio::println ( payload )\n",
            " _result satisfy candidate => candidate != \"\"\n",
            "end\n",
            "fn helper(value)\n",
            "value\n",
            "end\n",
        ),
    );

    let output = project.fmt(&["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "mod app\n",
            "use stdio\n",
            "\n",
            "pub fn main(name: String) -> Result((), AppError) effects [stdio]\n",
            "\trequire name != \"\"\n",
            "\tlet payload: { message : String, values : Vec(Int) } = { message: name, values: [1, 2, add(3, 4)] }\n",
            "\tstdio::println(payload)\n",
            "\t_result satisfy candidate => candidate != \"\"\n",
            "end\n",
            "\n",
            "fn helper(value)\n",
            "\tvalue\n",
            "end\n",
        )
    );

    let second_output = project.fmt(&["main.veln"]);

    assert!(second_output.status.success(), "{}", stderr(&second_output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "mod app\n",
            "use stdio\n",
            "\n",
            "pub fn main(name: String) -> Result((), AppError) effects [stdio]\n",
            "\trequire name != \"\"\n",
            "\tlet payload: { message : String, values : Vec(Int) } = { message: name, values: [1, 2, add(3, 4)] }\n",
            "\tstdio::println(payload)\n",
            "\t_result satisfy candidate => candidate != \"\"\n",
            "end\n",
            "\n",
            "fn helper(value)\n",
            "\tvalue\n",
            "end\n",
        )
    );
}

#[test]
fn fmt_formats_focused_supported_forms_across_multiple_files() {
    let project = TestProject::new("fmt-focused-golden");
    project.write(
        "main.veln",
        concat!(
            "fn parse ( raw : String ) -> Result ( Int , AppError )\n",
            " Ok ( 1 )\n",
            "end\n",
            "pub fn main ( raw : String ) -> Result ( { value : Int, tags : Vec(String) } , AppError )\n",
            " ensure output.value >= 0 and not ( output.value == - 1 )\n",
            " let parsed : Int = parse ( raw ) ?\n",
            " { value : parsed + 1 * ( 2 + 3 ) , tags : [ choose ( raw , \"fallback\" ) , \"done\" ] }\n",
            "end\n",
        ),
    );
    project.write(
        "helpers.veln",
        concat!(
            "fn choose ( value : String , fallback : String ) -> String\n",
            " if_missing ( { primary : value, nested : { fallback : fallback } } )\n",
            "end\n",
        ),
    );

    let output = project.fmt(&["main.veln", "helpers.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn parse(raw: String) -> Result(Int, AppError)\n",
            "\tOk(1)\n",
            "end\n",
            "\n",
            "pub fn main(raw: String) -> Result({ value : Int, tags : Vec(String) }, AppError)\n",
            "\tensure output.value >= 0 and not(output.value == - 1)\n",
            "\tlet parsed: Int = parse(raw)?\n",
            "\t{ value: parsed + 1 * (2 + 3), tags: [choose(raw, \"fallback\"), \"done\"] }\n",
            "end\n",
        )
    );
    assert_eq!(
        project.read("helpers.veln"),
        concat!(
            "fn choose(value: String, fallback: String) -> String\n",
            "\tif_missing({ primary: value, nested: { fallback: fallback } })\n",
            "end\n",
        )
    );

    let second_output = project.fmt(&["main.veln", "helpers.veln"]);

    assert!(second_output.status.success(), "{}", stderr(&second_output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn parse(raw: String) -> Result(Int, AppError)\n",
            "\tOk(1)\n",
            "end\n",
            "\n",
            "pub fn main(raw: String) -> Result({ value : Int, tags : Vec(String) }, AppError)\n",
            "\tensure output.value >= 0 and not(output.value == - 1)\n",
            "\tlet parsed: Int = parse(raw)?\n",
            "\t{ value: parsed + 1 * (2 + 3), tags: [choose(raw, \"fallback\"), \"done\"] }\n",
            "end\n",
        )
    );
    assert_eq!(
        project.read("helpers.veln"),
        concat!(
            "fn choose(value: String, fallback: String) -> String\n",
            "\tif_missing({ primary: value, nested: { fallback: fallback } })\n",
            "end\n",
        )
    );
}

#[test]
fn fmt_formats_match_expressions_with_tab_relative_indentation() {
    let project = TestProject::new("fmt-match-indent");
    project.write(
        "main.veln",
        concat!(
            "fn describe ( value : Option(Int) ) -> String\n",
            " match value\n",
            " Some(count) => \"some\"\n",
            " None => \"none\"\n",
            " end\n",
            "end\n",
            "fn nested ( value : Option(Int) ) -> { labels : Vec(String), primary : String }\n",
            " { labels : [ wrap ( match value\n",
            " Some(count) => \"some\"\n",
            " None => \"none\"\n",
            " end ) ], primary : match value\n",
            " Some(count) => \"some\"\n",
            " None => \"none\"\n",
            " end }\n",
            "end\n",
        ),
    );

    let output = project.fmt(&["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn describe(value: Option(Int)) -> String\n",
            "\tmatch value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend\n",
            "end\n",
            "\n",
            "fn nested(value: Option(Int)) -> { labels : Vec(String), primary : String }\n",
            "\t{ labels: [wrap(match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend)], primary: match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend }\n",
            "end\n",
        )
    );

    let second_output = project.fmt(&["main.veln"]);

    assert!(second_output.status.success(), "{}", stderr(&second_output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn describe(value: Option(Int)) -> String\n",
            "\tmatch value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend\n",
            "end\n",
            "\n",
            "fn nested(value: Option(Int)) -> { labels : Vec(String), primary : String }\n",
            "\t{ labels: [wrap(match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend)], primary: match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend }\n",
            "end\n",
        )
    );
}

#[test]
fn fmt_rejects_unknown_flags_before_writing_files() {
    let project = TestProject::new("fmt-unknown-flag");
    let text = "fn   ok ( ) -> ()\n()\nend\n";
    project.write("main.veln", text);

    let output = project.fmt(&["--json", "main.veln"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "veln: unknown fmt flag `--json`\n");
    assert_eq!(project.read("main.veln"), text);
}

#[test]
fn fmt_preserves_files_when_any_input_has_parse_errors() {
    let project = TestProject::new("fmt-parse-error");
    project.write("bad.veln", "fn bad() -> ()\n  @\nend\n");
    project.write("good.veln", "fn   ok ( ) -> ()\n()\nend\n");

    let output = project.fmt(&["bad.veln", "good.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(project.read("bad.veln"), "fn bad() -> ()\n  @\nend\n");
    assert_eq!(project.read("good.veln"), "fn   ok ( ) -> ()\n()\nend\n");
    assert_contains_all(
        stderr(&output),
        &["bad.veln:2:3: error[parse.invalid_token]: invalid token in expression"],
    );
}

#[test]
fn fmt_formats_comment_bearing_files() {
    let project = TestProject::new("fmt-comments");
    let text = concat!(
        "# keep leading comment\n",
        "fn   main ( ) -> ()\n",
        "  () # keep trailing comment\n",
        "# keep closing comment\n",
        "end # keep end comment\n",
    );
    project.write("main.veln", text);

    let output = project.fmt(&["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "# keep leading comment\n",
            "fn main() -> ()\n",
            "\t()  # keep trailing comment\n",
            "\t# keep closing comment\n",
            "end  # keep end comment\n",
        )
    );
}

#[test]
fn fmt_rejects_legacy_slash_comment_source() {
    let project = TestProject::new("fmt-slash-comments");
    project.write(
        "main.veln",
        concat!(
            "// keep leading comment\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.fmt(&["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "// keep leading comment\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        )
    );
    assert_contains_all(
        stderr(&output),
        &[
            "main.veln:1:1: error[parse.expected_item]: expected a function, test, or type declaration",
        ],
    );
}

#[test]
fn fmt_formats_files_with_attached_standalone_comments() {
    let project = TestProject::new("fmt-attached-comments");
    project.write(
        "main.veln",
        concat!(
            "# module docs\n",
            "mod   app\n",
            "## public docs\n",
            "pub  fn   main ( value : Unit ) -> Unit effects [stdio]\n",
            "# return docs\n",
            "()\n",
            "end\n",
        ),
    );

    let expected = concat!(
        "# module docs\n",
        "mod app\n",
        "\n",
        "## public docs\n",
        "pub fn main(value: ()) -> () effects [stdio]\n",
        "\t# return docs\n",
        "\t()\n",
        "end\n",
    );
    project.assert_fmt_idempotent(&["main.veln"], &[("main.veln", expected)]);
}

#[test]
fn fmt_attaches_comments_to_imports_contracts_and_end_lines() {
    let project = TestProject::new("fmt-comment-targets");
    project.write(
        "main.veln",
        concat!(
            "mod   app\n",
            "# import docs\n",
            "use   platform.io\n",
            "# function docs\n",
            "fn   main ( ready : Bool ) -> Unit\n",
            "# require docs\n",
            "require ready\n",
            "# body docs\n",
            "()\n",
            "# end docs\n",
            "end\n",
        ),
    );

    let expected = concat!(
        "mod app\n",
        "# import docs\n",
        "use platform.io\n",
        "\n",
        "# function docs\n",
        "fn main(ready: Bool) -> ()\n",
        "\t# require docs\n",
        "\trequire ready\n",
        "\t# body docs\n",
        "\t()\n",
        "\t# end docs\n",
        "end\n",
    );
    project.assert_fmt_idempotent(&["main.veln"], &[("main.veln", expected)]);
}

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
        "pub fn main() -> Result((), AppError)\n  _\nend\n",
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
            "\"message\":\"hole requires a `Result((), AppError)` value\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":2,\"column\":3,\"offset\":40},\"end\":{\"line\":2,\"column\":4,\"offset\":41}},",
            "\"details\":{\"phase\":\"hole\",\"node_id\":\"hole-3\",\"label\":null,",
            "\"expected_type\":\"Result((), AppError)\",\"expected_type_source\":\"declared\",",
            "\"constraints\":[],\"local_bindings\":[],",
            "\"candidate_queries\":[{\"kind\":\"symbol\",",
            "\"candidate_status\":\"query_only\",",
            "\"application_policy\":\"manual_review_required\",",
            "\"query\":\"fn() -> Result((), AppError)\"}]},",
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
    project.write(
        "util.veln",
        "mod app.util\npub fn value() -> Int\n  1\nend\n",
    );
    project.write(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.util\n",
            "pub fn main() -> Int\n",
            "  util::value()\n",
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
            "fn main(value: Result(Int, String)) -> String\n",
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
            "\"scrutinee_type\":\"Result(Int, String)\"",
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
    project.write("src/target/generated.veln", "fn broken() -> ()\n  @\nend\n");
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
    assert!(!stdout.contains("parse.invalid_token"), "{stdout}");
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
            "main.veln:1:1: error[effect.unknown]: declared effect `telepathy` is not known\n",
            "  note: Use a known effect label or remove the declaration.\n",
        ),
    );
}

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

#[test]
fn check_json_reports_recovery_with_required_details() {
    let project = TestProject::new("recovery");
    project.write("main.veln", "garbage\nfn ok() -> ()\n  ()\nend\n");

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"parse.expected_item\"",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":1,\"column\":1,\"offset\":0},\"end\":{\"line\":1,\"column\":8,\"offset\":7}}",
            "\"details\":{\"phase\":\"parse\",\"node_id\":null,\"parser_context\":\"module\"",
            "\"unexpected\":{\"kind\":\"identifier\",\"text\":\"garbage\"}",
            "\"expected\":[\"pub\",\"fn\",\"test\",\"type\"]",
            "\"recovery\":{\"strategy\":\"synchronize_to_anchor\",\"anchor\":\"fn\",\"dropped_token_count\":2}",
        ],
    );
}

#[test]
fn check_json_reports_missing_end_at_eof_span() {
    let project = TestProject::new("missing-end");
    project.write("main.veln", "fn broken() -> ()\n  _\n");

    let output = project.check_json(&["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        concat!(
            "{\"schema_version\":1,",
            "\"tool\":{\"name\":\"veln\",\"version\":\"0.1.0\"},",
            "\"status\":\"error\",",
            "\"diagnostics\":[{",
            "\"id\":\"parse.expected_end\",",
            "\"severity\":\"error\",",
            "\"kind\":\"parse\",",
            "\"message\":\"expected `end` to close function declaration\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":3,\"column\":1,\"offset\":22},\"end\":{\"line\":3,\"column\":1,\"offset\":22}},",
            "\"details\":{\"phase\":\"parse\",\"node_id\":null,\"parser_context\":\"function_body\",",
            "\"unexpected\":{\"kind\":\"end of file\",\"text\":\"\"},",
            "\"expected\":[\"end\"],",
            "\"recovery\":{\"strategy\":\"close_block\",\"anchor\":\"end\",\"dropped_token_count\":0}},",
            "\"related\":[]}],",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"parse\":1}}}\n"
        )
    );
}

#[test]
fn check_json_reports_malformed_declaration() {
    let project = TestProject::new("malformed-declaration");
    project.write("main.veln", "pub main() -> ()\n  ()\nend\n");

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"parse.expected_token\"",
            "\"message\":\"expected fn\"",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":1,\"column\":5,\"offset\":4},\"end\":{\"line\":1,\"column\":9,\"offset\":8}}",
            "\"parser_context\":\"function_declaration\"",
            "\"unexpected\":{\"kind\":\"identifier\",\"text\":\"main\"}",
            "\"expected\":[\"fn\"]",
            "\"recovery\":{\"strategy\":\"insert_token\",\"anchor\":null,\"dropped_token_count\":0}",
        ],
    );
}

#[test]
fn check_json_reports_contract_predicate_parse_errors_as_contract_kind() {
    let project = TestProject::new("contract-predicate-parse");
    project.write(
        "main.veln",
        "fn bad(value: Int) -> Int\nrequire _missing\n  value\nend\n",
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"parse.contract_predicate\"",
            "\"kind\":\"contract\"",
            "\"message\":\"hole syntax is not allowed in a contract predicate\"",
            "\"parser_context\":\"contract_predicate\"",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"contract\":1}}",
        ],
    );
}

#[test]
fn check_json_reports_invalid_tokens() {
    let project = TestProject::new("invalid-token");
    project.write("main.veln", "fn bad() -> ()\n  @\nend\n");

    let output = project.check_json(&["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        concat!(
            "{\"schema_version\":1,",
            "\"tool\":{\"name\":\"veln\",\"version\":\"0.1.0\"},",
            "\"status\":\"error\",",
            "\"diagnostics\":[{",
            "\"id\":\"parse.invalid_token\",",
            "\"severity\":\"error\",",
            "\"kind\":\"parse\",",
            "\"message\":\"invalid token in expression\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":2,\"column\":3,\"offset\":17},\"end\":{\"line\":2,\"column\":4,\"offset\":18}},",
            "\"details\":{\"phase\":\"parse\",\"node_id\":null,\"parser_context\":\"expression_line\",",
            "\"unexpected\":{\"kind\":\"invalid token\",\"text\":\"@\"},",
            "\"expected\":[\"expression\"],",
            "\"recovery\":{\"strategy\":\"skip_token\",\"anchor\":\"newline\",\"dropped_token_count\":1}},",
            "\"related\":[]}],",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"parse\":1}}}\n"
        )
    );
}

#[test]
fn check_json_orders_diagnostics_by_source_discovery_order() {
    let project = TestProject::new("ordering");
    project.write("b.veln", "fn b() -> ()\n  _\n");
    project.write("a.veln", "fn a() -> ()\n  @\nend\n");

    let output = project.check_json(&["b.veln", "a.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let a_index = stdout
        .find("\"file\":\"a.veln\"")
        .expect("a.veln diagnostic should be present");
    let b_index = stdout
        .find("\"file\":\"b.veln\"")
        .expect("b.veln diagnostic should be present");
    assert!(a_index < b_index, "{stdout}");
}

#[test]
fn run_forwards_stdout_and_stderr_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-stdio");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [stdio]\n",
            "  stdio::println(\"out\")\n",
            "  stdio::eprintln(\"err\")\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "out\n");
    assert_eq!(stderr(&output), "err\n");
}

#[test]
fn run_passes_string_entry_arguments_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-entry-args");
    project.write(
        "main.veln",
        concat!(
            "pub fn greet(name: String) -> () effects [stdio]\n",
            "  stdio::println(name)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["greet", "main.veln", "--", "Ada"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "Ada\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_treats_flag_like_values_after_separator_as_entry_arguments() {
    let project = TestProject::new("run-flag-like-entry-arg");
    project.write(
        "main.veln",
        "pub fn main(value: String) -> String\n  value\nend\n",
    );

    let output = project.run_with_path(&["main", "main.veln", "--", "--wat"], "");

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &["veln: `java` was not found; install a JDK to use `veln run`"],
    );
    assert!(
        !stderr(&output).contains("unknown run flag"),
        "post-separator entry argument should not be parsed as a flag: {}",
        stderr(&output)
    );
}

#[test]
fn run_converts_primitive_entry_arguments_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-primitive-entry-args");
    project.write(
        "main.veln",
        concat!(
            "pub fn main(count: Int, ratio: Float, enabled: Bool) -> {count: Int, ratio: Float, enabled: Bool}\n",
            "  {count: count + 1, ratio: ratio + 0.5, enabled: not enabled}\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln", "--", "41", "1.5", "false"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_function_typed_value_calls_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-function-typed-value-call");
    project.write(
        "main.veln",
        concat!(
            "fn increment(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "pub fn main() -> output: Int\n",
            "  ensure output == 2\n",
            "  let callback: fn(Int) -> Int effects [] = increment\n",
            "  callback(1)\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_bounded_channel_send_and_receive_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-bounded-channel");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [concurrency, stdio]\n",
            "  let pair: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
            "  let _ = channel::send(pair.tx, \"hello\")\n",
            "  let output: String = match channel::recv(pair.rx)\n",
            "    Some(value) => value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "  stdio::println(output)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "hello\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_explicit_type_argument_bounded_channel_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-bounded-channel-type-arg");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [concurrency, stdio]\n",
            "  let pair = channel::bounded[String](1)\n",
            "  let _ = channel::send(pair.tx, \"hello\")\n",
            "  let output: String = match channel::recv(pair.rx)\n",
            "    Some(value) => value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "  stdio::println(output)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "hello\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_channel_select_timeout_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-channel-select-timeout");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [concurrency, stdio]\n",
            "  let left: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
            "  let right: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
            "  let _ = channel::send(right.tx, \"hello\")\n",
            "  let output: String = match channel::select_timeout(left.rx, right.rx, 10)\n",
            "    Some(selected) => selected.value\n",
            "    None => \"missing\"\n",
            "  end\n",
            "  stdio::println(output)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "hello\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_channel_select_result_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-channel-select-result");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects [concurrency, stdio]\n",
            "  let left: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
            "  let right: {tx: Sender(String), rx: Receiver(String)} = channel::bounded(1)\n",
            "  let _ = channel::send(right.tx, \"hello\")\n",
            "  let output: String = match channel::select_result(left.rx, right.rx)\n",
            "    Ok(Some(selected)) => selected.value\n",
            "    Ok(None) => \"missing\"\n",
            "    Err(_) => \"interrupted\"\n",
            "  end\n",
            "  stdio::println(output)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "hello\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_executes_task_spawn_and_join_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("run-task-spawn-join");
    project.write(
        "main.veln",
        concat!(
            "fn produce() -> String\n",
            "  \"hello\"\n",
            "end\n",
            "pub fn main() -> () effects [concurrency, stdio]\n",
            "  let task = task::spawn(produce)\n",
            "  let output: String = match task::join(task)\n",
            "    Ok(value) => value\n",
            "    Err(_) => \"failed\"\n",
            "  end\n",
            "  stdio::println(output)\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "hello\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn run_blocks_reachable_holes_before_jdk_execution() {
    let project = TestProject::new("run-hole");
    project.write(
        "main.veln",
        "pub fn main() -> Result((), AppError)\n  _\nend\n",
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &[
            "hint[hole.unfilled]: hole requires a `Result((), AppError)` value",
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
            "pub fn main() -> Vec(String)\n",
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
        "text.veln",
        concat!(
            "mod app.text\n",
            "fn stringify(value: Int) -> String\n",
            "  _\n",
            "end\n",
        ),
    );
    project.write(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.text\n",
            "pub fn main() -> Vec(String)\n",
            "  vec_map([1], text::stringify)\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln", "text.veln"]);

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
        "util.veln",
        concat!("mod app.util\n", "fn value() -> Int\n", "  _\n", "end\n",),
    );
    project.write(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.util\n",
            "pub fn main() -> Int\n",
            "  util::value()\n",
            "end\n",
        ),
    );

    let output = project.run(&["main", "main.veln", "util.veln"]);

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
        "pub fn main(value: Vec(Int)) -> Vec(Int)\n  value\nend\n",
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

#[test]
fn test_json_reports_no_discovered_test_declarations() {
    let project = TestProject::new("test-no-declarations");
    project.write(
        "main_test.veln",
        "fn takes_arg(value: Int) -> Int\n  value\nend\n",
    );

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"blocked\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"main_test.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":0,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":0,\"errors\":1}",
            "\"suite_errors\":[{\"kind\":\"discovery\",\"message\":\"no test declarations were discovered\"}]",
            "\"cases\":[]",
        ],
    );
}

#[test]
fn test_human_reports_no_discovered_test_declarations() {
    let project = TestProject::new("test-human-no-declarations");
    project.write(
        "main_test.veln",
        "fn takes_arg(value: Int) -> Int\n  value\nend\n",
    );

    let output = project.test(&[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        "veln: test discovery: no test declarations were discovered\n"
    );
}

#[test]
fn test_json_blocks_duplicate_function_like_names_with_origin_note() {
    let project = TestProject::new("test-duplicate-function-like-names-json");
    project.write("first_test.veln", "test same() -> ()\n  ()\nend\n");
    project.write("second_test.veln", "fn same() -> ()\n  ()\nend\n");

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"blocked\"",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":1,\"errors\":0}",
            "\"id\":\"name.duplicate\"",
            "\"message\":\"duplicate function declaration name `same`\"",
            "\"details\":{\"phase\":\"name\",\"node_id\":\"fn-1\",\"name\":\"same\",\"namespace\":\"function\",\"first_node_id\":\"test-1\"}",
            "\"related\":[{\"kind\":\"duplicate_origin\",\"message\":\"First function declaration with this name is here.\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn test_human_blocks_duplicate_function_like_names_with_origin_note() {
    let project = TestProject::new("test-duplicate-function-like-names-human");
    project.write("first_test.veln", "test same() -> ()\n  ()\nend\n");
    project.write("second_test.veln", "fn same() -> ()\n  ()\nend\n");

    let output = project.test(&[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "blocked same\n");
    assert_contains_all(
        stderr(&output),
        &[
            "second_test.veln:1:1: error[name.duplicate]: duplicate function declaration name `same`",
            "  note: first_test.veln:1:1: First function declaration with this name is here.",
        ],
    );
}

#[test]
fn test_json_blocks_static_gate_before_jdk_execution() {
    let project = TestProject::new("test-static-gate");
    project.write(
        "main_test.veln",
        "test blocked() -> Result((), AppError)\n  _\nend\n",
    );

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"schema_version\":\"veln-test-json/v0\"",
            "\"command\":\"test\"",
            "\"status\":\"blocked\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"main_test.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":1,\"errors\":0}",
            "\"id\":\"hole.unfilled\"",
            "\"cases\":[{\"id\":\"case-1\",\"name\":\"blocked\",\"kind\":\"test\",\"status\":\"blocked\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn test_json_reports_parse_static_gate_without_jdk_execution() {
    let project = TestProject::new("test-parse-static-gate");
    project.write("broken_test.veln", "test broken() -> ()\n  @\nend\n");

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"blocked\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"broken_test.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":0,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":0,\"errors\":0}",
            "\"diagnostics\":[{\"id\":\"parse.invalid_token\"",
            "\"message\":\"invalid token in expression\"",
            "\"span\":{\"file\":\"broken_test.veln\"",
            "\"cases\":[]",
        ],
    );
}

#[test]
fn test_json_blocks_cases_from_multiple_files_on_semantic_static_gate() {
    let project = TestProject::new("test-multiple-files-static-gate");
    project.write("first_test.veln", "test first() -> ()\n  ()\nend\n");
    project.write("second_test.veln", "test second() -> Int\n  \"no\"\nend\n");

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"blocked\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"first_test.veln\",\"second_test.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":2,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":2,\"errors\":0}",
            "\"id\":\"type.mismatch\"",
            "\"message\":\"expected `Int`, but found `String`\"",
            "\"span\":{\"file\":\"second_test.veln\"",
            "\"cases\":[{\"id\":\"case-1\",\"name\":\"first\",\"kind\":\"test\",\"status\":\"blocked\"",
            "\"source\":{\"file\":\"first_test.veln\"",
            "{\"id\":\"case-2\",\"name\":\"second\",\"kind\":\"test\",\"status\":\"blocked\"",
            "\"source\":{\"file\":\"second_test.veln\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn test_json_auto_discovers_same_file_test_declarations() {
    let project = TestProject::new("test-same-file-discovery");
    project.write(
        "main.veln",
        concat!(
            "fn helper() -> ()\n",
            "  ()\n",
            "end\n",
            "test same_file() -> Result((), AppError)\n",
            "  _\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"blocked\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"main.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":1,\"errors\":0}",
            "\"name\":\"same_file\"",
            "\"source\":{\"file\":\"main.veln\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn check_json_typechecks_executable_doctest_fences() {
    let project = TestProject::new("check-json-doctest");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## let value: Int = \"no\"\n",
            "## ```\n",
            "pub fn main() -> ()\n",
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
            "\"status\":\"error\"",
            "\"id\":\"type.mismatch\"",
            "\"message\":\"expected `Int`, but found `String`\"",
            "\"span\":{\"file\":\"main.veln#doctest-1_test.veln\"",
        ],
    );
}

#[test]
fn check_json_uses_doctest_error_type_fence_attribute() {
    let project = TestProject::new("check-json-doctest-error-type");
    project.write(
        "main.veln",
        concat!(
            "## ```veln error=AppError\n",
            "## let value: Int = Ok(1)?\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
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
            "\"summary\":{\"diagnostic_count\":0",
        ],
    );
}

#[test]
fn check_json_infers_doctest_error_type_from_public_result() {
    let project = TestProject::new("check-json-doctest-public-result-error-type");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## let value: Int = Ok(1)?\n",
            "## ```\n",
            "pub fn parse(raw: String) -> Result(Int, AppError)\n",
            "  Ok(1)\n",
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
            "\"summary\":{\"diagnostic_count\":0",
        ],
    );
}

#[test]
fn check_reports_duplicate_doctest_output_stream() {
    let project = TestProject::new("check-duplicate-doctest-output");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## ready\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## duplicate\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let json_output = project.check_json(&["main.veln"]);
    let json_stdout = stdout(&json_output);

    assert_eq!(
        json_output.status.code(),
        Some(1),
        "{}",
        stderr(&json_output)
    );
    assert_contains_all(
        json_stdout,
        &[
            "\"status\":\"error\"",
            "\"id\":\"doctest.duplicate_output\"",
            "\"kind\":\"doc\"",
            "\"message\":\"duplicate expected stdout output fence\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"stream\":\"stdout\"}",
            "\"related\":[{\"kind\":\"duplicate_origin\",\"message\":\"First expected stdout output fence is here.\"",
        ],
    );

    let human_output = project.veln(&["check"], &["main.veln"]);
    let human_stdout = stdout(&human_output);

    assert_eq!(
        human_output.status.code(),
        Some(1),
        "{}",
        stderr(&human_output)
    );
    assert_eq!(stderr(&human_output), "");
    assert_contains_all(
        human_stdout,
        &[
            "error[doctest.duplicate_output]: duplicate expected stdout output fence",
            "note: main.veln:4:1: First expected stdout output fence is here.",
        ],
    );
}

#[test]
fn check_reports_unknown_doctest_metadata() {
    let project = TestProject::new("check-unknown-doctest-metadata");
    project.write(
        "main.veln",
        concat!(
            "## ```veln skip=true\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout trim=true\n",
            "## ready\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let json_output = project.check_json(&["main.veln"]);
    let json_stdout = stdout(&json_output);

    assert_eq!(
        json_output.status.code(),
        Some(1),
        "{}",
        stderr(&json_output)
    );
    assert_contains_all(
        json_stdout,
        &[
            "\"status\":\"error\"",
            "\"id\":\"doctest.unknown_metadata\"",
            "\"message\":\"unknown doctest attribute `skip`\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"attribute\":\"skip\",\"fence\":\"veln\"}",
            "\"message\":\"unknown doctest output attribute `trim`\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"attribute\":\"trim\",\"fence\":\"veln-output\"}",
        ],
    );

    let human_output = project.veln(&["check"], &["main.veln"]);
    let human_stdout = stdout(&human_output);

    assert_eq!(
        human_output.status.code(),
        Some(1),
        "{}",
        stderr(&human_output)
    );
    assert_eq!(stderr(&human_output), "");
    assert_contains_all(
        human_stdout,
        &[
            "error[doctest.unknown_metadata]: unknown doctest attribute `skip`",
            "error[doctest.unknown_metadata]: unknown doctest output attribute `trim`",
        ],
    );
}

#[test]
fn check_reports_invalid_doctest_metadata() {
    let project = TestProject::new("check-invalid-doctest-metadata");
    project.write(
        "main.veln",
        concat!(
            "## ```veln error=\n",
            "## let value = Ok(1)?\n",
            "## ```\n",
            "## ```veln-output\n",
            "## ready\n",
            "## ```\n",
            "## ```veln-output stream=combined\n",
            "## mixed\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let json_output = project.check_json(&["main.veln"]);
    let json_stdout = stdout(&json_output);

    assert_eq!(
        json_output.status.code(),
        Some(1),
        "{}",
        stderr(&json_output)
    );
    assert_contains_all(
        json_stdout,
        &[
            "\"status\":\"error\"",
            "\"id\":\"doctest.invalid_metadata\"",
            "\"message\":\"empty doctest error type\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"attribute\":\"error\"}",
            "\"message\":\"missing doctest output stream\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"attribute\":\"stream\"}",
            "\"message\":\"unknown doctest output stream `combined`\"",
            "\"details\":{\"kind\":\"doctest_metadata\",\"attribute\":\"stream\",\"stream\":\"combined\"}",
        ],
    );

    let human_output = project.veln(&["check"], &["main.veln"]);
    let human_stdout = stdout(&human_output);

    assert_eq!(
        human_output.status.code(),
        Some(1),
        "{}",
        stderr(&human_output)
    );
    assert_eq!(stderr(&human_output), "");
    assert_contains_all(
        human_stdout,
        &[
            "error[doctest.invalid_metadata]: empty doctest error type",
            "error[doctest.invalid_metadata]: missing doctest output stream",
            "error[doctest.invalid_metadata]: unknown doctest output stream `combined`",
        ],
    );
}

#[test]
fn check_ignores_non_runnable_doctest_fences() {
    let project = TestProject::new("check-ignore-doctest");
    project.write(
        "main.veln",
        concat!(
            "## ```veln ignore\n",
            "## missing_function()\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(stdout, &["\"status\":\"ok\"", "\"diagnostics\":[]"]);
}

#[test]
fn check_accepts_negative_doctest_with_static_diagnostic() {
    let project = TestProject::new("check-negative-doctest");
    project.write(
        "main.veln",
        concat!(
            "## ```veln fail\n",
            "## let value: Int = \"no\"\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(stdout, &["\"status\":\"ok\"", "\"diagnostics\":[]"]);
}

#[test]
fn check_reports_negative_doctest_that_does_not_fail() {
    let project = TestProject::new("check-negative-doctest-missing-failure");
    project.write(
        "main.veln",
        concat!(
            "## ```veln fail\n",
            "## let value: Int = 1\n",
            "## ```\n",
            "pub fn main() -> ()\n",
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
            "\"status\":\"error\"",
            "\"id\":\"doctest.expected_failure_missing\"",
            "\"message\":\"negative doctest produced no error diagnostics\"",
            "\"details\":{\"kind\":\"doctest_metadata\"}",
        ],
    );
}

#[test]
fn check_reports_negative_doctest_with_only_hole_hint() {
    let project = TestProject::new("check-negative-doctest-hole-hint");
    project.write(
        "main.veln",
        concat!(
            "## ```veln fail\n",
            "## let value: Int = _\n",
            "## ```\n",
            "pub fn main() -> ()\n",
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
            "\"status\":\"error\"",
            "\"id\":\"hole.unfilled\"",
            "\"severity\":\"hint\"",
            "\"span\":{\"file\":\"main.veln#doctest-1_test.veln\"",
            "\"id\":\"doctest.expected_failure_missing\"",
            "\"message\":\"negative doctest produced no error diagnostics\"",
            "\"summary\":{\"diagnostic_count\":2",
        ],
    );
}

#[test]
fn check_json_typechecks_hidden_doctest_setup_lines() {
    let project = TestProject::new("check-json-hidden-doctest-setup");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## > let greeting = \"ready\"\n",
            "## stdio::println(greeting)\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(stdout, &["\"status\":\"ok\"", "\"diagnostics\":[]"]);
}

#[test]
fn check_json_typechecks_hash_doctest_setup_with_visible_comment() {
    let project = TestProject::new("check-json-hash-doctest-setup");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## > let greeting = \"ready\"\n",
            "## # visible example comment\n",
            "## stdio::println(greeting)\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(stdout, &["\"status\":\"ok\"", "\"diagnostics\":[]"]);
}

#[test]
fn test_json_maps_explicit_source_file_to_paired_test_file() {
    let project = TestProject::new("test-source-to-test-convention");
    project.write("app.veln", "fn helper() -> ()\n  ()\nend\n");
    project.write(
        "app_test.veln",
        concat!(
            "test paired() -> Result((), AppError)\n",
            "  helper()\n",
            "  _\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json", "app.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"selection\":{\"mode\":\"explicit\",\"targets\":[\"app.veln\",\"app_test.veln\"],\"confidence\":\"unknown\",\"reason\":\"widened_dependency_graph\",\"notes\":[\"added 1 test file by source-to-test convention\",\"dependency graph is missing module identity for selected source `app.veln`\",\"selected all discovered tests because dependency graph evidence is incomplete\"]}",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":1,\"errors\":0}",
            "\"name\":\"paired\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn test_json_runs_doctest_and_compares_expected_output_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-json-doctest-output");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"ready\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## ready\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json", "main.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"passed\"",
            "\"summary\":{\"total\":1,\"passed\":1,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":0,\"errors\":0}",
            "\"name\":\"doctest_1\",\"kind\":\"doctest\",\"status\":\"passed\"",
            "\"events\":[{\"kind\":\"stdio\",\"stream\":\"stdout\",\"operation\":\"println\",\"text\":\"ready\",\"terminator\":\"newline\"",
        ],
    );
}

#[test]
fn test_json_reports_doctest_expected_output_mismatch_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-json-doctest-output-mismatch");
    project.write(
        "main.veln",
        concat!(
            "## ```veln\n",
            "## stdio::println(\"waiting\")\n",
            "## ```\n",
            "## ```veln-output stream=stdout\n",
            "## ready\n",
            "## ```\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json", "main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"failed\"",
            "\"name\":\"doctest_1\",\"kind\":\"doctest\",\"status\":\"failed\"",
            "\"reason\":\"expected_output\"",
            "\"failure\":{\"kind\":\"output\",\"message\":\"expected stdout output did not match\"",
            "\"details\":{\"kind\":\"output\",\"stream\":\"stdout\",\"expected\":\"ready\",\"actual\":\"waiting\\n\",\"first_difference\":{\"line\":1,\"expected\":\"ready\",\"actual\":\"waiting\"}",
            "\"actual_events\":[{\"kind\":\"stdio\",\"stream\":\"stdout\",\"operation\":\"println\",\"text\":\"waiting\"",
            "\"expected_span\":{\"file\":\"main.veln\"",
        ],
    );
}

#[test]
fn test_human_reports_source_to_test_selection_note() {
    let project = TestProject::new("test-human-source-to-test-convention");
    project.write("app.veln", "fn helper() -> ()\n  ()\nend\n");
    project.write(
        "app_test.veln",
        concat!(
            "test paired() -> Result((), AppError)\n",
            "  helper()\n",
            "  _\n",
            "end\n",
        ),
    );

    let output = project.test(&["app.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "blocked paired\n");
    assert_contains_all(
        stderr(&output),
        &[
            "veln: test selection: added 1 test file by source-to-test convention",
            "app_test.veln:3:3: hint[hole.unfilled]: hole requires a `Result((), AppError)` value",
        ],
    );
}

#[test]
fn test_json_treats_explicit_directory_target_as_user_selected() {
    let project = TestProject::new("test-explicit-directory-target");
    project.write(
        "tests/app_test.veln",
        "test directory_case() -> Result((), AppError)\n  _\nend\n",
    );
    project.write("tests/helper.veln", "fn helper() -> ()\n  ()\nend\n");

    let output = project.test(&["--json", "tests"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"selection\":{\"mode\":\"explicit\",\"targets\":[\"tests/app_test.veln\",\"tests/helper.veln\"],\"confidence\":\"complete\",\"reason\":\"user_selected\"}",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":1,\"errors\":0}",
            "\"name\":\"directory_case\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn test_human_prints_blocked_cases_and_static_gate_diagnostics() {
    let project = TestProject::new("test-human-static-gate");
    project.write(
        "main_test.veln",
        "test blocked() -> Result((), AppError)\n  _\nend\n",
    );

    let output = project.test(&[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "blocked blocked\n");
    assert_contains_all(
        stderr(&output),
        &["main_test.veln:2:3: hint[hole.unfilled]: hole requires a `Result((), AppError)` value"],
    );
}

#[test]
fn test_human_reports_passed_and_failed_cases_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-human-cases");
    project.write(
        "main_test.veln",
        concat!(
            "test passes() -> ()\n",
            "  ()\n",
            "end\n",
            "test fails() -> Result((), String)\n",
            "  Err(\"bad\")\n",
            "end\n",
        ),
    );

    let output = project.test(&[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "ok passes\nnot ok fails\n");
    assert_contains_all(
        stderr(&output),
        &["veln: test `fails` failed: runtime result failure: Err(bad)"],
    );
}

#[test]
fn test_json_discovers_runs_and_captures_stdio_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-json-cases");
    project.write("app.veln", "fn helper() -> ()\n  ()\nend\n");
    project.write(
        "main_test.veln",
        concat!(
            "test passes() -> () effects [stdio]\n",
            "  stdio::println(\"out\")\n",
            "  stdio::eprintln(\"err\")\n",
            "  ()\n",
            "end\n",
            "test fails() -> Result((), String)\n",
            "  Err(\"bad\")\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"failed\"",
            "\"selection\":{\"mode\":\"discovered\",\"targets\":[\"main_test.veln\"],\"confidence\":\"complete\",\"reason\":\"pattern_discovery\"}",
            "\"summary\":{\"total\":2,\"passed\":1,\"failed\":1,\"skipped\":0,\"todo\":0,\"blocked\":0,\"errors\":0}",
            "\"name\":\"passes\",\"kind\":\"test\",\"status\":\"passed\"",
            "\"events\":[{\"kind\":\"stdio\",\"stream\":\"stdout\",\"operation\":\"println\",\"text\":\"out\",\"terminator\":\"newline\"",
            "{\"kind\":\"stdio\",\"stream\":\"stderr\",\"operation\":\"eprintln\",\"text\":\"err\",\"terminator\":\"newline\"",
            "\"name\":\"fails\",\"kind\":\"test\",\"status\":\"failed\"",
            "\"failure\":{\"kind\":\"result\",\"message\":\"runtime result failure: Err(bad)\"",
            "\"details\":{\"kind\":\"result\",\"phase\":\"runtime\",\"value\":\"bad\"}",
        ],
    );
}

#[test]
fn test_json_embeds_runtime_contract_failures_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-json-contract-failure");
    project.write(
        "main_test.veln",
        concat!(
            "test rejects() -> ()\n",
            "require false\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.test(&["--json"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"name\":\"rejects\",\"kind\":\"test\",\"status\":\"failed\"",
            "\"failure\":{\"kind\":\"contract\",\"message\":\"contract failure: require `false` in `rejects` blame caller\"",
            "\"details\":{\"kind\":\"contract\",\"phase\":\"runtime\",\"clause\":\"require\",\"predicate\":\"false\"",
            "\"function\":\"rejects\",\"blame\":\"caller\",\"node_id\":\"contract-",
            "\"span\":{\"file\":\"main_test.veln\"",
        ],
    );
}

#[test]
fn test_explicit_target_runs_same_file_test_declaration_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-explicit-same-file");
    project.write("example.veln", "test example() -> ()\n  ()\nend\n");

    let output = project.test(&["--json", "example.veln"]);
    let stdout = stdout(&output);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"passed\"",
            "\"selection\":{\"mode\":\"explicit\",\"targets\":[\"example.veln\"],\"confidence\":\"complete\",\"reason\":\"user_selected\"}",
            "\"summary\":{\"total\":1,\"passed\":1,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":0,\"errors\":0}",
            "\"name\":\"example\"",
        ],
    );
}

#[test]
fn comparison_line_item_order_summary_example_runs_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("comparison-line-item-order-summary");
    let complete = repo_file("examples/comparison/line_item_order_summary.veln");
    let hole = repo_file("examples/comparison/line_item_order_summary_hole.veln");

    let check_output = project.veln(&["check"], &[complete.as_str()]);
    assert!(check_output.status.success(), "{}", stderr(&check_output));
    assert_eq!(stdout(&check_output), "ok\n");
    assert_eq!(stderr(&check_output), "");

    let test_output = project.test(&[complete.as_str()]);
    assert!(test_output.status.success(), "{}", stderr(&test_output));
    assert_contains_all(
        stdout(&test_output),
        &[
            "ok summarizes_success",
            "ok rejects_malformed_input",
            "ok rejects_bad_quantity",
            "ok rejects_unknown_sku",
        ],
    );
    assert_eq!(stderr(&test_output), "");

    let run_output = project.run(&["main", complete.as_str()]);
    assert!(run_output.status.success(), "{}", stderr(&run_output));
    assert_eq!(stdout(&run_output), "900\n");
    assert_eq!(stderr(&run_output), "");

    let hole_output = project.check_json(&[hole.as_str()]);
    assert!(hole_output.status.success(), "{}", stderr(&hole_output));
    assert_contains_all(
        stdout(&hole_output),
        &[
            "\"status\":\"partial\"",
            "\"id\":\"hole.unfilled\"",
            "\"expected_type\":\"Int\"",
            "\"text\":\"candidate > 0\"",
        ],
    );
    assert_eq!(stderr(&hole_output), "");
}

fn stdout(output: &Output) -> &str {
    std::str::from_utf8(&output.stdout).expect("stdout should be UTF-8")
}

fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("stderr should be UTF-8")
}

fn assert_contains_all(haystack: &str, needles: &[&str]) {
    for needle in needles {
        assert!(
            haystack.contains(needle),
            "missing substring `{needle}` in {haystack}"
        );
    }
}

fn jdk_is_available() -> bool {
    Command::new("java").arg("-version").output().is_ok()
        && Command::new("java")
            .arg("--list-modules")
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("jdk.compiler"))
}
