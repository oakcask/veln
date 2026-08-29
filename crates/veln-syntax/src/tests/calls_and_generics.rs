use super::*;

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
fn parses_task_spawn_with_result_and_context_type_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(job: fn({payload: String}) -> String effects [concurrency], context: {payload: String}) -> ()\n",
            "  task::spawn_with<String, {payload: String}>(job, context)\n",
            "end\n",
        ),
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
    assert_eq!(args.len(), 2);
    let ExprKind::TypeApply { callee, type_args } = &callee.kind else {
        panic!("expected type-applied callee");
    };
    assert_eq!(
        type_args,
        &vec!["String".to_string(), "{payload:String}".to_string()]
    );
    assert!(matches!(
        &callee.kind,
        ExprKind::NamePath(segments) if segments == &vec!["task".to_string(), "spawn_with".to_string()]
    ));
}

#[test]
fn type_argument_commas_split_only_at_the_outer_delimiter() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> ()\n",
            "  factory::make<fn(Int, String) -> Bool effects [stdio, concurrency], ",
            "{left: Int, right: String}, Result<Int, AppError>>()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression line");
    };
    let ExprKind::Call { callee, .. } = &expr.kind else {
        panic!("expected call expression");
    };
    let ExprKind::TypeApply { type_args, .. } = &callee.kind else {
        panic!("expected type-applied callee");
    };
    assert_eq!(
        type_args,
        &vec![
            "fn(Int,String)->Booleffects[stdio,concurrency]".to_string(),
            "{left:Int,right:String}".to_string(),
            "Result<Int,AppError>".to_string(),
        ]
    );
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
