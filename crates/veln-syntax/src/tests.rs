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
fn parses_wildcard_let_without_binding_a_name() {
    let source = SourceFile::new(
        "main.veln",
        "fn discard(value: Int) -> ()\n  let _: Int = value\n  ()\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(format_tree(&output.tree), source.text());
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Let {
        pattern,
        annotation,
        ..
    } = &function.body[0]
    else {
        panic!("first body line should be a let statement");
    };
    assert!(matches!(pattern.kind, PatternKind::Wildcard));
    assert_eq!(annotation.as_deref(), Some("Int"));
}

#[test]
fn parses_record_let_pattern() {
    let source = SourceFile::new(
        "main.veln",
        "fn unpack(value: {count: Int}) -> Int\n  let {count: amount}: {count: Int} = value\n  amount\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        "fn unpack(value: { count : Int }) -> Int\n  let { count: amount }: { count : Int } = value\n  amount\nend\n"
    );
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Let {
        pattern,
        annotation,
        ..
    } = &function.body[0]
    else {
        panic!("first body line should be a let statement");
    };
    let PatternKind::Record(fields) = &pattern.kind else {
        panic!("let pattern should be a record pattern");
    };
    assert_eq!(fields[0].name, "count");
    assert!(matches!(
        fields[0].pattern.kind,
        PatternKind::Binding(ref name) if name == "amount"
    ));
    assert_eq!(annotation.as_deref(), Some("{ count : Int }"));
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
fn parses_adr_lite_records_from_doc_comments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "/// @adr\n",
            "/// id: order-summary\n",
            "/// status: accepted\n",
            "/// scope: pub fn summarize\n",
            "/// context: Summaries need source-adjacent rationale.\n",
            "/// decision: Keep the public API pure.\n",
            "/// consequences: Runtime behavior ignores this record.\n",
            "pub fn summarize() -> () effects []\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tree.adr_lite_records.len(), 1);
    let record = &output.tree.adr_lite_records[0];
    assert_eq!(record.id, "order-summary");
    assert_eq!(record.status, "accepted");
    assert_eq!(record.scope, "pub fn summarize");
    assert_eq!(
        record.anchor,
        Some(AdrLiteAnchor::Function {
            name: "summarize".to_string()
        })
    );
    assert_eq!(format_tree(&output.tree), source.text());
}

#[test]
fn anchors_adr_lite_records_to_modules() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "/// @adr-lite\n",
            "/// id: module-boundary\n",
            "/// status: accepted\n",
            "/// scope: module\n",
            "/// context: Module identity is compiler-visible.\n",
            "/// decision: Keep the source header canonical.\n",
            "/// consequences: Manifest metadata cannot rename the module.\n",
            "mod app.core\n",
            "fn helper() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        output.tree.adr_lite_records[0].anchor,
        Some(AdrLiteAnchor::Module {
            name: "app.core".to_string()
        })
    );
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
        (TokenKind::Test, "test"),
        (TokenKind::Effects, "effects"),
        (TokenKind::Let, "let"),
        (TokenKind::End, "end"),
        (TokenKind::Require, "require"),
        (TokenKind::Ensure, "ensure"),
        (TokenKind::Mod, "mod"),
        (TokenKind::Use, "use"),
        (TokenKind::Match, "match"),
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
fn parses_and_formats_result_binding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn clamp(value: Int) -> output: Int\n",
            "ensure output >= value\n",
            "  value\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let SyntaxItem::Function(function) = &output.tree.items[0];
    assert_eq!(
        function
            .return_binding
            .as_ref()
            .map(|binding| binding.name.as_str()),
        Some("output")
    );
    assert_eq!(function.return_type.as_deref(), Some("Int"));
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn clamp(value: Int) -> output: Int\n",
            "  ensure output >= value\n",
            "  value\n",
            "end\n",
        )
    );
}

#[test]
fn parses_contract_predicate_subset() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn clamp(value: Int, limit: Int) -> output: Int\n",
            "require value >= 0 and value <= limit\n",
            "ensure output.total == value + limit\n",
            "  value\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let SyntaxItem::Function(function) = &output.tree.items[0];
    assert_eq!(function.contracts.len(), 2);
    assert_eq!(function.contracts[0].text, "value >= 0 and value <= limit");
    assert_eq!(function.contracts[1].text, "output.total == value + limit");
}

#[test]
fn rejects_non_predicate_contract_syntax() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn bad(value: Int) -> Int\n",
            "require _missing\n",
            "ensure [value]\n",
            "  value\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.contract_predicate"
            && diagnostic.message == "hole syntax is not allowed in a contract predicate"
            && diagnostic.parser_context == "contract_predicate"
    }));
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.contract_predicate"
            && diagnostic.message == "list syntax is not allowed in a contract predicate"
            && diagnostic.parser_context == "contract_predicate"
    }));
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
fn reports_malformed_hole_satisfy_clause() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> ()\n",
            "  _first satisfy => candidate > 0\n",
            "  _second satisfy candidate candidate > 0\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.satisfy_candidate"
            && diagnostic.message == "satisfy clause is missing a candidate binding"
            && diagnostic.expected == vec!["candidate binding"]
            && diagnostic.recovery.anchor.as_deref() == Some("=>")
    }));
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.satisfy_arrow"
            && diagnostic.message == "satisfy clause is missing `=>`"
            && diagnostic.expected == vec!["=>"]
    }));
}

#[test]
fn rejects_non_predicate_satisfy_syntax() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose() -> Int\n",
            "  _value satisfy candidate => candidate |> valid\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "parse.satisfy_predicate"
            && diagnostic.message == "pipeline syntax is not allowed in a contract predicate"
            && diagnostic.parser_context == "satisfy_predicate"
    }));
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
fn parses_boolean_literals_as_literals() {
    let source = SourceFile::new(
        "main.veln",
        "fn main(flag: Bool) -> Bool\n  true and false or flag\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(format_tree(&output.tree), source.text());
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Binary { left, right, .. } = &expr.kind else {
        panic!("expected boolean binary expression");
    };
    assert!(
        matches!(&right.kind, ExprKind::NamePath(segments) if segments == &vec!["flag".to_string()])
    );
    let ExprKind::Binary { left, right, .. } = &left.kind else {
        panic!("expected nested boolean binary expression");
    };
    assert!(matches!(left.kind, ExprKind::BoolLiteral(true)));
    assert!(matches!(right.kind, ExprKind::BoolLiteral(false)));
}

#[test]
fn parses_boolean_literals_as_patterns() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(flag: Bool) -> String\n",
            "  match flag\n",
            "    true => \"yes\"\n",
            "    false => \"no\"\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(format_tree(&output.tree), source.text());
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Match { arms, .. } = &expr.kind else {
        panic!("expected match expression");
    };
    assert!(matches!(
        arms[0].pattern.kind,
        PatternKind::BoolLiteral(true)
    ));
    assert!(matches!(
        arms[1].pattern.kind,
        PatternKind::BoolLiteral(false)
    ));
}

#[test]
fn parses_dictionary_literals_with_expression_keys() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Dict(String, Int)\n  {\"one\": 1, \"two\": 2}\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        format_tree(&output.tree),
        "fn main() -> Dict(String, Int)\n  { \"one\": 1, \"two\": 2 }\nend\n"
    );
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Dict(entries) = &expr.kind else {
        panic!("expected dictionary expression");
    };
    assert_eq!(entries.len(), 2);
    assert!(matches!(&entries[0].key.kind, ExprKind::StringLiteral(value) if value == "\"one\""));
    assert!(matches!(&entries[0].value.kind, ExprKind::IntLiteral(value) if value == "1"));
}

#[test]
fn parses_dictionary_literals_with_identifier_led_expression_keys() {
    let source = SourceFile::new(
        "main.veln",
        "fn main(seed: Int) -> Dict(Int, String)\n  {seed + 1: \"next\"}\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        format_tree(&output.tree),
        "fn main(seed: Int) -> Dict(Int, String)\n  { seed + 1: \"next\" }\nend\n"
    );
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Dict(entries) = &expr.kind else {
        panic!("expected dictionary expression");
    };
    assert_eq!(entries.len(), 1);
    assert!(matches!(
        &entries[0].key.kind,
        ExprKind::Binary {
            op: BinaryOp::Add,
            ..
        }
    ));
    assert!(matches!(
        &entries[0].value.kind,
        ExprKind::StringLiteral(value) if value == "\"next\""
    ));
}

#[test]
fn parses_newlines_inside_grouped_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn data() -> ()\n",
            "  let record = {\n",
            "    name: \"veln\",\n",
            "    values: [\n",
            "      1,\n",
            "      add(\n",
            "        2,\n",
            "        3,\n",
            "      ),\n",
            "    ],\n",
            "  }\n",
            "  record\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn data() -> ()\n",
            "  let record = { name: \"veln\", values: [1, add(2, 3)] }\n",
            "  record\n",
            "end\n",
        )
    );
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Let { expr, .. } = &function.body[0] else {
        panic!("expected let statement");
    };
    let ExprKind::Record(fields) = &expr.kind else {
        panic!("expected record expression");
    };
    assert_eq!(fields.len(), 2);
    let ExprKind::List(items) = &fields[1].expr.kind else {
        panic!("expected list expression");
    };
    assert_eq!(items.len(), 2);
    assert!(matches!(&items[1].kind, ExprKind::Call { args, .. } if args.len() == 2));
}

#[test]
fn parses_field_access_as_postfix_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn data() -> ()\n",
            "  let count = { nested: { count: 1 } }.nested.count\n",
            "  count\n",
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
    let ExprKind::FieldAccess { base, field, .. } = &expr.kind else {
        panic!("expected field access expression");
    };
    assert_eq!(field, "count");
    assert!(matches!(
        &base.kind,
        ExprKind::FieldAccess { field, .. } if field == "nested"
    ));
}

#[test]
fn rejects_method_call_shaped_syntax() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: {name: String}) -> ()\n",
            "  value.name()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.method_call")
        .expect("expected method-call diagnostic");
    assert_eq!(diagnostic.message, "method-call syntax is not implemented");
    assert_eq!(diagnostic.parser_context, "expression_line");
    assert_eq!(diagnostic.expected, vec!["function call or field access"]);
    assert_eq!(diagnostic.unexpected.text, "(");
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

#[test]
fn parses_and_formats_match_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: Option(Int)) -> String effects []\n",
            "  match value\n",
            "    Some(count) => \"some\"\n",
            "    None => \"none\"\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Match { scrutinee, arms } = &expr.kind else {
        panic!("expected match expression");
    };
    assert!(matches!(
        &scrutinee.kind,
        ExprKind::NamePath(segments) if segments == &vec!["value".to_string()]
    ));
    assert_eq!(arms.len(), 2);
    assert_eq!(format_tree(&output.tree), source.text());
}

#[test]
fn parses_match_expression_inside_call_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: Option(Int)) -> String effects []\n",
            "  wrap(match value\n",
            "    Some(count) => \"some\"\n",
            "    None => \"none\"\n",
            "  end)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Call { args, .. } = &expr.kind else {
        panic!("expected call expression");
    };
    assert!(matches!(args[0].kind, ExprKind::Match { .. }));
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn describe(value: Option(Int)) -> String effects []\n",
            "  wrap(match value\n",
            "    Some(count) => \"some\"\n",
            "    None => \"none\"\n",
            "  end)\n",
            "end\n",
        )
    );
}

#[test]
fn parses_match_expression_inside_aggregate_literals() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: Option(Int)) -> {labels: [String], primary: String} effects []\n",
            "  {labels: [match value\n",
            "    Some(count) => \"some\"\n",
            "    None => \"none\"\n",
            "  end], primary: match value\n",
            "    Some(count) => \"some\"\n",
            "    None => \"none\"\n",
            "  end}\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Record(fields) = &expr.kind else {
        panic!("expected record expression");
    };
    let ExprKind::List(items) = &fields[0].expr.kind else {
        panic!("expected list field");
    };
    assert!(matches!(items[0].kind, ExprKind::Match { .. }));
    assert!(matches!(fields[1].expr.kind, ExprKind::Match { .. }));
}

#[test]
fn parses_and_formats_qualified_builtin_constructors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: Result(Option(Int), String)) -> Result(String, String) effects []\n",
            "  match value\n",
            "    Result::Ok(Option::Some(count)) => Result::Ok(\"some\")\n",
            "    Result::Ok(Option::None) => Result::Ok(\"none\")\n",
            "    Result::Err(error) => Result::Err(error)\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Match { arms, .. } = &expr.kind else {
        panic!("expected match expression");
    };
    assert_eq!(arms.len(), 3);
    assert!(matches!(
        &arms[0].pattern.kind,
        PatternKind::Constructor { name, args } if name == &vec!["Result".to_string(), "Ok".to_string()]
            && matches!(
                &args[0].kind,
                PatternKind::Constructor { name, .. } if name == &vec!["Option".to_string(), "Some".to_string()]
            )
    ));
    let ExprKind::Call { callee, .. } = &arms[0].expr.kind else {
        panic!("expected constructor call");
    };
    assert!(matches!(
        &callee.kind,
        ExprKind::NamePath(segments) if segments == &vec!["Result".to_string(), "Ok".to_string()]
    ));
    assert_eq!(format_tree(&output.tree), source.text());
}

#[test]
fn parses_and_formats_record_patterns() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: {count: Int, label: String}) -> String effects []\n",
            "  match value\n",
            "    {count: 0, label: name} => name\n",
            "    {count: count, label: _} => \"many\"\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn describe(value: { count : Int, label : String }) -> String effects []\n",
            "  match value\n",
            "    { count: 0, label: name } => name\n",
            "    { count: count, label: _ } => \"many\"\n",
            "  end\n",
            "end\n",
        )
    );
    let SyntaxItem::Function(function) = &output.tree.items[0];
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Match { arms, .. } = &expr.kind else {
        panic!("expected match expression");
    };
    let PatternKind::Record(fields) = &arms[0].pattern.kind else {
        panic!("expected record pattern");
    };
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "count");
    assert!(matches!(
        &fields[0].pattern.kind,
        PatternKind::IntLiteral(value) if value == "0"
    ));
    assert!(matches!(
        &fields[1].pattern.kind,
        PatternKind::Binding(name) if name == "name"
    ));
}
