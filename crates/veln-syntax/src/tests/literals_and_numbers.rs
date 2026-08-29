use super::*;

#[test]
fn lexes_binary_and_hexadecimal_integer_candidates_as_complete_tokens() {
    let source = SourceFile::new("main.veln", "0b00101 0x00Cafe 0b102 0Xff 0x1.2 0b10_01 0x");

    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Eof))
        .map(|token| (token.kind, token.text))
        .collect::<Vec<_>>();

    assert_eq!(
        tokens,
        vec![
            (TokenKind::Int, "0b00101".to_string()),
            (TokenKind::Int, "0x00Cafe".to_string()),
            (TokenKind::MalformedInt, "0b102".to_string()),
            (TokenKind::MalformedInt, "0Xff".to_string()),
            (TokenKind::MalformedInt, "0x1.2".to_string()),
            (TokenKind::MalformedInt, "0b10_01".to_string()),
            (TokenKind::MalformedInt, "0x".to_string()),
        ]
    );
}

#[test]
fn number_tokens_preserve_fraction_and_member_access_boundaries() {
    let source = SourceFile::new("numbers.veln", "42.5 42.member 0b10.1 0b10.member 0xCafe+1");

    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Eof))
        .map(|token| (token.kind, token.text))
        .collect::<Vec<_>>();

    assert_eq!(
        tokens,
        vec![
            (TokenKind::Float, "42.5".to_string()),
            (TokenKind::Int, "42".to_string()),
            (TokenKind::Dot, ".".to_string()),
            (TokenKind::Ident, "member".to_string()),
            (TokenKind::MalformedInt, "0b10.1".to_string()),
            (TokenKind::Int, "0b10".to_string()),
            (TokenKind::Dot, ".".to_string()),
            (TokenKind::Ident, "member".to_string()),
            (TokenKind::Int, "0xCafe".to_string()),
            (TokenKind::Plus, "+".to_string()),
            (TokenKind::Int, "1".to_string()),
        ]
    );
}

#[test]
fn lexes_compound_operators_with_longest_matching_tokens() {
    let source = SourceFile::new(
        "operators.veln",
        "-> => :: == != <= << >= >>> >> > |> | & ^ ~",
    );

    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Eof))
        .map(|token| (token.kind, token.text))
        .collect::<Vec<_>>();

    assert_eq!(
        tokens,
        vec![
            (TokenKind::Arrow, "->".to_string()),
            (TokenKind::FatArrow, "=>".to_string()),
            (TokenKind::DoubleColon, "::".to_string()),
            (TokenKind::EqualEqual, "==".to_string()),
            (TokenKind::BangEqual, "!=".to_string()),
            (TokenKind::LessEqual, "<=".to_string()),
            (TokenKind::ShiftLeft, "<<".to_string()),
            (TokenKind::GreaterEqual, ">=".to_string()),
            (TokenKind::ShiftRightLogical, ">>>".to_string()),
            (TokenKind::ShiftRight, ">>".to_string()),
            (TokenKind::Greater, ">".to_string()),
            (TokenKind::PipeGreater, "|>".to_string()),
            (TokenKind::Pipe, "|".to_string()),
            (TokenKind::Ampersand, "&".to_string()),
            (TokenKind::Caret, "^".to_string()),
            (TokenKind::Tilde, "~".to_string()),
        ]
    );
}

#[test]
fn nested_generic_closers_remain_type_syntax_next_to_shift_operators() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(values: List<Receiver<Int>>) -> Result<Option<Int>, String>\n",
            "  Ok(Some((8 >> 1) + (8 >>> 1)))\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        first_function(&output).params[0].ty.as_deref(),
        Some("List<Receiver<Int>>")
    );
}

#[test]
fn reports_one_focused_diagnostic_for_each_malformed_prefixed_integer() {
    let cases = [
        (
            "0b",
            "binary integer literal requires at least one digit",
            3,
            5,
        ),
        ("0b102", "`2` is not a valid binary integer digit", 7, 8),
        ("0xg1", "`g` is not a valid hexadecimal integer digit", 5, 6),
        (
            "0B10",
            "uppercase binary integer literal prefix is unsupported",
            4,
            5,
        ),
        (
            "0x1_0",
            "digit separators are not supported in hexadecimal integer literals",
            6,
            7,
        ),
        (
            "0b1.0",
            "binary floating-point literals are unsupported",
            3,
            8,
        ),
        (
            "0x8000000000000000",
            "hexadecimal integer literal exceeds the maximum Int value 9223372036854775807",
            3,
            21,
        ),
    ];

    for (literal, message, start_column, end_column) in cases {
        let source = SourceFile::new("main.veln", format!("fn main() -> Int\n  {literal}\nend\n"));
        let output = parse(&source);
        assert_eq!(
            output.diagnostics.len(),
            1,
            "{literal}: {:#?}",
            output.diagnostics
        );
        let diagnostic = &output.diagnostics[0];
        assert_eq!(diagnostic.id, "parse.integer_literal", "{literal}");
        assert_eq!(diagnostic.message, message, "{literal}");
        let span = diagnostic.span.as_ref().unwrap();
        assert_eq!(
            (span.start.line, span.start.column, span.end.column),
            (2, start_column, end_column),
            "{literal}"
        );
    }
}

#[test]
fn formatter_preserves_prefixed_integer_spelling_in_expressions_and_patterns() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn classify(value: Int) -> Int\n",
            "match value\n",
            "0x0A=>0b001010\n",
            "_=>0xCafe\n",
            "end\n",
            "end\n",
        ),
    );
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn classify(value: Int) -> Int\n",
            "\tmatch value\n",
            "\t\t0x0A => 0b001010\n",
            "\t\t_ => 0xCafe\n",
            "\tend\n",
            "end\n",
        )
    );
}
