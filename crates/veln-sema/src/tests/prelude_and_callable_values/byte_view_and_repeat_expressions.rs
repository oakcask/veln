use super::*;

#[test]
fn generated_schema_helpers_resolve_reserved_payload_dispatch_binary_schemas() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PackedReservedPayload\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(2, 1)\n",
            "  value: UInt6\n",
            "end\n",
            "\n",
            "schema ByteReservedPayload\n",
            "  format binary\n",
            "\n",
            "  marker: ReservedBits(8, 171)\n",
            "  code: UInt8\n",
            "end\n",
            "\n",
            "schema ClosedReservedPacket\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => PackedReservedPayload)\n",
            "end\n",
            "\n",
            "schema ExtensionReservedPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 1 => ByteReservedPayload)\n",
            "end\n",
            "\n",
            "pub fn read_closed(view: ByteView) -> Result<{kind: Int, payload: {value: Int}}, String>\n",
            "  byte_decode_closed_reserved_packet(view)\n",
            "end\n",
            "\n",
            "pub fn write_closed(packet: {kind: Int, payload: {value: Int}}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_closed_reserved_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn read_extension(view: ByteView) -> Result<{length: Int, kind: Int, payload: SchemaDispatchPayload<{code: Int}>}, String>\n",
            "  byte_decode_extension_reserved_packet(view)\n",
            "end\n",
            "\n",
            "pub fn write_extension(packet: {length: Int, kind: Int, payload: SchemaDispatchPayload<{code: Int}>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_extension_reserved_packet(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let closed = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "ClosedReservedPacket")
        .expect("closed dispatch metadata should be emitted");
    let closed_dispatch = closed.fields[1]
        .dispatch
        .as_ref()
        .expect("closed payload should carry dispatch metadata");
    assert_eq!(
        closed_dispatch.cases[0]
            .payload_schema
            .as_ref()
            .expect("closed case should carry nested schema metadata")
            .schema_name,
        "PackedReservedPayload"
    );

    let extension = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "ExtensionReservedPacket")
        .expect("extension dispatch metadata should be emitted");
    let extension_dispatch = extension.fields[2]
        .dispatch
        .as_ref()
        .expect("extension payload should carry dispatch metadata");
    assert_eq!(extension_dispatch.length_field.as_deref(), Some("length"));
    assert_eq!(
        extension_dispatch.cases[0]
            .payload_schema
            .as_ref()
            .expect("extension case should carry nested schema metadata")
            .schema_name,
        "ByteReservedPayload"
    );
}

#[test]
fn generated_schema_decode_helpers_require_int_byte_view_length_field() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "  first_payload: ByteView(wire_length)\n",
            "  second_payload: ByteView(first_payload)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.byte_view_reference"
                && diagnostic.message
                    == "ByteView length operand `first_payload` decodes as `ByteView`, not `Int`"
        }),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing ByteView length field should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_subtracted_byte_view_length_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8\n",
            "  payload: ByteView(length - padding_length)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, padding_length: Int, payload: ByteView}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, padding_length: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "payload");
    assert_eq!(
        schema.fields[2].length_field.as_deref(),
        Some("length - padding_length")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_added_byte_view_length_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8\n",
            "  payload: ByteView(length + padding_length)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, padding_length: Int, payload: ByteView}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, padding_length: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "payload");
    assert_eq!(
        schema.fields[2].length_field.as_deref(),
        Some("length + padding_length")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_product_byte_view_length_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  row_count: UInt8\n",
            "  column_count: UInt8\n",
            "  payload: ByteView(row_count * column_count)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{row_count: Int, column_count: Int, payload: ByteView}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {row_count: Int, column_count: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "payload");
    assert_eq!(
        schema.fields[2].length_field.as_deref(),
        Some("row_count * column_count")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_quotient_byte_view_length_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  chunk_count: UInt8\n",
            "  payload: ByteView(length / chunk_count)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, chunk_count: Int, payload: ByteView}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, chunk_count: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "payload");
    assert_eq!(
        schema.fields[2].length_field.as_deref(),
        Some("length / chunk_count")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_byte_view_multiple_constraints() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema FieldMultiplePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  frame_count: UInt8\n",
            "  payload: ByteView(length) where payload_count multiple of frame_count\n",
            "end\n",
            "\n",
            "schema LiteralMultiplePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length) where payload_count multiple of 4\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let field_multiple = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "FieldMultiplePacket")
        .expect("field multiple schema should be emitted");
    assert_eq!(
        field_multiple.fields[2].length_multiple.as_deref(),
        Some("frame_count")
    );
    let literal_multiple = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "LiteralMultiplePacket")
        .expect("literal multiple schema should be emitted");
    assert_eq!(
        literal_multiple.fields[1].length_multiple.as_deref(),
        Some("4")
    );
}

#[test]
fn generated_schema_decode_helpers_reject_byte_view_multiple_constraints() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingMultiplePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length) where payload_count multiple of frame_count\n",
            "end\n",
            "\n",
            "schema ForwardMultiplePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length) where payload_count multiple of frame_count\n",
            "  frame_count: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindMultiplePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  flags: ByteView(length)\n",
            "  payload: ByteView(length) where payload_count multiple of flags\n",
            "end\n",
            "\n",
            "schema MalformedMultiplePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length) where payload_count multiple of 0\n",
            "end\n",
            "\n",
            "schema InvalidKindMultiplePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8 where payload_count multiple of 4\n",
            "end\n",
            "\n",
            "schema InvalidRepeatMultiplePacket\n",
            "  format binary\n",
            "\n",
            "  count: UInt8\n",
            "  payloads: Repeat(count, UInt8) where payload_count multiple of count\n",
            "end\n",
            "\n",
            "schema InvalidReservedMultiplePacket\n",
            "  format binary\n",
            "\n",
            "  reserved: ReservedBits(1, 0) where payload_count multiple of 4\n",
            "  visible: UInt7\n",
            "end\n",
            "\n",
            "schema InvalidDispatchMultiplePacket\n",
            "  format binary\n",
            "\n",
            "  tag: UInt8\n",
            "  payload: Dispatch(tag, 1 => UInt8) where payload_count multiple of tag\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "ByteView multiple operand `frame_count` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "ByteView multiple operand `frame_count` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "ByteView multiple operand `flags` decodes as `ByteView`, not `Int`",
        ),
        (
            "unsupported_multiple_predicate",
            "ByteView field validation must use `payload_count multiple of <field-or-positive-integer>`",
        ),
        (
            "invalid_field_kind",
            "ByteView multiple validation can only be used on length-bounded `ByteView` fields",
        ),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.byte_view_reference"
                    && diagnostic.message == message
                    && diagnostic.details.to_json().contains(reason)
                    && diagnostic
                        .details
                        .to_json()
                        .contains("\"role\":\"multiple\"")
            }),
            "missing {reason}: {:#?}",
            lowered.diagnostics
        );
    }
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing ByteView multiple constraints should not emit typed IR"
    );
    for schema_name in [
        "InvalidKindMultiplePacket",
        "InvalidRepeatMultiplePacket",
        "InvalidReservedMultiplePacket",
        "InvalidDispatchMultiplePacket",
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.byte_view_reference"
                    && diagnostic.message
                        == "ByteView multiple validation can only be used on length-bounded `ByteView` fields"
                    && diagnostic.details.to_json().contains(schema_name)
            }),
            "missing invalid field-kind diagnostic for {schema_name}: {:#?}",
            lowered.diagnostics
        );
    }
}

#[test]
fn repeat_count_expressions_accept_product_lengths() {
    let repeat = repeat_schema_primitive("Repeat(row_count * column_count, UInt16be)")
        .expect("product repeat count should parse");

    assert_eq!(repeat.count_field, "row_count * column_count");
}

#[test]
fn repeat_count_expressions_accept_quotient_lengths() {
    let repeat = repeat_schema_primitive("Repeat(length / chunk_count, UInt16be)")
        .expect("quotient repeat count should parse");

    assert_eq!(repeat.count_field, "length / chunk_count");
}

#[test]
fn canonical_repeat_syntax_preserves_payload_then_count_order() {
    let primitive = repeat_schema_primitive("[uint16be; row_count * column_count]")
        .expect("canonical lowercase primitive repeat should parse");
    assert_eq!(primitive.count_field, "row_count * column_count");
    assert_eq!(
        primitive.payload,
        SchemaRepeatPayload::Primitive {
            width: 2,
            max_value: 0xffff,
            little_endian: false,
        }
    );

    let compatibility_primitive = repeat_schema_primitive("[UInt16le; count]")
        .expect("canonical repeat with compatibility primitive payload should parse");
    assert_eq!(compatibility_primitive.count_field, "count");
    assert_eq!(
        compatibility_primitive.payload,
        SchemaRepeatPayload::Primitive {
            width: 2,
            max_value: 0xffff,
            little_endian: true,
        }
    );

    let legacy_lowercase_primitive = repeat_schema_primitive("Repeat(count, uint16be)")
        .expect("legacy repeat with lowercase primitive payload should parse");
    assert_eq!(legacy_lowercase_primitive.count_field, "count");
    assert_eq!(
        legacy_lowercase_primitive.payload,
        SchemaRepeatPayload::Primitive {
            width: 2,
            max_value: 0xffff,
            little_endian: false,
        }
    );

    let byte_view = repeat_schema_primitive("[ByteView(left_length + right_length); count]")
        .expect("canonical ByteView repeat should parse");
    assert_eq!(byte_view.count_field, "count");
    assert_eq!(
        byte_view.payload,
        SchemaRepeatPayload::ByteView {
            length_field: "left_length + right_length".to_string(),
        }
    );

    let nested = repeat_schema_primitive("[wire::Payload; count]")
        .expect("canonical nested schema repeat should parse");
    assert_eq!(nested.count_field, "count");
    assert_eq!(
        nested.payload,
        SchemaRepeatPayload::Schema {
            schema_name: "wire::Payload".to_string(),
        }
    );

    assert!(repeat_schema_primitive("[uint16be count]").is_none());
    assert!(repeat_schema_primitive("[uint16be; count; extra]").is_none());
    assert!(repeat_schema_primitive("Repeat(count, uint1)").is_none());
}
