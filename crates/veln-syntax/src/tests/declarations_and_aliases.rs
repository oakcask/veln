use super::*;

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
    assert_eq!(function_alias.target_spans.len(), 2);
    assert_eq!(function_alias.target_spans[1].start.column, 22);
    assert_eq!(function_alias.target_spans[1].end.column, 27);
    let SyntaxItem::PublicAlias(type_alias) = &output.tree.items[1] else {
        panic!("expected type alias");
    };
    assert_eq!(type_alias.kind, PublicAliasKind::Type);
    assert_eq!(type_alias.name.as_deref(), Some("Document"));
    assert_eq!(type_alias.target, vec!["impl", "Document"]);
    assert_eq!(type_alias.target_spans.len(), 2);
    assert_eq!(type_alias.target_spans[1].start.column, 27);
    assert_eq!(type_alias.target_spans[1].end.column, 35);
    let SyntaxItem::PublicAlias(schema_alias) = &output.tree.items[2] else {
        panic!("expected schema alias");
    };
    assert_eq!(schema_alias.kind, PublicAliasKind::Schema);
    assert_eq!(schema_alias.name.as_deref(), Some("Packet"));
    assert_eq!(schema_alias.target, vec!["impl", "Packet"]);
    assert_eq!(schema_alias.target_spans.len(), 2);
    assert_eq!(schema_alias.target_spans[1].start.column, 27);
    assert_eq!(schema_alias.target_spans[1].end.column, 33);
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
