use super::super::*;
use super::collect_text;

#[test]
fn collector_classifies_declarations_references_holes_and_prelude_calls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.core\n",
            "use stdio\n",
            "test parses(value: Int) -> result: Result<Int, String> effects [stdio]\n",
            "  let next: Int = int_to_string(value)\n",
            "  _todo satisfy candidate => candidate > 0\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);

    assert!(
        tokens.contains(&(
            "core".to_string(),
            SemanticTokenType::Namespace,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .bits()
        ))
    );
    assert!(
        tokens.contains(&(
            "parses".to_string(),
            SemanticTokenType::Function,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .with(SemanticTokenModifier::Test)
                .bits()
        ))
    );
    assert!(
        tokens.contains(&(
            "value".to_string(),
            SemanticTokenType::Parameter,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
    assert!(
        tokens.contains(&(
            "result".to_string(),
            SemanticTokenType::Variable,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .with(SemanticTokenModifier::Readonly)
                .with(SemanticTokenModifier::Result)
                .bits()
        ))
    );
    assert!(
        tokens.contains(&(
            "int_to_string".to_string(),
            SemanticTokenType::Function,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::DefaultLibrary)
                .bits()
        ))
    );
    assert!(
        tokens.contains(&(
            "_todo".to_string(),
            SemanticTokenType::Variable,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Hole)
                .bits()
        ))
    );
    assert!(tokens.contains(&(
        "satisfy".to_string(),
        SemanticTokenType::Keyword,
        SemanticTokenModifiers::empty().bits()
    )));
}

#[test]
fn collector_classifies_complete_prefixed_integers_as_numbers() {
    let source = SourceFile::new("main.veln", "fn values() -> Int\n  0b00101 + 0xCafe\nend\n");

    let tokens = collect_text(&source);

    for literal in ["0b00101", "0xCafe"] {
        assert!(tokens.contains(&(
            literal.to_string(),
            SemanticTokenType::Number,
            SemanticTokenModifiers::empty().bits(),
        )));
    }
}

#[test]
fn collector_classifies_variadic_parameter_names_like_parameters() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn collect(values: ...String) -> String\n",
            "  values\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);

    assert!(
        tokens.contains(&(
            "values".to_string(),
            SemanticTokenType::Parameter,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
    assert!(
        tokens.contains(&(
            "values".to_string(),
            SemanticTokenType::Parameter,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
    assert!(tokens.contains(&(
        "String".to_string(),
        SemanticTokenType::Type,
        SemanticTokenModifiers::empty().bits()
    )));
}

#[test]
fn collector_classifies_ordinary_callsite_bindings_and_references() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn local() -> Int\n",
            "  let callsite = 1\n",
            "  callsite\n",
            "end\n",
            "fn pattern(value: {field: Int}) -> Int\n",
            "  let {field: callsite} = value\n",
            "  callsite\n",
            "end\n",
            "fn result_name() -> callsite: Int\n",
            "ensure callsite > 0\n",
            "  1\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);
    let declaration_readonly = SemanticTokenModifiers::empty()
        .with(SemanticTokenModifier::Declaration)
        .with(SemanticTokenModifier::Readonly)
        .bits();
    let readonly = SemanticTokenModifiers::empty()
        .with(SemanticTokenModifier::Readonly)
        .bits();
    let result = SemanticTokenModifiers::empty()
        .with(SemanticTokenModifier::Declaration)
        .with(SemanticTokenModifier::Readonly)
        .with(SemanticTokenModifier::Result)
        .bits();

    assert!(tokens.contains(&(
        "callsite".to_string(),
        SemanticTokenType::Variable,
        declaration_readonly,
    )));
    assert_eq!(
        tokens
            .iter()
            .filter(|token| {
                token
                    == &&(
                        "callsite".to_string(),
                        SemanticTokenType::Variable,
                        declaration_readonly,
                    )
            })
            .count(),
        2
    );
    assert!(tokens.contains(&(
        "callsite".to_string(),
        SemanticTokenType::Variable,
        readonly,
    )));
    assert!(tokens.contains(&("callsite".to_string(), SemanticTokenType::Variable, result,)));
    assert!(tokens.contains(&(
        "field".to_string(),
        SemanticTokenType::Property,
        SemanticTokenModifiers::empty().bits(),
    )));
}

#[test]
fn collector_prefers_ordinary_callsite_bindings_over_a_same_named_function() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn callsite(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "fn parameter(callsite: Int) -> Int\n",
            "  callsite\n",
            "end\n",
            "fn local() -> Int\n",
            "  let callsite = 1\n",
            "  callsite\n",
            "end\n",
            "fn pattern(value: {field: Int}) -> Int\n",
            "  let {field: callsite} = value\n",
            "  callsite\n",
            "end\n",
            "fn result_name() -> callsite: Int\n",
            "ensure callsite > 0\n",
            "  1\n",
            "end\n",
            "fn caller() -> Int\n",
            "  callsite(1)\n",
            "end\n",
        ),
    );

    let callsites = collect_text(&source)
        .into_iter()
        .filter(|(text, _, _)| text == "callsite")
        .collect::<Vec<_>>();
    let declaration = SemanticTokenModifier::Declaration.bit();
    let readonly = SemanticTokenModifier::Readonly.bit();
    let result = declaration | readonly | SemanticTokenModifier::Result.bit();

    assert_eq!(
        callsites,
        [
            (
                "callsite".to_string(),
                SemanticTokenType::Function,
                declaration
            ),
            (
                "callsite".to_string(),
                SemanticTokenType::Parameter,
                declaration | readonly,
            ),
            (
                "callsite".to_string(),
                SemanticTokenType::Parameter,
                readonly,
            ),
            (
                "callsite".to_string(),
                SemanticTokenType::Variable,
                declaration | readonly,
            ),
            (
                "callsite".to_string(),
                SemanticTokenType::Variable,
                readonly,
            ),
            (
                "callsite".to_string(),
                SemanticTokenType::Variable,
                declaration | readonly,
            ),
            (
                "callsite".to_string(),
                SemanticTokenType::Variable,
                readonly,
            ),
            ("callsite".to_string(), SemanticTokenType::Variable, result),
            (
                "callsite".to_string(),
                SemanticTokenType::Variable,
                readonly,
            ),
            (
                "callsite".to_string(),
                SemanticTokenType::Function,
                SemanticTokenModifiers::empty().bits(),
            ),
        ]
    );
}
#[test]
fn collector_classifies_schema_declarations_and_format_clauses() {
    let source = SourceFile::new(
        "schema.veln",
        concat!(
            "pub schema Http2FrameHeader\n",
            "  format binary\n",
            "\n",
            "  length: UInt24be\n",
            "  padding_length: UInt8 where padding_length <= length\n",
            "  stream_reserved: ReservedBits(1, 0)\n",
            "  settings: Repeat(length - padding_length, UInt16be)\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);

    assert!(tokens.contains(&(
        "schema".to_string(),
        SemanticTokenType::Keyword,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(
        tokens.contains(&(
            "Http2FrameHeader".to_string(),
            SemanticTokenType::Type,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .bits()
        ))
    );
    assert!(tokens.contains(&(
        "format".to_string(),
        SemanticTokenType::Keyword,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "binary".to_string(),
        SemanticTokenType::EnumMember,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "where".to_string(),
        SemanticTokenType::Keyword,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "ReservedBits".to_string(),
        SemanticTokenType::Type,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "Repeat".to_string(),
        SemanticTokenType::Type,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "length".to_string(),
        SemanticTokenType::Property,
        SemanticTokenModifiers::empty().bits()
    )));
}

#[test]
fn collector_classifies_schema_member_alias_declarations() {
    let source = SourceFile::new(
        "facade.veln",
        "use wire\n\npub schema PublicPacket = wire::Packet\n",
    );

    let tokens = collect_text(&source);

    assert!(tokens.contains(&(
        "schema".to_string(),
        SemanticTokenType::Keyword,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(
        tokens.contains(&(
            "PublicPacket".to_string(),
            SemanticTokenType::Type,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .bits()
        ))
    );
}

#[test]
fn collector_marks_public_type_names_as_declarations() {
    let source = SourceFile::new(
        "facade.veln",
        "use implementation\n\npub type Document = implementation::Document\n",
    );

    let tokens = collect_text(&source);

    assert!(
        tokens.contains(&(
            "Document".to_string(),
            SemanticTokenType::Type,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .bits()
        ))
    );
}

#[test]
fn collector_classifies_handler_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub handler ask(ctx: Int) for Ask effects [stdio]\n",
            "  value(item) => provide_value(ctx, item)\n",
            "end\n",
            "\n",
            "fn provide_value(ctx: Int, item: Int) -> Int\n",
            "  ctx + item\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);

    assert!(tokens.contains(&(
        "handler".to_string(),
        SemanticTokenType::Keyword,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(
        tokens.contains(&(
            "ask".to_string(),
            SemanticTokenType::Function,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .bits()
        ))
    );
    assert!(
        tokens.contains(&(
            "ctx".to_string(),
            SemanticTokenType::Parameter,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
    assert!(tokens.contains(&(
        "for".to_string(),
        SemanticTokenType::Keyword,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "Ask".to_string(),
        SemanticTokenType::EnumMember,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "stdio".to_string(),
        SemanticTokenType::EnumMember,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "value".to_string(),
        SemanticTokenType::Property,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(
        tokens.contains(&(
            "item".to_string(),
            SemanticTokenType::Parameter,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
    assert!(tokens.contains(&(
        "provide_value".to_string(),
        SemanticTokenType::Function,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(
        tokens.contains(&(
            "item".to_string(),
            SemanticTokenType::Parameter,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
}

#[test]
fn collector_distinguishes_callsite_modifier_builtin_and_ordinary_identifier() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn located() -> SourceLocation callsite\n",
            "require callsite.start_line > 0\n",
            "  callsite.callsite\n",
            "end\n",
            "fn ordinary(callsite: Int) -> Int\n",
            "  callsite\n",
            "end\n",
            "fn duplicate() -> SourceLocation callsite callsite\n",
            "  callsite\n",
            "end\n",
        ),
    );
    let tokens = collect_text(&source);
    let callsites = tokens
        .iter()
        .filter(|(text, _, _)| text == "callsite")
        .collect::<Vec<_>>();
    assert_eq!(callsites[0].1, SemanticTokenType::Keyword);
    assert_eq!(callsites[1].1, SemanticTokenType::Variable);
    assert_eq!(callsites[1].2, SemanticTokenModifiers::empty().bits());
    assert_eq!(callsites[2].1, SemanticTokenType::Variable);
    assert_eq!(
        callsites[2].2,
        SemanticTokenModifiers::empty()
            .with(SemanticTokenModifier::Readonly)
            .bits()
    );
    assert_eq!(callsites[3].1, SemanticTokenType::Property);
    assert_eq!(callsites[4].1, SemanticTokenType::Parameter);
    assert_eq!(callsites[5].1, SemanticTokenType::Parameter);
    assert_eq!(callsites[6].1, SemanticTokenType::Keyword);
    assert_eq!(callsites[7].1, SemanticTokenType::Keyword);
    assert_eq!(callsites[8].1, SemanticTokenType::Variable);
}

#[test]
fn collector_classifies_callsite_function_declarations_and_references() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn callsite(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "fn caller() -> Int\n",
            "  callsite(1)\n",
            "end\n",
        ),
    );
    let tokens = collect_text(&source);
    let callsites = tokens
        .iter()
        .filter(|(text, _, _)| text == "callsite")
        .collect::<Vec<_>>();

    assert_eq!(
        callsites,
        [
            &(
                "callsite".to_string(),
                SemanticTokenType::Function,
                SemanticTokenModifiers::empty()
                    .with(SemanticTokenModifier::Declaration)
                    .bits(),
            ),
            &(
                "callsite".to_string(),
                SemanticTokenType::Function,
                SemanticTokenModifiers::empty().bits(),
            ),
        ]
    );
}

#[test]
fn collector_keeps_callsite_builtin_readonly_across_function_collisions_and_call_syntax() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn callsite(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "fn located() -> SourceLocation callsite\n",
            "  callsite\n",
            "  callsite(1)\n",
            "  other::callsite(1)\n",
            "end\n",
        ),
    );
    let tokens = collect_text(&source);
    let callsites = tokens
        .iter()
        .filter(|(text, _, _)| text == "callsite")
        .collect::<Vec<_>>();
    let readonly = SemanticTokenModifiers::empty()
        .with(SemanticTokenModifier::Readonly)
        .bits();

    assert_eq!(callsites[0].1, SemanticTokenType::Function);
    assert_eq!(callsites[1].1, SemanticTokenType::Keyword);
    assert_eq!(callsites[2].1, SemanticTokenType::Variable);
    assert_eq!(callsites[2].2, readonly);
    assert_eq!(callsites[3].1, SemanticTokenType::Variable);
    assert_eq!(callsites[3].2, readonly);
    assert_eq!(callsites[4].1, SemanticTokenType::Function);
    assert_eq!(callsites[4].2, SemanticTokenModifiers::empty().bits());
}

#[test]
fn collector_classifies_multiline_handler_operation_clause_bodies() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Bool) -> Int\n",
            "  fallback() -> Int\n",
            "end\n",
            "\n",
            "handler choose() for Choose\n",
            "  pick(value) => match value\n",
            "    true => value\n",
            "    false => match value\n",
            "      true => value\n",
            "      false => 0\n",
            "    end\n",
            "  end\n",
            "  fallback() => 1\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);
    let readonly_parameter = SemanticTokenModifiers::empty()
        .with(SemanticTokenModifier::Readonly)
        .bits();

    assert!(
        tokens
            .iter()
            .filter(|(text, kind, modifiers)| {
                text == "value"
                    && *kind == SemanticTokenType::Parameter
                    && *modifiers == readonly_parameter
            })
            .count()
            >= 4
    );
    assert!(tokens.contains(&(
        "fallback".to_string(),
        SemanticTokenType::Property,
        SemanticTokenModifiers::empty().bits()
    )));
}

#[test]
fn collector_bounds_handler_operation_cleanup_regions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Resource\n",
            "  access(value: Int) -> Int\n",
            "  fallback() -> Int\n",
            "end\n",
            "\n",
            "handler resource() for Resource\n",
            "  access(value) => begin\n",
            "    defer\n",
            "      release(value)\n",
            "    end\n",
            "    value\n",
            "  end\n",
            "  fallback() => 0\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);
    let readonly_parameter = SemanticTokenModifiers::empty()
        .with(SemanticTokenModifier::Readonly)
        .bits();

    assert!(tokens.contains(&(
        "value".to_string(),
        SemanticTokenType::Parameter,
        readonly_parameter
    )));
    assert!(tokens.contains(&(
        "fallback".to_string(),
        SemanticTokenType::Property,
        SemanticTokenModifiers::empty().bits()
    )));
}

#[test]
fn collector_bounds_handler_operation_clause_else_if_bodies() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Int) -> Int\n",
            "  fallback() -> Int\n",
            "end\n",
            "\n",
            "handler choose() for Choose\n",
            "  pick(value) => if value == 0\n",
            "    value\n",
            "  else if value == 1\n",
            "    value\n",
            "  else\n",
            "    value\n",
            "  end\n",
            "  fallback() => 1\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);
    let readonly_parameter = SemanticTokenModifiers::empty()
        .with(SemanticTokenModifier::Readonly)
        .bits();

    assert!(
        tokens
            .iter()
            .filter(|(text, kind, modifiers)| {
                text == "value"
                    && *kind == SemanticTokenType::Parameter
                    && *modifiers == readonly_parameter
            })
            .count()
            >= 5
    );
    assert!(tokens.contains(&(
        "fallback".to_string(),
        SemanticTokenType::Property,
        SemanticTokenModifiers::empty().bits()
    )));
}

#[test]
fn collector_keeps_satisfy_arrow_inside_handler_operation_clause_body() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Int) -> Int\n",
            "  fallback() -> Int\n",
            "end\n",
            "\n",
            "handler choose() for Choose\n",
            "  pick(value) => _choice satisfy candidate => candidate == value\n",
            "  fallback() => 0\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);
    let readonly_parameter = SemanticTokenModifiers::empty()
        .with(SemanticTokenModifier::Readonly)
        .bits();

    assert!(tokens.contains(&(
        "candidate".to_string(),
        SemanticTokenType::Variable,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "value".to_string(),
        SemanticTokenType::Parameter,
        readonly_parameter
    )));
    assert!(tokens.contains(&(
        "fallback".to_string(),
        SemanticTokenType::Property,
        SemanticTokenModifiers::empty().bits()
    )));
}
