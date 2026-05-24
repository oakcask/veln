use super::*;
use veln_source::SourceFile;

#[test]
fn parses_minimal_public_function() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> Result((), AppError) effects [stdio]\n  Ok(())\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tree.items.len(), 1);
    let SyntaxItem::Function(function) = &output.tree.items[0];
    assert_eq!(function.name.as_deref(), Some("main"));
    assert_eq!(
        function.effects.as_ref().unwrap(),
        &vec!["stdio".to_string()]
    );
    assert!(function.end_present);
}

#[test]
fn parses_explicit_test_declaration() {
    let source = SourceFile::new(
        "main_test.veln",
        "test returns_ok() -> Result((), String) effects []\n  Ok(())\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tree.items.len(), 1);
    let SyntaxItem::Function(function) = &output.tree.items[0];
    assert_eq!(function.kind, FunctionKind::Test);
    assert_eq!(function.visibility, Visibility::Private);
    assert_eq!(function.name.as_deref(), Some("returns_ok"));
    assert_eq!(
        format_tree(&output.tree),
        "test returns_ok() -> Result((), String) effects []\n  Ok(())\nend\n"
    );
}

#[test]
fn parses_omitted_signature_annotations_as_recoverable_ast_facts() {
    let source = SourceFile::new("main.veln", "fn helper(value)\n  value\nend\n");

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let SyntaxItem::Function(function) = &output.tree.items[0];
    assert_eq!(function.params[0].ty, None);
    assert_eq!(function.return_type, None);
    assert_eq!(function.effects, None);
}

#[test]
fn reports_missing_end() {
    let source = SourceFile::new("main.veln", "fn broken() -> ()\n  _\n");
    let output = parse(&source);

    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.expected_end")
    );
}

#[test]
fn lossless_tree_retains_trivia() {
    let source = SourceFile::new(
        "main.veln",
        "// module comment\nfn id(value: Int) -> Int\n  value // tail comment\nend\n",
    );

    let output = parse(&source);
    let tokens = output.tree.lossless_tokens().collect::<Vec<_>>();

    assert!(
        tokens
            .iter()
            .any(|token| token.kind == TokenKind::Comment && token.text == "// module comment")
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == TokenKind::Whitespace)
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == TokenKind::Comment && token.text == "// tail comment")
    );
    assert_eq!(output.tree.items.len(), 1);
}

#[test]
fn lossless_tree_groups_declarations_for_formatting() {
    let text = concat!(
        "mod app\n",
        "use stdio\n",
        "pub fn main() -> () effects [stdio]\n",
        "  require ready\n",
        "  let message = \"hello\"\n",
        "  stdio::println(message)\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", text);

    let output = parse(&source);
    let kinds = output
        .tree
        .descendant_nodes()
        .map(|node| node.kind)
        .collect::<Vec<_>>();
    let rendered = output
        .tree
        .lossless_tokens()
        .map(|token| token.text.as_str())
        .collect::<String>();

    assert_eq!(rendered, text);
    assert!(kinds.contains(&SyntaxNodeKind::ModuleDecl));
    assert!(kinds.contains(&SyntaxNodeKind::UseDecl));
    assert!(kinds.contains(&SyntaxNodeKind::FunctionDecl));
    assert!(kinds.contains(&SyntaxNodeKind::FunctionSignature));
    assert!(kinds.contains(&SyntaxNodeKind::ContractClause));
    assert!(kinds.contains(&SyntaxNodeKind::Body));
    assert!(kinds.contains(&SyntaxNodeKind::LetStatement));
    assert!(kinds.contains(&SyntaxNodeKind::ExprLine));
}

#[test]
fn parses_structured_calls_and_holes() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> ()\n  stdio::println(_message)\n  _\nend\n",
    );

    let output = parse(&source);
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };

    let ExprKind::Call { callee, args } = &expr.kind else {
        panic!("expected call expression");
    };
    assert!(matches!(
        &callee.kind,
        ExprKind::NamePath(segments) if segments == &vec!["stdio".to_string(), "println".to_string()]
    ));
    assert!(matches!(
        &args[0].kind,
        ExprKind::Hole {
            name: Some(name), ..
        } if name == "message"
    ));

    let BodyLine::Expr { expr, .. } = &function.body[1] else {
        panic!("expected expression line");
    };
    assert!(matches!(&expr.kind, ExprKind::Hole { name: None, .. }));
}

#[test]
fn lexes_number_string_hole_and_invalid_boundaries() {
    let source = SourceFile::new(
        "tokens.veln",
        r#"1 1.5 1.foo "a\"b" @ test _ _name
"#,
    );

    let lexed = lex(&source);
    let significant = lexed
        .tokens
        .iter()
        .filter(|token| token.kind != TokenKind::Whitespace)
        .map(|token| (token.kind.clone(), token.text.clone()))
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
            (TokenKind::Underscore, "_".to_string()),
            (TokenKind::Hole, "_name".to_string()),
            (TokenKind::Newline, "\n".to_string()),
            (TokenKind::Eof, String::new()),
        ]
    );
}

#[test]
fn parses_module_use_nested_types_and_multiple_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.core\n",
            "use platform.io\n",
            "fn collect(items: List(Result(Int, Error))) -> Result(List(Int), Error) effects [fs, net]\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tree.module.as_ref().unwrap().name, "app.core");
    assert_eq!(output.tree.uses[0].name, "platform.io");
    let SyntaxItem::Function(function) = &output.tree.items[0];
    assert_eq!(
        function.params[0].ty.as_deref(),
        Some("List(Result(Int, Error))")
    );
    assert_eq!(
        function.return_type.as_deref(),
        Some("Result(List(Int), Error)")
    );
    assert_eq!(
        function.effects.as_ref().unwrap(),
        &vec!["fs".to_string(), "net".to_string()]
    );
}

#[test]
fn formats_unit_type_with_empty_tuple_spelling() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: Unit) -> Result(Unit, AppError)\n",
            "  let ready: Unit = ()\n",
            "  Ok(ready)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn main(value: ()) -> Result((), AppError)\n",
            "  let ready: () = ()\n",
            "  Ok(ready)\n",
            "end\n",
        )
    );
}

#[test]
fn parses_hole_satisfy_clause() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose() -> ()\n",
            "  _value satisfy candidate => candidate > 0 and candidate < 10\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Hole {
        name,
        satisfy: Some(satisfy),
    } = &expr.kind
    else {
        panic!("expected hole with satisfy clause");
    };
    assert_eq!(name.as_deref(), Some("value"));
    assert_eq!(satisfy.candidate.as_deref(), Some("candidate"));
    assert_eq!(satisfy.predicate, "candidate > 0 and candidate < 10");
    assert_eq!(format_tree(&output.tree), source.text());
}

#[test]
fn parses_records_lists_and_formats_precedence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn data() -> ()\n",
            "  let record = { name: \"veln\", values: [1, 2 + 3 * 4] }\n",
            "  1 * (2 + 3)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(format_tree(&output.tree), source.text());
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Let { expr, .. } = &function.body[0] else {
        panic!("expected let statement");
    };
    let ExprKind::Record(fields) = &expr.kind else {
        panic!("expected record expression");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "name");
    assert_eq!(fields[1].name, "values");
    let ExprKind::List(items) = &fields[1].expr.kind else {
        panic!("expected list expression");
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(
        &items[1].kind,
        ExprKind::Binary {
            op: BinaryOp::Add,
            right,
            ..
        } if matches!(right.kind, ExprKind::Binary { op: BinaryOp::Multiply, .. })
    ));

    let BodyLine::Expr { expr, .. } = &function.body[1] else {
        panic!("expected expression line");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Binary {
            op: BinaryOp::Multiply,
            right,
            ..
        } if matches!(right.kind, ExprKind::Binary { op: BinaryOp::Add, .. })
    ));
}

#[test]
fn format_tree_preserves_commented_source_losslessly() {
    let source = SourceFile::new(
        "main.veln",
        "// header\nfn main() -> ()\n  _ // hole\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(format_tree(&output.tree), source.text());
}

#[test]
fn reports_invalid_expression_token_and_recovers_to_next_line() {
    let source = SourceFile::new("main.veln", "fn main() -> ()\n  @\n  1\nend\n");

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.invalid_token")
        .expect("expected invalid token diagnostic");
    assert_eq!(diagnostic.parser_context, "expression_line");
    assert_eq!(diagnostic.unexpected.text, "@");
    assert_eq!(diagnostic.recovery.strategy, RecoveryStrategy::SkipToken);
    assert_eq!(diagnostic.recovery.anchor.as_deref(), Some("newline"));
    assert_eq!(diagnostic.recovery.dropped_token_count, 1);

    let SyntaxItem::Function(function) = &output.tree.items[0];
    assert_eq!(function.body.len(), 2);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    assert!(matches!(expr.kind, ExprKind::Missing));
    let BodyLine::Expr { expr, .. } = &function.body[1] else {
        panic!("expected expression line");
    };
    assert!(matches!(expr.kind, ExprKind::IntLiteral(ref value) if value == "1"));
}

#[test]
fn synchronizes_top_level_garbage_to_next_function() {
    let source = SourceFile::new("main.veln", "let stray = 1\nfn main()\nend\n");

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.expected_item")
        .expect("expected top-level item diagnostic");
    assert_eq!(
        diagnostic.recovery.strategy,
        RecoveryStrategy::SynchronizeToAnchor
    );
    assert_eq!(diagnostic.recovery.anchor.as_deref(), Some("fn"));
    assert!(diagnostic.recovery.dropped_token_count > 0);
    assert_eq!(output.tree.items.len(), 1);
    let SyntaxItem::Function(function) = &output.tree.items[0];
    assert_eq!(function.name.as_deref(), Some("main"));
}

#[test]
fn synchronizes_top_level_garbage_to_next_test_declaration() {
    let source = SourceFile::new(
        "main.veln",
        "let stray = 1\ntest main() -> () effects []\nend\n",
    );

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.expected_item")
        .expect("expected top-level item diagnostic");
    assert_eq!(
        diagnostic.recovery.strategy,
        RecoveryStrategy::SynchronizeToAnchor
    );
    assert_eq!(diagnostic.recovery.anchor.as_deref(), Some("test"));
    assert_eq!(output.tree.items.len(), 1);
    let SyntaxItem::Function(function) = &output.tree.items[0];
    assert_eq!(function.kind, FunctionKind::Test);
    assert_eq!(function.name.as_deref(), Some("main"));
}
