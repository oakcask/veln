use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use veln_source::SourceFile;

fn first_function(output: &ParseOutput) -> &FunctionDecl {
    match &output.tree.items[0] {
        SyntaxItem::Function(function) => function,
        SyntaxItem::Effect(_)
        | SyntaxItem::Handler(_)
        | SyntaxItem::Type(_)
        | SyntaxItem::Schema(_)
        | SyntaxItem::Codec(_)
        | SyntaxItem::PublicAlias(_) => {
            panic!("expected function item")
        }
    }
}

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
            if matches!(&callee.kind, ExprKind::NamePath(segments)
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
fn parameter_and_result_binding_name_spans_cover_only_written_names() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn exact(Bad: Int, _alsoBad: String) -> ResultName: Int\n",
            "  1\n",
            "end\n",
            "\n",
            "handler serve(Bad: Int) handles Audit\n",
            "  save(_Entry) => _Entry\n",
            "end\n",
        ),
    );

    let output = parse(&source);
    let SyntaxItem::Function(function) = &output.tree.items[0] else {
        panic!("expected function declaration");
    };
    assert_eq!(
        (
            function.params[0].name_span.start.column,
            function.params[0].name_span.end.column,
        ),
        (10, 13)
    );
    assert_eq!(
        (
            function.params[1].name_span.start.column,
            function.params[1].name_span.end.column,
        ),
        (20, 28)
    );
    assert_eq!(
        function
            .return_binding
            .as_ref()
            .map(|binding| (binding.name_span.start.column, binding.name_span.end.column,)),
        Some((41, 51))
    );
    let SyntaxItem::Handler(handler) = &output.tree.items[1] else {
        panic!("expected handler declaration");
    };
    assert_eq!(
        (
            handler.params[0].name_span.start.column,
            handler.params[0].name_span.end.column,
        ),
        (15, 18)
    );
    assert_eq!(
        (
            handler.operation_clauses[0].params[0]
                .name_span
                .start
                .column,
            handler.operation_clauses[0].params[0].name_span.end.column,
        ),
        (8, 14)
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

#[test]
fn lexes_binary_and_hexadecimal_integer_candidates_as_complete_tokens() {
    let source = SourceFile::new("main.veln", "0b00101 0x00Cafe 0b102 0Xff 0x1.2 0b10_01 0x");

    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Eof))
        .map(|token| (token.kind, token.text))
        .collect::<Vec<_>>();

    assert_eq!(
        tokens,
        vec![
            (TokenKind::Int, "0b00101".to_string()),
            (TokenKind::Int, "0x00Cafe".to_string()),
            (TokenKind::MalformedInt, "0b102".to_string()),
            (TokenKind::MalformedInt, "0Xff".to_string()),
            (TokenKind::MalformedInt, "0x1.2".to_string()),
            (TokenKind::MalformedInt, "0b10_01".to_string()),
            (TokenKind::MalformedInt, "0x".to_string()),
        ]
    );
}

#[test]
fn number_tokens_preserve_fraction_and_member_access_boundaries() {
    let source = SourceFile::new("numbers.veln", "42.5 42.member 0b10.1 0b10.member 0xCafe+1");

    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Eof))
        .map(|token| (token.kind, token.text))
        .collect::<Vec<_>>();

    assert_eq!(
        tokens,
        vec![
            (TokenKind::Float, "42.5".to_string()),
            (TokenKind::Int, "42".to_string()),
            (TokenKind::Dot, ".".to_string()),
            (TokenKind::Ident, "member".to_string()),
            (TokenKind::MalformedInt, "0b10.1".to_string()),
            (TokenKind::Int, "0b10".to_string()),
            (TokenKind::Dot, ".".to_string()),
            (TokenKind::Ident, "member".to_string()),
            (TokenKind::Int, "0xCafe".to_string()),
            (TokenKind::Plus, "+".to_string()),
            (TokenKind::Int, "1".to_string()),
        ]
    );
}

#[test]
fn lexes_compound_operators_with_longest_matching_tokens() {
    let source = SourceFile::new(
        "operators.veln",
        "-> => :: == != <= << >= >>> >> > |> | & ^ ~",
    );

    let tokens = lex(&source)
        .tokens
        .into_iter()
        .filter(|token| !matches!(token.kind, TokenKind::Whitespace | TokenKind::Eof))
        .map(|token| (token.kind, token.text))
        .collect::<Vec<_>>();

    assert_eq!(
        tokens,
        vec![
            (TokenKind::Arrow, "->".to_string()),
            (TokenKind::FatArrow, "=>".to_string()),
            (TokenKind::DoubleColon, "::".to_string()),
            (TokenKind::EqualEqual, "==".to_string()),
            (TokenKind::BangEqual, "!=".to_string()),
            (TokenKind::LessEqual, "<=".to_string()),
            (TokenKind::ShiftLeft, "<<".to_string()),
            (TokenKind::GreaterEqual, ">=".to_string()),
            (TokenKind::ShiftRightLogical, ">>>".to_string()),
            (TokenKind::ShiftRight, ">>".to_string()),
            (TokenKind::Greater, ">".to_string()),
            (TokenKind::PipeGreater, "|>".to_string()),
            (TokenKind::Pipe, "|".to_string()),
            (TokenKind::Ampersand, "&".to_string()),
            (TokenKind::Caret, "^".to_string()),
            (TokenKind::Tilde, "~".to_string()),
        ]
    );
}

#[test]
fn nested_generic_closers_remain_type_syntax_next_to_shift_operators() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn choose(values: List<Receiver<Int>>) -> Result<Option<Int>, String>\n",
            "  Ok(Some((8 >> 1) + (8 >>> 1)))\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        first_function(&output).params[0].ty.as_deref(),
        Some("List<Receiver<Int>>")
    );
}

#[test]
fn reports_one_focused_diagnostic_for_each_malformed_prefixed_integer() {
    let cases = [
        (
            "0b",
            "binary integer literal requires at least one digit",
            3,
            5,
        ),
        ("0b102", "`2` is not a valid binary integer digit", 7, 8),
        ("0xg1", "`g` is not a valid hexadecimal integer digit", 5, 6),
        (
            "0B10",
            "uppercase binary integer literal prefix is unsupported",
            4,
            5,
        ),
        (
            "0x1_0",
            "digit separators are not supported in hexadecimal integer literals",
            6,
            7,
        ),
        (
            "0b1.0",
            "binary floating-point literals are unsupported",
            3,
            8,
        ),
        (
            "0x8000000000000000",
            "hexadecimal integer literal exceeds the maximum Int value 9223372036854775807",
            3,
            21,
        ),
    ];

    for (literal, message, start_column, end_column) in cases {
        let source = SourceFile::new("main.veln", format!("fn main() -> Int\n  {literal}\nend\n"));
        let output = parse(&source);
        assert_eq!(
            output.diagnostics.len(),
            1,
            "{literal}: {:#?}",
            output.diagnostics
        );
        let diagnostic = &output.diagnostics[0];
        assert_eq!(diagnostic.id, "parse.integer_literal", "{literal}");
        assert_eq!(diagnostic.message, message, "{literal}");
        let span = diagnostic.span.as_ref().unwrap();
        assert_eq!(
            (span.start.line, span.start.column, span.end.column),
            (2, start_column, end_column),
            "{literal}"
        );
    }
}

#[test]
fn formatter_preserves_prefixed_integer_spelling_in_expressions_and_patterns() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn classify(value: Int) -> Int\n",
            "match value\n",
            "0x0A=>0b001010\n",
            "_=>0xCafe\n",
            "end\n",
            "end\n",
        ),
    );
    let output = parse(&source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn classify(value: Int) -> Int\n",
            "\tmatch value\n",
            "\t\t0x0A => 0b001010\n",
            "\t\t_ => 0xCafe\n",
            "\tend\n",
            "end\n",
        )
    );
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
fn parses_and_formats_variadic_parameter_marker() {
    let source = SourceFile::new(
        "main.veln",
        "fn collect(prefix: String, values: ...String) -> String\n  prefix\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    assert!(!function.params[0].is_variadic);
    assert!(function.params[1].is_variadic);
    assert_eq!(function.params[1].ty.as_deref(), Some("String"));
    assert_eq!(
        format_tree(&output.tree),
        "fn collect(prefix: String, values: ...String) -> String\n\tprefix\nend\n"
    );
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
            "pub schema Packet = impl::Packet\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tree.items.len(), 3);
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
    let SyntaxItem::PublicAlias(schema_alias) = &output.tree.items[2] else {
        panic!("expected schema alias");
    };
    assert_eq!(schema_alias.kind, PublicAliasKind::Schema);
    assert_eq!(schema_alias.name.as_deref(), Some("Packet"));
    assert_eq!(schema_alias.target, vec!["impl", "Packet"]);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "mod spec.api\n",
            "use spec.impl\n",
            "\n",
            "pub fn parse = impl::parse\n",
            "\n",
            "pub type Document = impl::Document\n",
            "\n",
            "pub schema Packet = impl::Packet\n",
        )
    );
}

#[test]
fn parses_underscore_led_function_and_type_alias_recovery_names() {
    let source = SourceFile::new(
        "api.veln",
        concat!(
            "mod spec.api\n",
            "use spec.impl\n",
            "\n",
            "pub fn _parse = impl::parse\n",
            "pub type _Document = impl::Document\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tree.items.len(), 2);
    let SyntaxItem::PublicAlias(function_alias) = &output.tree.items[0] else {
        panic!("expected function alias");
    };
    assert_eq!(function_alias.kind, PublicAliasKind::Function);
    assert_eq!(function_alias.name.as_deref(), Some("_parse"));
    let SyntaxItem::PublicAlias(type_alias) = &output.tree.items[1] else {
        panic!("expected type alias");
    };
    assert_eq!(type_alias.kind, PublicAliasKind::Type);
    assert_eq!(type_alias.name.as_deref(), Some("_Document"));
}

#[test]
fn dispatches_mixed_public_and_private_top_level_declarations_in_source_order() {
    let source = SourceFile::new(
        "mixed.veln",
        concat!(
            "pub fn public_fn() -> ()\n  ()\nend\n",
            "fn private_fn() -> ()\n  ()\nend\n",
            "test parser_test() -> ()\n  ()\nend\n",
            "pub type PublicType\n  PublicValue\nend\n",
            "type PrivateType\n  PrivateValue\nend\n",
            "pub schema PublicSchema\n  format binary\n  value: UInt8\nend\n",
            "schema PrivateSchema\n  format binary\n  value: UInt8\nend\n",
            "pub effect PublicEffect\n  call() -> ()\nend\n",
            "effect PrivateEffect\n  call() -> ()\nend\n",
            "pub handler public_handler() handles PublicEffect\n  call() => public_fn()\nend\n",
            "handler private_handler() handles PrivateEffect\n  call() => private_fn()\nend\n",
            "pub fn alias = implementation::function\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let item_kinds = output
        .tree
        .items
        .iter()
        .map(|item| match item {
            SyntaxItem::Function(function) => match function.kind {
                FunctionKind::Function => "function",
                FunctionKind::Test => "test",
            },
            SyntaxItem::Type(_) => "type",
            SyntaxItem::Schema(_) => "schema",
            SyntaxItem::Effect(_) => "effect",
            SyntaxItem::Handler(_) => "handler",
            SyntaxItem::PublicAlias(_) => "alias",
            SyntaxItem::Codec(_) => "codec",
        })
        .collect::<Vec<_>>();
    assert_eq!(
        item_kinds,
        [
            "function", "function", "test", "type", "type", "schema", "schema", "effect", "effect",
            "handler", "handler", "alias",
        ]
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
fn parses_and_formats_schema_decode_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode wire::PacketWire from view at base\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression body line");
    };
    let ExprKind::SchemaDecode {
        schema,
        input,
        base,
    } = &expr.kind
    else {
        panic!("expected schema decode expression");
    };
    assert_eq!(schema, &vec!["wire".to_string(), "PacketWire".to_string()]);
    assert!(
        matches!(input.kind, ExprKind::NamePath(ref segments) if segments == &vec!["view".to_string()])
    );
    assert!(
        matches!(base.kind, ExprKind::NamePath(ref segments) if segments == &vec!["base".to_string()])
    );
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{ length : Int }>\n",
            "\tdecode wire::PacketWire from view at base\n",
            "end\n",
        )
    );
}

#[test]
fn rejects_schema_decode_expression_missing_at() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode PacketWire from view base\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.schema_decode_expression")
        .expect("expected schema decode expression diagnostic");
    assert_eq!(
        diagnostic.message,
        "schema decode expression is missing `at`"
    );
    assert_eq!(diagnostic.expected, vec!["at"]);
}

#[test]
fn parses_and_formats_schema_encode_expression() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn packet(value: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode wire::PacketWire from value\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression body line");
    };
    let ExprKind::SchemaEncode { schema, value } = &expr.kind else {
        panic!("expected schema encode expression");
    };
    assert_eq!(schema, &vec!["wire".to_string(), "PacketWire".to_string()]);
    assert!(
        matches!(value.kind, ExprKind::NamePath(ref segments) if segments == &vec!["value".to_string()])
    );
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn packet(value: { length : Int }) -> Result<ByteChunk, EncodeError>\n",
            "\tencode wire::PacketWire from value\n",
            "end\n",
        )
    );
}

#[test]
fn rejects_schema_encode_expression_missing_from() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn packet(value: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PacketWire value\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    let diagnostic = output
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "parse.schema_encode_expression")
        .expect("expected schema encode expression diagnostic");
    assert_eq!(
        diagnostic.message,
        "schema encode expression is missing `from`"
    );
    assert_eq!(diagnostic.expected, vec!["from"]);
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
fn parses_schema_declarations_and_formats_canonical_layout() {
    let source = SourceFile::new(
        "schema.veln",
        concat!(
            "pub schema Http2FrameHeader\n",
            "  format binary\n",
            "  length: UInt24be\n",
            "  kind: UInt8\n",
            "  padding_length: UInt8 where padding_length <= length\n",
            "  stream_reserved: ReservedBits( 1,0 )\n",
            "  stream_id: UInt31be\n",
            "  settings: Repeat( length - padding_length , UInt16be )\n",
            "  canonical_settings: [ uint16be ; length - padding_length ]\n",
            "  payload: ByteView(length - padding_length)\n",
            "  aligned_payload: ByteView(length) where payload_count multiple of padding_length\n",
            "  validate padding_length <= length\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Schema(schema) = &output.tree.items[0] else {
        panic!("expected schema declaration");
    };
    assert_eq!(schema.visibility, Visibility::Public);
    assert_eq!(schema.name.as_deref(), Some("Http2FrameHeader"));
    assert_eq!(
        schema.format.as_ref().map(|format| format.name.as_str()),
        Some("binary")
    );
    assert_eq!(schema.fields.len(), 9);
    assert_eq!(schema.fields[0].name, "length");
    assert_eq!(schema.fields[0].ty, "UInt24be");
    assert_eq!(schema.fields[1].name, "kind");
    assert_eq!(schema.fields[1].ty, "UInt8");
    assert_eq!(schema.fields[2].name, "padding_length");
    assert_eq!(schema.fields[2].ty, "UInt8");
    let where_clause = schema.fields[2]
        .where_clause
        .as_ref()
        .expect("field should carry where clause");
    assert_eq!(where_clause.predicate, "padding_length <= length");
    assert_eq!(schema.fields[3].name, "stream_reserved");
    assert_eq!(schema.fields[3].ty, "ReservedBits(1, 0)");
    assert_eq!(schema.fields[4].name, "stream_id");
    assert_eq!(schema.fields[4].ty, "UInt31be");
    assert_eq!(schema.fields[5].name, "settings");
    assert_eq!(
        schema.fields[5].ty,
        "Repeat(length - padding_length, UInt16be)"
    );
    assert_eq!(schema.fields[6].name, "canonical_settings");
    assert_eq!(schema.fields[6].ty, "[uint16be; length - padding_length]");
    assert_eq!(schema.fields[7].name, "payload");
    assert_eq!(schema.fields[7].ty, "ByteView(length - padding_length)");
    assert_eq!(schema.fields[8].name, "aligned_payload");
    assert_eq!(schema.fields[8].ty, "ByteView(length)");
    assert_eq!(
        schema.fields[8]
            .where_clause
            .as_ref()
            .map(|where_clause| where_clause.predicate.as_str()),
        Some("payload_count multiple of padding_length")
    );
    assert_eq!(schema.validations.len(), 1);
    assert_eq!(schema.validations[0].predicate, "padding_length <= length");
    assert!(schema.end_present);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "pub schema Http2FrameHeader\n",
            "\tformat binary\n",
            "\n",
            "\tlength: uint24be\n",
            "\tkind: uint8\n",
            "\tpadding_length: uint8 where padding_length <= length\n",
            "\tstream_reserved: uint1 reserves 0\n",
            "\tstream_id: uint31be\n",
            "\tsettings: [uint16be; length - padding_length]\n",
            "\tcanonical_settings: [uint16be; length - padding_length]\n",
            "\tpayload: ByteView(length - padding_length)\n",
            "\taligned_payload: ByteView(length) where payload_count multiple of padding_length\n",
            "\n",
            "\tvalidate padding_length <= length\n",
            "end\n",
        )
    );
}

#[test]
fn format_tree_canonicalizes_binary_schema_compatibility_primitives() {
    let source = SourceFile::new(
        "schema.veln",
        concat!(
            "schema Wire\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  flags: UInt16le\n",
            "  padding: ReservedBits( 16 , 43981 )\n",
            "  values: Repeat( count , UInt24be )\n",
            "  payload: Dispatch( count, 1 => UInt8, 2 => ReservedBits(16, 43981), 3 => UInt8 )\n",
            "  wrapped: List<UInt8>\n",
            "  qualified: wire::UInt8\n",
            "end\n",
            "\n",
            "schema Neutral\n",
            "  value: UInt8\n",
            "  qualified: wire::UInt8\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "schema Wire\n",
            "\tformat binary\n",
            "\n",
            "\tcount: uint8\n",
            "\tflags: uint16le\n",
            "\tpadding: uint16be reserves 43981\n",
            "\tvalues: [uint24be; count]\n",
            "\tpayload: Dispatch(count, 1 => uint8, 2 => uint16be reserves 43981, 3 => uint8)\n",
            "\twrapped: List<UInt8>\n",
            "\tqualified: wire::UInt8\n",
            "end\n",
            "\n",
            "schema Neutral\n",
            "\n",
            "\tvalue: UInt8\n",
            "\tqualified: wire::UInt8\n",
            "end\n",
        )
    );
}

#[test]
fn repeated_schema_field_syntax_requires_semicolon() {
    let source = SourceFile::new(
        "schema.veln",
        concat!(
            "schema Packet\n",
            "  format binary\n",
            "  count: uint8\n",
            "  items: [uint16be count]\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(
        output.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "parse.schema_repeat_semicolon"
                && diagnostic.message
                    == "expected `;` between repeated schema payload and count expression"
                && diagnostic.expected == vec![";"]
        }),
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn parses_schema_fields_without_format_clause() {
    let source = SourceFile::new(
        "schema.veln",
        concat!(
            "schema Metadata\n",
            "  version: Int\n",
            "  title: String\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Schema(schema) = &output.tree.items[0] else {
        panic!("expected schema declaration");
    };
    assert_eq!(schema.name.as_deref(), Some("Metadata"));
    assert!(schema.format.is_none());
    assert_eq!(schema.fields.len(), 2);
    assert_eq!(schema.fields[0].name, "version");
    assert_eq!(schema.fields[0].ty, "Int");
    assert_eq!(schema.fields[1].name, "title");
    assert_eq!(schema.fields[1].ty, "String");
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "schema Metadata\n",
            "\n",
            "\tversion: Int\n",
            "\ttitle: String\n",
            "end\n",
        )
    );
}

#[test]
fn parses_and_formats_nominal_schema_field_references_without_new_syntax() {
    let source = SourceFile::new(
        "schema.veln",
        concat!(
            "use wire\n",
            "schema Envelope\n",
            "  metadata: LocalMetadata\n",
            "  imported_metadata: wire::MetadataAlias\n",
            "end\n",
            "schema Frame\n",
            "  format binary\n",
            "  header: LocalHeader\n",
            "  imported_header: wire::HeaderAlias\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Schema(envelope) = &output.tree.items[0] else {
        panic!("expected format-neutral schema declaration");
    };
    assert_eq!(envelope.fields[0].ty, "LocalMetadata");
    assert_eq!(envelope.fields[1].ty, "wire::MetadataAlias");
    let SyntaxItem::Schema(frame) = &output.tree.items[1] else {
        panic!("expected binary schema declaration");
    };
    assert_eq!(frame.fields[0].ty, "LocalHeader");
    assert_eq!(frame.fields[1].ty, "wire::HeaderAlias");
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "use wire\n",
            "\n",
            "schema Envelope\n",
            "\n",
            "\tmetadata: LocalMetadata\n",
            "\timported_metadata: wire::MetadataAlias\n",
            "end\n",
            "\n",
            "schema Frame\n",
            "\tformat binary\n",
            "\n",
            "\theader: LocalHeader\n",
            "\timported_header: wire::HeaderAlias\n",
            "end\n",
        )
    );
}

#[test]
fn rejects_qualified_codec_schema_references() {
    let source = SourceFile::new(
        "codec.veln",
        concat!(
            "codec ImportedHeader for wire::Http2FrameHeader decode\n",
            "  derive decode\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_eq!(output.diagnostics.len(), 1, "{:#?}", output.diagnostics);
    assert_eq!(output.diagnostics[0].id, "parse.codec_declaration_removed");
    assert_eq!(output.diagnostics[0].parser_context, "codec_declaration");
    assert!(output.tree.items.is_empty());
}

#[test]
fn reports_schema_declaration_syntax_diagnostics() {
    let source = SourceFile::new(
        "bad.veln",
        concat!(
            "schema BadHeader\n",
            "  length: UInt24be\n",
            "  format binary\n",
            "  format binary\n",
            "  _reserved: UInt8\n",
            "  broken: UInt8 where\n",
            "  map to EmptyHeader\n",
            "  map to OtherHeader when length == 1\n",
            "    length = length\n",
            "    length = kind\n",
            "    = stream_id\n",
            "    stream_reserved\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.schema_field_before_format"),
        "{:#?}",
        output.diagnostics
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.schema_multiple_format"),
        "{:#?}",
        output.diagnostics
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.schema_field_name"),
        "{:#?}",
        output.diagnostics
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "parse.schema_field_where"),
        "{:#?}",
        output.diagnostics
    );
    assert!(
        output
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "parse.schema_mapping_removed")
            .count()
            == 2,
        "{:#?}",
        output.diagnostics
    );
}

#[test]
fn rejects_schema_mapping_clauses() {
    let source = SourceFile::new(
        "mapping.veln",
        concat!(
            "schema HeaderWire\n",
            "\tformat binary\n",
            "\tlength: UInt16be\n",
            "\tkind: UInt8\n",
            "\n",
            "\tmap to Header\n",
            "\t\tlength = length\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_eq!(output.diagnostics.len(), 1, "{:#?}", output.diagnostics);
    assert_eq!(output.diagnostics[0].id, "parse.schema_mapping_removed");
}

#[test]
fn schema_body_recovery_preserves_later_clauses_and_declarations() {
    let source = SourceFile::new(
        "schema.veln",
        concat!(
            "schema Packet\n",
            "  format binary\n",
            "  length: uint8\n",
            "  map to LegacyPacket\n",
            "    length = length\n",
            "  payload: ByteView(length)\n",
            "  validate length >= 0\n",
            "end\n",
            "fn packet_count() -> result: Int\n",
            "  1\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_eq!(output.diagnostics.len(), 1, "{:#?}", output.diagnostics);
    assert_eq!(output.diagnostics[0].id, "parse.schema_mapping_removed");
    let SyntaxItem::Schema(schema) = &output.tree.items[0] else {
        panic!("expected schema declaration");
    };
    assert_eq!(schema.fields.len(), 2);
    assert_eq!(schema.fields[1].name, "payload");
    assert_eq!(schema.validations.len(), 1);
    assert!(schema.end_present);
    assert!(matches!(output.tree.items[1], SyntaxItem::Function(_)));
}

#[test]
fn rejects_codec_declarations_with_migration_diagnostic() {
    let source = SourceFile::new(
        "codec.veln",
        concat!(
            "pub  codec   Http2FrameHeaderCodec for  Http2FrameHeader   decode   encode\n",
            "  derive   decode\n",
            "  encode   with encode_header\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_eq!(output.diagnostics.len(), 1, "{:#?}", output.diagnostics);
    let diagnostic = &output.diagnostics[0];
    assert_eq!(diagnostic.id, "parse.codec_declaration_removed");
    assert_eq!(
        diagnostic.message,
        "codec declarations are no longer accepted; use ordinary functions plus explicit schema decode and encode expressions"
    );
    assert_eq!(diagnostic.unexpected.text, "codec");
    assert_eq!(diagnostic.recovery.anchor.as_deref(), Some("end"));
    assert!(output.tree.items.is_empty());
}

#[test]
fn reports_one_removed_codec_diagnostic_per_codec_declaration() {
    let source = SourceFile::new(
        "bad.veln",
        concat!(
            "codec Empty for Header\n",
            "end\n",
            "\n",
            "codec DuplicateDirection for Header decode decode\n",
            "  derive decode\n",
            "end\n",
            "\n",
            "codec UnknownDirection for Header decode inspect\n",
            "  derive decode\n",
            "end\n",
            "\n",
            "codec MissingImplementation for Header decode encode\n",
            "  derive decode\n",
            "end\n",
            "\n",
            "codec UnlistedImplementation for Header decode\n",
            "  derive decode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "codec DuplicateImplementation for Header decode\n",
            "  derive decode\n",
            "  decode with decode_header\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_eq!(output.diagnostics.len(), 6, "{:#?}", output.diagnostics);
    assert!(output.diagnostics.iter().all(|diagnostic| {
        diagnostic.id == "parse.codec_declaration_removed"
            && diagnostic.parser_context == "codec_declaration"
    }));
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
fn rejects_hole_tokens_in_qualified_expression_paths() {
    let cases = [
        (
            "qualified expression path",
            "fn main(module) -> ()\n  module::_name()\nend\n",
        ),
        (
            "perform effect path",
            "fn main() -> ()\n  perform _Effect::op()\nend\n",
        ),
        (
            "handler path",
            "fn main(body) -> ()\n  handle body with _handler()\nend\n",
        ),
    ];

    for (name, text) in cases {
        let source = SourceFile::new(format!("{name}.veln"), text);
        let output = parse(&source);

        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "parse.name_path"),
            "{name} should reject underscore-led path segments: {:#?}",
            output.diagnostics
        );
    }
}

#[test]
fn preserves_contextual_keyword_segments_in_paths_and_alias_targets() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "use handler\n",
            "\n",
            "pub fn alias = handler::target\n",
            "\n",
            "fn main(body: ()) -> ()\n",
            "  module::handler()\n",
            "  perform handler::op()\n",
            "  handle body with handler()\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tree.uses[0].name, "handler");
    let SyntaxItem::PublicAlias(alias) = &output.tree.items[0] else {
        panic!("expected public alias");
    };
    assert_eq!(alias.target, vec!["handler", "target"]);
    let SyntaxItem::Function(function) = &output.tree.items[1] else {
        panic!("expected function");
    };
    let BodyLine::Expr { expr, .. } = &function.body[0] else {
        panic!("expected expression");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Call { callee, .. }
            if matches!(&callee.kind, ExprKind::NamePath(segments)
                if segments == &vec!["module".to_string(), "handler".to_string()])
    ));
    let BodyLine::Expr { expr, .. } = &function.body[1] else {
        panic!("expected expression");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Perform { effect, operation, .. }
            if effect == &vec!["handler".to_string()] && operation == "op"
    ));
    let BodyLine::Expr { expr, .. } = &function.body[2] else {
        panic!("expected expression");
    };
    assert!(matches!(
        &expr.kind,
        ExprKind::Handle { handler, .. } if handler == &vec!["handler".to_string()]
    ));
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

#[test]
fn lexes_number_string_hole_and_invalid_boundaries() {
    let source = SourceFile::new(
        "tokens.veln",
        r#"1 1.5 1.foo "a\"b" @ test where if else at _ _name
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
            (TokenKind::Where, "where".to_string()),
            (TokenKind::If, "if".to_string()),
            (TokenKind::Else, "else".to_string()),
            (TokenKind::At, "at".to_string()),
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
        (TokenKind::Type, "type"),
        (TokenKind::Schema, "schema"),
        (TokenKind::Format, "format"),
        (TokenKind::Where, "where"),
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
        (TokenKind::At, "at"),
        (TokenKind::Match, "match"),
        (TokenKind::If, "if"),
        (TokenKind::Else, "else"),
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
