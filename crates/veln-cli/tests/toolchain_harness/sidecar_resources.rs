use super::*;

#[test]
pub(super) fn manifest_jsonrpc_workspace_uri_directives_do_not_rewrite_marker_like_case_text() {
    let root = test_temp_root("jsonrpc-workspace-uri-case-text-collision");
    let case_dir = root.join("case");
    fs::create_dir_all(case_dir.join("case-text")).expect("case text directory should be created");
    fs::write(case_dir.join("main.veln"), "").expect("workspace file should be written");
    let marker_like_text = format!("{WORKSPACE_FILE_URI_MARKER}not-hex\nordinary text");
    fs::write(case_dir.join("case-text/marker.raw"), &marker_like_text)
        .expect("marker-like case text should be written");
    fs::write(
        case_dir.join("requests.json"),
        r#"[
  {"jsonrpc":"2.0","method":"text","params":{"text":{"$case_text":"case-text/marker.raw"}}},
  {"jsonrpc":"2.0","method":"uri","params":{"uri":{"$workspace_file_uri":"main.veln"}}}
]"#,
    )
    .expect("JSON-RPC fixture should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
    );
    assert!(
        manifest
            .invocation
            .stdin
            .as_deref()
            .is_some_and(|stdin| stdin.contains(WORKSPACE_FILE_URI_MARKER))
    );
    let framed = manifest
        .invocation
        .materialized_stdin(&case_dir)
        .expect("JSON-RPC stdin should materialize");
    let messages = decode_lsp_stdout(&framed).expect("framed JSON-RPC input should decode");
    assert_eq!(
        json_path(&messages[0], "params.text"),
        Some(&JsonValue::String(marker_like_text))
    );
    assert_eq!(
        json_path(&messages[1], "params.uri"),
        Some(&JsonValue::String(
            workspace_file_uri(&case_dir, "main.veln").expect("workspace URI should resolve")
        ))
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn manifest_jsonrpc_workspace_uri_directive_materializes_later_duplicate_member() {
    let root = test_temp_root("jsonrpc-workspace-uri-duplicate-member");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(case_dir.join("main.veln"), "").expect("workspace file should be written");
    fs::write(
        case_dir.join("requests.json"),
        r#"[
  {"jsonrpc":"2.0","method":"uri","params":{"uri":"preserve-first","uri":{"$workspace_file_uri":"main.veln"}}}
]"#,
    )
    .expect("JSON-RPC fixture should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        "command = [\"lsp\"]\nstdin_jsonrpc_file = \"requests.json\"\nexit = 0\n",
    );
    let framed = manifest
        .invocation
        .materialized_stdin(&case_dir)
        .expect("JSON-RPC stdin should materialize");
    assert!(
        !framed.contains(WORKSPACE_FILE_URI_MARKER),
        "workspace URI marker must not reach command input"
    );
    let messages = decode_lsp_stdout(&framed).expect("framed JSON-RPC input should decode");
    let JsonValue::Object(message_entries) = &messages[0] else {
        panic!("message should be an object");
    };
    let params = message_entries
        .iter()
        .find_map(|(key, value)| (key == "params").then_some(value))
        .expect("params should exist");
    let JsonValue::Object(params_entries) = params else {
        panic!("params should be an object");
    };
    let uri_values = params_entries
        .iter()
        .filter_map(|(key, value)| (key == "uri").then_some(value))
        .collect::<Vec<_>>();
    assert_eq!(uri_values.len(), 2);
    assert_eq!(
        uri_values[0],
        &JsonValue::String("preserve-first".to_string())
    );
    let expected_uri =
        workspace_file_uri(&case_dir, "main.veln").expect("workspace URI should resolve");
    assert_eq!(uri_values[1], &JsonValue::String(expected_uri));

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[cfg(unix)]
#[test]
pub(super) fn manifest_jsonrpc_resources_fail_before_skip_fixture_copy_and_command_start() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("jsonrpc-fixture-lifecycle");
    let case_dir = root.join("case");
    fs::create_dir_all(&case_dir).expect("case directory should be created");
    fs::write(case_dir.join("requests.json"), "{").expect("invalid fixture should be written");
    symlink("missing-target", case_dir.join("copy-must-not-start"))
        .expect("fixture-copy sentinel link should be created");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["command-must-not-start"]
stdin_jsonrpc_file = "requests.json"
exit = 0

[skip]
platforms = ["linux", "macos"]
reason = "skip evaluation must not run"
"#,
    )
    .expect("case manifest should be written");
    let panic = std::panic::catch_unwind(|| run_case(&case_dir))
        .expect_err("resource failure should precede the case lifecycle");
    let message = panic_message(panic);
    assert!(message.contains("invalid JSON-RPC fixture"));
    assert!(!message.contains("skip evaluation must not run"));
    assert!(!message.contains("fixture entries must not be links"));
    assert!(!message.contains("command-must-not-start"));

    fs::write(
        case_dir.join("requests.json"),
        r#"[{"jsonrpc":"2.0","method":"m","params":{"$case_text":"copy-must-not-start"}}]"#,
    )
    .expect("link directive fixture should be written");
    let panic = std::panic::catch_unwind(|| CaseManifest::read(&case_dir.join("case.toml")))
        .expect_err("link-like directive sidecar should fail");
    assert!(panic_message(panic).contains("must not traverse a link or reparse point"));
    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn manifest_sidecar_choice_cardinality_is_checked_before_file_io() {
    assert_manifest_parse_error(
        r#"
command = ["check"]
stdin = "inline"
stdin_file = "case-text/missing-sidecar.txt"
stdin_jsonrpc_file = "missing.json"
exit = 0
"#,
        "root invocation needs at most one of `stdin`, `stdin_file`, or `stdin_jsonrpc_file`",
    );
    assert_manifest_parse_error(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[json_assert]]
path = "stdout"
equals = "inline"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "json_assert 0 needs exactly one of `equals`, `equals_file`, `equals_json_file`, `contains`, `length`, `workspace_file_uri`, or `missing = true`",
    );
    assert_manifest_parse_error(
        r#"
command = ["run", "main", "main.veln"]
exit = 0

[[file_assert]]
path = "out.txt"
equals = "inline"
equals_file = "case-text/missing-sidecar.txt"
"#,
        "file_assert 0 needs exactly one of `equals`, `equals_file`, or `missing = true`",
    );
}

#[test]
pub(super) fn manifest_stream_fragments_accumulate_in_manifest_order() {
    let root = test_temp_root("manifest-fragment-order");
    let case_dir = root.join("examples/specification/check/manifest-fragment-order");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("middle.txt"), "middle").expect("sidecar should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"
command = ["check"]
exit = 0

[stdout]
contains = ["before"]
contains_file = "case-text/middle.txt"
contains = ["after"]
not_contains = ["forbidden before"]
not_contains = ["forbidden after"]
"#,
    );

    assert_eq!(
        manifest.expectations.stdout.contains,
        ["before", "middle", "after"]
    );
    assert_eq!(
        manifest.expectations.stdout.not_contains,
        ["forbidden before", "forbidden after"]
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn case_text_git_attributes_cover_text_and_raw_sidecars() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo = manifest_dir
        .parent()
        .and_then(Path::parent)
        .expect("manifest directory should be under the repository");
    let migrated_lsp_raw_sidecars = [
        "examples/specification/lsp/ambiguous-bare-prelude-definition/case-text/root-stdin-1.raw",
        "examples/specification/lsp/companion-private-function-identity/case-text/root-stdin-1.raw",
        "examples/specification/lsp/companion-private-function-rename-overlay/case-text/root-stdin-1.raw",
        "examples/specification/lsp/direct-dependency-virtual-document-boundary/case-text/root-stdin-1.raw",
        "examples/specification/lsp/direct-git-dependency-virtual-document/case-text/root-stdin-1.raw",
        "examples/specification/lsp/handler-context-callable-binding/case-text/root-stdin-1.raw",
        "examples/specification/lsp/handler-context-operation-heading-isolation/case-text/root-stdin-1.raw",
        "examples/specification/lsp/handler-operation-editor/case-text/root-stdin-1.raw",
        "examples/specification/lsp/handler-satisfy-semantic-tokens/case-text/root-stdin-1.raw",
        "examples/specification/lsp/handler-semantic-tokens/case-text/root-stdin-1.raw",
        "examples/specification/lsp/imported-constructor-bare-prelude-definition/case-text/root-stdin-1.raw",
        "examples/specification/lsp/open-document-nested-boundary/case-text/root-stdin-1.raw",
        "examples/specification/lsp/private-import-bare-prelude-definition/case-text/root-stdin-1.raw",
        "examples/specification/lsp/reexported-constructor-bare-prelude-definition/case-text/root-stdin-1.raw",
        "examples/specification/lsp/schema-semantic-tokens/case-text/root-stdin-1.raw",
        "examples/specification/lsp/standard-library-virtual-document/case-text/root-stdin-1.raw",
        "examples/specification/lsp/unopened-missing-file/case-text/root-stdin-1.raw",
        "examples/specification/lsp/workspace-diagnostics/case-text/root-stdin-1.raw",
        "examples/specification/lsp/workspace-package-root-selection/case-text/root-stdin-1.raw",
    ];
    let ordinary_sidecars = [
        "crates/veln-cli/tests/toolchain_cases/run/json-success/case-text/json-assert-equals-1.txt",
        "crates/veln-cli/tests/toolchain_cases/run/json-success/case-text/nested/json-assert-equals-1.txt",
    ];
    let pattern_raw_sidecars = [
        "crates/veln-cli/tests/toolchain_cases/run/json-success/case-text/protocol.raw",
        "crates/veln-cli/tests/toolchain_cases/run/json-success/case-text/nested/protocol.raw",
        "examples/specification/lsp/semantic-tokens/case-text/protocol.raw",
        "examples/specification/lsp/semantic-tokens/case-text/nested/protocol.raw",
    ];
    let mut check_attr_args = vec!["check-attr", "text", "eol", "diff", "whitespace", "--"];
    check_attr_args.extend(ordinary_sidecars);
    check_attr_args.extend(pattern_raw_sidecars);
    check_attr_args.extend(migrated_lsp_raw_sidecars);
    let output = Command::new("git")
        .current_dir(repo)
        .args(check_attr_args)
        .output()
        .expect("git check-attr should run");
    assert!(
        output.status.success(),
        "git check-attr failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("attribute output should be utf-8");
    for ordinary in ordinary_sidecars {
        assert!(stdout.contains(&format!("{ordinary}: text: set")));
        assert!(stdout.contains(&format!("{ordinary}: eol: lf")));
        assert!(stdout.contains(&format!("{ordinary}: whitespace: unset")));
    }
    for raw in pattern_raw_sidecars
        .into_iter()
        .chain(migrated_lsp_raw_sidecars)
    {
        assert!(stdout.contains(&format!("{raw}: text: unset")));
        assert!(stdout.contains(&format!("{raw}: eol: unset")));
        assert!(stdout.contains(&format!("{raw}: diff: unset")));
    }
    let mut ls_files_args = vec!["ls-files", "--eol", "--"];
    ls_files_args.extend(migrated_lsp_raw_sidecars);
    let output = Command::new("git")
        .current_dir(repo)
        .args(ls_files_args)
        .output()
        .expect("git ls-files --eol should run");
    assert!(
        output.status.success(),
        "git ls-files --eol failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("eol output should be utf-8");
    for raw in migrated_lsp_raw_sidecars {
        assert!(
            stdout.contains(&format!("i/crlf  w/crlf  attr/-text            \t{raw}")),
            "{raw} should be checked out without text normalization:\n{stdout}"
        );
    }
}

#[test]
pub(super) fn manifest_sidecar_paths_reject_nonportable_components_before_io() {
    for relative in [
        "case-text/CON.txt",
        "case-text/lpt9.log",
        "case-text/trailing.",
        "case-text/space name.txt",
        "../escape.txt",
    ] {
        let source = format!("command = [\"check\"]\nstdin_file = {relative:?}\nexit = 0\n");
        assert_manifest_parse_error(&source, "must use portable relative components");
    }
}

#[cfg(unix)]
#[test]
pub(super) fn manifest_sidecar_paths_reject_links_without_following_targets() {
    use std::os::unix::fs::symlink;

    let root = test_temp_root("manifest-sidecar-link");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(root.join("outside.txt"), "target").expect("link target should be written");
    symlink("../../outside.txt", text_dir.join("linked.txt")).expect("file symlink should exist");
    symlink("../../outside-dir", text_dir.join("linked-dir")).expect("dir symlink should exist");

    for relative in ["case-text/linked.txt", "case-text/linked-dir/file.txt"] {
        let source = format!("command = [\"check\"]\nstdin_file = {relative:?}\nexit = 0\n");
        let panic = std::panic::catch_unwind(|| {
            parse_manifest(&case_dir.join("case.toml"), &source);
        })
        .expect_err("sidecar link traversal should be rejected");
        let message = panic_message(panic);
        assert!(
            message.contains("must not traverse a link or reparse point"),
            "unexpected sidecar link error: {message}"
        );
        assert!(
            !message.contains("outside"),
            "sidecar diagnostics should not expose followed targets: {message}"
        );
    }

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn manifest_sidecar_files_must_be_utf8_before_skip_evaluation() {
    let root = test_temp_root("manifest-sidecar-utf8-skip");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("invalid.raw"), [0xff, b'a']).expect("invalid sidecar should exist");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["run-command-that-must-not-start"]
stdin_file = "case-text/invalid.raw"
exit = 0

[skip]
platforms = ["linux", "macos", "windows"]
reason = "skip evaluation must not hide sidecar loading"
"#,
    )
    .expect("case manifest should be written");

    let panic = std::panic::catch_unwind(|| run_case(&case_dir))
        .expect_err("invalid UTF-8 sidecar should fail before skip evaluation");
    let message = panic_message(panic);
    assert!(message.contains("failed to read case file `case-text/invalid.raw` as UTF-8"));
    assert!(
        !message.contains("skip evaluation must not hide sidecar loading"),
        "skip evaluation should be bypassed by sidecar loading failure"
    );
    assert!(
        !message.contains("run-command-that-must-not-start"),
        "command execution should be bypassed by sidecar loading failure"
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn manifest_sidecar_files_preserve_bom_crlf_and_final_line_breaks() {
    let root = test_temp_root("manifest-sidecar-exact-text");
    let case_dir = root.join("case");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("exact.raw"), b"\xef\xbb\xbfalpha\r\nbeta\r\n")
        .expect("exact text sidecar should be written");

    let manifest = parse_manifest(
        &case_dir.join("case.toml"),
        r#"
command = ["check"]
stdin_file = "case-text/exact.raw"
exit = 0
"#,
    );

    assert_eq!(
        manifest.invocation.stdin.as_deref(),
        Some("\u{feff}alpha\r\nbeta\r\n")
    );

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn manifest_sidecar_snapshots_are_immutable_across_repeated_invocations() {
    let root = test_temp_root("manifest-sidecar-repeat-snapshot");
    let case_dir = root.join("examples/specification/check/manifest-sidecar-repeat-snapshot");
    let text_dir = case_dir.join("case-text");
    fs::create_dir_all(&text_dir).expect("case text directory should be created");
    fs::write(text_dir.join("out.txt"), "original").expect("sidecar should be written");
    fs::write(case_dir.join("out.txt"), "original").expect("asserted file should be written");
    fs::write(case_dir.join("main.veln"), "fn main() -> ()\n\t()\nend\n")
        .expect("source file should be written");
    fs::write(
        case_dir.join("case.toml"),
        r#"
command = ["check", "main.veln"]
repeat = 2
exit = 0

[[file_assert]]
path = "out.txt"
equals_file = "case-text/out.txt"
"#,
    )
    .expect("case manifest should be written");

    let panic = std::panic::catch_unwind(|| {
        run_case_with_after_invocation(&case_dir, |context, project_root| {
            if context.run_number == 1 {
                fs::write(project_root.join("out.txt"), "mutated")
                    .expect("copied fixture should be mutable after first run");
            }
        });
    })
    .expect_err("second run should compare against the original sidecar snapshot");
    let message = panic_message(panic);
    assert!(message.contains("run 2"));
    assert!(message.contains("file `out.txt` contents mismatch"));

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn file_assert_missing_checks_copied_project_after_execution() {
    let root = test_temp_root("file-assert-missing");
    let pass_case_dir = root.join("examples/specification/check/file-assert-missing-pass");
    let fail_case_dir = root.join("examples/specification/check/file-assert-missing-fail");
    fs::create_dir_all(&pass_case_dir).expect("pass case directory should be created");
    fs::create_dir_all(&fail_case_dir).expect("fail case directory should be created");
    for case_dir in [&pass_case_dir, &fail_case_dir] {
        fs::write(case_dir.join("main.veln"), "fn main() -> ()\n\t()\nend\n")
            .expect("source should be written");
        fs::write(
            case_dir.join("case.toml"),
            r#"
command = ["check", "main.veln"]
exit = 0

[[file_assert]]
path = "absent.txt"
missing = true
"#,
        )
        .expect("case manifest should be written");
    }
    fs::write(fail_case_dir.join("absent.txt"), "present").expect("asserted file should exist");

    run_case(&pass_case_dir);
    let panic = std::panic::catch_unwind(|| {
        run_case(&fail_case_dir);
    })
    .expect_err("present file should fail a missing assertion");
    let message = panic_message(panic);
    assert!(message.contains("file `absent.txt` exists but should be missing"));

    fs::remove_dir_all(root).expect("case root should be removed");
}

#[test]
pub(super) fn manifest_validation_rejects_incomplete_section_expectations() {
    assert_manifest_parse_error(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[binary_fixture]]
name = "short-u24"
hex = "0001"
byte_offset = 2
field_path = []
expected_count = 3
"#,
        "binary_fixture 0 has incomplete byte count metadata",
    );
    assert_manifest_parse_error(
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[output_chunk_list]]
name = "protocol-output"
"#,
        "output_chunk_list 0 is missing `chunks`",
    );
}

pub(super) fn lsp_frame(body: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{body}", body.len())
}

pub(super) fn parsed_lsp_assertions(source: &str) -> Vec<LspAssertion> {
    parse_manifest(Path::new("case.toml"), source)
        .expectations
        .lsp_assertions
}

pub(super) fn parsed_mcp_assertions(source: &str) -> Vec<McpAssertion> {
    parse_manifest(Path::new("case.toml"), source)
        .expectations
        .mcp_assertions
}
