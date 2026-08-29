use super::super::*;
use super::collect_text;

#[test]
fn collector_classifies_schema_decode_expression_tokens() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode wire::PacketWire from view at base\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);

    for keyword in ["decode", "from", "at"] {
        assert!(tokens.contains(&(
            keyword.to_string(),
            SemanticTokenType::Keyword,
            SemanticTokenModifiers::empty().bits()
        )));
    }
    assert!(tokens.contains(&(
        "wire".to_string(),
        SemanticTokenType::Variable,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "PacketWire".to_string(),
        SemanticTokenType::Type,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(
        tokens.contains(&(
            "view".to_string(),
            SemanticTokenType::Parameter,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
    assert!(
        tokens.contains(&(
            "base".to_string(),
            SemanticTokenType::Parameter,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
}

#[test]
fn collector_classifies_schema_encode_expression_tokens() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn packet(value: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode wire::PacketWire from value\n",
            "end\n",
        ),
    );

    let tokens = collect_text(&source);

    for keyword in ["encode", "from"] {
        assert!(tokens.contains(&(
            keyword.to_string(),
            SemanticTokenType::Keyword,
            SemanticTokenModifiers::empty().bits()
        )));
    }
    assert!(tokens.contains(&(
        "wire".to_string(),
        SemanticTokenType::Variable,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "PacketWire".to_string(),
        SemanticTokenType::Type,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(
        tokens.contains(&(
            "value".to_string(),
            SemanticTokenType::Parameter,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
}
#[test]
fn collector_classifies_unnamed_holes_and_boolean_literals() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "# boolean path\n",
            "fn main(flag: Bool) -> Bool\n",
            "  _ satisfy candidate => true # always true\n",
            "end\n"
        ),
    );

    let tokens = collect_text(&source);

    assert!(
        tokens.contains(&(
            "_".to_string(),
            SemanticTokenType::Variable,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Hole)
                .bits()
        ))
    );
    assert!(tokens.contains(&(
        "true".to_string(),
        SemanticTokenType::Keyword,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "# boolean path".to_string(),
        SemanticTokenType::Comment,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(tokens.contains(&(
        "# always true".to_string(),
        SemanticTokenType::Comment,
        SemanticTokenModifiers::empty().bits()
    )));
}

#[test]
fn collector_keeps_let_bindings_distinct_from_record_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main(value: {count: Int}) -> Int\n",
            "  let message: String = \"ready\"\n",
            "  let {count: amount}: {count: Int} = value\n",
            "  amount\n",
            "end\n"
        ),
    );

    let tokens = collect_text(&source);

    assert!(
        tokens.contains(&(
            "message".to_string(),
            SemanticTokenType::Variable,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
    assert!(tokens.contains(&(
        "count".to_string(),
        SemanticTokenType::Property,
        SemanticTokenModifiers::empty().bits()
    )));
    assert!(
        tokens.contains(&(
            "amount".to_string(),
            SemanticTokenType::Variable,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Declaration)
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
    assert!(
        tokens.contains(&(
            "amount".to_string(),
            SemanticTokenType::Variable,
            SemanticTokenModifiers::empty()
                .with(SemanticTokenModifier::Readonly)
                .bits()
        ))
    );
}
