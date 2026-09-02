use super::*;

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
fn anchors_adr_lite_records_to_the_next_public_function() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn helper() -> ()\n",
            "  ()\n",
            "end\n",
            "## @adr\n",
            "## id: public-entry\n",
            "## status: accepted\n",
            "## scope: pub fn main\n",
            "## context: The public entry point owns this decision.\n",
            "## decision: Keep the helper outside the record anchor.\n",
            "## consequences: Navigation opens the public function.\n",
            "pub fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        output.tree.adr_lite_records[0].anchor,
        Some(AdrLiteAnchor::Function {
            name: "main".to_string()
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
fn lossless_tree_preserves_mixed_top_level_declaration_order() {
    let text = concat!(
        "mod app\n",
        "use stdio\n",
        "type State\n  Ready\nend\n",
        "effect Notify\n  send() -> ()\nend\n",
        "schema Packet\n  format binary\n  value: UInt8\nend\n",
        "fn main() -> ()\n  ()\nend\n",
        "handler notify() handles Notify\n  send() => ()\nend\n",
        "pub fn exported = main\n",
    );
    let source = SourceFile::new("main.veln", text);

    let output = parse(&source);
    let top_level_kinds = output
        .tree
        .root
        .children
        .iter()
        .filter_map(|child| match child {
            SyntaxElement::Node(node) => Some(node.kind),
            SyntaxElement::Token(_) => None,
        })
        .collect::<Vec<_>>();
    let rendered = output
        .tree
        .lossless_tokens()
        .map(|token| token.text.as_str())
        .collect::<String>();

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(rendered, text);
    assert_eq!(
        top_level_kinds,
        [
            SyntaxNodeKind::ModuleDecl,
            SyntaxNodeKind::UseDecl,
            SyntaxNodeKind::TypeDecl,
            SyntaxNodeKind::EffectDecl,
            SyntaxNodeKind::SchemaDecl,
            SyntaxNodeKind::FunctionDecl,
            SyntaxNodeKind::HandlerDecl,
            SyntaxNodeKind::PublicAliasDecl,
        ]
    );
}
