use super::*;

fn assert_supported_trailing_reserved_widths(total_bit_width: u8) {
    debug_assert!(matches!(total_bit_width, 8 | 16 | 24 | 32));

    for visible_width in 1..=7 {
        let reserved_width = total_bit_width - visible_width;
        let reserved_value = (1_i64 << reserved_width) - 1;
        let source = SourceFile::new(
            "main.veln",
            format!(
                concat!(
                    "schema PackedSuffixHeader\n",
                    "  format binary\n",
                    "\n",
                    "  prefix: UInt8\n",
                    "  control: UInt{}\n",
                    "  control_padding: ReservedBits({}, {})\n",
                    "  suffix: UInt8\n",
                    "end\n",
                    "\n",
                    "pub fn read_header(view: ByteView) -> Result<{{prefix: Int, control: Int, suffix: Int}}, String>\n",
                    "  byte_decode_packed_suffix_header(view)\n",
                    "end\n",
                    "\n",
                    "pub fn write_header(packet: {{prefix: Int, control: Int, suffix: Int}}) -> Result<ByteChunk, EncodeError>\n",
                    "  byte_encode_packed_suffix_header(packet)\n",
                    "end\n",
                ),
                visible_width, reserved_width, reserved_value
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);
        let lowered = lower_checked_surface_module(&module);

        assert!(
            lowered.diagnostics.is_empty(),
            "{total_bit_width}-bit layout, visible width {visible_width}: {:#?}",
            lowered.diagnostics
        );
        let ir = lowered.ir.expect("typed IR should be built");
        assert_eq!(
            ir.schema_decoders.len(),
            1,
            "{total_bit_width}-bit layout, visible width {visible_width}"
        );
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
                ("control", 1, (1_i64 << visible_width) - 1, None),
                (
                    "control_padding",
                    0,
                    0,
                    Some((reserved_width, reserved_value)),
                ),
                ("suffix", 1, 0xff, None),
            ],
            "{total_bit_width}-bit layout, visible width {visible_width}"
        );
    }
}

fn assert_supported_leading_reserved_widths(total_bit_width: u8) {
    debug_assert!(matches!(total_bit_width, 8 | 16 | 24 | 32));

    for reserved_width in (total_bit_width - 7)..=(total_bit_width - 1) {
        let visible_width = total_bit_width - reserved_width;
        let reserved_value = (1_i64 << reserved_width) - 1;
        let source = SourceFile::new(
            "main.veln",
            format!(
                concat!(
                    "schema PackedHeader\n",
                    "  format binary\n",
                    "\n",
                    "  control_reserved: ReservedBits({}, {})\n",
                    "  control: UInt{}\n",
                    "end\n",
                    "\n",
                    "pub fn read_header(view: ByteView) -> Result<{{control: Int}}, String>\n",
                    "  byte_decode_packed_header(view)\n",
                    "end\n",
                    "\n",
                    "pub fn write_header(packet: {{control: Int}}) -> Result<ByteChunk, EncodeError>\n",
                    "  byte_encode_packed_header(packet)\n",
                    "end\n",
                ),
                reserved_width, reserved_value, visible_width
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);
        let lowered = lower_checked_surface_module(&module);

        assert!(
            lowered.diagnostics.is_empty(),
            "{total_bit_width}-bit layout, reserved width {reserved_width}: {:#?}",
            lowered.diagnostics
        );
        let ir = lowered.ir.expect("typed IR should be built");
        assert_eq!(
            ir.schema_decoders.len(),
            1,
            "{total_bit_width}-bit layout, reserved width {reserved_width}"
        );
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
                (
                    "control_reserved",
                    0,
                    0,
                    Some((reserved_width, reserved_value)),
                ),
                ("control", 1, (1_i64 << visible_width) - 1, None),
            ],
            "{total_bit_width}-bit layout, reserved width {reserved_width}"
        );
    }
}

fn assert_unsupported_trailing_reserved_shapes(total_bit_width: u8) {
    debug_assert!(matches!(total_bit_width, 16 | 24));

    let source = SourceFile::new(
        "main.veln",
        format!(
            concat!(
                "schema TooWidePackedSuffixHeader\n",
                "  format binary\n",
                "\n",
                "  control: UInt8\n",
                "  control_padding: ReservedBits({}, 0)\n",
                "end\n",
                "\n",
                "schema TooNarrowPackedSuffixHeader\n",
                "  format binary\n",
                "\n",
                "  control: UInt5\n",
                "  control_padding: ReservedBits({}, 0)\n",
                "end\n",
                "\n",
                "schema MissingVisiblePackedSuffixHeader\n",
                "  format binary\n",
                "\n",
                "  control_padding: ReservedBits({}, 0)\n",
                "end\n",
            ),
            total_bit_width - 7,
            total_bit_width - 6,
            total_bit_width - 1,
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
        unsupported_shapes, 2,
        "remaining unsupported {total_bit_width}-bit packed reserved suffix shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported {total_bit_width}-bit packed reserved suffix shapes should not emit typed IR"
    );
}

#[test]
fn generated_schema_helpers_accept_all_one_byte_packed_reserved_suffix_widths() {
    assert_supported_trailing_reserved_widths(8);
}

#[test]
fn generated_schema_helpers_accept_all_two_byte_packed_reserved_suffix_widths() {
    assert_supported_trailing_reserved_widths(16);
}

#[test]
fn generated_schema_helpers_accept_all_three_byte_packed_reserved_suffix_widths() {
    assert_supported_trailing_reserved_widths(24);
}

#[test]
fn generated_schema_helpers_accept_all_four_byte_packed_reserved_suffix_widths() {
    assert_supported_trailing_reserved_widths(32);
}

#[test]
fn generated_schema_helpers_accept_all_one_byte_packed_reserved_widths() {
    assert_supported_leading_reserved_widths(8);
}

#[test]
fn generated_schema_helpers_accept_all_two_byte_packed_reserved_widths() {
    assert_supported_leading_reserved_widths(16);
}

#[test]
fn generated_schema_helpers_accept_all_three_byte_packed_reserved_widths() {
    assert_supported_leading_reserved_widths(24);
}

#[test]
fn generated_schema_helpers_accept_all_four_byte_packed_reserved_widths() {
    assert_supported_leading_reserved_widths(32);
}

#[test]
fn generated_schema_helpers_reject_unsupported_two_byte_packed_reserved_suffix_shapes() {
    assert_unsupported_trailing_reserved_shapes(16);
}

#[test]
fn generated_schema_helpers_reject_unsupported_three_byte_packed_reserved_suffix_shapes() {
    assert_unsupported_trailing_reserved_shapes(24);
}
