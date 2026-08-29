use super::*;

#[test]
fn generated_schema_helpers_reject_reserved_byte_prefix_outside_boundaries() {
    for (bit_width, expected_value) in [(3, 8), (57, 0)] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "schema UnsupportedReservedBytePrefix\n\
                 \tformat binary\n\
                 \n\
                 \tguard: ReservedBits({bit_width}, {expected_value})\n\
                 \tpayload: UInt8\n\
                 end\n"
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let lowered = lower_checked_surface_module(&module);

        assert!(
            lowered
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id == "schema.reserved_bits_encode"),
            "reserved byte prefix width {bit_width} value {expected_value} should be rejected: {:#?}",
            lowered.diagnostics
        );
        assert!(lowered.ir.is_none());
    }
}

#[test]
fn generated_schema_helpers_accept_one_byte_packed_reserved_suffix_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PackedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: UInt8\n",
            "  control: UInt3\n",
            "  control_padding: ReservedBits(5, 0)\n",
            "  suffix: UInt8\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{prefix: Int, control: Int, suffix: Int}, String>\n",
            "  byte_decode_packed_suffix_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {prefix: Int, control: Int, suffix: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packed_suffix_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
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
            ("prefix", 1, 0xff, None),
            ("control", 1, 0x7, None),
            ("control_padding", 0, 0, Some((5, 0))),
            ("suffix", 1, 0xff, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_middle_reserved_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MiddleReservedHeader\n",
            "  format binary\n",
            "\n",
            "  high: UInt3\n",
            "  gap: ReservedBits(2, 1)\n",
            "  low: UInt3\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_middle_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_middle_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "middle reserved bits should be accepted: {:#?}",
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
            ("high", 1, 7, None),
            ("gap", 0, 0, Some((2, 1))),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_byte_interleaved_middle_reserved_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ByteInterleavedMiddleReservedHeader\n",
            "  format binary\n",
            "\n",
            "  high: UInt4\n",
            "  guard: ReservedBits(1, 0)\n",
            "  middle: UInt8\n",
            "  low: UInt3\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, middle: Int, low: Int}, String>\n",
            "  byte_decode_byte_interleaved_middle_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, middle: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_byte_interleaved_middle_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "byte-interleaved middle reserved bits should be accepted: {:#?}",
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
            ("high", 1, 15, None),
            ("guard", 0, 0, Some((1, 0))),
            ("middle", 1, 255, None),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_prefix_reserved_visible_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(2, 2)\n",
            "  high: UInt3\n",
            "  low: UInt3\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "prefix reserved visible group bits should be accepted: {:#?}",
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
            ("prefix", 0, 0, Some((2, 2))),
            ("high", 1, 7, None),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_two_byte_prefix_reserved_byte_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema BytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  marker: ReservedBits(8, 171)\n",
            "  high: UInt3\n",
            "  low: UInt5\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "two-byte prefix reserved byte group bits should be accepted: {:#?}",
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
            ("marker", 0, 0, Some((8, 171))),
            ("high", 1, 7, None),
            ("low", 1, 31, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_min_width_two_byte_prefix_reserved_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MinWidthPrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(1, 1)\n",
            "  high: UInt7\n",
            "  low: UInt8\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_min_width_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_min_width_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "minimum-width two-byte prefix reserved group bits should be accepted: {:#?}",
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
            ("prefix", 0, 0, Some((1, 1))),
            ("high", 1, 127, None),
            ("low", 1, 255, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_three_byte_prefix_reserved_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ThreeBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(17, 87381)\n",
            "  high: UInt4\n",
            "  low: UInt3\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_three_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_three_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "three-byte prefix reserved group bits should be accepted: {:#?}",
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
            ("prefix", 0, 0, Some((17, 87381))),
            ("high", 1, 15, None),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_four_byte_prefix_reserved_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema FourBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(25, 22369621)\n",
            "  high: UInt4\n",
            "  low: UInt3\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_four_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_four_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "four-byte prefix reserved group bits should be accepted: {:#?}",
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
            ("prefix", 0, 0, Some((25, 22369621))),
            ("high", 1, 15, None),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_five_byte_prefix_reserved_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema FiveBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(33, 5726623061)\n",
            "  high: UInt3\n",
            "  low: UInt4\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_five_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_five_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "five-byte prefix reserved group bits should be accepted: {:#?}",
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
            ("prefix", 0, 0, Some((33, 5726623061))),
            ("high", 1, 7, None),
            ("low", 1, 15, None),
        ]
    );
}
