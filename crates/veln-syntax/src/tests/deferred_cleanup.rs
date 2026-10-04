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
        keyword_span,
        block_span,
        span,
    } = &function.body[1]
    else {
        panic!("expected defer statement");
    };
    assert_eq!(body.len(), 1);
    assert_eq!(
        &SOURCE[keyword_span.start.offset..keyword_span.end.offset],
        "defer"
    );
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
fn embedded_begin_expressions_preserve_blocks_and_surrounding_expression_tokens() {
    let input = concat!(
        "fn embedded()\n",
        " let called = consume(begin\n",
        "  let value = 1\n",
        "  value\n",
        " end, 2)\n",
        " let listed = [0, begin\n",
        "  let value = 1\n",
        "  value\n",
        " end, 2]\n",
        " let recorded = { value: begin\n",
        "  let value = 1\n",
        "  value\n",
        " end, other: 2 }\n",
        " let operated = 0 + begin\n",
        "  let value = 1\n",
        "  value\n",
        " end * 2\n",
        " let suffixed = begin\n",
        "  let value = resource()\n",
        "  value\n",
        " end.field?\n",
        " ()\n",
        "end\n",
    );
    let source = SourceFile::new("embedded-cleanup.veln", input);
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let function = first_function(&output);
    let expressions = function
        .body
        .iter()
        .take(5)
        .map(|line| match line {
            BodyLine::Let { expr, .. } => expr,
            _ => panic!("expected let binding"),
        })
        .collect::<Vec<_>>();
    assert!(matches!(
        expressions[0].kind,
        ExprKind::Call { ref args, .. }
            if args.len() == 2 && matches!(args[0].kind, ExprKind::Begin { ref body, .. } if body.len() == 2)
    ));
    assert!(matches!(
        expressions[1].kind,
        ExprKind::List(ref items)
            if items.len() == 3 && matches!(items[1].kind, ExprKind::Begin { ref body, .. } if body.len() == 2)
    ));
    assert!(matches!(
        expressions[2].kind,
        ExprKind::Record(ref fields)
            if fields.len() == 2 && matches!(fields[0].expr.kind, ExprKind::Begin { ref body, .. } if body.len() == 2)
    ));
    assert!(matches!(
        expressions[3].kind,
        ExprKind::Binary { ref right, .. }
            if matches!(right.kind, ExprKind::Binary { ref left, .. }
                if matches!(left.kind, ExprKind::Begin { ref body, .. } if body.len() == 2))
    ));
    let ExprKind::Try {
        expr: inner,
        question_span,
    } = &expressions[4].kind
    else {
        panic!("expected try expression");
    };
    assert!(matches!(
        inner.kind,
        ExprKind::FieldAccess { ref base, .. }
            if matches!(base.kind, ExprKind::Begin { ref body, .. } if body.len() == 2)
    ));
    assert_eq!(
        &input[question_span.start.offset..question_span.end.offset],
        "?"
    );

    let first = format_tree(&output.tree);
    let second_source = SourceFile::new("embedded-cleanup.veln", first.clone());
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
fn moves_begin_continuation_comments_after_the_enclosing_binary_expression() {
    let input = concat!(
        "fn calculate() -> Int\n",
        " let simple = begin\n",
        "  1\n",
        " end + 2 # keep the complete sum\n",
        " let nested = begin\n",
        "  3\n",
        " end + begin # keep both regions\n",
        "  4\n",
        " end\n",
        " simple + nested\n",
        "end\n",
    );
    let source = SourceFile::new("cleanup.veln", input);
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let first = format_tree(&output.tree);
    let expected = concat!(
        "fn calculate() -> Int\n",
        "\tlet simple = begin\n",
        "\t\t1\n",
        "\tend + 2  # keep the complete sum\n",
        "\tlet nested = begin\n",
        "\t\t3\n",
        "\tend + begin\n",
        "\t\t4\n",
        "\tend  # keep both regions\n",
        "\tsimple + nested\n",
        "end\n",
    );
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
fn moves_begin_continuation_comments_after_every_enclosing_expression() {
    let input = concat!(
        "fn continued()\n",
        " let postfix = begin\n",
        "  value\n",
        " end.field? # keep postfix\n",
        " let called = consume(begin\n",
        "  value\n",
        " end, 2) # keep call\n",
        " let listed = [begin\n",
        "  value\n",
        " end, 2] # keep list\n",
        " let recorded = { value: begin\n",
        "  value\n",
        " end, other: 2 } # keep record\n",
        " let decoded = decode Packet from begin\n",
        "  value\n",
        " end at 0 # keep decode\n",
        " ()\n",
        "end\n",
    );
    let source = SourceFile::new("continued-cleanup.veln", input);
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let first = format_tree(&output.tree);
    let expected = concat!(
        "fn continued()\n",
        "\tlet postfix = begin\n",
        "\t\tvalue\n",
        "\tend.field?  # keep postfix\n",
        "\tlet called = consume(begin\n",
        "\t\tvalue\n",
        "\tend, 2)  # keep call\n",
        "\tlet listed = [begin\n",
        "\t\tvalue\n",
        "\tend, 2]  # keep list\n",
        "\tlet recorded = { value: begin\n",
        "\t\tvalue\n",
        "\tend, other: 2 }  # keep record\n",
        "\tlet decoded = decode Packet from begin\n",
        "\t\tvalue\n",
        "\tend at 0  # keep decode\n",
        "\t()\n",
        "end\n",
    );
    assert_eq!(first, expected);

    let second_source = SourceFile::new("continued-cleanup.veln", first.clone());
    let second_output = parse(&second_source);
    assert!(
        second_output.diagnostics.is_empty(),
        "{:#?}",
        second_output.diagnostics
    );
    assert_eq!(format_tree(&second_output.tree), first);
}

#[test]
fn moves_begin_continuation_comments_through_generic_and_effect_expressions() {
    let input = concat!(
        "fn continued()\n",
        " let generic = consume<Int>(begin\n",
        "  value\n",
        " end) # keep generic call\n",
        " let effectful = handle perform Audit::record(begin\n",
        "  value\n",
        " end) with audit() # keep handled effect\n",
        " ()\n",
        "end\n",
    );
    let source = SourceFile::new("continued-cleanup.veln", input);
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let first = format_tree(&output.tree);
    let expected = concat!(
        "fn continued()\n",
        "\tlet generic = consume<Int>(begin\n",
        "\t\tvalue\n",
        "\tend)  # keep generic call\n",
        "\tlet effectful = handle perform Audit::record(begin\n",
        "\t\tvalue\n",
        "\tend) with audit()  # keep handled effect\n",
        "\t()\n",
        "end\n",
    );
    assert_eq!(first, expected);

    let second_source = SourceFile::new("continued-cleanup.veln", first.clone());
    let second_output = parse(&second_source);
    assert!(
        second_output.diagnostics.is_empty(),
        "{:#?}",
        second_output.diagnostics
    );
    assert_eq!(format_tree(&second_output.tree), first);
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
        "effect Resource\n close() -> ()\nend\n\nhandler Cleanup() for Resource\n close() => begin\n  ()\n end\nend\n",
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
    for (text, expected_diagnostic) in [
        (
            "fn demo()\n defer\n  release()\n",
            "parse.defer_missing_end",
        ),
        (
            "fn demo()\n let value = begin\n  acquire()\n",
            "parse.begin_missing_end",
        ),
    ] {
        let source = SourceFile::new("cleanup.veln", text);
        let output = parse(&source);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == expected_diagnostic),
            "{:#?}",
            output.diagnostics
        );
    }
}

#[test]
fn missing_cleanup_end_preserves_following_top_level_declaration() {
    for (body, expected_diagnostic) in [
        (" defer\n  release()\n", "parse.defer_missing_end"),
        (
            " let value = begin\n  acquire()\n",
            "parse.begin_missing_end",
        ),
    ] {
        let source = SourceFile::new(
            "cleanup.veln",
            format!("fn incomplete() -> ()\n{body}end\n\nfn following() -> Int\n  1\nend\n"),
        );
        let output = parse(&source);
        let functions = output
            .tree
            .items
            .iter()
            .filter_map(|item| match item {
                SyntaxItem::Function(function) => Some(function.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let diagnostic_ids = output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id)
            .collect::<Vec<_>>();

        assert_eq!(
            functions
                .iter()
                .filter_map(|function| function.name.as_deref())
                .collect::<Vec<_>>(),
            ["incomplete", "following"]
        );
        assert!(functions.iter().all(|function| function.end_present));
        assert_eq!(diagnostic_ids, [expected_diagnostic]);
        assert_eq!(
            output.tree.lossless_tokens().count(),
            lex(&source).tokens.len()
        );
    }
}

#[test]
fn missing_cleanup_end_in_final_declaration_preserves_the_declaration_end() {
    for (declaration, expected_diagnostic) in [
        (
            "fn incomplete() -> ()\n defer\n  release()\nend\n",
            "parse.defer_missing_end",
        ),
        (
            "fn incomplete() -> ()\n begin\n  ()\nend\n",
            "parse.begin_missing_end",
        ),
        (
            "test incomplete() -> ()\n defer\n  release()\nend\n",
            "parse.defer_missing_end",
        ),
        (
            "test incomplete() -> ()\n begin\n  ()\nend\n",
            "parse.begin_missing_end",
        ),
    ] {
        let source = SourceFile::new("cleanup.veln", declaration);
        let expected_token_count = lex(&source).tokens.len();

        let output = parse(&source);
        let function = output
            .tree
            .items
            .iter()
            .find_map(|item| match item {
                SyntaxItem::Function(function) => Some(function.as_ref()),
                _ => None,
            })
            .expect("function or test declaration");

        assert!(function.end_present, "{:#?}", output.diagnostics);
        assert_eq!(
            output
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.id)
                .collect::<Vec<_>>(),
            [expected_diagnostic]
        );
        assert_eq!(output.tree.lossless_tokens().count(), expected_token_count);
    }
}

#[test]
fn nested_missing_cleanup_ends_preserve_following_top_level_declaration() {
    for (body, expected_diagnostics) in [
        (
            " let value = begin\n  defer\n   release()\n",
            ["parse.defer_missing_end", "parse.begin_missing_end"],
        ),
        (
            " defer\n  let value = begin\n   release()\n",
            ["parse.begin_missing_end", "parse.defer_missing_end"],
        ),
    ] {
        let source = SourceFile::new(
            "cleanup.veln",
            format!("fn incomplete() -> ()\n{body}end\n\nfn following() -> Int\n  1\nend\n"),
        );
        let expected_token_count = lex(&source).tokens.len();

        let output = parse(&source);

        let functions = output
            .tree
            .items
            .iter()
            .filter_map(|item| match item {
                SyntaxItem::Function(function) => Some(function.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            functions
                .iter()
                .filter_map(|function| function.name.as_deref())
                .collect::<Vec<_>>(),
            ["incomplete", "following"],
            "{:#?}",
            output.diagnostics
        );
        assert!(functions.iter().all(|function| function.end_present));
        assert_eq!(
            output
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.id)
                .collect::<Vec<_>>(),
            expected_diagnostics
        );
        assert_eq!(output.tree.lossless_tokens().count(), expected_token_count);
    }
}

#[test]
fn missing_begin_end_below_if_preserves_branch_and_declaration_boundaries() {
    let source = SourceFile::new(
        "cleanup.veln",
        concat!(
            "fn incomplete() -> ()\n",
            " if true\n",
            "  begin\n",
            "   ()\n",
            " else\n",
            "  ()\n",
            " end\n",
            "end\n\n",
            "fn following() -> Int\n",
            " 1\n",
            "end\n",
        ),
    );
    let expected_token_count = lex(&source).tokens.len();

    let output = parse(&source);

    let functions = output
        .tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Function(function) => Some(function.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        functions
            .iter()
            .filter_map(|function| function.name.as_deref())
            .collect::<Vec<_>>(),
        ["incomplete", "following"],
        "{:#?}",
        output.diagnostics
    );
    assert!(functions.iter().all(|function| function.end_present));
    let BodyLine::Expr { expr, .. } = &functions[0].body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::If {
        then_branch,
        else_branch,
        ..
    } = &expr.kind
    else {
        panic!("expected recovered if expression");
    };
    assert!(matches!(then_branch.kind, ExprKind::Begin { .. }));
    assert!(matches!(else_branch.kind, ExprKind::Unit));
    assert_eq!(
        output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id)
            .collect::<Vec<_>>(),
        ["parse.begin_missing_end"]
    );
    assert_eq!(output.tree.lossless_tokens().count(), expected_token_count);
}

#[test]
fn missing_begin_end_below_match_preserves_arm_and_declaration_boundaries() {
    let source = SourceFile::new(
        "cleanup.veln",
        concat!(
            "fn incomplete() -> ()\n",
            " match 0\n",
            "  0 => begin\n",
            "   ()\n",
            "  1 => ()\n",
            " end\n",
            "end\n\n",
            "fn following() -> Int\n",
            " 1\n",
            "end\n",
        ),
    );
    let expected_token_count = lex(&source).tokens.len();

    let output = parse(&source);

    let functions = output
        .tree
        .items
        .iter()
        .filter_map(|item| match item {
            SyntaxItem::Function(function) => Some(function.as_ref()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        functions
            .iter()
            .filter_map(|function| function.name.as_deref())
            .collect::<Vec<_>>(),
        ["incomplete", "following"],
        "{:#?}",
        output.diagnostics
    );
    assert!(functions.iter().all(|function| function.end_present));
    let BodyLine::Expr { expr, .. } = &functions[0].body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Match { arms, .. } = &expr.kind else {
        panic!("expected recovered match expression");
    };
    assert_eq!(arms.len(), 2);
    assert!(matches!(arms[0].expr.kind, ExprKind::Begin { .. }));
    assert!(matches!(arms[1].expr.kind, ExprKind::Unit));
    assert_eq!(
        output
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id)
            .collect::<Vec<_>>(),
        ["parse.begin_missing_end"]
    );
    assert_eq!(output.tree.lossless_tokens().count(), expected_token_count);
}

#[test]
fn complete_nested_control_blocks_are_not_recovery_boundaries() {
    let source = SourceFile::new(
        "cleanup.veln",
        concat!(
            "fn complete() -> ()\n",
            " match 0\n",
            "  0 => begin\n",
            "   ()\n",
            "  end\n",
            "  _ => begin\n",
            "   if true\n",
            "    ()\n",
            "   else\n",
            "    ()\n",
            "   end\n",
            "  end\n",
            " end\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        output.tree.lossless_tokens().count(),
        lex(&source).tokens.len()
    );
}

#[test]
fn satisfy_arrow_is_not_a_match_arm_recovery_boundary() {
    let source = SourceFile::new(
        "cleanup.veln",
        "_value satisfy candidate => candidate == 1\n",
    );
    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !token.kind.is_trivia())
        .collect::<Vec<_>>();

    assert!(!crate::parser::line_starts_match_arm(&tokens, 0));
}

#[test]
fn malformed_multiline_match_recovery_lookahead_grows_linearly() {
    fn lookahead_work(line_count: usize) -> (usize, usize) {
        let mut text = String::from("fn malformed(value: Int) -> Int\n  match value\n");
        for _ in 0..line_count {
            text.push_str("    (\n");
        }
        text.push_str("  end\nend\n");
        let source = SourceFile::new("cleanup.veln", text);
        let token_count = lex(&source).tokens.len();

        crate::parser::reset_match_arm_lookahead_token_visits();
        let output = parse(&source);
        assert_eq!(output.tree.lossless_tokens().count(), token_count);
        (
            crate::parser::match_arm_lookahead_token_visits(),
            token_count,
        )
    }

    let (shallow_work, shallow_tokens) = lookahead_work(32);
    let (deep_work, deep_tokens) = lookahead_work(64);
    assert!(
        shallow_work > 0 && deep_work > shallow_work,
        "generated malformed matches must exercise recovery lookahead: \
         shallow={shallow_work}/{shallow_tokens}, deep={deep_work}/{deep_tokens}"
    );
    assert!(
        deep_work <= shallow_work * 2,
        "match-arm recovery lookahead must remain linear: \
         shallow={shallow_work}/{shallow_tokens}, deep={deep_work}/{deep_tokens}"
    );
    assert!(
        shallow_work <= shallow_tokens * 2 && deep_work <= deep_tokens * 2,
        "match-arm recovery lookahead must remain proportional to input tokens: \
         shallow={shallow_work}/{shallow_tokens}, deep={deep_work}/{deep_tokens}"
    );
}

#[test]
fn expression_position_defer_preserves_following_declaration_boundary() {
    for invalid_call in ["  consume(defer)\n", "  consume(\n    defer\n  )\n"] {
        let source = SourceFile::new(
            "cleanup.veln",
            format!("fn invalid() -> ()\n{invalid_call}end\nfn following() -> Int\n  1\nend\n"),
        );
        let expected_token_count = lex(&source).tokens.len();

        let output = parse(&source);

        assert_eq!(output.tree.lossless_tokens().count(), expected_token_count);
        let function_names = output
            .tree
            .items
            .iter()
            .filter_map(|item| match item {
                SyntaxItem::Function(function) => function.name.as_deref(),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            function_names,
            ["invalid", "following"],
            "{:#?}",
            output.diagnostics
        );
        assert_eq!(output.diagnostics.len(), 1, "{:#?}", output.diagnostics);
        assert_eq!(output.diagnostics[0].id, "parse.expected_expression");
        assert_eq!(
            output.diagnostics[0].message,
            "`defer` is only valid as a direct cleanup-body line"
        );
        assert_eq!(output.diagnostics[0].unexpected.text, "defer");
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

#[test]
fn rejects_two_thousand_directly_nested_defer_statements_without_aborting() {
    let depth = 2_000;
    let mut text = String::from("fn deeply_deferred() -> ()\n");
    for _ in 0..depth {
        text.push_str(" defer\n");
    }
    text.push_str(" ()\n");
    for _ in 0..depth {
        text.push_str(" end\n");
    }
    text.push_str("end\n");

    let source = SourceFile::new("deep-defer.veln", text);
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

#[test]
fn shares_cleanup_nesting_limit_across_defer_and_begin_parsers() {
    for (total_forms, expect_limit) in [(128, false), (129, true)] {
        let mut text = String::from("fn mixed_cleanup() -> ()\n defer\n");
        for _ in 1..total_forms {
            text.push_str(" begin\n");
        }
        text.push_str(" ()\n");
        for _ in 1..total_forms {
            text.push_str(" end\n");
        }
        text.push_str(" end\nend\n");

        let source = SourceFile::new("mixed-cleanup.veln", text);
        let expected_token_count = lex(&source).tokens.len();
        let output = parse(&source);
        let has_limit = output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.cleanup_nesting_limit");

        assert_eq!(has_limit, expect_limit, "{:#?}", output.diagnostics);
        if !expect_limit {
            assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
        }
        assert_eq!(output.tree.lossless_tokens().count(), expected_token_count);
    }
}

#[test]
fn rejects_excessive_alternating_cleanup_nesting_without_aborting() {
    let total_forms = 129;
    let mut text = String::from("fn alternating_cleanup() -> ()\n defer\n");
    for index in 1..total_forms {
        if index % 2 == 0 {
            text.push_str(" defer\n");
        } else {
            text.push_str(" begin\n");
        }
    }
    text.push_str(" ()\n");
    for _ in 1..total_forms {
        text.push_str(" end\n");
    }
    text.push_str(" end\nend\n");

    let source = SourceFile::new("alternating-cleanup.veln", text);
    let expected_token_count = lex(&source).tokens.len();
    let output = parse(&source);

    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.cleanup_nesting_limit"),
        "{:#?}",
        output.diagnostics
    );
    assert_eq!(output.tree.lossless_tokens().count(), expected_token_count);
}

#[test]
fn lossless_tree_handles_large_flat_binary_expression_without_input_depth_recursion() {
    let term_count = 10_000;
    let mut text = String::from("fn flat() -> Int\n  0");
    for _ in 1..term_count {
        text.push_str(" + 0");
    }
    text.push_str("\nend\n");

    let source = SourceFile::new("flat.veln", text);
    let expected_token_count = lex(&source).tokens.len();
    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tree.lossless_tokens().count(), expected_token_count);

    // The parsed binary AST is deliberately left-deep. Avoid making its
    // recursive drop behavior part of this lossless-tree construction test.
    std::mem::forget(output);
}
