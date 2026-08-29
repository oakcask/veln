use super::*;

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
