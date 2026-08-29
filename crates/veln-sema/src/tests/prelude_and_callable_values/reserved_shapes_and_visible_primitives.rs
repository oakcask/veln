use super::*;

#[test]
fn generated_schema_helpers_reject_unsupported_four_byte_packed_reserved_suffix_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema TooWidePackedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control: UInt8\n",
            "  control_padding: ReservedBits(25, 0)\n",
            "end\n",
            "\n",
            "schema TooNarrowPackedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control: UInt5\n",
            "  control_padding: ReservedBits(26, 0)\n",
            "end\n",
            "\n",
            "schema MissingVisiblePackedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control_padding: ReservedBits(31, 0)\n",
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
        unsupported_shapes, 2,
        "remaining unsupported four-byte packed reserved suffix shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported four-byte packed reserved suffix shapes should not emit typed IR"
    );
}

#[test]
fn generated_schema_helpers_accept_two_visible_suffix_reserved_group() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ByteBeforeSuffixReservedHeader\n",
            "  format binary\n",
            "\n",
            "  channel: UInt3\n",
            "  code: UInt8\n",
            "  guard: ReservedBits(5, 21)\n",
            "end\n",
            "\n",
            "pub fn read_packet(view: ByteView) -> Result<{code: Int, channel: Int}, String>\n",
            "  byte_decode_byte_before_suffix_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_packet(packet: {code: Int, channel: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_byte_before_suffix_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "two-visible suffix reserved groups should be accepted: {:#?}",
        lowered.diagnostics
    );
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let high_schema = &ir.schema_decoders[0];
    assert_eq!(
        high_schema
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
            ("channel", 1, 7, None),
            ("code", 1, 255, None),
            ("guard", 0, 0, Some((5, 21))),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_byte_visible_reserved_suffix_widths() {
    for reserved_width in (9..=55).filter(|width| width % 8 != 0) {
        let reserved_value = (1_i64 << reserved_width) - 1;
        let source = SourceFile::new(
            "main.veln",
            format!(
                concat!(
                    "schema ByteVisibleReservedSuffixHeader\n",
                    "  format binary\n",
                    "\n",
                    "  control: UInt8\n",
                    "  control_padding: ReservedBits({}, {})\n",
                    "end\n",
                    "\n",
                    "pub fn read_header(view: ByteView) -> Result<{{control: Int}}, String>\n",
                    "  byte_decode_byte_visible_reserved_suffix_header(view)\n",
                    "end\n",
                    "\n",
                    "pub fn write_header(packet: {{control: Int}}) -> Result<ByteChunk, EncodeError>\n",
                    "  byte_encode_byte_visible_reserved_suffix_header(packet)\n",
                    "end\n",
                ),
                reserved_width, reserved_value
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let lowered = lower_checked_surface_module(&module);

        assert!(
            lowered.diagnostics.is_empty(),
            "reserved width {reserved_width}: {:#?}",
            lowered.diagnostics
        );
        let ir = lowered.ir.expect("typed IR should be built");
        assert_eq!(
            ir.schema_decoders.len(),
            1,
            "reserved width {reserved_width}"
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
                ("control", 1, 255, None),
                (
                    "control_padding",
                    0,
                    0,
                    Some((reserved_width as u8, reserved_value)),
                ),
            ],
            "reserved width {reserved_width}"
        );
    }
}

#[test]
fn generated_schema_helpers_accept_five_byte_reserved_suffix_shape() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema FiveByteReservedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control: UInt7\n",
            "  control_padding: ReservedBits(33, 5726623061)\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{control: Int}, String>\n",
            "  byte_decode_five_byte_reserved_suffix_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {control: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_five_byte_reserved_suffix_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "five-byte reserved suffix shape should be accepted: {:#?}",
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
            ("control", 1, 127, None),
            ("control_padding", 0, 0, Some((33, 5726623061))),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_all_six_byte_reserved_suffix_widths() {
    for visible_width in 1..=7 {
        let reserved_width = 48 - visible_width;
        let reserved_value = (1_i64 << reserved_width) - 1;
        let source = SourceFile::new(
            "main.veln",
            format!(
                concat!(
                    "schema SixByteReservedSuffixHeader\n",
                    "  format binary\n",
                    "\n",
                    "  control: UInt{}\n",
                    "  control_padding: ReservedBits({}, {})\n",
                    "end\n",
                    "\n",
                    "pub fn read_header(view: ByteView) -> Result<{{control: Int}}, String>\n",
                    "  byte_decode_six_byte_reserved_suffix_header(view)\n",
                    "end\n",
                    "\n",
                    "pub fn write_header(packet: {{control: Int}}) -> Result<ByteChunk, EncodeError>\n",
                    "  byte_encode_six_byte_reserved_suffix_header(packet)\n",
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
            "width {visible_width}: {:#?}",
            lowered.diagnostics
        );
        let ir = lowered.ir.expect("typed IR should be built");
        assert_eq!(ir.schema_decoders.len(), 1, "width {visible_width}");
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
                ("control", 1, (1_i64 << visible_width) - 1, None),
                (
                    "control_padding",
                    0,
                    0,
                    Some((reserved_width as u8, reserved_value)),
                ),
            ],
            "width {visible_width}"
        );
    }
}

#[test]
fn generated_schema_helpers_reject_unsupported_five_byte_reserved_suffix_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ByteVisibleReservedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control: UInt8\n",
            "  control_padding: ReservedBits(33, 0)\n",
            "end\n",
            "\n",
            "schema TooNarrowReservedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control: UInt5\n",
            "  control_padding: ReservedBits(34, 0)\n",
            "end\n",
            "\n",
            "schema MissingVisibleReservedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control_padding: ReservedBits(39, 0)\n",
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
        unsupported_shapes, 2,
        "remaining unsupported five-byte reserved suffix shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported five-byte reserved suffix shapes should not emit typed IR"
    );
}

#[test]
fn generated_schema_helpers_reject_unsupported_six_byte_reserved_suffix_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ByteVisibleReservedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control: UInt8\n",
            "  control_padding: ReservedBits(41, 0)\n",
            "end\n",
            "\n",
            "schema TooNarrowReservedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control: UInt5\n",
            "  control_padding: ReservedBits(44, 0)\n",
            "end\n",
            "\n",
            "schema MissingVisibleReservedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control_padding: ReservedBits(47, 0)\n",
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
        unsupported_shapes, 2,
        "remaining unsupported six-byte reserved suffix shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported six-byte reserved suffix shapes should not emit typed IR"
    );
}

#[test]
fn generated_schema_helpers_reject_unsupported_two_byte_packed_reserved_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema TooWidePackedHeader\n",
            "  format binary\n",
            "\n",
            "  control_reserved: ReservedBits(9, 0)\n",
            "  control: UInt16be\n",
            "end\n",
            "\n",
            "schema TooNarrowPackedHeader\n",
            "  format binary\n",
            "\n",
            "  control_reserved: ReservedBits(10, 0)\n",
            "  control: UInt5\n",
            "end\n",
            "\n",
            "schema MissingVisiblePackedHeader\n",
            "  format binary\n",
            "\n",
            "  control_reserved: ReservedBits(15, 0)\n",
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
        unsupported_shapes, 3,
        "unsupported two-byte packed reserved shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported two-byte packed reserved shapes should not emit typed IR"
    );
}

#[test]
fn generated_schema_helpers_reject_unsupported_three_byte_packed_reserved_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema TooNarrowPackedHeader\n",
            "  format binary\n",
            "\n",
            "  control_reserved: ReservedBits(18, 0)\n",
            "  control: UInt5\n",
            "end\n",
            "\n",
            "schema MissingVisiblePackedHeader\n",
            "  format binary\n",
            "\n",
            "  control_reserved: ReservedBits(23, 0)\n",
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
        unsupported_shapes, 2,
        "remaining unsupported three-byte packed reserved shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported three-byte packed reserved shapes should not emit typed IR"
    );
}

#[test]
fn generated_schema_helpers_reject_unsupported_four_byte_packed_reserved_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema TooNarrowPackedHeader\n",
            "  format binary\n",
            "\n",
            "  control_reserved: ReservedBits(26, 0)\n",
            "  control: UInt5\n",
            "end\n",
            "\n",
            "schema MissingVisiblePackedHeader\n",
            "  format binary\n",
            "\n",
            "  control_reserved: ReservedBits(31, 0)\n",
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
        unsupported_shapes, 2,
        "remaining unsupported four-byte packed reserved shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported four-byte packed reserved shapes should not emit typed IR"
    );
}
