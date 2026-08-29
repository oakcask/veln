use super::*;

#[test]
fn multiline_decoder_preserves_provenance_across_continuations_and_escapes() {
    let decoded = decode_toml_string(
        "\"\"\"\nfirst\\\n  second\\u0021\r\nthird\"\"\"",
        7,
        StringForm::MultilineBasic,
        3,
    )
    .expect("multiline basic string should decode");

    assert_eq!(decoded.text(), "firstsecond!\nthird");
    assert_eq!(
        decoded
            .chars
            .iter()
            .map(|decoded| (decoded.value, decoded.source_line, decoded.escaped))
            .collect::<Vec<_>>(),
        [
            ('f', 8, false),
            ('i', 8, false),
            ('r', 8, false),
            ('s', 8, false),
            ('t', 8, false),
            ('s', 9, false),
            ('e', 9, false),
            ('c', 9, false),
            ('o', 9, false),
            ('n', 9, false),
            ('d', 9, false),
            ('!', 9, true),
            ('\n', 9, false),
            ('t', 10, false),
            ('h', 10, false),
            ('i', 10, false),
            ('r', 10, false),
            ('d', 10, false),
        ]
    );
}

#[test]
fn policy_scan_provenance_covers_toml_and_nested_json_string_tokens() {
    let source = r#"value = ["\n", '\n', {"json":"\u000A", "nested":["\\n"]}]
physical = """
line
break"""
# "ignored\r"
"#;
    let tokens = Lexer::new(Path::new("case.toml"), source).lex().tokens;
    let strings = tokens
        .iter()
        .filter_map(|token| match &token.kind {
            TokenKind::String(string) => Some(string),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        strings
            .iter()
            .map(|string| string.source)
            .collect::<Vec<_>>(),
        [
            r#""\n""#,
            r#"'\n'"#,
            r#""json""#,
            r#""\u000A""#,
            r#""nested""#,
            r#""\\n""#,
            "\"\"\"\nline\nbreak\"\"\"",
        ]
    );
    assert_eq!(strings[0].decoded.as_ref().unwrap().text(), "\n");
    assert_eq!(strings[1].decoded.as_ref().unwrap().text(), r#"\n"#);
    assert_eq!(strings[3].decoded.as_ref().unwrap().text(), "\n");
    assert_eq!(strings[5].decoded.as_ref().unwrap().text(), r#"\n"#);

    let physical = strings[6].decoded.as_ref().unwrap();
    assert_eq!(physical.text(), "line\nbreak");
    assert_eq!(
        physical
            .chars
            .iter()
            .map(|decoded| (decoded.value, decoded.source_line))
            .collect::<Vec<_>>(),
        [
            ('l', 3),
            ('i', 3),
            ('n', 3),
            ('e', 3),
            ('\n', 3),
            ('b', 4),
            ('r', 4),
            ('e', 4),
            ('a', 4),
            ('k', 4),
        ]
    );
}

#[test]
fn policy_scan_provenance_retains_escape_lines_and_local_decode_errors() {
    let source = "first = \"\"\"\nphysical\n\\u000A\"\"\"\ninvalid = \"bad\\q\"\n";
    let strings = Lexer::new(Path::new("case.toml"), source)
        .lex()
        .tokens
        .into_iter()
        .filter_map(|token| match token.kind {
            TokenKind::String(string) => Some(string),
            _ => None,
        })
        .collect::<Vec<_>>();

    let decoded = strings[0].decoded.as_ref().unwrap();
    assert_eq!(decoded.text(), "physical\n\n");
    assert_eq!(
        decoded
            .chars
            .iter()
            .filter(|decoded| decoded.value == '\n')
            .map(|decoded| decoded.source_line)
            .collect::<Vec<_>>(),
        [2, 3]
    );

    assert_eq!(strings[1].source, r#""bad\q""#);
    let error = strings[1].decoded.as_ref().unwrap_err();
    assert_eq!(error.line, 4);
    assert_eq!(error.message, "unsupported manifest string escape `q`");
}
