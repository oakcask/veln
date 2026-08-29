use super::*;

#[test]
fn generated_schema_helpers_accept_six_byte_prefix_reserved_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema SixBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(41, 1466015503701)\n",
            "  high: UInt3\n",
            "  low: UInt4\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_six_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_six_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "six-byte prefix reserved group bits should be accepted: {:#?}",
        lowered.diagnostics
    );
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field
                        .reserved_bits
                        .as_ref()
                        .map(|reserved| (reserved.bit_width, reserved.expected_value)),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("prefix", 0, 0, Some((41, 1466015503701))),
            ("high", 1, 7, None),
            ("low", 1, 15, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_seven_byte_prefix_reserved_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema SevenBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(49, 375299968947541)\n",
            "  high: UInt3\n",
            "  low: UInt4\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_seven_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_seven_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "seven-byte prefix reserved group bits should be accepted: {:#?}",
        lowered.diagnostics
    );
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field
                        .reserved_bits
                        .as_ref()
                        .map(|reserved| (reserved.bit_width, reserved.expected_value)),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("prefix", 0, 0, Some((49, 375299968947541))),
            ("high", 1, 7, None),
            ("low", 1, 15, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_eight_byte_prefix_reserved_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema EightBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(57, 96076792050570581)\n",
            "  high: UInt3\n",
            "  low: UInt4\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_eight_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_eight_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "eight-byte prefix reserved group bits should be accepted: {:#?}",
        lowered.diagnostics
    );
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field
                        .reserved_bits
                        .as_ref()
                        .map(|reserved| (reserved.bit_width, reserved.expected_value)),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("prefix", 0, 0, Some((57, 96076792050570581))),
            ("high", 1, 7, None),
            ("low", 1, 15, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_reject_malformed_three_byte_prefix_reserved_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema TooWideThreeBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(17, 87381)\n",
            "  high: UInt4\n",
            "  low: UInt8\n",
            "end\n",
            "\n",
            "schema TooNarrowThreeBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(17, 87381)\n",
            "  high: UInt4\n",
            "  low: UInt2\n",
            "end\n",
            "\n",
            "schema TooWideVisibleThreeBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(1, 1)\n",
            "  high: UInt16be\n",
            "  low: UInt7\n",
            "end\n",
            "\n",
            "schema LittleEndianThreeBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(1, 1)\n",
            "  high: UInt16le\n",
            "  low: UInt7\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    let unsupported_shapes = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "schema.reserved_bits_encode"
                && diagnostic
                    .details
                    .to_json()
                    .contains("\"reason\":\"unsupported_encode_shape\"")
        })
        .count();
    assert_eq!(
        unsupported_shapes, 4,
        "malformed three-byte prefix reserved group bits should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "malformed three-byte prefix reserved group bits should not emit typed IR"
    );
}

#[test]
fn generated_schema_helpers_accept_split_reserved_bit_groups() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema SplitReservedHeader\n",
            "  format binary\n",
            "\n",
            "  top: ReservedBits(1, 1)\n",
            "  high: UInt2\n",
            "  gap: ReservedBits(2, 2)\n",
            "  low: UInt3\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_split_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_split_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "split reserved bit groups should be accepted: {:#?}",
        lowered.diagnostics
    );
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field
                        .reserved_bits
                        .as_ref()
                        .map(|reserved| (reserved.bit_width, reserved.expected_value)),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("top", 0, 0, Some((1, 1))),
            ("high", 1, 3, None),
            ("gap", 0, 0, Some((2, 2))),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_five_byte_split_reserved_bit_groups() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema FiveByteSplitReservedHeader\n",
            "  format binary\n",
            "\n",
            "  lead: UInt3\n",
            "  guard: ReservedBits(10, 682)\n",
            "  mode: UInt5\n",
            "  gap: ReservedBits(17, 87381)\n",
            "  tail: UInt5\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{lead: Int, mode: Int, tail: Int}, String>\n",
            "  byte_decode_five_byte_split_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {lead: Int, mode: Int, tail: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_five_byte_split_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "five-byte split reserved bit groups should be accepted: {:#?}",
        lowered.diagnostics
    );
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field
                        .reserved_bits
                        .as_ref()
                        .map(|reserved| (reserved.bit_width, reserved.expected_value)),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("lead", 1, 7, None),
            ("guard", 0, 0, Some((10, 682))),
            ("mode", 1, 31, None),
            ("gap", 0, 0, Some((17, 87381))),
            ("tail", 1, 31, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_six_byte_split_reserved_bit_groups() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema SixByteSplitReservedHeader\n",
            "  format binary\n",
            "\n",
            "  lead: UInt4\n",
            "  guard: ReservedBits(12, 2748)\n",
            "  mode: UInt6\n",
            "  gap: ReservedBits(20, 703710)\n",
            "  tail: UInt6\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{lead: Int, mode: Int, tail: Int}, String>\n",
            "  byte_decode_six_byte_split_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {lead: Int, mode: Int, tail: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_six_byte_split_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "six-byte split reserved bit groups should be accepted: {:#?}",
        lowered.diagnostics
    );
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field
                        .reserved_bits
                        .as_ref()
                        .map(|reserved| (reserved.bit_width, reserved.expected_value)),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("lead", 1, 15, None),
            ("guard", 0, 0, Some((12, 2748))),
            ("mode", 1, 63, None),
            ("gap", 0, 0, Some((20, 703710))),
            ("tail", 1, 63, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_seven_byte_split_reserved_bit_groups() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema SevenByteSplitReservedHeader\n",
            "  format binary\n",
            "\n",
            "  lead: UInt5\n",
            "  guard: ReservedBits(14, 10922)\n",
            "  mode: UInt7\n",
            "  gap: ReservedBits(23, 5614165)\n",
            "  tail: UInt7\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{lead: Int, mode: Int, tail: Int}, String>\n",
            "  byte_decode_seven_byte_split_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {lead: Int, mode: Int, tail: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_seven_byte_split_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "seven-byte split reserved bit groups should be accepted: {:#?}",
        lowered.diagnostics
    );
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field
                        .reserved_bits
                        .as_ref()
                        .map(|reserved| (reserved.bit_width, reserved.expected_value)),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("lead", 1, 31, None),
            ("guard", 0, 0, Some((14, 10922))),
            ("mode", 1, 127, None),
            ("gap", 0, 0, Some((23, 5614165))),
            ("tail", 1, 127, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_eight_byte_split_reserved_bit_groups() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema EightByteSplitReservedHeader\n",
            "  format binary\n",
            "\n",
            "  lead: UInt6\n",
            "  guard: ReservedBits(15, 21845)\n",
            "  mode: UInt7\n",
            "  gap: ReservedBits(29, 357913941)\n",
            "  tail: UInt7\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{lead: Int, mode: Int, tail: Int}, String>\n",
            "  byte_decode_eight_byte_split_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {lead: Int, mode: Int, tail: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_eight_byte_split_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "eight-byte split reserved bit groups should be accepted: {:#?}",
        lowered.diagnostics
    );
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field
                        .reserved_bits
                        .as_ref()
                        .map(|reserved| (reserved.bit_width, reserved.expected_value)),
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("lead", 1, 63, None),
            ("guard", 0, 0, Some((15, 21845))),
            ("mode", 1, 127, None),
            ("gap", 0, 0, Some((29, 357913941))),
            ("tail", 1, 127, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_isolated_one_byte_reserved_suffix_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema OneByteReservedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  value: UInt7\n",
            "  reserved: ReservedBits(1, 0)\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{value: Int}, String>\n",
            "  byte_decode_one_byte_reserved_suffix_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {value: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_one_byte_reserved_suffix_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "isolated one-byte reserved suffix should be accepted: {:#?}",
        lowered.diagnostics
    );
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field
                        .reserved_bits
                        .as_ref()
                        .map(|reserved| (reserved.bit_width, reserved.expected_value)),
                )
            })
            .collect::<Vec<_>>(),
        vec![("value", 1, 127, None), ("reserved", 0, 0, Some((1, 0))),]
    );
}
