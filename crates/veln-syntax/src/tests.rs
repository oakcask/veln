use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use veln_source::SourceFile;

fn first_function(output: &ParseOutput) -> &FunctionDecl {
    match &output.tree.items[0] {
        SyntaxItem::Function(function) => function,
        SyntaxItem::Type(_) | SyntaxItem::PublicAlias(_) => panic!("expected function item"),
    }
}

#[test]
fn parses_minimal_public_function() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> Result<(), AppError> effects [stdio]\n  Ok(())\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tree.items.len(), 1);
    let function = first_function(&output);
    assert_eq!(function.name.as_deref(), Some("main"));
    assert_eq!(
        function.effects.as_ref().unwrap(),
        &vec!["stdio".to_string()]
    );
    assert!(function.end_present);
}

#[test]
fn parses_public_member_aliases() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "use spec.impl\n",
            "\n",
            "pub fn parse = impl::parse\n",
            "pub type Document = impl::Document\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tree.items.len(), 2);
    let SyntaxItem::PublicAlias(function_alias) = &output.tree.items[0] else {
        panic!("expected function alias");
    };
    assert_eq!(function_alias.kind, PublicAliasKind::Function);
    assert_eq!(function_alias.name.as_deref(), Some("parse"));
    assert_eq!(function_alias.target, vec!["impl", "parse"]);
    let SyntaxItem::PublicAlias(type_alias) = &output.tree.items[1] else {
        panic!("expected type alias");
    };
    assert_eq!(type_alias.kind, PublicAliasKind::Type);
    assert_eq!(type_alias.name.as_deref(), Some("Document"));
    assert_eq!(type_alias.target, vec!["impl", "Document"]);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "mod spec.api\n",
            "use spec.impl\n",
            "\n",
            "pub fn parse = impl::parse\n",
            "\n",
            "pub type Document = impl::Document\n",
        )
    );
}

#[test]
fn rejects_public_member_alias_call_targets() {
    let source = SourceFile::new(
        "api.veln",
        concat!("mod spec.api\n", "pub fn parse = impl::parse()\n"),
    );

    let output = parse(&source);

    assert!(
        output.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "parse.expected_newline"
                && diagnostic.message.contains("expected a newline")
        }),
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn rejects_public_member_alias_signatures() {
    let function_source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "pub fn parse(input: String) = impl::parse\n",
        ),
    );

    let function_output = parse(&function_source);

    assert!(
        function_output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.expected_newline"),
        "{:#?}",
        function_output.diagnostics
    );

    let type_source = SourceFile::new(
        "api.veln",
        concat!("mod spec.api\n", "pub type Document<T> = impl::Document\n"),
    );

    let type_output = parse(&type_source);

    assert!(
        type_output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.expected_newline"),
        "{:#?}",
        type_output.diagnostics
    );
}

#[test]
fn parses_explicit_test_declaration() {
    let source = SourceFile::new(
        "main_test.veln",
        "test returns_ok() -> Result<(), String>\n\tOk(())\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tree.items.len(), 1);
    let function = first_function(&output);
    assert_eq!(function.kind, FunctionKind::Test);
    assert_eq!(function.visibility, Visibility::Private);
    assert_eq!(function.name.as_deref(), Some("returns_ok"));
    assert_eq!(
        format_tree(&output.tree),
        "test returns_ok() -> Result<(), String>\n\tOk(())\nend\n"
    );
}

#[test]
fn parses_minimal_list_type_declaration() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "\tNil\n",
            "\tCons(head: A, tail: List<A>)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tree.items.len(), 1);
    let SyntaxItem::Type(list) = &output.tree.items[0] else {
        panic!("expected type declaration");
    };
    assert_eq!(list.name.as_deref(), Some("List"));
    assert_eq!(list.params, vec!["A"]);
    assert_eq!(list.variants.len(), 2);
    assert_eq!(list.variants[0].name.as_deref(), Some("Nil"));
    assert!(list.variants[0].fields.is_empty());
    assert_eq!(list.variants[1].name.as_deref(), Some("Cons"));
    assert_eq!(list.variants[1].fields[0].name, "head");
    assert_eq!(list.variants[1].fields[0].ty, "A");
    assert_eq!(list.variants[1].fields[1].name, "tail");
    assert_eq!(list.variants[1].fields[1].ty, "List<A>");
    assert_eq!(
        format_tree(&output.tree),
        "type List<A>\n\tNil\n\tCons(head: A, tail: List<A>)\nend\n"
    );
}

#[test]
fn parses_angle_bracket_type_parameters_and_annotations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Envelope<A, E>\n",
            "\tOk(A)\n",
            "\tErr(E)\n",
            "end\n",
            "\n",
            "fn nested(value: domain::Envelope<String, Result<Int, AppError>>) -> Bool\n",
            "\t1 < 2\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tree.items.len(), 2);
    let SyntaxItem::Type(envelope) = &output.tree.items[0] else {
        panic!("expected type declaration");
    };
    assert_eq!(envelope.name.as_deref(), Some("Envelope"));
    assert_eq!(envelope.params, vec!["A", "E"]);
    assert_eq!(envelope.variants[0].fields[0].ty, "A");
    let SyntaxItem::Function(function) = &output.tree.items[1] else {
        panic!("expected function declaration");
    };
    assert_eq!(
        function.params[0].ty.as_deref(),
        Some("domain::Envelope<String, Result<Int, AppError>>")
    );
}

#[test]
fn parses_public_type_declaration_with_public_record_variant() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub type Shape\n",
            "\tpub Circle { radius: Int }\n",
            "\tRectangle { width: Int, height: Int }\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Type(shape) = &output.tree.items[0] else {
        panic!("expected type declaration");
    };
    assert_eq!(shape.visibility, Visibility::Public);
    assert_eq!(shape.variants[0].visibility, Visibility::Public);
    assert_eq!(shape.variants[0].fields[0].name, "radius");
    assert_eq!(shape.variants[0].fields[0].ty, "Int");
    assert_eq!(shape.variants[1].visibility, Visibility::Private);
    assert_eq!(shape.variants[1].fields[1].name, "height");
}

#[test]
fn parses_omitted_signature_annotations_as_recoverable_ast_facts() {
    let source = SourceFile::new("main.veln", "fn helper(value)\n  value\nend\n");

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let function = first_function(&output);
    assert_eq!(function.params[0].ty, None);
    assert_eq!(function.return_type, None);
    assert_eq!(function.effects, None);
}

#[test]
fn parses_wildcard_let_without_binding_a_name() {
    let source = SourceFile::new(
        "main.veln",
        "fn discard(value: Int) -> ()\n\tlet _: Int = value\n\t()\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(format_tree(&output.tree), source.text());
    let function = first_function(&output);
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
        "fn unpack(value: {count: Int}) -> Int\n\tlet {count: amount}: {count: Int} = value\n\tamount\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        "fn unpack(value: { count : Int }) -> Int\n\tlet { count: amount }: { count : Int } = value\n\tamount\nend\n"
    );
    let function = first_function(&output);
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
fn lexer_keeps_slash_slash_as_division_tokens() {
    let source = SourceFile::new(
        "main.veln",
        "// module comment\nfn id(value: Int) -> Int\n  value // tail text\nend\n",
    );

    let lexed = lex(&source);
    let slash_tokens = lexed
        .tokens
        .iter()
        .filter(|token| token.kind == TokenKind::Slash)
        .count();

    assert_eq!(slash_tokens, 4);
    assert!(
        !lexed
            .tokens
            .iter()
            .any(|token| token.kind == TokenKind::Comment)
    );
}

#[test]
fn lossless_tree_retains_hash_line_comments() {
    let source = SourceFile::new(
        "main.veln",
        "# module comment\nfn id(value: Int) -> Int\n  value # tail comment\nend\n",
    );

    let output = parse(&source);
    let tokens = output.tree.lossless_tokens().collect::<Vec<_>>();

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == TokenKind::Comment && token.text == "# module comment")
    );
    assert!(
        tokens
            .iter()
            .any(|token| token.kind == TokenKind::Comment && token.text == "# tail comment")
    );
    assert_eq!(output.tree.items.len(), 1);
}

#[test]
fn slash_doc_comments_do_not_create_adr_lite_records() {
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
            "pub fn summarize() -> ()\n",
            "\t()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.tree.adr_lite_records.is_empty());
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.expected_item")
    );
}

#[test]
fn parses_adr_lite_records_from_hash_doc_comments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "## @adr\n",
            "## id: order-summary\n",
            "## status: accepted\n",
            "## scope: pub fn summarize\n",
            "## context: Summaries need source-adjacent rationale.\n",
            "## decision: Keep the public API pure.\n",
            "## consequences: Runtime behavior ignores this record.\n",
            "pub fn summarize() -> ()\n",
            "\t()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tree.adr_lite_records.len(), 1);
    let record = &output.tree.adr_lite_records[0];
    assert_eq!(record.id, "order-summary");
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
            "## @adr-lite\n",
            "## id: module-boundary\n",
            "## status: accepted\n",
            "## scope: module\n",
            "## context: Module identity is compiler-visible.\n",
            "## decision: Keep the source header canonical.\n",
            "## consequences: Manifest metadata cannot rename the module.\n",
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
    let function = first_function(&output);
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
fn parses_type_argument_call_callees() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> ()\n  channel::bounded<String>(1)\nend\n",
    );

    let output = parse(&source);
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };

    let ExprKind::Call { callee, args } = &expr.kind else {
        panic!("expected call expression");
    };
    assert_eq!(args.len(), 1);
    let ExprKind::TypeApply { callee, type_args } = &callee.kind else {
        panic!("expected type-applied callee");
    };
    assert_eq!(type_args, &vec!["String".to_string()]);
    assert!(matches!(
        &callee.kind,
        ExprKind::NamePath(segments) if segments == &vec!["channel".to_string(), "bounded".to_string()]
    ));
    assert_eq!(
        format_tree(&output.tree),
        "fn main() -> ()\n\tchannel::bounded<String>(1)\nend\n"
    );
}

#[test]
fn parses_task_spawn_type_argument_call_callee() {
    let source = SourceFile::new(
        "main.veln",
        "fn main(job: fn() -> String effects [concurrency]) -> ()\n  task::spawn<String>(job)\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Call { callee, args } = &expr.kind else {
        panic!("expected call expression");
    };
    assert_eq!(args.len(), 1);
    let ExprKind::TypeApply { callee, type_args } = &callee.kind else {
        panic!("expected type-applied callee");
    };
    assert_eq!(type_args, &vec!["String".to_string()]);
    assert!(matches!(
        &callee.kind,
        ExprKind::NamePath(segments) if segments == &vec!["task".to_string(), "spawn".to_string()]
    ));
}

#[test]
fn parses_angle_type_argument_call_without_hiding_comparisons() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn compare(value: Int, limit: Int) -> Bool\n",
            "\tvalue < limit\n",
            "end\n",
            "\n",
            "fn make() -> ()\n",
            "\tchannel::bounded<Result<String, AppError>>(1)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let compare = first_function(&output);
    let BodyLine::Expr { expr, .. } = &compare.body[0] else {
        panic!("expected expression line");
    };
    assert!(matches!(
        expr.kind,
        ExprKind::Binary {
            op: BinaryOp::Less,
            ..
        }
    ));
    let SyntaxItem::Function(make) = &output.tree.items[1] else {
        panic!("expected function declaration");
    };
    let BodyLine::Expr { expr, .. } = &make.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Call { callee, .. } = &expr.kind else {
        panic!("expected call expression");
    };
    let ExprKind::TypeApply { type_args, .. } = &callee.kind else {
        panic!("expected type-applied callee");
    };
    assert_eq!(type_args, &vec!["Result<String,AppError>".to_string()]);
}

#[test]
fn rejects_legacy_square_type_argument_call_callees() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> ()\n  channel::bounded[String](1)\nend\n",
    );

    let output = parse(&source);

    assert_eq!(output.diagnostics.len(), 1);
    let diagnostic = &output.diagnostics[0];
    assert_eq!(diagnostic.id, "parse.expected_newline");
    assert_eq!(diagnostic.span.as_ref().unwrap().start.column, 19);
    assert!(diagnostic.repair_candidates.is_empty());
}

#[test]
fn rejects_legacy_parenthesized_type_parameters() {
    let source = SourceFile::new("main.veln", "type Box(A)\n  Wrap(A)\nend\n");

    let output = parse(&source);

    assert_eq!(output.diagnostics.len(), 1);
    let diagnostic = &output.diagnostics[0];
    assert_eq!(diagnostic.id, "parse.expected_newline");
    assert_eq!(diagnostic.span.as_ref().unwrap().start.column, 9);
    assert!(diagnostic.repair_candidates.is_empty());
}

#[test]
fn rejects_legacy_parenthesized_type_arguments() {
    let source = SourceFile::new(
        "main.veln",
        "fn make(value: Result(Int, String)) -> Box(Result(Int, String))\n  _\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(make) = &output.tree.items[0] else {
        panic!("expected function declaration");
    };
    assert_eq!(make.params[0].ty.as_deref(), Some("Result(Int, String)"));
    assert_eq!(
        make.return_type.as_deref(),
        Some("Box(Result(Int, String))")
    );
}

#[test]
fn reports_missing_separator_between_call_arguments() {
    let source = SourceFile::new("main.veln", "fn main() -> ()\n  pair(1 2)\nend\n");

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.call_argument")
        .expect("expected missing call argument separator diagnostic");
    assert_eq!(diagnostic.message, "call argument is missing `,` or `)`");
    assert_eq!(diagnostic.parser_context, "expression_line");
    assert_eq!(diagnostic.expected, vec![",", ")"]);
    assert_eq!(diagnostic.recovery.strategy, RecoveryStrategy::InsertToken);
    assert_eq!(diagnostic.recovery.anchor.as_deref(), Some(","));

    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Call { args, .. } = &expr.kind else {
        panic!("expected call expression");
    };
    assert_eq!(args.len(), 2);
}

#[test]
fn reports_missing_newline_between_body_expressions() {
    let source = SourceFile::new("main.veln", "fn main() -> ()\n  1 2\nend\n");

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.expected_newline")
        .expect("expected missing newline diagnostic");
    assert_eq!(diagnostic.message, "expected a newline before this token");
    assert_eq!(diagnostic.parser_context, "expression_line");
    assert_eq!(diagnostic.unexpected.text, "2");
    assert_eq!(diagnostic.expected, vec!["newline"]);
    assert_eq!(diagnostic.recovery.strategy, RecoveryStrategy::InsertToken);
    assert_eq!(diagnostic.recovery.anchor.as_deref(), Some("newline"));

    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    assert!(matches!(expr.kind, ExprKind::IntLiteral(ref value) if value == "1"));
}

#[test]
fn reports_extra_tokens_after_let_pattern() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Int\n  let value extra = 1\n  value\nend\n",
    );

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.pattern")
        .expect("expected trailing pattern token diagnostic");
    assert_eq!(
        diagnostic.message,
        "expected the pattern to end before this token"
    );
    assert_eq!(diagnostic.parser_context, "let_statement");
    assert_eq!(diagnostic.unexpected.text, "extra");
    assert_eq!(diagnostic.expected, vec!["pattern end"]);
    assert_eq!(diagnostic.recovery.strategy, RecoveryStrategy::InsertToken);

    let function = first_function(&output);
    let BodyLine::Let { pattern, .. } = &function.body[0] else {
        panic!("expected let statement");
    };
    assert!(matches!(pattern.kind, PatternKind::Binding(ref name) if name == "value"));
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
        (TokenKind::Invariant, "invariant"),
        (TokenKind::Mod, "mod"),
        (TokenKind::Use, "use"),
        (TokenKind::From, "from"),
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

#[test]
fn parses_module_use_nested_types_and_multiple_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.core\n",
            "use platform.io\n",
            "fn collect(items: Vec<Result<Int, Error>>) -> Result<Vec<Int>, Error> effects [fs, net]\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tree.module.as_ref().unwrap().name, "app.core");
    assert_eq!(output.tree.uses[0].name, "platform.io");
    let function = first_function(&output);
    assert_eq!(
        function.params[0].ty.as_deref(),
        Some("Vec<Result<Int, Error>>")
    );
    assert_eq!(
        function.return_type.as_deref(),
        Some("Result<Vec<Int>, Error>")
    );
    assert_eq!(
        function.effects.as_ref().unwrap(),
        &vec!["fs".to_string(), "net".to_string()]
    );
}

#[test]
fn parses_external_package_use_declaration() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "use sub::module from \"github.com/oakcask/foo\"\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(output.tree.uses[0].name, "sub::module");
    let package = output.tree.uses[0]
        .package
        .as_ref()
        .expect("use declaration should keep package source");
    assert_eq!(package.name, "github.com/oakcask/foo");
    assert_eq!(package.span.start.line, 1);
    assert_eq!(package.span.start.column, 22);
}

#[test]
fn parses_function_return_type_effects_before_declaration_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn callback_factory() -> fn(String) -> () effects [stdio]\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let function = first_function(&output);
    assert_eq!(
        function.return_type.as_deref(),
        Some("fn(String) -> () effects [stdio]")
    );
    assert_eq!(function.effects, None);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "pub fn callback_factory() -> fn(String) -> () effects [stdio]\n",
            "end\n",
        )
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
    let function = first_function(&output);
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
            "\tensure output >= value\n",
            "\tvalue\n",
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
    let function = first_function(&output);
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
            "fn main(value: Unit) -> Result<Unit, AppError>\n",
            "\tlet ready: Unit = ()\n",
            "\tOk(ready)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn main(value: ()) -> Result<(), AppError>\n",
            "\tlet ready: () = ()\n",
            "\tOk(ready)\n",
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
            "\t_value satisfy candidate => candidate > 0 and candidate < 10\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let function = first_function(&output);
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
            "\tlet record = { name: \"veln\", values: [1, 2 + 3 * 4] }\n",
            "\t1 * (2 + 3)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(format_tree(&output.tree), source.text());
    let function = first_function(&output);
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
fn parses_try_prefix_and_pipeline_precedence() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(input: Int) -> ()\n",
            "\t-input? |> sink(\"ok\", ())\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(format_tree(&output.tree), source.text());
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Binary {
        op: BinaryOp::PipeGreater,
        left,
        right,
    } = &expr.kind
    else {
        panic!("expected pipeline expression");
    };
    assert!(matches!(
        &left.kind,
        ExprKind::Prefix {
            op: PrefixOp::Negate,
            expr,
        } if matches!(
            &expr.kind,
            ExprKind::Try(inner)
                if matches!(&inner.kind, ExprKind::NamePath(segments) if segments == &vec!["input".to_string()])
        )
    ));
    let ExprKind::Call { callee, args } = &right.kind else {
        panic!("expected call on right side of pipeline");
    };
    assert!(matches!(
        &callee.kind,
        ExprKind::NamePath(segments) if segments == &vec!["sink".to_string()]
    ));
    assert!(matches!(&args[0].kind, ExprKind::StringLiteral(value) if value == "\"ok\""));
    assert!(matches!(&args[1].kind, ExprKind::Unit));
}

#[test]
fn parses_boolean_literals_as_literals() {
    let source = SourceFile::new(
        "main.veln",
        "fn main(flag: Bool) -> Bool\n\ttrue and false or flag\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(format_tree(&output.tree), source.text());
    let function = first_function(&output);
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
            "\tmatch flag\n",
            "\t\ttrue => \"yes\"\n",
            "\t\tfalse => \"no\"\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(format_tree(&output.tree), source.text());
    let function = first_function(&output);
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
        "fn main() -> Dict<String, Int>\n\t{\"one\": 1, \"two\": 2}\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        format_tree(&output.tree),
        "fn main() -> Dict<String, Int>\n\t{ \"one\": 1, \"two\": 2 }\nend\n"
    );
    let function = first_function(&output);
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
        "fn main(seed: Int) -> Dict<Int, String>\n\t{seed + 1: \"next\"}\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        format_tree(&output.tree),
        "fn main(seed: Int) -> Dict<Int, String>\n\t{ seed + 1: \"next\" }\nend\n"
    );
    let function = first_function(&output);
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
            "\tlet record = { name: \"veln\", values: [1, add(2, 3)] }\n",
            "\trecord\n",
            "end\n",
        )
    );
    let function = first_function(&output);
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
            "\tlet count = { nested: { count: 1 } }.nested.count\n",
            "\tcount\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn data() -> ()\n",
            "\tlet count = { nested: { count: 1 } }.nested.count\n",
            "\tcount\n",
            "end\n",
        )
    );
    let function = first_function(&output);
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
fn parses_method_call_shape_as_call_on_field_access() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: {name: String}) -> ()\n",
            "  value.name()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected tail expression");
    };
    let ExprKind::Call { callee, args } = &expr.kind else {
        panic!("expected call expression");
    };
    assert!(args.is_empty());
    let ExprKind::FieldAccess { field, .. } = &callee.kind else {
        panic!("expected field-access callee");
    };
    assert_eq!(field, "name");
}

#[test]
fn format_tree_formats_attached_line_comments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "# header\n",
            "fn   main ( ) -> ()\n",
            "  _ # hole\n",
            "# close docs\n",
            "end # function end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "# header\n",
            "fn main() -> ()\n",
            "\t_  # hole\n",
            "\t# close docs\n",
            "end  # function end\n",
        )
    );
}

#[test]
fn format_tree_formats_attached_hash_line_comments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "# header\n",
            "fn   main ( ) -> ()\n",
            "  _ # hole\n",
            "## close docs\n",
            "end # function end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "# header\n",
            "fn main() -> ()\n",
            "\t_  # hole\n",
            "\t## close docs\n",
            "end  # function end\n",
        )
    );
}

#[test]
fn format_tree_attaches_standalone_comments_to_formatted_lines() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "# module docs\n",
            "mod   app\n",
            "## helper docs\n",
            "fn   helper ( value : Unit ) -> Unit\n",
            "# body docs\n",
            "()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "# module docs\n",
            "mod app\n",
            "\n",
            "## helper docs\n",
            "fn helper(value: ()) -> ()\n",
            "\t# body docs\n",
            "\t()\n",
            "end\n",
        )
    );
}

#[test]
fn format_tree_attaches_comments_to_imports_contracts_and_end_lines() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod   app\n",
            "# import docs\n",
            "use   platform.io\n",
            "# function docs\n",
            "fn   main ( ready : Bool ) -> Unit\n",
            "# require docs\n",
            "require ready\n",
            "# body docs\n",
            "()\n",
            "# end docs\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "mod app\n",
            "# import docs\n",
            "use platform.io\n",
            "\n",
            "# function docs\n",
            "fn main(ready: Bool) -> ()\n",
            "\t# require docs\n",
            "\trequire ready\n",
            "\t# body docs\n",
            "\t()\n",
            "\t# end docs\n",
            "end\n",
        )
    );
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

    let function = first_function(&output);
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
    let function = first_function(&output);
    assert_eq!(function.name.as_deref(), Some("main"));
}

#[test]
fn synchronizes_top_level_garbage_to_next_test_declaration() {
    let source = SourceFile::new("main.veln", "let stray = 1\ntest main() -> ()\nend\n");

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
    let function = first_function(&output);
    assert_eq!(function.kind, FunctionKind::Test);
    assert_eq!(function.name.as_deref(), Some("main"));
}

#[test]
fn parses_and_formats_match_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: Option<Int>) -> String\n",
            "\tmatch value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
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
fn reports_missing_match_arm_arrow_and_keeps_arm_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: Option<Int>) -> String\n",
            "  match value\n",
            "    Some(count) \"some\"\n",
            "    None => \"none\"\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.match_arm")
        .expect("expected missing match arm arrow diagnostic");
    assert_eq!(diagnostic.message, "match arm is missing `=>`");
    assert_eq!(diagnostic.parser_context, "expression_line");
    assert_eq!(diagnostic.unexpected.text, "\"some\"");
    assert_eq!(diagnostic.expected, vec!["=>"]);
    assert_eq!(diagnostic.recovery.strategy, RecoveryStrategy::InsertToken);

    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Match { arms, .. } = &expr.kind else {
        panic!("expected match expression");
    };
    assert_eq!(arms.len(), 2);
    assert!(matches!(
        &arms[0].expr.kind,
        ExprKind::StringLiteral(value) if value == "\"some\""
    ));
}

#[test]
fn parses_match_expression_inside_call_argument() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: Option<Int>) -> String\n",
            "\twrap(match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
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
            "fn describe(value: Option<Int>) -> String\n",
            "\twrap(match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend)\n",
            "end\n",
        )
    );
}

#[test]
fn parses_match_expression_inside_aggregate_literals() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: Option<Int>) -> {labels: [String], primary: String}\n",
            "\t{labels: [match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend], primary: match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend}\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
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
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn describe(value: Option<Int>) -> { labels : [String], primary : String }\n",
            "\t{ labels: [match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend], primary: match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend }\n",
            "end\n",
        )
    );
}

#[test]
fn parses_and_formats_qualified_builtin_constructors() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: Result<Option<Int>, String>) -> Result<String, String>\n",
            "\tmatch value\n",
            "\t\tResult::Ok(Option::Some(count)) => Result::Ok(\"some\")\n",
            "\t\tResult::Ok(Option::None) => Result::Ok(\"none\")\n",
            "\t\tResult::Err(error) => Result::Err(error)\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
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
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn describe(value: Result<Option<Int>, String>) -> Result<String, String>\n",
            "\tmatch value\n",
            "\t\tResult::Ok(Option::Some(count)) => Result::Ok(\"some\")\n",
            "\t\tResult::Ok(Option::None) => Result::Ok(\"none\")\n",
            "\t\tResult::Err(error) => Result::Err(error)\n",
            "\tend\n",
            "end\n",
        )
    );
}

#[test]
fn parses_and_formats_record_patterns() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: {count: Int, label: String}) -> String\n",
            "\tmatch value\n",
            "\t\t{count: 0, label: name} => name\n",
            "\t\t{count: count, label: _} => \"many\"\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn describe(value: { count : Int, label : String }) -> String\n",
            "\tmatch value\n",
            "\t\t{ count: 0, label: name } => name\n",
            "\t\t{ count: count, label: _ } => \"many\"\n",
            "\tend\n",
            "end\n",
        )
    );
    let function = first_function(&output);
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

#[test]
fn reports_missing_record_pattern_field_colon_and_keeps_field_pattern() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: {count: Int}) -> String\n",
            "  match value\n",
            "    {count 0} => \"zero\"\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.pattern")
        .expect("expected missing record field colon diagnostic");
    assert_eq!(diagnostic.message, "record pattern field is missing `:`");
    assert_eq!(diagnostic.parser_context, "expression_line");
    assert_eq!(diagnostic.unexpected.text, "0");
    assert_eq!(diagnostic.expected, vec![":"]);
    assert_eq!(diagnostic.recovery.strategy, RecoveryStrategy::InsertToken);

    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Match { arms, .. } = &expr.kind else {
        panic!("expected match expression");
    };
    let PatternKind::Record(fields) = &arms[0].pattern.kind else {
        panic!("expected record pattern");
    };
    assert_eq!(fields[0].name, "count");
    assert!(matches!(
        &fields[0].pattern.kind,
        PatternKind::IntLiteral(value) if value == "0"
    ));
    assert!(matches!(
        &arms[0].expr.kind,
        ExprKind::StringLiteral(value) if value == "\"zero\""
    ));
}
