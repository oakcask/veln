use super::*;

#[test]
fn generated_schema_helpers_accept_all_two_byte_packed_reserved_suffix_widths() {
    for visible_width in 1..=7 {
        let reserved_width = 16 - visible_width;
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
                ("prefix", 1, 0xff, None),
                ("control", 1, (1_i64 << visible_width) - 1, None),
                (
                    "control_padding",
                    0,
                    0,
                    Some((reserved_width as u8, reserved_value)),
                ),
                ("suffix", 1, 0xff, None),
            ],
            "width {visible_width}"
        );
    }
}

#[test]
fn generated_schema_helpers_accept_all_three_byte_packed_reserved_suffix_widths() {
    for visible_width in 1..=7 {
        let reserved_width = 24 - visible_width;
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
                ("prefix", 1, 0xff, None),
                ("control", 1, (1_i64 << visible_width) - 1, None),
                (
                    "control_padding",
                    0,
                    0,
                    Some((reserved_width as u8, reserved_value)),
                ),
                ("suffix", 1, 0xff, None),
            ],
            "width {visible_width}"
        );
    }
}

#[test]
fn generated_schema_helpers_accept_all_four_byte_packed_reserved_suffix_widths() {
    for visible_width in 1..=7 {
        let reserved_width = 32 - visible_width;
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
                ("prefix", 1, 0xff, None),
                ("control", 1, (1_i64 << visible_width) - 1, None),
                (
                    "control_padding",
                    0,
                    0,
                    Some((reserved_width as u8, reserved_value)),
                ),
                ("suffix", 1, 0xff, None),
            ],
            "width {visible_width}"
        );
    }
}

#[test]
fn generated_schema_helpers_accept_all_one_byte_packed_reserved_widths() {
    for reserved_width in 1..=7 {
        let visible_width = 8 - reserved_width;
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
            "width {reserved_width}: {:#?}",
            lowered.diagnostics
        );
        let ir = lowered.ir.expect("typed IR should be built");
        assert_eq!(ir.schema_decoders.len(), 1, "width {reserved_width}");
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
                    Some((reserved_width as u8, reserved_value)),
                ),
                ("control", 1, (1_i64 << visible_width) - 1, None),
            ],
            "width {reserved_width}"
        );
    }
}

#[test]
fn generated_schema_helpers_accept_all_two_byte_packed_reserved_widths() {
    for reserved_width in 9..=15 {
        let visible_width = 16 - reserved_width;
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
            "width {reserved_width}: {:#?}",
            lowered.diagnostics
        );
        let ir = lowered.ir.expect("typed IR should be built");
        assert_eq!(ir.schema_decoders.len(), 1, "width {reserved_width}");
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
                    Some((reserved_width as u8, reserved_value)),
                ),
                ("control", 1, (1_i64 << visible_width) - 1, None),
            ],
            "width {reserved_width}"
        );
    }
}

#[test]
fn generated_schema_helpers_accept_all_three_byte_packed_reserved_widths() {
    for reserved_width in 17..=23 {
        let visible_width = 24 - reserved_width;
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
            "width {reserved_width}: {:#?}",
            lowered.diagnostics
        );
        let ir = lowered.ir.expect("typed IR should be built");
        assert_eq!(ir.schema_decoders.len(), 1, "width {reserved_width}");
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
                    Some((reserved_width as u8, reserved_value)),
                ),
                ("control", 1, (1_i64 << visible_width) - 1, None),
            ],
            "width {reserved_width}"
        );
    }
}

#[test]
fn generated_schema_helpers_accept_all_four_byte_packed_reserved_widths() {
    for reserved_width in 25..=31 {
        let visible_width = 32 - reserved_width;
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
            "width {reserved_width}: {:#?}",
            lowered.diagnostics
        );
        let ir = lowered.ir.expect("typed IR should be built");
        assert_eq!(ir.schema_decoders.len(), 1, "width {reserved_width}");
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
                    Some((reserved_width as u8, reserved_value)),
                ),
                ("control", 1, (1_i64 << visible_width) - 1, None),
            ],
            "width {reserved_width}"
        );
    }
}

#[test]
fn generated_schema_helpers_reject_unsupported_two_byte_packed_reserved_suffix_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema TooWidePackedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control: UInt8\n",
            "  control_padding: ReservedBits(9, 0)\n",
            "end\n",
            "\n",
            "schema TooNarrowPackedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control: UInt5\n",
            "  control_padding: ReservedBits(10, 0)\n",
            "end\n",
            "\n",
            "schema MissingVisiblePackedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control_padding: ReservedBits(15, 0)\n",
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
        "remaining unsupported two-byte packed reserved suffix shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported two-byte packed reserved suffix shapes should not emit typed IR"
    );
}

#[test]
fn generated_schema_helpers_reject_unsupported_three_byte_packed_reserved_suffix_shapes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema TooWidePackedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control: UInt8\n",
            "  control_padding: ReservedBits(17, 0)\n",
            "end\n",
            "\n",
            "schema TooNarrowPackedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control: UInt5\n",
            "  control_padding: ReservedBits(18, 0)\n",
            "end\n",
            "\n",
            "schema MissingVisiblePackedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  control_padding: ReservedBits(23, 0)\n",
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
        "remaining unsupported three-byte packed reserved suffix shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported three-byte packed reserved suffix shapes should not emit typed IR"
    );
}
