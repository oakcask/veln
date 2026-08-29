use super::*;

#[test]
fn lexes_number_string_hole_and_invalid_boundaries() {
    let source = SourceFile::new(
        "tokens.veln",
        r#"1 1.5 1.foo "a\"b" @ test where if else at _ _name
"#,
    );

    let lexed = lex(&source);
    let significant = lexed
        .tokens
        .iter()
        .filter(|token| token.kind != TokenKind::Whitespace)
        .map(|token| (token.kind, token.text.clone()))
        .collect::<Vec<_>>();

    assert_eq!(
        significant,
        vec![
            (TokenKind::Int, "1".to_string()),
            (TokenKind::Float, "1.5".to_string()),
            (TokenKind::Int, "1".to_string()),
            (TokenKind::Dot, ".".to_string()),
            (TokenKind::Ident, "foo".to_string()),
            (TokenKind::String, r#""a\"b""#.to_string()),
            (TokenKind::Invalid, "@".to_string()),
            (TokenKind::Test, "test".to_string()),
            (TokenKind::Where, "where".to_string()),
            (TokenKind::If, "if".to_string()),
            (TokenKind::Else, "else".to_string()),
            (TokenKind::At, "at".to_string()),
            (TokenKind::Underscore, "_".to_string()),
            (TokenKind::Hole, "_name".to_string()),
            (TokenKind::Newline, "\n".to_string()),
            (TokenKind::Eof, String::new()),
        ]
    );
}

#[test]
fn token_kind_labels_cover_every_surface_token() {
    let cases = [
        (TokenKind::Whitespace, "whitespace"),
        (TokenKind::Comment, "comment"),
        (TokenKind::Ident, "identifier"),
        (TokenKind::Hole, "hole"),
        (TokenKind::String, "string"),
        (TokenKind::Int, "integer"),
        (TokenKind::Float, "float"),
        (TokenKind::Newline, "newline"),
        (TokenKind::Eof, "end of file"),
        (TokenKind::Invalid, "invalid token"),
        (TokenKind::Pub, "pub"),
        (TokenKind::Fn, "fn"),
        (TokenKind::Type, "type"),
        (TokenKind::Schema, "schema"),
        (TokenKind::Format, "format"),
        (TokenKind::Where, "where"),
        (TokenKind::Test, "test"),
        (TokenKind::Effects, "effects"),
        (TokenKind::Let, "let"),
        (TokenKind::End, "end"),
        (TokenKind::Require, "require"),
        (TokenKind::Ensure, "ensure"),
        (TokenKind::Invariant, "invariant"),
        (TokenKind::Mod, "mod"),
        (TokenKind::Use, "use"),
        (TokenKind::From, "from"),
        (TokenKind::At, "at"),
        (TokenKind::Match, "match"),
        (TokenKind::If, "if"),
        (TokenKind::Else, "else"),
        (TokenKind::Or, "or"),
        (TokenKind::And, "and"),
        (TokenKind::Not, "not"),
        (TokenKind::LParen, "("),
        (TokenKind::RParen, ")"),
        (TokenKind::LBracket, "["),
        (TokenKind::RBracket, "]"),
        (TokenKind::LBrace, "{"),
        (TokenKind::RBrace, "}"),
        (TokenKind::Comma, ","),
        (TokenKind::Colon, ":"),
        (TokenKind::Dot, "."),
        (TokenKind::DoubleColon, "::"),
        (TokenKind::Arrow, "->"),
        (TokenKind::FatArrow, "=>"),
        (TokenKind::PipeGreater, "|>"),
        (TokenKind::Question, "?"),
        (TokenKind::Underscore, "_"),
        (TokenKind::Equal, "="),
        (TokenKind::EqualEqual, "=="),
        (TokenKind::BangEqual, "!="),
        (TokenKind::Less, "<"),
        (TokenKind::LessEqual, "<="),
        (TokenKind::Greater, ">"),
        (TokenKind::GreaterEqual, ">="),
        (TokenKind::Plus, "+"),
        (TokenKind::Minus, "-"),
        (TokenKind::Star, "*"),
        (TokenKind::Slash, "/"),
    ];

    for (kind, label) in cases {
        assert_eq!(kind.label(), label);
    }
}

#[test]
fn accepted_source_surface_fixtures_parse_without_diagnostics() {
    for fixture in source_surface_fixtures("accepted") {
        let text = fs::read_to_string(&fixture).expect("fixture should be readable");
        let source = SourceFile::new(source_surface_fixture_name(&fixture), text);

        let output = parse(&source);

        assert!(
            output.diagnostics.is_empty(),
            "{} should parse without diagnostics: {:#?}",
            fixture.display(),
            output.diagnostics
        );
    }
}

#[test]
fn rejected_source_surface_fixtures_produce_diagnostics() {
    for fixture in source_surface_fixtures("rejected") {
        let text = fs::read_to_string(&fixture).expect("fixture should be readable");
        let source = SourceFile::new(source_surface_fixture_name(&fixture), text);

        let output = parse(&source);

        assert!(
            !output.diagnostics.is_empty(),
            "{} should produce at least one parse diagnostic",
            fixture.display()
        );
    }
}

fn source_surface_fixtures(outcome: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/specification/source-surface-fixtures")
        .join(outcome);
    let mut fixtures = fs::read_dir(&root)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", root.display()))
        .map(|entry| entry.expect("fixture entry should be readable").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "veln")
        })
        .collect::<Vec<_>>();
    fixtures.sort();
    assert!(
        !fixtures.is_empty(),
        "source-surface {outcome} fixtures should not be empty"
    );
    fixtures
}

fn source_surface_fixture_name(path: &Path) -> String {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    path.strip_prefix(&root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
