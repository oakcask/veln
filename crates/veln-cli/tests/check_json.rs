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

    fn run(&self, args: &[&str]) -> Output {
        self.veln(&["run"], args)
    }

    fn test(&self, args: &[&str]) -> Output {
        self.veln(&["test"], args)
    }

    fn test_with_path(&self, args: &[&str], path: &str) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_veln"));
        command.current_dir(&self.root);
        command.env("PATH", path);
        command.arg("test");
        for arg in args {
            command.arg(arg);
        }
        command.output().expect("veln should run")
    }

    fn run_with_path(&self, args: &[&str], path: &str) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_veln"));
        command.current_dir(&self.root);
        command.env("PATH", path);
        command.arg("run");
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

#[test]
fn cli_prints_help_for_empty_invocation_and_subcommand_help() {
    let project = TestProject::new("cli-help");
    let expected = concat!(
        "veln check [--json] [path ...]\n",
        "veln fmt [path ...]\n",
        "veln run <entry> [path ...]\n",
        "veln test [--json] [target ...]\n",
    );

    let empty_output = project.veln(&[], &[]);
    let check_help_output = project.veln(&["check"], &["--help"]);

    assert!(empty_output.status.success(), "{}", stderr(&empty_output));
    assert_eq!(stdout(&empty_output), expected);
    assert_eq!(stderr(&empty_output), "");
    assert!(
        check_help_output.status.success(),
        "{}",
        stderr(&check_help_output)
    );
    assert_eq!(stdout(&check_help_output), expected);
    assert_eq!(stderr(&check_help_output), "");
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
    let unknown_check_flag = project.veln(&["check"], &["--wat"]);
    let unknown_run_flag = project.veln(&["run"], &["--wat"]);
    let unknown_test_flag = project.veln(&["test"], &["--wat"]);
    let missing_run_entry = project.veln(&["run"], &[]);

    assert_eq!(unknown_command.status.code(), Some(2));
    assert_eq!(stdout(&unknown_command), "");
    assert_eq!(stderr(&unknown_command), "veln: unknown command `wat`\n");

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
    project.write("main.veln", "pub fn main() -> () effects []\n  ()\nend\n");

    let output = project.veln(&["check"], &["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "ok\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn check_human_reports_diagnostics_to_stdout() {
    let project = TestProject::new("check-human-diagnostics");
    project.write(
        "main.veln",
        "pub fn main() -> Int effects []\n  \"no\"\nend\n",
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &["main.veln:2:3: error[type.mismatch]: expected `Int`, but found `String`"],
    );
}

#[test]
fn check_human_reports_missing_module_identity_for_imports() {
    let project = TestProject::new("check-human-module-identity");
    project.write(
        "main.veln",
        concat!(
            "use platform.io\n",
            "fn main() -> () effects []\n",
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
        concat!(
            "use platform.io\n",
            "fn main() -> () effects []\n",
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
            "\"id\":\"module.missing_identity\"",
            "\"kind\":\"module\"",
            "\"message\":\"module import requires a module identity\"",
            "\"details\":{\"phase\":\"module\",\"node_id\":\"use-1\",\"field\":\"module_identity\",\"expected_owner\":\"source\",\"observed_owner\":\"missing\"}",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"module\":1}}",
        ],
    );
}

#[test]
fn fmt_formats_first_slice_golden_and_is_idempotent() {
    let project = TestProject::new("fmt-golden");
    project.write(
        "main.veln",
        concat!(
            "mod app\n",
            "use stdio\n",
            "pub   fn   main ( name : String ) -> Result ( () , AppError ) effects [ stdio ]\n",
            " require name != \"\"\n",
            " let payload : { message : String, values : List(Int) } = { message : name , values : [ 1 , 2 , add ( 3 , 4 ) ] }\n",
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
            "  require name != \"\"\n",
            "  let payload: { message : String, values : List(Int) } = { message: name, values: [1, 2, add(3, 4)] }\n",
            "  stdio::println(payload)\n",
            "  _result satisfy candidate => candidate != \"\"\n",
            "end\n",
            "\n",
            "fn helper(value)\n",
            "  value\n",
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
            "  require name != \"\"\n",
            "  let payload: { message : String, values : List(Int) } = { message: name, values: [1, 2, add(3, 4)] }\n",
            "  stdio::println(payload)\n",
            "  _result satisfy candidate => candidate != \"\"\n",
            "end\n",
            "\n",
            "fn helper(value)\n",
            "  value\n",
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
fn fmt_preserves_comment_bearing_files_byte_for_byte() {
    let project = TestProject::new("fmt-comments");
    let text = concat!(
        "// keep leading comment\n",
        "fn   main ( ) -> ()\n",
        "  () // keep trailing comment\n",
        "end\n",
    );
    project.write("main.veln", text);

    let output = project.fmt(&["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(project.read("main.veln"), text);
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
            "\"related\":[]},{",
            "\"id\":\"effect.missing_public\",",
            "\"severity\":\"error\",",
            "\"kind\":\"effect\",",
            "\"message\":\"public function has no effects annotation\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":1,\"column\":1,\"offset\":0},\"end\":{\"line\":4,\"column\":1,\"offset\":31}},",
            "\"details\":{\"phase\":\"effect\",\"node_id\":\"fn-1\",\"effect\":\"unknown\",",
            "\"boundary\":\"public_function\",\"declared_effects\":[],\"inferred_effects\":[],",
            "\"provenance\":[],\"provenance_truncated\":false},",
            "\"related\":[{\"kind\":\"repair_hint\",\"message\":\"Use `effects []` for a pure public function.\"}]}],",
            "\"summary\":{\"diagnostic_count\":3,\"by_severity\":{\"error\":3},\"by_kind\":{\"effect\":1,\"type\":2}}}\n"
        )
    );
}

#[test]
fn check_json_reports_hole_with_return_expected_type() {
    let project = TestProject::new("hole-return");
    project.write(
        "main.veln",
        "pub fn main() -> Result((), AppError) effects []\n  _\nend\n",
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
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":2,\"column\":3,\"offset\":51},\"end\":{\"line\":2,\"column\":4,\"offset\":52}},",
            "\"details\":{\"phase\":\"hole\",\"node_id\":\"hole-3\",\"label\":null,",
            "\"expected_type\":\"Result((), AppError)\",\"expected_type_source\":\"declared\",",
            "\"constraints\":[],\"local_bindings\":[],",
            "\"candidate_queries\":[{\"kind\":\"symbol\",\"query\":\"fn() -> Result((), AppError)\"}]},",
            "\"related\":[{\"kind\":\"expected_type_origin\",\"message\":\"Return type declared here.\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":1,\"column\":1,\"offset\":0},\"end\":{\"line\":4,\"column\":1,\"offset\":57}}}]}],",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"hint\":1},\"by_kind\":{\"hole\":1}}}\n"
        )
    );
}

#[test]
fn check_json_keeps_sema_for_other_files_when_one_file_has_parse_errors() {
    let project = TestProject::new("parse-and-sema");
    project.write("a_parse.veln", "fn broken() -> ()\n  @\nend\n");
    project.write(
        "b_type.veln",
        "pub fn main() -> Int effects []\n  \"no\"\nend\n",
    );

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
fn check_json_reports_return_type_mismatch() {
    let project = TestProject::new("return-mismatch");
    project.write(
        "main.veln",
        "pub fn main() -> Int effects []\n  \"no\"\nend\n",
    );

    let output = project.check_json(&["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout(&output),
        &[
            "\"id\":\"type.mismatch\"",
            "\"kind\":\"type\"",
            "\"message\":\"expected `Int`, but found `String`\"",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":2,\"column\":3,\"offset\":34},\"end\":{\"line\":2,\"column\":7,\"offset\":38}}",
            "\"details\":{\"phase\":\"type\",\"node_id\":\"expr-3\",\"expected_type\":\"Int\",\"actual_type\":\"String\",\"expected_type_source\":\"declared_return\",\"actual_type_source\":\"inferred_expression\",\"constraint\":\"return_value\"",
        ],
    );
}

#[test]
fn check_human_reports_missing_record_field_with_base_note() {
    let project = TestProject::new("field-missing-human");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> Int effects []\n",
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
            "pub fn main() -> () effects []\n",
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
            "pub fn main() -> () effects []\n",
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
fn check_human_reports_missing_public_effect_cause() {
    let project = TestProject::new("effect-human-provenance");
    project.write(
        "main.veln",
        concat!(
            "pub fn main() -> () effects []\n",
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
fn check_human_reports_public_effect_annotation_hint_as_note() {
    let project = TestProject::new("effect-human-boundary-hint");
    project.write("main.veln", "pub fn main() -> ()\n  ()\nend\n");

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_eq!(
        stdout(&output),
        concat!(
            "main.veln:1:1: error[effect.missing_public]: public function has no effects annotation\n",
            "  note: Use `effects []` for a pure public function.\n",
        ),
    );
}

#[test]
fn check_json_reports_contract_validation_diagnostics() {
    let project = TestProject::new("contract-diagnostics");
    project.write(
        "main.veln",
        concat!(
            "pub fn main(ready: Bool) -> () effects []\n",
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
fn check_json_reports_hole_constraints_from_contracts_and_satisfy() {
    let project = TestProject::new("hole-constraints");
    project.write(
        "main.veln",
        concat!(
            "pub fn default_port(max: Int) -> Int effects []\n",
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
            "\"repair_status\":\"blocked_until_discharged\"",
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
            "\"expected\":[\"pub\",\"fn\",\"test\"]",
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
fn run_blocks_reachable_holes_before_jdk_execution() {
    let project = TestProject::new("run-hole");
    project.write(
        "main.veln",
        "pub fn main() -> Result((), AppError) effects []\n  _\nend\n",
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
    project.write(
        "main.veln",
        "pub fn main() -> Int effects []\n  \"no\"\nend\n",
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &["main.veln:2:3: error[type.mismatch]: expected `Int`, but found `String`"],
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
    project.write("main.veln", "pub fn main() -> () effects []\n  ()\nend\n");

    let output = project.run(&["missing", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &["veln: run entry `missing` was not found"],
    );
}

#[test]
fn run_rejects_parameterized_entry_before_jdk_execution() {
    let project = TestProject::new("run-entry-params");
    project.write(
        "main.veln",
        "pub fn main(value: Int) -> Int effects []\n  value\nend\n",
    );

    let output = project.run(&["main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        stderr(&output),
        concat!(
            "veln: run entry `main` has parameters\n",
            "veln: note: this slice only executes zero-argument entries\n",
        )
    );
}

#[test]
fn run_reports_missing_javac_clearly() {
    let project = TestProject::new("run-no-javac");
    project.write("main.veln", "pub fn main() -> () effects []\n  ()\nend\n");

    let output = project.run_with_path(&["main", "main.veln"], "");

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_contains_all(
        stderr(&output),
        &["veln: `javac` was not found; install a JDK to use `veln run`"],
    );
}

#[test]
fn test_json_reports_no_discovered_test_declarations() {
    let project = TestProject::new("test-no-declarations");
    project.write(
        "main_test.veln",
        "fn takes_arg(value: Int) -> Int effects []\n  value\nend\n",
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
        "fn takes_arg(value: Int) -> Int effects []\n  value\nend\n",
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
    project.write(
        "first_test.veln",
        "test same() -> () effects []\n  ()\nend\n",
    );
    project.write(
        "second_test.veln",
        "fn same() -> () effects []\n  ()\nend\n",
    );

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
    project.write(
        "first_test.veln",
        "test same() -> () effects []\n  ()\nend\n",
    );
    project.write(
        "second_test.veln",
        "fn same() -> () effects []\n  ()\nend\n",
    );

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
        "test blocked() -> Result((), AppError) effects []\n  _\nend\n",
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
fn test_json_maps_explicit_source_file_to_paired_test_file() {
    let project = TestProject::new("test-source-to-test-convention");
    project.write("app.veln", "fn helper() -> () effects []\n  ()\nend\n");
    project.write(
        "app_test.veln",
        concat!(
            "test paired() -> Result((), AppError) effects []\n",
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
            "\"selection\":{\"mode\":\"explicit\",\"targets\":[\"app.veln\",\"app_test.veln\"],\"confidence\":\"partial\",\"reason\":\"source_to_test_convention\",\"notes\":[\"added 1 test file by source-to-test convention\"]}",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":1,\"errors\":0}",
            "\"name\":\"paired\"",
            "\"reason\":\"static_gate\"",
        ],
    );
}

#[test]
fn test_human_reports_source_to_test_selection_note() {
    let project = TestProject::new("test-human-source-to-test-convention");
    project.write("app.veln", "fn helper() -> () effects []\n  ()\nend\n");
    project.write(
        "app_test.veln",
        concat!(
            "test paired() -> Result((), AppError) effects []\n",
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
fn test_human_prints_blocked_cases_and_static_gate_diagnostics() {
    let project = TestProject::new("test-human-static-gate");
    project.write(
        "main_test.veln",
        "test blocked() -> Result((), AppError) effects []\n  _\nend\n",
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
fn test_json_reports_missing_javac_as_runner_error() {
    let project = TestProject::new("test-no-javac");
    project.write(
        "main_test.veln",
        "test passes() -> () effects []\n  ()\nend\n",
    );

    let output = project.test_with_path(&["--json"], "");
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout,
        &[
            "\"status\":\"error\"",
            "\"summary\":{\"total\":1,\"passed\":0,\"failed\":0,\"skipped\":0,\"todo\":0,\"blocked\":0,\"errors\":1}",
            "\"name\":\"passes\"",
            "\"failure\":{\"kind\":\"runtime\",\"message\":\"veln: `javac` was not found; install a JDK to use `veln test`\"",
        ],
    );
}

#[test]
fn test_human_reports_missing_javac_as_runner_error() {
    let project = TestProject::new("test-human-no-javac");
    project.write(
        "main_test.veln",
        "test passes() -> () effects []\n  ()\nend\n",
    );

    let output = project.test_with_path(&[], "");

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "error passes\n");
    assert_contains_all(
        stderr(&output),
        &[
            "veln: test `passes` failed: veln: `javac` was not found; install a JDK to use `veln test`",
        ],
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
            "test passes() -> () effects []\n",
            "  ()\n",
            "end\n",
            "test fails() -> Result((), String) effects []\n",
            "  Err(\"bad\")\n",
            "end\n",
        ),
    );

    let output = project.test(&[]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stdout(&output), "ok passes\nnot ok fails\n");
    assert_contains_all(
        stderr(&output),
        &["veln: test `fails` failed: test process exited with status exit status: 1"],
    );
}

#[test]
fn test_json_discovers_runs_and_captures_stdio_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-json-cases");
    project.write("app.veln", "fn helper() -> () effects []\n  ()\nend\n");
    project.write(
        "main_test.veln",
        concat!(
            "test passes() -> () effects [stdio]\n",
            "  stdio::println(\"out\")\n",
            "  stdio::eprintln(\"err\")\n",
            "  ()\n",
            "end\n",
            "test fails() -> Result((), String) effects []\n",
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
            "\"failure\":{\"kind\":\"runtime\",\"message\":\"test process exited with status exit status: 1\"",
        ],
    );
}

#[test]
fn test_explicit_target_runs_same_file_test_declaration_when_jdk_is_available() {
    if !jdk_is_available() {
        return;
    }

    let project = TestProject::new("test-explicit-same-file");
    project.write(
        "example.veln",
        "test example() -> () effects []\n  ()\nend\n",
    );

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
    Command::new("javac").arg("-version").output().is_ok()
        && Command::new("java").arg("-version").output().is_ok()
}
