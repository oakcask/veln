use super::*;

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
fn parses_the_complete_binary_precedence_ladder() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Bool\n",
            "  a |> b or c and d == e < f + g * h\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn main() -> Bool\n",
            "\ta |> b or c and d == e < f + g * h\n",
            "end\n",
        )
    );
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let mut current = expr;
    let mut right_spine = Vec::new();
    while let ExprKind::Binary { op, right, .. } = &current.kind {
        right_spine.push(*op);
        current = right;
    }
    assert_eq!(
        right_spine,
        vec![
            BinaryOp::PipeGreater,
            BinaryOp::Or,
            BinaryOp::And,
            BinaryOp::Equal,
            BinaryOp::Less,
            BinaryOp::Add,
            BinaryOp::Multiply,
        ]
    );
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
                if matches!(&inner.kind, ExprKind::NamePath { segments, .. } if segments == &vec!["input".to_string()])
        )
    ));
    let ExprKind::Call { callee, args } = &right.kind else {
        panic!("expected call on right side of pipeline");
    };
    assert!(matches!(
        &callee.kind,
        ExprKind::NamePath { segments, .. } if segments == &vec!["sink".to_string()]
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
        matches!(&right.kind, ExprKind::NamePath { segments, .. } if segments == &vec!["flag".to_string()])
    );
    let ExprKind::Binary { left, right, .. } = &left.kind else {
        panic!("expected nested boolean binary expression");
    };
    assert!(matches!(left.kind, ExprKind::BoolLiteral(true)));
    assert!(matches!(right.kind, ExprKind::BoolLiteral(false)));
}

#[test]
fn parses_qualified_boolean_literal_spelling_as_name_path() {
    let source = SourceFile::new(
        "main.veln",
        "fn main() -> Bool\n\tprelude::true and prelude::false\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty());
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Binary { left, right, .. } = &expr.kind else {
        panic!("expected boolean binary expression");
    };
    assert!(matches!(&left.kind, ExprKind::NamePath { segments, .. }
            if segments == &vec!["prelude".to_string(), "true".to_string()]));
    assert!(matches!(&right.kind, ExprKind::NamePath { segments, .. }
            if segments == &vec!["prelude".to_string(), "false".to_string()]));
    assert_eq!(
        bare_expression_bool_literal(&["true".to_string()]),
        Some(true)
    );
    assert_eq!(
        bare_expression_bool_literal(&["prelude".to_string(), "true".to_string()]),
        None
    );
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
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn main(flag: Bool) -> String\n",
            "\tif flag\n",
            "\t\t\"yes\"\n",
            "\telse\n",
            "\t\t\"no\"\n",
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
