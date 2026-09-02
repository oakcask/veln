use super::*;

#[test]
pub(super) fn manifest_tools_parse_controlled_java_availability() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "main", "main.veln"]
exit = 0

[tools]
java = "fake-success"
git = "fake-git-rev-parse"
"#,
    );

    assert!(manifest.tools.needs_path());
    assert!(!manifest.tools.requires_jdk());
    assert_eq!(manifest.tools.java, Some(ToolAvailability::FakeSuccess));
    assert_eq!(manifest.tools.git, Some(ToolAvailability::FakeGitRevParse));

    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "main", "main.veln"]
exit = 0

[tools]
java = "real"
"#,
    );

    assert!(manifest.tools.needs_path());
    assert!(manifest.tools.requires_jdk());
    assert_eq!(manifest.tools.java, Some(ToolAvailability::Real));
}

#[test]
pub(super) fn toolchain_inventory_discovers_both_roots_with_stable_root_qualified_order() {
    let root = test_temp_root("inventory-order");
    for case_dir in ["a/zeta", "b/alpha", "a/alpha"] {
        fs::create_dir_all(root.join(case_dir)).expect("case directory should be created");
        fs::write(
            root.join(case_dir).join("case.toml"),
            "command = [\"check\"]\nexit = 0\n",
        )
        .expect("case manifest should be written");
    }

    let preflight = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[
            test_discovery_root("root-a", "a"),
            test_discovery_root("root-b", "b"),
        ],
    )
    .expect("inventory should be discovered");

    assert_eq!(
        preflight
            .cases
            .iter()
            .map(|case| case.id.as_str())
            .collect::<Vec<_>>(),
        ["root-a/alpha", "root-a/zeta", "root-b/alpha"]
    );
    assert_eq!(
        preflight
            .cases
            .iter()
            .map(|case| case.manifest_relative.to_string_lossy().replace('\\', "/"))
            .collect::<Vec<_>>(),
        ["a/alpha", "a/zeta", "b/alpha"]
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[test]
pub(super) fn toolchain_inventory_rejects_overlapping_root_and_manifest_boundaries() {
    let root = test_temp_root("inventory-boundaries");
    fs::create_dir_all(root.join("outer/nested")).expect("directories should be created");
    fs::write(
        root.join("outer/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("outer manifest should be written");
    fs::write(
        root.join("outer/nested/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("nested manifest should be written");

    let overlap = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[
            test_discovery_root("outer", "outer"),
            test_discovery_root("nested", "outer/nested"),
        ],
    )
    .expect_err("overlapping roots should fail");
    assert!(overlap.contains("configured discovery roots overlap"));

    let nested = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("outer", "outer")],
    )
    .expect_err("root and nested manifests should fail");
    assert!(nested.contains("root-level case.toml"));
    assert!(nested.contains("nested case.toml"));

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(unix)]
#[test]
pub(super) fn toolchain_inventory_reports_mixed_entry_failures_in_stable_path_order() {
    use std::os::unix::fs::symlink;

    fn write_inventory(root: &Path, entries: &[&str]) {
        fs::create_dir_all(root.join("cases/nested"))
            .expect("inventory directories should be created");
        for entry in entries {
            match *entry {
                "case.toml" => fs::write(
                    root.join("cases/case.toml"),
                    "command = [\"check\"]\nexit = 0\n",
                )
                .expect("root manifest should be written"),
                "link" => symlink("nested", root.join("cases/link"))
                    .expect("directory link should be created"),
                "nested/case.toml" => fs::write(
                    root.join("cases/nested/case.toml"),
                    "command = [\"check\"]\nexit = 0\n",
                )
                .expect("nested manifest should be written"),
                entry => panic!("unexpected fixture entry {entry}"),
            }
        }
    }

    let first = test_temp_root("inventory-stable-entry-errors-a");
    let second = test_temp_root("inventory-stable-entry-errors-b");
    write_inventory(&first, &["link", "nested/case.toml", "case.toml"]);
    write_inventory(&second, &["case.toml", "nested/case.toml", "link"]);

    let first_error = toolchain_case_inventory::run_preflight_with_roots(
        &first,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("mixed invalid entries should fail discovery");
    let second_error = toolchain_case_inventory::run_preflight_with_roots(
        &second,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("mixed invalid entries should fail discovery");

    assert_eq!(first_error, second_error);
    assert!(first_error.contains("toolchain case preflight found 4 problem(s)"));
    assert!(first_error.contains("cases/link: replace the link or reparse point"));
    assert!(first_error.contains("cases: remove root-level case.toml"));
    assert!(
        first_error.contains("cases/nested/case.toml: nested case.toml is below cases/case.toml")
    );

    fs::remove_dir_all(first).expect("first inventory root should be removed");
    fs::remove_dir_all(second).expect("second inventory root should be removed");
}

#[cfg(unix)]
#[test]
pub(super) fn toolchain_inventory_rejects_links_without_following_them() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("inventory-links");
    fs::create_dir_all(root.join("cases/ordinary")).expect("directories should be created");
    fs::write(
        root.join("cases/ordinary/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("case manifest should be written");
    symlink("ordinary", root.join("cases/link")).expect("symlink should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("link should fail discovery");
    assert!(error.contains("replace the link or reparse point"));
    assert!(error.contains("cases/link"));

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(unix)]
#[test]
pub(super) fn toolchain_inventory_rejects_root_links_without_following_them() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("inventory-root-links");
    fs::create_dir_all(root.join("external/hidden")).expect("target case directory should exist");
    fs::write(
        root.join("external/hidden/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("target case manifest should be written");
    symlink("external", root.join("linked-root")).expect("root symlink should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("linked-root", "linked-root")],
    )
    .expect_err("root symlink should fail discovery before traversal");

    assert!(error.contains("linked-root: replace the link or reparse point"));
    assert!(
        !error.contains("hidden"),
        "discovery should not resolve root symlink targets"
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(unix)]
#[test]
pub(super) fn toolchain_inventory_rejects_broken_root_links_without_resolving_them() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("inventory-broken-root-link");
    symlink("missing-target", root.join("linked-root"))
        .expect("broken root symlink should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("linked-root", "linked-root")],
    )
    .expect_err("broken root symlink should fail as a link-like root");

    assert!(error.contains("linked-root: replace the link or reparse point"));
    assert!(
        !error.contains("missing-target"),
        "discovery should not resolve broken root link targets"
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(unix)]
#[test]
pub(super) fn toolchain_inventory_rejects_broken_file_links_and_link_cycles() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("inventory-link-kinds");
    fs::create_dir_all(root.join("cases/ordinary")).expect("case directory should be created");
    fs::write(
        root.join("cases/ordinary/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("case manifest should be written");
    symlink("missing-target", root.join("cases/broken-link"))
        .expect("broken symlink should be created");
    fs::write(root.join("target-file"), "fixture").expect("target file should be written");
    symlink("../target-file", root.join("cases/file-link"))
        .expect("file symlink should be created");
    symlink("cycle-b", root.join("cases/cycle-a")).expect("first cycle link should be created");
    symlink("cycle-a", root.join("cases/cycle-b")).expect("second cycle link should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("link-like entries should fail discovery before following them");

    assert!(error.contains("cases/broken-link: replace the link or reparse point"));
    assert!(error.contains("cases/file-link: replace the link or reparse point"));
    assert!(error.contains("cases/cycle-a: replace the link or reparse point"));
    assert!(error.contains("cases/cycle-b: replace the link or reparse point"));
    assert!(
        !error.contains("missing-target"),
        "discovery should not resolve broken link targets"
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(windows)]
#[test]
pub(super) fn toolchain_inventory_rejects_windows_reparse_point_roots() {
    use std::os::windows::fs::symlink_dir;

    let root = test_temp_root("inventory-windows-root-reparse");
    fs::create_dir_all(root.join("external/hidden")).expect("target case directory should exist");
    fs::write(
        root.join("external/hidden/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("target case manifest should be written");
    symlink_dir(root.join("external"), root.join("linked-root"))
        .expect("root reparse point should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("linked-root", "linked-root")],
    )
    .expect_err("root reparse point should fail discovery before traversal");

    assert!(error.contains("linked-root: replace the link or reparse point"));
    assert!(
        !error.contains("hidden"),
        "discovery should not resolve root reparse point targets"
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[cfg(windows)]
#[test]
pub(super) fn toolchain_inventory_rejects_windows_reparse_point_files_and_directories() {
    use std::os::windows::fs::{symlink_dir, symlink_file};

    let root = test_temp_root("inventory-windows-reparse");
    fs::create_dir_all(root.join("cases/ordinary")).expect("case directory should be created");
    fs::write(
        root.join("cases/ordinary/case.toml"),
        "command = [\"check\"]\nexit = 0\n",
    )
    .expect("case manifest should be written");
    fs::write(root.join("target-file"), "fixture").expect("target file should be written");
    symlink_file(root.join("target-file"), root.join("cases/file-link"))
        .expect("file reparse point should be created");
    symlink_dir(
        root.join("cases/ordinary"),
        root.join("cases/directory-link"),
    )
    .expect("directory reparse point should be created");

    let error = toolchain_case_inventory::run_preflight_with_roots(
        &root,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("Windows reparse points should fail discovery before traversal");

    assert!(error.contains("cases/directory-link: replace the link or reparse point"));
    assert!(error.contains("cases/file-link: replace the link or reparse point"));
    assert!(
        !error.contains("target-file"),
        "discovery should not resolve reparse point targets"
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[test]
pub(super) fn toolchain_inventory_parity_reports_stale_generated_cases() {
    let error = toolchain_case_inventory::compare_generated_inventory_with_policy(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        &[],
        false,
    )
    .expect_err("empty generated inventory should be stale");

    assert!(error.contains("rebuild the toolchain harness"));
    assert!(error.contains("case manifest was added after test generation"));
}

#[test]
pub(super) fn generated_toolchain_tests_run_inventory_parity_once() {
    let generated = toolchain_case_inventory::generated_toolchain_tests(&[
        PathBuf::from("tests/toolchain_cases/first"),
        PathBuf::from("tests/toolchain_cases/second"),
    ]);

    assert!(generated.contains("fn generated_toolchain_inventory_matches_manifests()"));
    assert_eq!(
        generated
            .matches("runtime_generated_inventory_barrier();")
            .count(),
        1,
        "generated cases should share one dedicated inventory parity test"
    );
    assert_eq!(
        generated.matches("run_case(&toolchain_case_path(").count(),
        2
    );
}

#[test]
pub(super) fn policy_preflight_failure_prevents_generated_test_module_creation() {
    let root = test_temp_root("policy-generation-block");
    fs::create_dir_all(root.join("cases/blocked")).expect("case directory should be created");
    fs::write(
        root.join("cases/blocked/case.toml"),
        "command = [\"check\"]\nstdin = \"\\n\"\nexit = 0\n",
    )
    .expect("blocked manifest should be written");

    let error = toolchain_case_inventory::generated_toolchain_tests_from_preflight(
        &root,
        &[test_discovery_root("cases", "cases")],
        true,
    )
    .expect_err("policy preflight should fail before generating tests");

    assert!(error.contains("toolchain case preflight found 1 problem(s)"));
    assert!(error.contains("field `stdin` contains escape-produced-line-break"));
    assert!(!error.contains("generated_toolchain_cases"));

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[test]
pub(super) fn manifest_policy_reports_encoded_line_breaks_in_toml_and_json_strings() {
    let source = r#"
command = ["check"]
stdin = ["\n", '\n', {"json":"\u000A", "nested":["\\r"]}]
physical = """
line
break"""
# "\n"
exit = 0
"#;

    let findings = manifest_syntax::manifest_policy_findings(Path::new("case.toml"), source);
    assert_eq!(
        findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            ("stdin", "escape-produced-line-break", "escape-produced LF"),
            ("stdin", "decoded-line-break-spelling", "\\n"),
            ("stdin", "escape-produced-line-break", "escape-produced LF"),
            ("stdin", "decoded-line-break-spelling", "\\r"),
        ]
    );
}

#[test]
pub(super) fn manifest_policy_reports_cr_unicode_and_obfuscated_spellings_in_order() {
    let source = r#"
command = ["check"]
cr = "\r"
unicode_lf = "\u000a"
unicode_cr = "\U0000000D"
literal = '\u000A'
assembled = "\u005Cn"
even_backslashes = "\\n"
odd_backslashes = "\\\n"
json = {"\u005Cr":"\\U0000000a"}
exit = 0
"#;

    let findings = manifest_syntax::manifest_policy_findings(Path::new("case.toml"), source);

    assert_eq!(
        findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            ("cr", "escape-produced-line-break", "escape-produced CR"),
            (
                "unicode_lf",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            (
                "unicode_cr",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            ("literal", "decoded-line-break-spelling", "\\u000A"),
            ("assembled", "decoded-line-break-spelling", "\\n"),
            ("even_backslashes", "decoded-line-break-spelling", "\\n"),
            (
                "odd_backslashes",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            ("json", "decoded-line-break-spelling", "\\r"),
            ("json", "decoded-line-break-spelling", "\\U0000000a"),
        ]
    );
}

#[test]
pub(super) fn manifest_policy_accepts_json_solidus_escape_in_keys_and_values() {
    let object_source = r#"
command = ["check"]
json = {"https:\/\/x":"https:\/\/v", "nested":{"slash\/key":"value\/ok", "line":"\u000A", "spelled":"\\r"}}
exit = 0
"#;
    let array_source = r#"
command = ["check"]
exit = 0
[[json_assert]]
path = "payload"
equals = ["https:\/\/x", {"slash\/key":"value\/ok"}, "\u000D", "\\n"]
"#;

    let manifest = parse_manifest(Path::new("case.toml"), array_source);
    let Some(ValueAssertionOperation::Equals(expected)) =
        &manifest.expectations.json_assertions[0].operation
    else {
        panic!("expected JSON equality operation");
    };
    assert_eq!(
        expected.to_compact_string(),
        r#"["https://x",{"slash/key":"value/ok"},"\r","\\n"]"#
    );

    let object_scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), object_source);
    assert_eq!(object_scan.error, None);
    assert_eq!(
        object_scan
            .findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            ("json", "escape-produced-line-break", "escape-produced LF"),
            ("json", "decoded-line-break-spelling", "\\r"),
        ]
    );

    let array_scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), array_source);
    assert_eq!(array_scan.error, None);
    assert_eq!(
        array_scan
            .findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [
            (
                "[[json_assert]].equals",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            (
                "[[json_assert]].equals",
                "decoded-line-break-spelling",
                "\\n"
            ),
        ]
    );
}

#[test]
pub(super) fn manifest_policy_scan_reports_invalid_json_escapes_in_object_and_array_roots() {
    let object_source = r#"
command = ["check"]
exit = 0
[[json_assert]]
path = "payload"
equals = {"bad":"\q"}
"#;
    let array_source = r#"
command = ["check"]
exit = 0
[[json_assert]]
path = "payload"
equals = ["valid\/solidus", {"bad":"\q"}]
"#;

    let object_scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), object_source);
    assert_eq!(
        object_scan.error.as_deref(),
        Some("unsupported JSON string escape `q`")
    );
    assert!(object_scan.findings.is_empty());

    let array_scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), array_source);
    assert_eq!(
        array_scan.error.as_deref(),
        Some("unsupported JSON string escape `q`")
    );
    assert!(array_scan.findings.is_empty());
}

#[test]
pub(super) fn policy_preflight_invalid_json_escape_prevents_generated_test_module_creation() {
    let root = test_temp_root("policy-invalid-json-escape-generation-block");
    fs::create_dir_all(root.join("cases/blocked")).expect("case directory should be created");
    fs::write(
        root.join("cases/blocked/case.toml"),
        r#"
command = ["check"]
exit = 0
[[json_assert]]
path = "payload"
equals = {"bad":"\q"}
"#,
    )
    .expect("blocked manifest should be written");

    let error = toolchain_case_inventory::generated_toolchain_tests_from_preflight(
        &root,
        &[test_discovery_root("cases", "cases")],
        true,
    )
    .expect_err("policy preflight should fail before generating tests");

    assert!(error.contains("toolchain case preflight found 1 problem(s)"));
    assert!(error.contains("cases/blocked: manifest policy scan failed"));
    assert!(error.contains("unsupported JSON string escape `q`"));
    assert!(!error.contains("generated_toolchain_cases"));

    fs::remove_dir_all(root).expect("inventory root should be removed");
}
