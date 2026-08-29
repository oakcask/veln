use super::*;

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
