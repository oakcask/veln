use super::*;

const ALPHA_POLICY_MANIFEST: &[u8] =
    b"command = [\"check\"]\nstdin = [\"\\n\", \"\\r\"]\nexit = 0\n";
const BETA_POLICY_MANIFEST: &[u8] =
    b"command = [\"check\"]\n[stdout]\ncontains = ['\\u000A', '\\U0000000d']\nexit = 0\n";

fn write_manifests(root: &Path, cases: &[(&str, &[u8])]) {
    for (relative_dir, manifest) in cases {
        let case_dir = root.join(relative_dir);
        fs::create_dir_all(&case_dir).expect("case directory should be created");
        fs::write(case_dir.join("case.toml"), manifest).expect("case manifest should be written");
    }
}

fn assert_contains_all(text: &str, expected_fragments: &[&str]) {
    for fragment in expected_fragments {
        assert!(
            text.contains(fragment),
            "expected text to contain {fragment:?}\n\n{text}"
        );
    }
}

fn assert_fragments_in_order(text: &str, expected_fragments: &[&str]) {
    let mut remainder = text;
    for fragment in expected_fragments {
        let Some((_, after_fragment)) = remainder.split_once(fragment) else {
            panic!("expected text to contain {fragment:?} in order\n\n{text}");
        };
        remainder = after_fragment;
    }
}

#[test]
pub(super) fn manifest_policy_reports_lowercase_and_uppercase_unicode_line_break_matrix() {
    let source = r#"
command = ["check"]
lower_lf = "\u000a"
upper_lf = "\u000A"
lower_cr = "\u000d"
upper_cr = "\u000D"
wide_lower_lf = "\U0000000a"
wide_upper_lf = "\U0000000A"
wide_lower_cr = "\U0000000d"
wide_upper_cr = "\U0000000D"
literal_lower_lf = '\u000a'
literal_upper_lf = '\u000A'
literal_lower_cr = '\u000d'
literal_upper_cr = '\u000D'
literal_wide_lower_lf = '\U0000000a'
literal_wide_upper_lf = '\U0000000A'
literal_wide_lower_cr = '\U0000000d'
literal_wide_upper_cr = '\U0000000D'
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
            (
                "lower_lf",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            (
                "upper_lf",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            (
                "lower_cr",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            (
                "upper_cr",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            (
                "wide_lower_lf",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            (
                "wide_upper_lf",
                "escape-produced-line-break",
                "escape-produced LF"
            ),
            (
                "wide_lower_cr",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            (
                "wide_upper_cr",
                "escape-produced-line-break",
                "escape-produced CR"
            ),
            ("literal_lower_lf", "decoded-line-break-spelling", "\\u000a"),
            ("literal_upper_lf", "decoded-line-break-spelling", "\\u000A"),
            ("literal_lower_cr", "decoded-line-break-spelling", "\\u000d"),
            ("literal_upper_cr", "decoded-line-break-spelling", "\\u000D"),
            (
                "literal_wide_lower_lf",
                "decoded-line-break-spelling",
                "\\U0000000a"
            ),
            (
                "literal_wide_upper_lf",
                "decoded-line-break-spelling",
                "\\U0000000A"
            ),
            (
                "literal_wide_lower_cr",
                "decoded-line-break-spelling",
                "\\U0000000d"
            ),
            (
                "literal_wide_upper_cr",
                "decoded-line-break-spelling",
                "\\U0000000D"
            ),
        ]
    );
}

#[test]
pub(super) fn manifest_policy_accepts_physical_comments_and_token_boundaries() {
    let source = r#"
command = ["check"]
stdin = """
first
second"""
split = ['\', 'n']
# "\r"
exit = 0
"#;

    assert!(manifest_syntax::manifest_policy_findings(Path::new("case.toml"), source).is_empty());
}

#[test]
pub(super) fn manifest_policy_scan_keeps_findings_before_unterminated_string_boundary() {
    let source = "command = [\"check\"]\nstdin = \"\\n\"\nlate = \"unterminated";
    let scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), source);

    assert_eq!(
        scan.findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [("stdin", "escape-produced-line-break", "escape-produced LF")]
    );
    assert_eq!(scan.error.as_deref(), Some("unterminated manifest string"));
}

#[test]
pub(super) fn manifest_policy_scan_keeps_findings_before_lone_cr_boundary() {
    let source = "command = [\"check\"]\nstdin = \"\\r\"\rbad = true\n";
    let scan = manifest_syntax::manifest_policy_scan(Path::new("case.toml"), source);

    assert_eq!(
        scan.findings
            .iter()
            .map(|finding| (
                finding.field.as_str(),
                finding.category,
                finding.spelling.as_str()
            ))
            .collect::<Vec<_>>(),
        [("stdin", "escape-produced-line-break", "escape-produced CR")]
    );
    assert_eq!(
        scan.error.as_deref(),
        Some("lone carriage return in manifest")
    );
}

#[test]
pub(super) fn toolchain_policy_preflight_aggregates_skipped_unavailable_and_lexical_cases() {
    let root = test_temp_root("policy-aggregation");
    write_manifests(
        &root,
        &[
            (
                "cases/skipped",
                br#"
command = ["check"]
stdin = "\n"
exit = 0

[skip]
platforms = ["linux"]
reason = "lifecycle sentinel"
"#,
            ),
            (
                "cases/unavailable",
                br#"
command = ["check"]
exit = 0

[tools]
java = "missing"

[stdout]
contains = ["\\r"]
"#,
            ),
            (
                "cases/malformed",
                b"command = [\"check\"]\nstdin = \"\\n\"\nlate = \"unterminated",
            ),
            (
                "cases/lone-cr",
                b"command = [\"check\"]\nstdin = \"\\r\"\rbad = true\n",
            ),
        ],
    );

    let error = toolchain_case_inventory::run_preflight_with_roots_and_policy(
        &root,
        &[test_discovery_root("cases", "cases")],
        true,
    )
    .expect_err("policy preflight should fail");

    assert_contains_all(
        &error,
        &[
            "toolchain case preflight found 6 problem(s) affecting 4 manifest(s)",
            "lone carriage return in manifest",
            "cases/skipped:",
            "cases/unavailable:",
            "field `[stdout].contains` contains decoded-line-break-spelling",
            "cases/malformed: manifest policy scan failed",
            "cases/malformed:2:",
            "field `stdin` contains escape-produced-line-break",
            "unterminated manifest string",
        ],
    );
    assert_fragments_in_order(
        &error,
        &[
            "cases/lone-cr",
            "cases/malformed",
            "cases/skipped",
            "cases/unavailable",
        ],
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}

#[test]
pub(super) fn toolchain_policy_preflight_reports_stable_detailed_aggregate_findings() {
    let first = test_temp_root("policy-stable-aggregate-a");
    let second = test_temp_root("policy-stable-aggregate-b");
    write_manifests(
        &first,
        &[
            ("cases/beta", BETA_POLICY_MANIFEST),
            ("cases/alpha", ALPHA_POLICY_MANIFEST),
        ],
    );
    write_manifests(
        &second,
        &[
            ("cases/alpha", ALPHA_POLICY_MANIFEST),
            ("cases/beta", BETA_POLICY_MANIFEST),
        ],
    );

    let first_error = toolchain_case_inventory::run_preflight_with_roots(
        &first,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("first policy preflight should fail");
    let second_error = toolchain_case_inventory::run_preflight_with_roots(
        &second,
        &[test_discovery_root("cases", "cases")],
    )
    .expect_err("second policy preflight should fail");

    assert_eq!(
        first_error, second_error,
        "aggregate policy output should not depend on filesystem entry order"
    );
    assert_contains_all(
        &first_error,
        &[
            "toolchain case preflight found 4 problem(s) affecting 2 manifest(s)",
            "cases/alpha:2:29-33 field `stdin` contains escape-produced-line-break `escape-produced LF`; use physical multiline text or a sidecar so line structure remains reviewable",
            "cases/alpha:2:35-39 field `stdin` contains escape-produced-line-break `escape-produced CR`; use physical multiline text or a sidecar so line structure remains reviewable",
            "cases/beta:3:",
            "field `[stdout].contains` contains decoded-line-break-spelling `\\\\u000A`",
            "field `[stdout].contains` contains decoded-line-break-spelling `\\\\U0000000d`",
            "use physical multiline text or a sidecar so line structure remains reviewable",
        ],
    );

    fs::remove_dir_all(first).expect("first inventory root should be removed");
    fs::remove_dir_all(second).expect("second inventory root should be removed");
}

#[test]
pub(super) fn synthetic_policy_guard_runs_before_manifest_loading_skip_and_fixtures() {
    let root = test_temp_root("synthetic-policy-guard");
    let case_dir = root.join("synthetic");
    fs::create_dir_all(&case_dir).expect("synthetic case directory should be created");
    fs::write(case_dir.join("fixture.txt"), "fixture").expect("fixture should be written");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["check"]
stdin = "\n"
stdin_file = "case-text/missing-sidecar.txt"
exit = 0

[skip]
platforms = ["linux", "macos", "windows"]
reason = "skip evaluation must not run before policy"
"#,
    )
    .expect("synthetic manifest should be written");

    let panic = std::panic::catch_unwind(|| run_case(&case_dir))
        .expect_err("synthetic policy violation should stop the case before setup");
    let message = panic_message(panic);

    assert!(message.contains(
        "synthetic toolchain case violates manifest line-break policy before loading resources"
    ));
    assert!(message.contains("field `stdin` contains escape-produced-line-break"));
    assert!(
        !message.contains("skip evaluation must not run"),
        "skip evaluation should be bypassed by the synthetic policy guard"
    );
    assert!(
        !message.contains("missing-sidecar"),
        "sidecar resource loading should be bypassed by the synthetic policy guard"
    );
    assert!(
        !root.join("command-started").exists(),
        "command execution should be bypassed by the synthetic policy guard"
    );

    fs::remove_dir_all(root).expect("synthetic root should be removed");
}

#[test]
pub(super) fn synthetic_policy_guard_also_applies_inside_crate_for_non_inventory_cases() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let case_dir = manifest_dir.join("target/non-inventory-synthetic-policy");
    let _ = fs::remove_dir_all(&case_dir);
    fs::create_dir_all(&case_dir).expect("synthetic crate-local case directory should be created");
    fs::write(
        case_dir.join("case.toml"),
        "command = [\"check\"]\nstdin = \"\\n\"\nexit = 0\n",
    )
    .expect("synthetic manifest should be written");

    assert!(!is_generated_inventory_member(&case_dir));
    let panic = std::panic::catch_unwind(|| run_case(&case_dir))
        .expect_err("crate-local non-inventory case should use the synthetic guard");
    let message = panic_message(panic);
    assert!(message.contains(
        "synthetic toolchain case violates manifest line-break policy before loading resources"
    ));

    fs::remove_dir_all(case_dir).expect("synthetic case should be removed");
}

#[test]
pub(super) fn generated_inventory_membership_is_exact() {
    let generated = toolchain_case_path(
        GENERATED_TOOLCHAIN_CASES
            .first()
            .expect("generated inventory should not be empty"),
    );
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let sibling = manifest_dir.join("tests/toolchain_cases/not-generated");

    assert!(is_generated_inventory_member(&generated));
    assert!(!is_generated_inventory_member(&sibling));
}

#[cfg(unix)]
#[test]
pub(super) fn fixture_copy_rejects_links_before_command_execution() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("copy-link-reject");
    let case_dir = root.join("synthetic");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(
        case_dir.join("case.toml"),
        "command = [\"run-command-that-must-not-start\"]\nexit = 0\n",
    )
    .expect("case manifest should be written");
    fs::write(root.join("target.txt"), "target").expect("link target should be written");
    symlink("../target.txt", case_dir.join("fixture-link.txt")).expect("symlink should be created");

    let panic = std::panic::catch_unwind(|| run_case(&case_dir))
        .expect_err("fixture copy should reject symlinks");
    let message = panic_message(panic);
    assert!(message.contains("replace the link or reparse point with a regular fixture entry"));
    assert!(!message.contains("run-command-that-must-not-start"));

    fs::remove_dir_all(root).expect("copy link root should be removed");
}

#[test]
pub(super) fn dedicated_inventory_parity_test_reports_stale_generated_cases() {
    let relative = "target/stale-generated".to_string();
    let mut stale_generated = GENERATED_TOOLCHAIN_CASES
        .iter()
        .map(|case| (*case).to_string())
        .collect::<Vec<_>>();
    stale_generated.push(relative.clone());
    with_test_generated_toolchain_cases(stale_generated, || {
        let panic = std::panic::catch_unwind(runtime_generated_inventory_barrier)
            .expect_err("dedicated parity test should reject stale inventory");
        let message = panic_message(panic);

        assert!(message.contains("toolchain case preflight found"));
        assert!(message.contains(&format!(
            "{relative}: rebuild the toolchain harness because this generated case manifest is no longer discovered"
        )));
    });
}

#[test]
pub(super) fn toolchain_policy_preflight_keeps_reliable_cases_when_discovery_has_errors() {
    let root = test_temp_root("policy-partial-inventory");
    write_manifests(
        &root,
        &[
            (
                "primary/rooted",
                b"command = [\"check\"]\nstdin = \"\\n\"\nexit = 0\n",
            ),
            (
                "primary/rooted/nested",
                b"command = [\"check\"]\nstdin = \"\\r\"\nexit = 0\n",
            ),
            (
                "secondary/readable",
                b"command = [\"check\"]\n[stdout]\ncontains = [\"\\\\r\"]\nexit = 0\n",
            ),
        ],
    );

    let error = toolchain_case_inventory::run_preflight_with_roots_and_policy(
        &root,
        &[
            test_discovery_root("missing", "missing"),
            test_discovery_root("primary", "primary"),
            test_discovery_root("secondary", "secondary"),
        ],
        true,
    )
    .expect_err("mixed discovery and policy failures should aggregate");

    assert_contains_all(
        &error,
        &[
            "toolchain case preflight found 4 problem(s)",
            "missing: discovery root must be readable",
            "nested case.toml",
            "primary/rooted:",
            "field `stdin` contains escape-produced-line-break",
            "secondary/readable:",
            "field `[stdout].contains` contains decoded-line-break-spelling",
        ],
    );

    fs::remove_dir_all(root).expect("inventory root should be removed");
}
