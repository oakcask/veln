use super::*;

const SOURCE: &str = r#"fn demo() -> Int
 let first = acquire()
 defer
  release(first)
 end
 let result = begin
  let second = acquire()
  defer
   release(second)
  end
  second
 end
 result
end
"#;

#[test]
fn lexes_defer_and_begin_as_keywords() {
    let source = SourceFile::new("cleanup.veln", "defer begin\n");
    let kinds = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| {
            !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::Newline | TokenKind::Eof
            )
        })
        .map(|token| token.kind)
        .collect::<Vec<_>>();

    assert_eq!(kinds, vec![TokenKind::Defer, TokenKind::Begin]);
}

#[test]
fn parses_cleanup_regions_and_preserves_block_spans() {
    let source = SourceFile::new("cleanup.veln", SOURCE);
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let function = first_function(&output);
    let BodyLine::Defer {
        body,
        block_span,
        span,
    } = &function.body[1]
    else {
        panic!("expected defer statement");
    };
    assert_eq!(body.len(), 1);
    assert_eq!(
        &SOURCE[span.start.offset..span.end.offset],
        "defer\n  release(first)\n end\n"
    );
    assert_eq!(
        &SOURCE[block_span.start.offset..block_span.end.offset],
        "  release(first)\n "
    );

    let BodyLine::Let { expr, .. } = &function.body[2] else {
        panic!("expected result binding");
    };
    let ExprKind::Begin { body, block_span } = &expr.kind else {
        panic!("expected begin expression");
    };
    assert_eq!(body.len(), 3);
    assert_eq!(
        &SOURCE[expr.span.start.offset..expr.span.end.offset],
        "begin\n  let second = acquire()\n  defer\n   release(second)\n  end\n  second\n end"
    );
    assert_eq!(
        &SOURCE[block_span.start.offset..block_span.end.offset],
        "  let second = acquire()\n  defer\n   release(second)\n  end\n  second\n "
    );
}

#[test]
fn formats_cleanup_regions_stably() {
    let source = SourceFile::new("cleanup.veln", SOURCE);
    let first = format_tree(&parse(&source).tree);
    let expected = "fn demo() -> Int\n\tlet first = acquire()\n\tdefer\n\t\trelease(first)\n\tend\n\tlet result = begin\n\t\tlet second = acquire()\n\t\tdefer\n\t\t\trelease(second)\n\t\tend\n\t\tsecond\n\tend\n\tresult\nend\n";
    assert_eq!(first, expected);

    let second_source = SourceFile::new("cleanup.veln", first.clone());
    let second_output = parse(&second_source);
    assert!(
        second_output.diagnostics.is_empty(),
        "{:#?}",
        second_output.diagnostics
    );
    assert_eq!(format_tree(&second_output.tree), first);
}

#[test]
fn keeps_cleanup_header_comments_on_the_header_line() {
    let input = concat!(
        "fn demo() -> Int\n",
        " defer # release the outer resource\n",
        "  ()\n",
        " end\n",
        " let value = begin # limit the inner resource lifetime\n",
        "  defer # release the inner resource\n",
        "   ()\n",
        "  end\n",
        "  1\n",
        " end\n",
        " value\n",
        "end\n",
    );
    let source = SourceFile::new("cleanup.veln", input);
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let first = format_tree(&output.tree);
    let expected = concat!(
        "fn demo() -> Int\n",
        "\tdefer  # release the outer resource\n",
        "\t\t()\n",
        "\tend\n",
        "\tlet value = begin  # limit the inner resource lifetime\n",
        "\t\tdefer  # release the inner resource\n",
        "\t\t\t()\n",
        "\t\tend\n",
        "\t\t1\n",
        "\tend\n",
        "\tvalue\n",
        "end\n",
    );
    assert_eq!(first, expected);

    let second_source = SourceFile::new("cleanup.veln", first.clone());
    assert_eq!(format_tree(&parse(&second_source).tree), first);
}

#[test]
fn lossless_tree_exposes_cleanup_region_nodes() {
    let source = SourceFile::new("cleanup.veln", SOURCE);
    let output = parse(&source);
    let nodes = output.tree.descendant_nodes().collect::<Vec<_>>();

    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.kind == SyntaxNodeKind::DeferStatement)
            .count(),
        2
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.kind == SyntaxNodeKind::BeginExpr)
            .count(),
        1
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node.kind == SyntaxNodeKind::CleanupBody)
            .count(),
        3
    );
    let begin = nodes
        .iter()
        .find(|node| node.kind == SyntaxNodeKind::BeginExpr)
        .expect("begin expression node");
    assert_eq!(
        &SOURCE[begin.range.start..begin.range.end],
        "begin\n  let second = acquire()\n  defer\n   release(second)\n  end\n  second\n end"
    );
    assert_eq!(
        output
            .tree
            .lossless_tokens()
            .filter(|token| token.kind != TokenKind::Eof)
            .map(|token| token.text.as_str())
            .collect::<String>(),
        SOURCE
    );
}

#[test]
fn handler_operation_navigation_exposes_begin_region() {
    let source = SourceFile::new(
        "handler.veln",
        "effect Resource\n close() -> ()\nend\n\nhandler Cleanup() handles Resource\n close() => begin\n  ()\n end\nend\n",
    );
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let kinds = output
        .tree
        .descendant_nodes()
        .map(|node| node.kind)
        .collect::<Vec<_>>();

    assert!(kinds.contains(&SyntaxNodeKind::HandlerOperationClause));
    assert!(kinds.contains(&SyntaxNodeKind::BeginExpr));
    assert!(kinds.contains(&SyntaxNodeKind::CleanupBody));
}

#[test]
fn reports_unterminated_cleanup_regions() {
    for text in [
        "fn demo()\n defer\n  release()\n",
        "fn demo()\n let value = begin\n  acquire()\n",
    ] {
        let source = SourceFile::new("cleanup.veln", text);
        let output = parse(&source);
        assert!(
            output.diagnostics.iter().any(|diagnostic| matches!(
                diagnostic.id,
                "parse.defer_missing_end" | "parse.begin_missing_end"
            )),
            "{:#?}",
            output.diagnostics
        );
    }
}

#[test]
fn rejects_excessive_cleanup_nesting_without_aborting() {
    let depth = 800;
    let mut text = String::from("fn deeply_nested() -> ()\n ");
    for _ in 0..depth {
        text.push_str("begin\n ");
    }
    text.push_str("()\n");
    for _ in 0..depth {
        text.push_str(" end\n");
    }
    text.push_str("end\n");

    let source = SourceFile::new("deep-cleanup.veln", text);
    let output = parse(&source);

    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.cleanup_nesting_limit")
    );
    assert_eq!(
        output.tree.lossless_tokens().count(),
        lex(&source).tokens.len()
    );
}
