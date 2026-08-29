use super::*;

#[test]
fn generated_schema_decode_helpers_resolve_subtracted_repeat_count_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8\n",
            "  items: Repeat(length - padding_length, UInt16be)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, padding_length: Int, items: List<Int>}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, padding_length: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
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
    assert_eq!(schema.fields[2].name, "items");
    assert_eq!(
        schema.fields[2]
            .repeat
            .as_ref()
            .map(|repeat| repeat.count_field.as_str()),
        Some("length - padding_length")
    );
}

#[test]
fn generated_schema_decode_helpers_accept_canonical_repeated_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ItemWire\n",
            "  format binary\n",
            "  value: uint8\n",
            "end\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  count: uint8\n",
            "  item_length: uint8\n",
            "  values: [uint16be; count]\n",
            "  views: [ByteView(item_length); count]\n",
            "  items: [ItemWire; count]\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{count: Int, item_length: Int, values: List<Int>, views: List<ByteView>, items: List<{value: Int}>}, String>\n",
            "  byte_decode_packet_wire(view)\n",
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
    assert_eq!(
        schema.fields[2].repeat.as_ref().map(|repeat| (
            repeat.count_field.as_str(),
            repeat.width,
            repeat.max_value
        )),
        Some(("count", 2, 0xffff))
    );
    assert_eq!(
        schema.fields[3]
            .repeat
            .as_ref()
            .and_then(|repeat| repeat.byte_view_length_field.as_deref()),
        Some("item_length")
    );
    assert!(
        schema.fields[4]
            .repeat
            .as_ref()
            .and_then(|repeat| repeat.payload_schema.as_ref())
            .is_some()
    );
}

#[test]
fn generated_schema_decode_helpers_reject_canonical_repeat_count_references() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  row_count: uint8\n",
            "  items: [uint16be; row_count * column_count]\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  row_count: uint8\n",
            "  items: [uint16be; row_count * column_count]\n",
            "  column_count: uint8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  row_count: uint8\n",
            "  flags: ByteView(row_count)\n",
            "  items: [uint16be; row_count * flags]\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "repeat count operand `column_count` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "repeat count operand `column_count` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "repeat count operand `flags` decodes as `ByteView`, not `Int`",
        ),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.repeat_reference"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{:#?}",
            lowered.diagnostics
        );
    }
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing canonical repeat count expression should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_added_repeat_count_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8\n",
            "  items: Repeat(length + padding_length, UInt16be)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, padding_length: Int, items: List<Int>}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, padding_length: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
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
    assert_eq!(schema.fields[2].name, "items");
    assert_eq!(
        schema.fields[2]
            .repeat
            .as_ref()
            .map(|repeat| repeat.count_field.as_str()),
        Some("length + padding_length")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_product_repeat_count_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  row_count: UInt8\n",
            "  column_count: UInt8\n",
            "  items: Repeat(row_count * column_count, UInt16be)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{row_count: Int, column_count: Int, items: List<Int>}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {row_count: Int, column_count: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
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
    assert_eq!(schema.fields[2].name, "items");
    assert_eq!(
        schema.fields[2]
            .repeat
            .as_ref()
            .map(|repeat| repeat.count_field.as_str()),
        Some("row_count * column_count")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_quotient_repeat_count_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  chunk_count: UInt8\n",
            "  items: Repeat(length / chunk_count, UInt16be)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, chunk_count: Int, items: List<Int>}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, chunk_count: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
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
    assert_eq!(schema.fields[2].name, "items");
    assert_eq!(
        schema.fields[2]
            .repeat
            .as_ref()
            .map(|repeat| repeat.count_field.as_str()),
        Some("length / chunk_count")
    );
}

#[test]
fn generated_schema_decode_helpers_reject_forward_subtracted_byte_view_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length - padding_length)\n",
            "  padding_length: UInt8\n",
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
                    == "ByteView length operand `padding_length` must be an earlier decoded `Int` field"
        }),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing ByteView length expression should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_reject_added_byte_view_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length + padding_length)\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length + padding_length)\n",
            "  padding_length: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  flags: ByteView(length)\n",
            "  payload: ByteView(length + flags)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "ByteView length operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "ByteView length operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "ByteView length operand `flags` decodes as `ByteView`, not `Int`",
        ),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.byte_view_reference"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{:#?}",
            lowered.diagnostics
        );
    }
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing ByteView length expression should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_reject_quotient_byte_view_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length / chunk_count)\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length / chunk_count)\n",
            "  chunk_count: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  flags: ByteView(length)\n",
            "  payload: ByteView(length / flags)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "ByteView length operand `chunk_count` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "ByteView length operand `chunk_count` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "ByteView length operand `flags` decodes as `ByteView`, not `Int`",
        ),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.byte_view_reference"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{:#?}",
            lowered.diagnostics
        );
    }
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing ByteView length expression should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_reject_subtracted_repeat_count_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  items: Repeat(length - padding_length, UInt16be)\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  items: Repeat(length - padding_length, UInt16be)\n",
            "  padding_length: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  flags: ByteView(length)\n",
            "  items: Repeat(length - flags, UInt16be)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "repeat count operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "repeat count operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "repeat count operand `flags` decodes as `ByteView`, not `Int`",
        ),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.repeat_reference"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{:#?}",
            lowered.diagnostics
        );
    }
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing Repeat count expression should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_reject_added_repeat_count_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  items: Repeat(length + padding_length, UInt16be)\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  items: Repeat(length + padding_length, UInt16be)\n",
            "  padding_length: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  flags: ByteView(length)\n",
            "  items: Repeat(length + flags, UInt16be)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "repeat count operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "repeat count operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "repeat count operand `flags` decodes as `ByteView`, not `Int`",
        ),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.repeat_reference"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{:#?}",
            lowered.diagnostics
        );
    }
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing Repeat count expression should not emit typed IR"
    );
}
