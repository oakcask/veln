use super::support::*;

#[test]
fn check_json_accepts_valid_input() {
    let project = TestProject::new("valid");
    project.write(
        "main.veln",
        "pub fn main() -> Result<(), AppError> effects [stdio]\n  Ok(())\nend\n",
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
            "fn main(value: Option<Int>) -> String\n",
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
            "  note: main.veln:2:9: Scrutinee has type `Option<Int>`.",
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
            "main.veln:1:1: error[module.invalid_import_path]: module import `platform.io` uses `.`; source module paths use `::`",
            "  note: Rewrite the import with `::` between module path segments.",
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
            "\"id\":\"module.invalid_import_path\"",
            "\"kind\":\"module\"",
            "\"message\":\"module import `platform.io` uses `.`; source module paths use `::`",
            "\"details\":{\"phase\":\"module\",\"field\":\"import_path\",\"module_path\":\"platform.io\",\"expected_delimiter\":\"::\",\"observed_delimiter\":\".\"}",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"module\":1}}",
        ],
    );
}

#[test]
fn check_human_reports_modules_manifest_section() {
    let project = TestProject::new("check-human-modules-manifest-section");
    project.write("veln.toml", "[modules]\n\"main.veln\" = \"app.manifest\"\n");
    project.write(
        "main.veln",
        concat!("pub fn main() -> ()\n", "  ()\n", "end\n",),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &[
            "veln.toml:1:2: error[manifest.unsupported_section]: `[modules]` is not supported; use `[lib].exports` for public source files",
            "  note: Replace `[modules]` entries with `[lib].exports` file paths.",
        ],
    );
}

#[test]
fn check_human_reports_unselected_manifest_export() {
    let project = TestProject::new("check-human-unselected-manifest-export");
    project.write("veln.toml", "[lib]\nexports = [\"other.veln\"]\n");
    project.write("other.veln", "pub fn other() -> ()\n  ()\nend\n");
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
            "veln.toml:2:13: error[manifest.unselected_export]: manifest export `other.veln` has no matching selected source file",
        ],
    );
}

#[test]
fn check_human_accepts_selected_manifest_export() {
    let project = TestProject::new("check-human-selected-manifest-export");
    project.write("veln.toml", "[lib]\nexports = [\"main.veln\"]\n");
    project.write(
        "main.veln",
        concat!("pub fn main() -> ()\n", "  ()\n", "end\n",),
    );

    let output = project.veln(&["check"], &["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "ok\n");
    assert_eq!(stderr(&output), "");
}

#[test]
fn check_json_reports_modules_manifest_section() {
    let project = TestProject::new("check-json-modules-manifest-section");
    project.write("veln.toml", "[modules]\n\"main.veln\" = \"app.manifest\"\n");
    project.write(
        "main.veln",
        concat!("pub fn main() -> ()\n", "  ()\n", "end\n",),
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"manifest.unsupported_section\"",
            "\"kind\":\"module\"",
            "\"message\":\"`[modules]` is not supported; use `[lib].exports` for public source files\"",
            "\"span\":{\"file\":\"veln.toml\",\"start\":{\"line\":1,\"column\":2,\"offset\":1},\"end\":{\"line\":1,\"column\":9,\"offset\":8}}",
            "\"details\":{\"phase\":\"module\",\"field\":\"manifest_section\",\"section\":\"modules\"}",
            "\"related\":[{\"message\":\"Replace `[modules]` entries with `[lib].exports` file paths.\"}]",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"module\":1}}",
        ],
    );
}

#[test]
fn check_json_reports_manifest_export_validation() {
    let project = TestProject::new("check-json-manifest-export-validation");
    project.write(
        "veln.toml",
        concat!(
            "[lib]\n",
            "exports = [\n",
            "  \"other.veln\",\n",
            "  \"main::helper\",\n",
            "  \"main.veln\",\n",
            "  \"./main.veln\",\n",
            "  \"../outside.veln\",\n",
            "]\n",
        ),
    );
    project.write(
        "other.veln",
        concat!("pub fn other() -> ()\n", "  ()\n", "end\n"),
    );
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
            "\"id\":\"manifest.unselected_export\"",
            "\"message\":\"manifest export `other.veln` has no matching selected source file\"",
            "\"id\":\"manifest.invalid_export\"",
            "\"message\":\"manifest export `main::helper` is invalid: module paths are not valid manifest exports; use a package-relative source file path\"",
            "\"message\":\"manifest export `./main.veln` duplicates module export `main`\"",
            "\"id\":\"manifest.duplicate_export\"",
            "\"related\":[{\"kind\":\"duplicate_origin\",\"message\":\"The first export for `main` is here.\"",
            "\"message\":\"manifest export `../outside.veln` is invalid: manifest exports must stay inside the package\"",
            "\"kind\":\"module\"",
            "\"summary\":{\"diagnostic_count\":4,\"by_severity\":{\"error\":4},\"by_kind\":{\"module\":4}}",
        ],
    );
}

#[test]
fn check_human_reports_git_dependency_selector_validation() {
    let project = TestProject::new("check-human-git-dependency-selectors");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/missing\"]\n",
            "git = \"https://example.invalid/missing.git\"\n",
            "[dependencies.\"github.com/oakcask/multiple\"]\n",
            "git = \"https://example.invalid/multiple.git\"\n",
            "tag = \"v1.2.0\"\n",
            "branch = \"main\"\n",
        ),
    );
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
            "veln.toml:1:16: error[manifest.missing_git_selector]: git dependency `github.com/oakcask/missing` must specify exactly one selector: `rev`, `tag`, or `branch`",
            "veln.toml:6:1: error[manifest.multiple_git_selectors]: git dependency `github.com/oakcask/multiple` specifies multiple selectors; use exactly one of `rev`, `tag`, or `branch`",
        ],
    );
}

#[test]
fn check_json_reports_git_dependency_selector_validation() {
    let project = TestProject::new("check-json-git-dependency-selectors");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/missing\"]\n",
            "git = \"https://example.invalid/missing.git\"\n",
            "[dependencies.\"github.com/oakcask/multiple\"]\n",
            "git = \"https://example.invalid/multiple.git\"\n",
            "tag = \"v1.2.0\"\n",
            "branch = \"main\"\n",
        ),
    );
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
            "\"id\":\"manifest.missing_git_selector\"",
            "\"message\":\"git dependency `github.com/oakcask/missing` must specify exactly one selector: `rev`, `tag`, or `branch`\"",
            "\"details\":{\"phase\":\"module\",\"field\":\"dependencies\",\"package\":\"github.com/oakcask/missing\",\"source_kind\":\"git\",\"reason\":\"missing_selector\"}",
            "\"id\":\"manifest.multiple_git_selectors\"",
            "\"message\":\"git dependency `github.com/oakcask/multiple` specifies multiple selectors; use exactly one of `rev`, `tag`, or `branch`\"",
            "\"selectors\":[\"tag\",\"branch\"]",
            "\"summary\":{\"diagnostic_count\":2,\"by_severity\":{\"error\":2},\"by_kind\":{\"module\":2}}",
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
            "main.veln:1:30: error[effect.unknown]: declared effect `telepathy` is not known\n",
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
            "\"expected\":[\"pub\",\"fn\",\"test\",\"type\",\"effect\",\"handler\",\"schema\"]",
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
