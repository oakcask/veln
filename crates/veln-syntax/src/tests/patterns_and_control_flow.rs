use super::*;

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
        PatternKind::Constructor { name, args, .. } if name == &vec!["Result".to_string(), "Ok".to_string()]
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
fn parses_lowercase_qualified_pattern_as_recovery_constructor() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: Item) -> Int\n",
            "\tmatch value\n",
            "\t\tItem::some(payload) => payload\n",
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
    let PatternKind::Constructor {
        name,
        name_spans,
        args,
    } = &arms[0].pattern.kind
    else {
        panic!("expected recovery constructor pattern");
    };
    assert_eq!(name, &vec!["Item".to_string(), "some".to_string()]);
    assert_eq!(args.len(), 1);
    assert_eq!(name_spans.len(), 2);
    assert_eq!(
        (
            name_spans[1].start.line,
            name_spans[1].start.column,
            name_spans[1].end.column
        ),
        (3, 9, 13)
    );
}

#[test]
fn parses_lowercase_qualified_nullary_pattern_as_recovery_constructor() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn describe(value: Item) -> Int\n",
            "\tmatch value\n",
            "\t\tItem::none => 0\n",
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
    let PatternKind::Constructor {
        name,
        name_spans,
        args,
    } = &arms[0].pattern.kind
    else {
        panic!("expected recovery constructor pattern");
    };
    assert_eq!(name, &vec!["Item".to_string(), "none".to_string()]);
    assert!(args.is_empty());
    assert_eq!(name_spans.len(), 2);
    assert_eq!(
        (
            name_spans[1].start.line,
            name_spans[1].start.column,
            name_spans[1].end.column
        ),
        (3, 9, 13)
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

#[test]
fn parses_if_else_expression_chain_as_distinct_surface_expr() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(first: Bool, second: Bool) -> Int\n",
            "  if first\n",
            "    1\n",
            "  else if second\n",
            "    2\n",
            "  else\n",
            "    3\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn choose(first: Bool, second: Bool) -> Int\n",
            "\tif first\n",
            "\t\t1\n",
            "\telse if second\n",
            "\t\t2\n",
            "\telse\n",
            "\t\t3\n",
            "\tend\n",
            "end\n",
        )
    );
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::If {
        condition,
        then_branch,
        else_if_branches,
        else_branch,
    } = &expr.kind
    else {
        panic!("expected if expression");
    };
    assert!(
        matches!(&condition.kind, ExprKind::NamePath(segments) if segments == &vec!["first".to_string()])
    );
    assert!(matches!(&then_branch.kind, ExprKind::IntLiteral(value) if value == "1"));
    assert_eq!(else_if_branches.len(), 1);
    assert!(
        matches!(&else_if_branches[0].condition.kind, ExprKind::NamePath(segments) if segments == &vec!["second".to_string()])
    );
    assert!(matches!(&else_if_branches[0].expr.kind, ExprKind::IntLiteral(value) if value == "2"));
    assert!(matches!(&else_branch.kind, ExprKind::IntLiteral(value) if value == "3"));
}

#[test]
fn reports_if_expression_missing_else_before_end() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(first: Bool) -> Int\n",
            "  if first\n",
            "    1\n",
            "  end\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.if_missing_else")
        .expect("expected missing else diagnostic");
    assert_eq!(
        diagnostic.message,
        "if expression is missing a final `else` branch"
    );
    assert_eq!(diagnostic.expected, vec!["else"]);
    assert_eq!(diagnostic.recovery.strategy, RecoveryStrategy::InsertToken);
}
