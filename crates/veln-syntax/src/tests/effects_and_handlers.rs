use super::*;

#[test]
fn parses_decode_as_an_explicit_module_member_name() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn decode(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "pub fn call(value: Int) -> Int\n",
            "  http2::frame::decode(value)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(decode) = &output.tree.items[0] else {
        panic!("expected decode function");
    };
    assert_eq!(decode.name.as_deref(), Some("decode"));
    let SyntaxItem::Function(call) = &output.tree.items[1] else {
        panic!("expected caller function");
    };
    let BodyLine::Expr { expr, .. } = &call.body[0] else {
        panic!("expected call expression");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Call { callee, .. }
            if matches!(&callee.kind, ExprKind::NamePath { segments, .. }
                if segments == &vec!["http2".to_string(), "frame".to_string(), "decode".to_string()])
    ));
}

#[test]
fn parses_and_formats_nominal_effect_operations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub effect Audit\n",
            "  record(user:String,count:Int)->String\n",
            "end\n",
            "\n",
            "pub fn main() -> String effects [Audit]\n",
            "  perform Audit::record(\"user\", 1)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Effect(effect) = &output.tree.items[0] else {
        panic!("expected effect declaration");
    };
    assert_eq!(effect.name.as_deref(), Some("Audit"));
    assert_eq!(effect.operations[0].name.as_deref(), Some("record"));
    let SyntaxItem::Function(function) = &output.tree.items[1] else {
        panic!("expected function declaration");
    };
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression body");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Perform { effect, operation, args, .. }
            if effect == &vec!["Audit".to_string()] && operation == "record" && args.len() == 2
    ));
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "pub effect Audit\n",
            "\trecord(user: String, count: Int) -> String\n",
            "end\n",
            "\n",
            "pub fn main() -> String effects [Audit]\n",
            "\tperform Audit::record(\"user\", 1)\n",
            "end\n",
        )
    );
}

#[test]
fn parses_and_formats_effect_row_binder_and_tail() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn apply<effect E>(callback: fn(Int) -> Int effects [stdio, ...E]) -> Int effects [stdio, ...E]\n",
            "  callback(1)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    assert_eq!(
        function
            .effect_binder
            .as_ref()
            .map(|binder| binder.name.as_str()),
        Some("E")
    );
    assert_eq!(
        function.effects.as_ref().unwrap(),
        &vec!["stdio".to_string(), "...E".to_string()]
    );
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "pub fn apply<effect E>(callback: fn(Int) -> Int effects [stdio, ...E]) -> Int effects [stdio, ...E]\n",
            "\tcallback(1)\n",
            "end\n",
        )
    );
}

#[test]
fn parses_and_formats_lexical_handler_declarations_and_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value()->Int\n",
            "end\n",
            "\n",
            "fn provide(ctx:Int)->Int\n",
            "  ctx\n",
            "end\n",
            "\n",
            "handler ask(ctx:Int) handles Ask effects [stdio]\n",
            "  value() => provide(ctx)\n",
            "end\n",
            "\n",
            "fn main() -> Int effects [stdio]\n",
            "  handle perform Ask::value() with ask(41)\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Handler(handler) = &output.tree.items[2] else {
        panic!("expected handler declaration");
    };
    assert_eq!(handler.name.as_deref(), Some("ask"));
    assert_eq!(handler.effect, vec!["Ask".to_string()]);
    assert_eq!(
        handler.operation_clauses[0].operation.as_deref(),
        Some("value")
    );
    let SyntaxItem::Function(function) = &output.tree.items[3] else {
        panic!("expected main function");
    };
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression body");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Handle { handler, args, body, .. }
            if handler == &vec!["ask".to_string()]
                && args.len() == 1
                && matches!(&body.kind, ExprKind::Perform { operation, .. } if operation == "value")
    ));
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "effect Ask\n",
            "\tvalue() -> Int\n",
            "end\n",
            "\n",
            "fn provide(ctx: Int) -> Int\n",
            "\tctx\n",
            "end\n",
            "\n",
            "handler ask(ctx: Int) handles Ask effects [stdio]\n",
            "\tvalue() => provide(ctx)\n",
            "end\n",
            "\n",
            "fn main() -> Int effects [stdio]\n",
            "\thandle perform Ask::value() with ask(41)\n",
            "end\n",
        )
    );
}

#[test]
fn handler_declaration_preserves_header_and_body_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub handler audit(ctx: Context) handles telemetry::Audit effects [stdio, net]\n",
            "  record(message) => ctx.record(message)\n",
            "\n",
            "  flush() => ctx.flush()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Handler(handler) = &output.tree.items[0] else {
        panic!("expected handler declaration");
    };
    assert_eq!(handler.visibility, Visibility::Public);
    assert_eq!(handler.name.as_deref(), Some("audit"));
    assert_eq!(handler.params.len(), 1);
    assert_eq!(handler.params[0].name, "ctx");
    assert_eq!(handler.effect, vec!["telemetry", "Audit"]);
    assert_eq!(
        handler.effects.as_deref(),
        Some(["stdio".to_string(), "net".to_string()].as_slice())
    );
    assert_eq!(handler.operation_clauses.len(), 2);
    assert_eq!(
        handler.operation_clauses[0].operation.as_deref(),
        Some("record")
    );
    assert_eq!(
        handler.operation_clauses[1].operation.as_deref(),
        Some("flush")
    );
    assert!(handler.end_present);
    assert_eq!(handler.span.start.line, 1);
    assert_eq!(handler.span.end.line, 6);
}

#[test]
fn rejects_trailing_comma_in_handler_operation_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Pick\n",
            "  next(step: Int) -> Int\n",
            "end\n",
            "\n",
            "handler pick() handles Pick\n",
            "  next(step,) => step\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(
        output.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "parse.handler_operation_parameter"
                && diagnostic.message == "handler operation parameter list cannot end with a comma"
                && diagnostic.parser_context == "handler_operation_clause"
        }),
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn rejects_old_handler_operation_syntax_with_one_migration_diagnostic() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "fn provide() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "handler ask() handles Ask\n",
            "  value = provide\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_eq!(output.diagnostics.len(), 1, "{:#?}", output.diagnostics);
    let diagnostic = &output.diagnostics[0];
    assert_eq!(diagnostic.id, "parse.handler_operation_old_syntax");
    assert_eq!(
        diagnostic.message,
        "handler operation clause must bind operation parameters with `(` and evaluate an expression with `=>`"
    );
    assert_eq!(diagnostic.parser_context, "handler_operation_clause");
    assert_eq!(diagnostic.unexpected.text, "=");
    assert_eq!(diagnostic.expected, vec!["("]);
    assert_eq!(
        diagnostic.recovery.strategy,
        RecoveryStrategy::SynchronizeToAnchor
    );
    assert_eq!(diagnostic.recovery.anchor.as_deref(), Some("newline"));
    assert_eq!(diagnostic.span.as_ref().unwrap().start.line, 10);
    assert_eq!(diagnostic.span.as_ref().unwrap().start.column, 9);
}

#[test]
fn rejects_effect_operation_parameter_without_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!("effect Audit\n", "  record(user) -> String\n", "end\n"),
    );

    let output = parse(&source);

    assert!(
        output.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "parse.effect_operation_parameter_type"
                && diagnostic.message == "effect operation parameter is missing a type annotation"
                && diagnostic
                    .span
                    .as_ref()
                    .is_some_and(|span| span.start.line == 2 && span.start.column == 10)
        }),
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn rejects_effect_declaration_without_operations() {
    let source = SourceFile::new("main.veln", concat!("effect Audit\n", "end\n"));

    let output = parse(&source);

    assert!(
        output.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "parse.effect_operation_required"
                && diagnostic.message == "effect declaration requires at least one operation"
        }),
        "{:#?}",
        output.diagnostics
    );
}
