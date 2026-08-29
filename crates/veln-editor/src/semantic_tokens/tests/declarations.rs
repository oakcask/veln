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
            "pub handler ask(ctx: Int) handles Ask effects [stdio]\n",
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
        "handles".to_string(),
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
fn collector_classifies_multiline_handler_operation_clause_bodies() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Bool) -> Int\n",
            "  fallback() -> Int\n",
            "end\n",
            "\n",
            "handler choose() handles Choose\n",
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
fn collector_bounds_handler_operation_clause_else_if_bodies() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Int) -> Int\n",
            "  fallback() -> Int\n",
            "end\n",
            "\n",
            "handler choose() handles Choose\n",
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
            "handler choose() handles Choose\n",
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
