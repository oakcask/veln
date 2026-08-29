use super::*;

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
fn format_tree_rewrites_literal_equality_match_chain() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn display(value: String) -> String\n",
            "\tmatch value == \"\\n\"\n",
            "\t\ttrue => \"<lf>\"\n",
            "\t\tfalse => match value == \"hpack-byte-00\"\n",
            "\t\t\ttrue => \"<nul>\"\n",
            "\t\t\tfalse => value\n",
            "\t\tend\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn display(value: String) -> String\n",
            "\tmatch value\n",
            "\t\t\"\\n\" => \"<lf>\"\n",
            "\t\t\"hpack-byte-00\" => \"<nul>\"\n",
            "\t\t_ => value\n",
            "\tend\n",
            "end\n",
        )
    );
}

#[test]
fn format_tree_rewrites_literal_equality_match_chain_with_reordered_bool_arms() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn static_name(index: Int) -> String\n",
            "\tmatch index == 1\n",
            "\t\tfalse => match 2 == index\n",
            "\t\t\tfalse => \":status\"\n",
            "\t\t\ttrue => \":method\"\n",
            "\t\tend\n",
            "\t\ttrue => \":authority\"\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn static_name(index: Int) -> String\n",
            "\tmatch index\n",
            "\t\t1 => \":authority\"\n",
            "\t\t2 => \":method\"\n",
            "\t\t_ => \":status\"\n",
            "\tend\n",
            "end\n",
        )
    );
}

#[test]
fn format_tree_rewrites_literal_equality_match_chain_with_nested_match_body() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn preview(value: String) -> ByteChunk\n",
            "\tmatch value == \"bad\"\n",
            "\t\ttrue => match byte_chunk_from_hex(\"626164\")\n",
            "\t\t\tOk(preview) => preview\n",
            "\t\t\tErr(_) => byte_chunk([])\n",
            "\t\tend\n",
            "\t\tfalse => byte_chunk([])\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn preview(value: String) -> ByteChunk\n",
            "\tmatch value\n",
            "\t\t\"bad\" => match byte_chunk_from_hex(\"626164\")\n",
            "\t\t\tOk(preview) => preview\n",
            "\t\t\tErr(_) => byte_chunk([])\n",
            "\t\tend\n",
            "\t\t_ => byte_chunk([])\n",
            "\tend\n",
            "end\n",
        )
    );
}

#[test]
fn format_tree_rewrites_or_grouped_literal_equality_match_chain() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn provenance(fact: String) -> String\n",
            "\tmatch fact == \"content_length_invalid\" or fact == \"content_length_mismatch\"\n",
            "\t\ttrue => \"rfc9113_content_length\"\n",
            "\t\tfalse => \"rfc9113_request_pseudo_headers\"\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn provenance(fact: String) -> String\n",
            "\tmatch fact\n",
            "\t\t\"content_length_invalid\" => \"rfc9113_content_length\"\n",
            "\t\t\"content_length_mismatch\" => \"rfc9113_content_length\"\n",
            "\t\t_ => \"rfc9113_request_pseudo_headers\"\n",
            "\tend\n",
            "end\n",
        )
    );
}

#[test]
fn format_tree_only_rewrites_safe_literal_equality_match_parts() {
    let mixed_scrutinee = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(left: String, right: String) -> String\n",
            "\tmatch left == \"a\"\n",
            "\t\ttrue => \"left\"\n",
            "\t\tfalse => match right == \"b\"\n",
            "\t\t\ttrue => \"right\"\n",
            "\t\t\tfalse => \"none\"\n",
            "\t\tend\n",
            "\tend\n",
            "end\n",
        ),
    );
    let bool_literal = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(flag: Bool) -> String\n",
            "\tmatch flag == true\n",
            "\t\ttrue => \"yes\"\n",
            "\t\tfalse => \"no\"\n",
            "\tend\n",
            "end\n",
        ),
    );
    let non_literal = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(value: String, fallback: String) -> String\n",
            "\tmatch value == fallback\n",
            "\t\ttrue => \"same\"\n",
            "\t\tfalse => value\n",
            "\tend\n",
            "end\n",
        ),
    );

    let mixed_output = parse(&mixed_scrutinee);
    let bool_output = parse(&bool_literal);
    let non_literal_output = parse(&non_literal);

    assert!(
        mixed_output.diagnostics.is_empty(),
        "{:#?}",
        mixed_output.diagnostics
    );
    assert!(
        bool_output.diagnostics.is_empty(),
        "{:#?}",
        bool_output.diagnostics
    );
    assert!(
        non_literal_output.diagnostics.is_empty(),
        "{:#?}",
        non_literal_output.diagnostics
    );
    assert_eq!(
        format_tree(&mixed_output.tree),
        concat!(
            "fn choose(left: String, right: String) -> String\n",
            "\tmatch left\n",
            "\t\t\"a\" => \"left\"\n",
            "\t\t_ => match right\n",
            "\t\t\t\"b\" => \"right\"\n",
            "\t\t\t_ => \"none\"\n",
            "\t\tend\n",
            "\tend\n",
            "end\n",
        )
    );
    assert_eq!(
        format_tree(&bool_output.tree),
        concat!(
            "fn choose(flag: Bool) -> String\n",
            "\tif flag == true\n",
            "\t\t\"yes\"\n",
            "\telse\n",
            "\t\t\"no\"\n",
            "\tend\n",
            "end\n",
        )
    );
    assert_eq!(
        format_tree(&non_literal_output.tree),
        concat!(
            "fn choose(value: String, fallback: String) -> String\n",
            "\tif value == fallback\n",
            "\t\t\"same\"\n",
            "\telse\n",
            "\t\tvalue\n",
            "\tend\n",
            "end\n",
        )
    );
}

#[test]
fn format_tree_keeps_commented_literal_equality_match_lossless() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn display(value: String) -> String\n",
            "\tmatch value == \"\\n\"  # keep this shape\n",
            "\t\ttrue => \"<lf>\"\n",
            "\t\tfalse => value\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(format_tree(&output.tree), source.text());
}

#[test]
fn format_tree_rewrites_bool_match_to_if_else() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(flag: Bool) -> String\n",
            "\tmatch flag\n",
            "\t\ttrue => \"yes\"\n",
            "\t\tfalse => \"no\"\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn choose(flag: Bool) -> String\n",
            "\tif flag\n",
            "\t\t\"yes\"\n",
            "\telse\n",
            "\t\t\"no\"\n",
            "\tend\n",
            "end\n",
        )
    );
}

#[test]
fn format_tree_rewrites_reordered_bool_match_to_if_else() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(flag: Bool) -> String\n",
            "\tmatch flag\n",
            "\t\tfalse => \"no\"\n",
            "\t\ttrue => \"yes\"\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn choose(flag: Bool) -> String\n",
            "\tif flag\n",
            "\t\t\"yes\"\n",
            "\telse\n",
            "\t\t\"no\"\n",
            "\tend\n",
            "end\n",
        )
    );
}

#[test]
fn format_tree_rewrites_bool_match_false_continuation_to_else_if() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(first: Bool, second: Bool) -> String\n",
            "\tmatch first\n",
            "\t\ttrue => \"first\"\n",
            "\t\tfalse => match second\n",
            "\t\t\ttrue => \"second\"\n",
            "\t\t\tfalse => \"none\"\n",
            "\t\tend\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn choose(first: Bool, second: Bool) -> String\n",
            "\tif first\n",
            "\t\t\"first\"\n",
            "\telse if second\n",
            "\t\t\"second\"\n",
            "\telse\n",
            "\t\t\"none\"\n",
            "\tend\n",
            "end\n",
        )
    );
}

#[test]
fn format_tree_keeps_commented_bool_match_lossless() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(flag: Bool) -> String\n",
            "\tmatch flag # keep this shape\n",
            "\t\ttrue => \"yes\"\n",
            "\t\tfalse => \"no\"\n",
            "\tend\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(format_tree(&output.tree), source.text());
}

#[test]
fn format_tree_keeps_nested_commented_bool_match_lossless() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(flag: Bool) -> String\n",
            "\twrap(match flag # keep this nested shape\n",
            "\t\ttrue => \"yes\"\n",
            "\t\tfalse => \"no\"\n",
            "\tend)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
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
