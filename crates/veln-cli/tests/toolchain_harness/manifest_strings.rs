use super::*;

#[test]
pub(super) fn manifest_string_forms_decode_in_scalar_and_array_fields() {
    for (spelling, expected) in [
        (r#""basic\nvalue""#, "basic\nvalue"),
        (r#"'literal\nvalue'"#, r#"literal\nvalue"#),
        ("\"\"\"\nmultiline basic\"\"\"", "multiline basic"),
        ("'''\nmultiline literal'''", "multiline literal"),
        ("\"\"\"\"\"\"", ""),
        ("''''''", ""),
    ] {
        let manifest = parse_manifest(
            Path::new("case.toml"),
            &format!("command = [\"check\"]\nstdin = {spelling}\nexit = 0\n"),
        );
        assert_eq!(manifest.invocation.stdin.as_deref(), Some(expected));
    }

    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\n  \"basic\",\n  'literal', # inter-element comment\n  \"\"\"\nmultiline basic\"\"\",\n  '''\nmultiline literal''',\n]\nexit = 0\n",
    );
    assert_eq!(
        manifest.invocation.command,
        ["basic", "literal", "multiline basic", "multiline literal"]
    );
}

#[test]
pub(super) fn manifest_basic_string_escape_matrix_decodes_unicode_scalars() {
    for (escape, expected) in [
        (r#"\b"#, "\u{08}"),
        (r#"\t"#, "\t"),
        (r#"\n"#, "\n"),
        (r#"\f"#, "\u{0c}"),
        (r#"\r"#, "\r"),
        (r#"\""#, "\""),
        (r#"\\"#, "\\"),
        (r#"\u03B1"#, "α"),
        (r#"\U0001F642"#, "🙂"),
    ] {
        let source = format!("command = [\"check\"]\nstdin = \"{escape}\"\nexit = 0\n");
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some(expected));
    }
}

#[test]
pub(super) fn manifest_invalid_string_token_matrix_rejects_toml_boundaries() {
    for (spelling, fact) in [
        (r#""\x""#, "unsupported manifest string escape"),
        (r#""\u12""#, "incomplete Unicode escape"),
        (r#""\u12x4""#, "invalid hexadecimal digit"),
        (r#""\uD800""#, "not a scalar value"),
        (r#""\U00110000""#, "not a scalar value"),
        ("\"control \u{1}\"", "prohibited control character"),
        ("'control \u{7f}'", "prohibited control character"),
        (
            "\"\"\"invalid\"\"\"\"\"\"",
            "invalid multiline string quote run",
        ),
        ("'''invalid''''''", "invalid multiline string quote run"),
    ] {
        let source = format!("command = [\"check\"]\nstdin = {spelling}\nexit = 0\n");
        assert_manifest_parse_error(&source, fact);
    }
}

#[test]
pub(super) fn manifest_multiline_strings_preserve_layout_folding_and_quote_runs() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nstdin = \"\"\"\nalpha\\\n \t beta\\  \n\n\t gamma\"\"\"\nexit = 0\n",
    );
    assert_eq!(manifest.invocation.stdin.as_deref(), Some("alphabetagamma"));

    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nstdin = \"\"\"\nalpha\\\\\nbeta\"\"\"\nexit = 0\n",
    );
    assert_eq!(manifest.invocation.stdin.as_deref(), Some("alpha\\\nbeta"));

    for (spelling, expected) in [
        ("\"\"\"one\"\"two\"\"\"\"", "one\"\"two\""),
        ("'''one''two''''", "one''two'"),
        (
            "'''\n  [section]\n\tkey = #,\n '''",
            "  [section]\n\tkey = #,\n ",
        ),
    ] {
        let source = format!("command = [\"check\"]\nstdin = {spelling}\nexit = 0\n");
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some(expected));
    }
}

#[test]
pub(super) fn manifest_multiline_indentation_and_closing_delimiters_are_value_bytes() {
    for (spelling, expected) in [
        (
            "\"\"\"\n\tleft\n  middle\n\t \nright\"\"\"",
            "\tleft\n  middle\n\t \nright",
        ),
        (
            "'''\n\tleft\n  middle\n\t \nright'''",
            "\tleft\n  middle\n\t \nright",
        ),
        ("\"\"\"\nvalue\n\"\"\"", "value\n"),
        ("\"\"\"\nvalue\n  \"\"\"", "value\n  "),
        ("\"\"\"\nvalue\"\"\"", "value"),
        ("'''\nvalue\n'''", "value\n"),
        ("'''\nvalue\n\t'''", "value\n\t"),
        ("'''\nvalue'''", "value"),
    ] {
        let source = format!("command = [\"check\"]\nstdin = {spelling}\nexit = 0\n");
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some(expected));
    }
}

#[test]
pub(super) fn manifest_multiline_array_placement_does_not_indent_values() {
    let scalar = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nstdin = \"\"\"\nvalue\n\"\"\"\nexit = 0\n",
    );
    let shallow_array = parse_manifest(
        Path::new("case.toml"),
        "command = [\n\"\"\"\nvalue\n\"\"\"\n]\nexit = 0\n",
    );
    let deep_array = parse_manifest(
        Path::new("case.toml"),
        "command = [\n        \"\"\"\nvalue\n\"\"\"\n]\nexit = 0\n",
    );

    assert_eq!(scalar.invocation.stdin.as_deref(), Some("value\n"));
    assert_eq!(shallow_array.invocation.command, ["value\n"]);
    assert_eq!(deep_array.invocation.command, ["value\n"]);
}

#[test]
pub(super) fn manifest_multiline_quote_run_matrix_preserves_terminal_quotes() {
    for (spelling, expected) in [
        ("\"\"\"one\"two\"\"\"", "one\"two"),
        ("\"\"\"one\"\"two\"\"\"", "one\"\"two"),
        ("\"\"\"tail\"\"\"", "tail"),
        ("\"\"\"tail\"\"\"\"", "tail\""),
        ("\"\"\"tail\"\"\"\"\"", "tail\"\""),
        ("'''one'two'''", "one'two"),
        ("'''one''two'''", "one''two"),
        ("'''tail'''", "tail"),
        ("'''tail''''", "tail'"),
        ("'''tail'''''", "tail''"),
    ] {
        let source = format!("command = [\"check\"]\nstdin = {spelling}\nexit = 0\n");
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some(expected));
    }
}

#[test]
pub(super) fn manifest_physical_newline_matrix_normalizes_multiline_values() {
    for delimiters in [("\"\"\"", "\"\"\""), ("'''", "'''")] {
        let lf = format!(
            "command = [\"check\"]\nstdin = {}\nfirst\nsecond{}\nexit = 0\n",
            delimiters.0, delimiters.1
        );
        let crlf = lf.replace('\n', "\r\n");
        let mixed = lf.replacen('\n', "\r\n", 2);
        for source in [lf, crlf, mixed] {
            let manifest = parse_manifest(Path::new("case.toml"), &source);
            assert_eq!(manifest.invocation.stdin.as_deref(), Some("first\nsecond"));
        }
    }

    for opening in ["\n", "\r\n"] {
        let source = format!(
            "command = [\"check\"]\nstdin = \"\"\"{opening}{opening}value\"\"\"\nexit = 0\n"
        );
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some("\nvalue"));
    }
}

#[test]
pub(super) fn manifest_multiline_basic_folding_accepts_lf_crlf_and_mixed_lines() {
    let lf = "command = [\"check\"]\nstdin = \"\"\"\nalpha\\\n \t beta\\  \n\n\t gamma\"\"\"\nexit = 0\n";
    let crlf = lf.replace('\n', "\r\n");
    let mixed = lf.replacen('\n', "\r\n", 4);
    for source in [lf.to_string(), crlf, mixed] {
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin.as_deref(), Some("alphabetagamma"));
    }
}

#[test]
pub(super) fn manifest_physical_newlines_match_escaped_lf_not_escaped_crlf() {
    let escaped_lf = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nstdin = \"first\\n  second\\n\"\nexit = 0\n",
    );
    for source in [
        "command = [\"check\"]\nstdin = \"\"\"\nfirst\n  second\n\"\"\"\nexit = 0\n".to_string(),
        "command = [\"check\"]\r\nstdin = \"\"\"\r\nfirst\r\n  second\r\n\"\"\"\r\nexit = 0\r\n"
            .to_string(),
    ] {
        let manifest = parse_manifest(Path::new("case.toml"), &source);
        assert_eq!(manifest.invocation.stdin, escaped_lf.invocation.stdin);
    }

    let escaped_crlf = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nstdin = \"first\\r\\n  second\\r\\n\"\nexit = 0\n",
    );
    assert_ne!(escaped_crlf.invocation.stdin, escaped_lf.invocation.stdin);
}

#[test]
pub(super) fn manifest_field_directed_containers_keep_array_and_json_grammars_distinct() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\n  \"check\",\n  'main.veln',\n]\nexit = 0\n\n[stdout]\ncontains = [\n  \"a,#[]{}\",\n  '''b''',\n]\n\n[[json_assert]]\npath = \"payload\"\nequals = {\n  \"array\": [1, {\"ok\": true}],\n  \"text\": \"a,#[]{}\"\n}\n",
    );
    assert_eq!(manifest.invocation.command, ["check", "main.veln"]);
    assert_eq!(manifest.expectations.stdout.contains, ["a,#[]{}", "b"]);
    let Some(ValueAssertionOperation::Equals(expected)) =
        &manifest.expectations.json_assertions[0].operation
    else {
        panic!("expected JSON equality operation");
    };
    assert_eq!(
        expected.to_compact_string(),
        r#"{"array":[1,{"ok":true}],"text":"a,#[]{}"}"#
    );

    for invalid in [
        "command = [\"check\", 1]\nexit = 0\n",
        "command = [\"check\", true]\nexit = 0\n",
        "command = [\"check\", null]\nexit = 0\n",
        "command = [\"check\", [\"nested\"]]\nexit = 0\n",
        "command = [\"check\", {\"nested\":\"object\"}]\nexit = 0\n",
        "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = [1, # no comments\n2]\n",
        "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = [1,]\n",
        "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = ['literal']\n",
        "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = [\"\"\"multi\"\"\"]\n",
    ] {
        assert_manifest_parse_error(invalid, "case.toml:");
    }

    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = '''\ntext\n'''",
    );
    assert_eq!(
        manifest.expectations.json_assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::String(
            "text\n".to_string()
        )))
    );
}

#[test]
pub(super) fn manifest_string_array_layout_matrix_accepts_schema_selected_fields() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\n  \"check\",\n  'main.veln',\n]\nexit = 0\n[stdout]\ncontains = []\nnot_contains = [\n  \"basic\",\n  'literal',\n  \"\"\"\nmultiline basic\"\"\",\n  '''\nmultiline literal''',\n  # trailing comment\n]\n[help]\ncommands = [\"check\",]\narguments = []\noptions = [\n  \"--json\",\n]\ncontains = [\n  \"done\",\n]\n",
    );

    assert_eq!(manifest.invocation.command, ["check", "main.veln"]);
    assert!(manifest.expectations.stdout.contains.is_empty());
    assert_eq!(
        manifest.expectations.stdout.not_contains,
        ["basic", "literal", "multiline basic", "multiline literal"]
    );
    let help = manifest.expectations.help.as_ref().expect("help section");
    assert_eq!(help.commands, ["check"]);
    assert!(help.arguments.is_empty());
    assert_eq!(help.options, ["--json"]);
    assert_eq!(help.contains, ["done"]);
}

#[test]
pub(super) fn manifest_array_boundaries_keep_punctuation_inside_string_tokens() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        "command = [\"check\"]\nexit = 0\n[stdout]\ncontains = [\n  \"brackets [] braces {} comma , hash # quote \\\"\",\n]\n[[json_assert]]\npath = \"x\"\nequals = [\n  \"brackets [] braces {} comma , hash # quote \\\"\"\n]\n",
    );

    let expected = "brackets [] braces {} comma , hash # quote \"";
    assert_eq!(manifest.expectations.stdout.contains, [expected]);
    let Some(ValueAssertionOperation::Equals(value)) =
        &manifest.expectations.json_assertions[0].operation
    else {
        panic!("expected JSON equality operation");
    };
    assert_eq!(value.to_compact_string(), format!("[{expected:?}]"));
}

#[test]
pub(super) fn manifest_container_trailing_tokens_and_local_errors_take_precedence() {
    for (value, line) in [
        ("\"\"\"\ntext\n\"\"\" trailing", 4),
        ("[\n  \"check\"\n] trailing", 3),
        ("{\n  \"ok\": true\n} trailing", 7),
    ] {
        let source = if value.starts_with('{') {
            format!(
                "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {value}\n"
            )
        } else if value.starts_with('[') {
            format!("command = {value}\nexit = 0\n")
        } else {
            format!("command = [\"check\"]\nstdin = {value}\nexit = 0\n")
        };
        let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), &source))
            .expect_err("trailing token should be rejected");
        let message = panic_message(panic);
        assert!(
            message.contains("unexpected token after completed manifest value"),
            "unexpected failure: {message}"
        );
        assert!(
            message.contains(&format!("case.toml:{line}:")),
            "unexpected error line: {message}"
        );
    }

    let source = "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {\n  \"bad\": # local JSON error\n";
    let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), source))
        .expect_err("local JSON error should be rejected");
    let message = panic_message(panic);
    assert!(message.contains("case.toml:6: invalid json assertion value"));
    assert!(!message.contains("unterminated container"));

    for (source, line, fact) in [
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {\n  \"bad\"\n",
            7,
            "expected `:`",
        ),
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {\n  \"bad\":\n",
            7,
            "unexpected end of input",
        ),
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = [\n  1,\n",
            7,
            "unexpected end of input",
        ),
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {\n  \"nested\": [\n    1\n",
            6,
            "unterminated container; expected `]`",
        ),
    ] {
        let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), source))
            .expect_err("incomplete JSON container should be rejected");
        let message = panic_message(panic);
        assert!(
            message.contains(&format!("case.toml:{line}:")),
            "unexpected error line: {message}"
        );
        assert!(
            message.contains(fact),
            "expected `{fact}` in error, got `{message}`"
        );
        if fact != "unterminated container; expected `]`" {
            assert!(
                !message.contains("unterminated container"),
                "local JSON error was replaced by outer delimiter error: {message}"
            );
        }
    }
}

#[test]
pub(super) fn manifest_syntax_errors_report_exact_physical_lines() {
    for (source, line, fact) in [
        (
            "command = [\"check\"]\nstdin = \"\"\"\nok\nbad \\q\n\"\"\"\nexit = 0\n",
            4,
            "unsupported manifest string escape",
        ),
        (
            "command = [\n  \"check\"\n  \"main.veln\"\n]\nexit = 0\n",
            3,
            "expected `,` before string array element",
        ),
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = [\n  1,\n  # forbidden\n  2\n]\n",
            7,
            "invalid json assertion value",
        ),
        (
            "command = [\"check\"] trailing\nexit = 0\n",
            1,
            "unexpected token after completed manifest value",
        ),
        (
            "command = [\"check\"]\nexit = 0\n[[json_assert]]\npath = \"x\"\nequals = {\n  \"nested\": [\n    1\n",
            6,
            "unterminated container; expected `]`",
        ),
        (
            "command = [\"check\"]\nstdin = \"\\u12\nexit = 0\n",
            2,
            "incomplete Unicode escape",
        ),
        (
            "command = [\"check\"]\r\nstdin = \"\\u12\r\nexit = 0\r\n",
            2,
            "incomplete Unicode escape",
        ),
    ] {
        let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), source))
            .expect_err("invalid manifest should be rejected");
        let message = panic_message(panic);
        assert!(
            message.contains(&format!("case.toml:{line}: {fact}")),
            "expected line {line} and `{fact}`, got `{message}`"
        );
    }

    for source in [
        "command = [\"check\"]\rstdin = \"x\"\nexit = 0\n",
        "command = [\"check\"] # comment\rstill comment\nexit = 0\n",
        "command = [\"check\"]\nstdin = '''a\rb'''\nexit = 0\n",
        "command = [\"check\"]\nstdin = \"\"\"a\\\rb\"\"\"\nexit = 0\n",
    ] {
        assert_manifest_parse_error(source, "lone carriage return");
    }
}

#[test]
pub(super) fn manifest_syntax_errors_report_equivalent_lines_with_lf_crlf_and_mixed_prefixes() {
    let lf_prefix = "command = [\"check\"]\nexit = 0\n[stdout]\n";
    let crlf_prefix = lf_prefix.replace('\n', "\r\n");
    let mixed_prefix = lf_prefix.replacen('\n', "\r\n", 2);
    for prefix in [lf_prefix.to_string(), crlf_prefix, mixed_prefix] {
        let source = format!("{prefix}contains = [\n  \"ok\"\n  \"missing comma\"\n]\n");
        let panic = std::panic::catch_unwind(|| parse_manifest(Path::new("case.toml"), &source))
            .expect_err("missing comma should be rejected");
        let message = panic_message(panic);
        assert!(
            message.contains("case.toml:6: expected `,` before string array element"),
            "unexpected error line: {message}"
        );
    }
}

#[test]
pub(super) fn manifest_binary_fixtures_parse_named_bytes_and_errors() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[binary_fixture]]
name = "short-u24"
hex = "0001"
consumed = 2
byte_offset = 2
expected_count = 3
available_count = 2
readiness = "need_bytes"
field_path = []

[[binary_fixture]]
name = "invalid-frame-kind"
schema = "DemoPacket"
hex = "ff0001"
consumed = 1
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"DemoPacket"},{"kind":"field","name":"kind"}]

[[binary_fixture]]
name = "bad-separator"
error = "fixture.hex.invalid_character"
"#,
    );

    assert!(manifest.expectations.needs_stdout_json());
    let fixtures = &manifest.expectations.binary_fixtures;
    assert_eq!(fixtures.len(), 3);
    assert_eq!(fixtures[0].name, "short-u24");
    assert_eq!(fixtures[0].bytes.as_ref().unwrap().hex, "0001");
    assert_eq!(fixtures[0].bytes.as_ref().unwrap().bytes, [0, 1]);
    assert_eq!(fixtures[0].consumed, Some(2));
    assert_eq!(
        expected_binary_fixture_line(&fixtures[0]),
        "fixture short-u24 hex 0001 count 2 consumed 2 offset 2 expected 3 available 2 readiness need_bytes field_path []"
    );
    assert_eq!(fixtures[1].name, "invalid-frame-kind");
    assert_eq!(fixtures[1].schema.as_deref(), Some("DemoPacket"));
    assert_eq!(fixtures[1].bytes.as_ref().unwrap().hex, "ff0001");
    assert_eq!(fixtures[1].consumed, Some(1));
    assert_eq!(
        expected_binary_fixture_line(&fixtures[1]),
        "fixture invalid-frame-kind hex ff0001 count 3 consumed 1 diagnostic schema.invalid_field_value offset 0 field_path [{\"kind\":\"schema\",\"name\":\"DemoPacket\"},{\"kind\":\"field\",\"name\":\"kind\"}]"
    );
    assert_eq!(fixtures[2].name, "bad-separator");
    assert_eq!(
        fixtures[2].error.as_deref(),
        Some("fixture.hex.invalid_character")
    );
    assert_eq!(
        expected_binary_fixture_line(&fixtures[2]),
        "fixture bad-separator error fixture.hex.invalid_character"
    );
}
