use super::*;

#[test]
fn parses_and_formats_typed_module_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let defaults : { state : State, attempts : Int } = {\n",
            " state: Idle,\n",
            " attempts: 3,\n",
            "}\n",
            "pub let initial: State = Active(3)\n",
            "let callsite: Int = 1\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::ModuleValue(defaults) = &output.tree.items[0] else {
        panic!("expected private module value");
    };
    assert_eq!(defaults.visibility, Visibility::Private);
    assert_eq!(defaults.name.as_deref(), Some("defaults"));
    assert_eq!(defaults.annotation, "{ state : State, attempts : Int }");
    assert!(matches!(defaults.initializer.kind, ExprKind::Record(_)));
    let SyntaxItem::ModuleValue(initial) = &output.tree.items[1] else {
        panic!("expected public module value");
    };
    assert_eq!(initial.visibility, Visibility::Public);
    let SyntaxItem::ModuleValue(contextual_name) = &output.tree.items[2] else {
        panic!("expected module value with contextual binding name");
    };
    assert_eq!(contextual_name.name.as_deref(), Some("callsite"));

    let formatted = format_tree(&output.tree);
    assert_eq!(
        formatted,
        concat!(
            "let defaults: { state : State, attempts : Int } = { state: Idle, attempts: 3 }\n",
            "\n",
            "pub let initial: State = Active(3)\n",
            "\n",
            "let callsite: Int = 1\n",
        )
    );
    let reparsed = parse(&SourceFile::new("formatted.veln", formatted.clone()));
    assert!(
        reparsed.diagnostics.is_empty(),
        "{:#?}",
        reparsed.diagnostics
    );
    assert_eq!(format_tree(&reparsed.tree), formatted);
}

#[test]
fn rejects_malformed_module_values_without_consuming_following_item() {
    for declaration in [
        "let missing = 1",
        "let empty: = 1",
        "let absent: Int =",
        "let _: Int = 1",
        "let _hidden: Int = 1",
        "let { value: value }: Int = { value: 1 }",
        "let Some(value): Int = Some(1)",
        "let generic<T>: Int = 1",
        "let Upper: Int = 1",
    ] {
        let source = SourceFile::new("main.veln", format!("{declaration}\nfn survivor()\nend\n"));
        let output = parse(&source);
        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "parse.module_value_declaration"),
            "{declaration}: {:#?}",
            output.diagnostics
        );
        assert!(matches!(
            output.tree.items.last(),
            Some(SyntaxItem::Function(function))
                if function.name.as_deref() == Some("survivor")
        ));
    }
}

#[test]
fn module_value_names_use_the_binding_identifier_class() {
    for (rejected_name, following_visibility) in [
        ("handle", Visibility::Private),
        ("handler", Visibility::Public),
    ] {
        let following_prefix = if following_visibility == Visibility::Public {
            "pub "
        } else {
            ""
        };
        let text =
            format!("let {rejected_name}: Int = 1\n{following_prefix}let survivor: Int = 2\n");
        let source = SourceFile::new("main.veln", text.clone());
        let output = parse(&source);

        assert!(
            output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "parse.module_value_declaration"),
            "{rejected_name}: {:#?}",
            output.diagnostics
        );
        assert_eq!(output.tree.items.len(), 2, "{rejected_name}");
        let SyntaxItem::ModuleValue(rejected) = &output.tree.items[0] else {
            panic!("expected recovered module value for {rejected_name}");
        };
        assert_eq!(rejected.name, None, "{rejected_name}");
        let SyntaxItem::ModuleValue(survivor) = &output.tree.items[1] else {
            panic!("expected following module value for {rejected_name}");
        };
        assert_eq!(survivor.visibility, following_visibility, "{rejected_name}");
        assert_eq!(
            survivor.name.as_deref(),
            Some("survivor"),
            "{rejected_name}"
        );
        let lossless = output
            .tree
            .lossless_tokens()
            .filter(|token| token.kind != TokenKind::Eof)
            .map(|token| token.text.as_str())
            .collect::<String>();
        assert_eq!(lossless, text, "{rejected_name}");
    }

    for accepted_name in ["ordinary", "callsite"] {
        let source = SourceFile::new("main.veln", format!("let {accepted_name}: Int = 1\n"));
        let output = parse(&source);

        assert!(
            output.diagnostics.is_empty(),
            "{accepted_name}: {:#?}",
            output.diagnostics
        );
        assert!(matches!(
            output.tree.items.as_slice(),
            [SyntaxItem::ModuleValue(value)] if value.name.as_deref() == Some(accepted_name)
        ));
    }
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
fn collects_nested_type_paths_without_effect_names() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn transform(input: outer::Map<domain::Key, List<domain::Value> >) ",
            "-> service::Result<domain::Item, Error> effects [io::Read]\n",
            "  input\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    assert_eq!(
        function.params[0]
            .ty_paths
            .iter()
            .map(|path| (path.written.as_str(), path.type_argument_count))
            .collect::<Vec<_>>(),
        [
            ("outer::Map<domain::Key, List<domain::Value>>", 2),
            ("domain::Key", 0),
            ("List<domain::Value>", 1),
            ("domain::Value", 0),
        ]
    );
    assert_eq!(
        function
            .return_type_paths
            .iter()
            .map(|path| (path.written.as_str(), path.type_argument_count))
            .collect::<Vec<_>>(),
        [
            ("service::Result<domain::Item, Error>", 2),
            ("domain::Item", 0),
            ("Error", 0),
        ]
    );
    assert_eq!(
        function.effects.as_deref(),
        Some(&["io::Read".to_string()][..])
    );
}

#[test]
fn parses_and_formats_callsite_modifier_after_optional_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn traced() -> SourceLocation effects [stdio] callsite\n",
            "  callsite\n",
            "end\n",
            "fn plain(callsite: Int) -> Int\n",
            "  callsite\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(traced) = &output.tree.items[0] else {
        panic!("expected function item");
    };
    assert!(traced.callsite.is_some());
    let SyntaxItem::Function(plain) = &output.tree.items[1] else {
        panic!("expected function item");
    };
    assert!(plain.callsite.is_none());
    assert_eq!(plain.params[0].name, "callsite");
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "pub fn traced() -> SourceLocation effects [stdio] callsite\n",
            "\tcallsite\n",
            "end\n",
            "\n",
            "fn plain(callsite: Int) -> Int\n",
            "\tcallsite\n",
            "end\n",
        )
    );
}

#[test]
fn parses_callsite_modifier_without_effects_and_rejects_a_duplicate() {
    let source = SourceFile::new(
        "main.veln",
        "fn located() -> SourceLocation callsite callsite\n  callsite\nend\n",
    );

    let output = parse(&source);

    let function = first_function(&output);
    assert_eq!(function.return_type.as_deref(), Some("SourceLocation"));
    assert!(function.callsite.is_some());
    assert_eq!(output.diagnostics.len(), 1, "{:#?}", output.diagnostics);
    let diagnostic = &output.diagnostics[0];
    assert_eq!(diagnostic.id, "parse.duplicate_callsite_modifier");
    assert_eq!(diagnostic.span.as_ref().unwrap().start.column, 41);
    assert_eq!(diagnostic.repair_candidates.len(), 1);
}

#[test]
fn reports_each_extra_callsite_modifier_with_an_independent_repair() {
    let source = SourceFile::new(
        "main.veln",
        "fn located() -> SourceLocation callsite callsite callsite\n  callsite\nend\n",
    );

    let output = parse(&source);

    let function = first_function(&output);
    assert!(function.callsite.is_some());
    assert_eq!(output.diagnostics.len(), 2, "{:#?}", output.diagnostics);
    for diagnostic in &output.diagnostics {
        assert_eq!(diagnostic.id, "parse.duplicate_callsite_modifier");
        assert_eq!(diagnostic.repair_candidates.len(), 1);
        assert_eq!(diagnostic.repair_candidates[0].edits.len(), 1);
    }
    assert_eq!(
        output.diagnostics[0].span.as_ref().unwrap().start.column,
        41
    );
    assert_eq!(
        output.diagnostics[1].span.as_ref().unwrap().start.column,
        50
    );
}

#[test]
fn preserves_contextual_callsite_in_qualified_return_type() {
    let source = SourceFile::new(
        "main.veln",
        "fn qualified() -> callsite::Type\n  value\nend\n",
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    assert_eq!(function.return_type.as_deref(), Some("callsite::Type"));
    assert!(function.callsite.is_none());
    assert_eq!(
        format_tree(&output.tree),
        "fn qualified() -> callsite::Type\n\tvalue\nend\n"
    );
}

#[test]
fn preserves_contextual_callsite_in_bare_return_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn plain(value: callsite) -> callsite\n",
            "  value\n",
            "end\n",
            "fn located(value: callsite) -> callsite callsite\n",
            "  value\n",
            "end\n",
            "fn callback() -> fn() -> callsite\n",
            "  callback\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let SyntaxItem::Function(plain) = &output.tree.items[0] else {
        panic!("expected function item");
    };
    assert_eq!(plain.return_type.as_deref(), Some("callsite"));
    assert!(plain.callsite.is_none());
    let SyntaxItem::Function(located) = &output.tree.items[1] else {
        panic!("expected function item");
    };
    assert_eq!(located.return_type.as_deref(), Some("callsite"));
    assert!(located.callsite.is_some());
    let SyntaxItem::Function(callback) = &output.tree.items[2] else {
        panic!("expected function item");
    };
    assert_eq!(callback.return_type.as_deref(), Some("fn() -> callsite"));
    assert!(callback.callsite.is_none());
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn plain(value: callsite) -> callsite\n",
            "\tvalue\n",
            "end\n",
            "\n",
            "fn located(value: callsite) -> callsite callsite\n",
            "\tvalue\n",
            "end\n",
            "\n",
            "fn callback() -> fn() -> callsite\n",
            "\tcallback\n",
            "end\n",
        )
    );
}

#[test]
fn preserves_returned_function_effects_before_callsite_modifier() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn expose() -> fn() -> () effects [stdio] callsite\n",
            "  callsite\n",
            "end\n",
            "fn expose_with_outer_effects() -> fn() -> () effects [stdio] effects [net] callsite\n",
            "  callsite\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    let function = first_function(&output);
    assert_eq!(
        function.return_type.as_deref(),
        Some("fn() -> () effects [stdio]")
    );
    assert!(function.effects.is_none());
    assert!(function.callsite.is_some());
    let SyntaxItem::Function(with_outer_effects) = &output.tree.items[1] else {
        panic!("expected function item");
    };
    assert_eq!(
        with_outer_effects.return_type.as_deref(),
        Some("fn() -> () effects [stdio]")
    );
    assert_eq!(
        with_outer_effects.effects.as_ref().unwrap(),
        &vec!["net".to_string()]
    );
    assert!(with_outer_effects.callsite.is_some());
    assert_eq!(
        format_tree(&output.tree),
        concat!(
            "fn expose() -> fn() -> () effects [stdio] callsite\n",
            "\tcallsite\n",
            "end\n",
            "\n",
            "fn expose_with_outer_effects() -> fn() -> () effects [stdio] effects [net] callsite\n",
            "\tcallsite\n",
            "end\n",
        )
    );
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
fn malformed_public_type_alias_does_not_stall_declaration_body_recovery() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Packet\n",
            "  Value\n",
            "end\n",
            "pub type alias AliasPacket = Packet\n",
            "\n",
            "schema Host\n",
            "  nested: AliasPacket\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_eq!(output.tree.items.len(), 2);
    assert!(output.diagnostics.len() <= 6, "{:#?}", output.diagnostics);
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.parser_context == "type_variant" && diagnostic.unexpected.text == "schema"
    }));
}

#[test]
fn malformed_private_type_alias_does_not_stall_declaration_body_recovery() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "use left\n\n",
            "type Alias = Int\n\n",
            "fn read(input: Alias) -> Alias\n",
            "  input\n",
            "end\n",
        ),
    );

    let output = parse(&source);

    assert_eq!(output.tree.items.len(), 1);
    assert!(output.diagnostics.len() <= 6, "{:#?}", output.diagnostics);
    assert!(output.diagnostics.iter().any(|diagnostic| {
        diagnostic.parser_context == "type_variant" && diagnostic.unexpected.text == "fn"
    }));
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
            "pub handler public_handler() for PublicEffect\n  call() => public_fn()\nend\n",
            "handler private_handler() for PrivateEffect\n  call() => private_fn()\nend\n",
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
            SyntaxItem::ModuleValue(_) => "module_value",
            SyntaxItem::Type(_) => "type",
            SyntaxItem::Schema(_) => "schema",
            SyntaxItem::Effect(_) => "effect",
            SyntaxItem::Handler(_) => "handler",
            SyntaxItem::PublicAlias(_) => "alias",
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
