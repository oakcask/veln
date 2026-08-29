use super::*;

#[test]
fn generated_schema_helpers_resolve_product_repeated_schema_and_byte_view_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ItemRecord\n",
            "  format binary\n",
            "\n",
            "  code: UInt8\n",
            "  value: UInt16be\n",
            "end\n",
            "\n",
            "schema CountedItems\n",
            "  format binary\n",
            "\n",
            "  row_count: UInt8\n",
            "  column_count: UInt8\n",
            "  items: Repeat(row_count * column_count, ItemRecord)\n",
            "end\n",
            "\n",
            "schema CountedViews\n",
            "  format binary\n",
            "\n",
            "  row_count: UInt8\n",
            "  column_count: UInt8\n",
            "  item_length: UInt8\n",
            "  items: Repeat(row_count * column_count, ByteView(item_length))\n",
            "end\n",
            "\n",
            "pub fn read_items(view: ByteView) -> Result<{row_count: Int, column_count: Int, items: List<{code: Int, value: Int}>}, String>\n",
            "  byte_decode_counted_items(view)\n",
            "end\n",
            "\n",
            "pub fn write_items(packet: {row_count: Int, column_count: Int, items: List<{code: Int, value: Int}>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_counted_items(packet)\n",
            "end\n",
            "\n",
            "pub fn read_views(view: ByteView) -> Result<{row_count: Int, column_count: Int, item_length: Int, items: List<ByteView>}, String>\n",
            "  byte_decode_counted_views(view)\n",
            "end\n",
            "\n",
            "pub fn write_views(packet: {row_count: Int, column_count: Int, item_length: Int, items: List<ByteView>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_counted_views(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    for (function_name, target_name) in [
        ("read_items", "CountedItems"),
        ("write_items", "CountedItems"),
        ("read_views", "CountedViews"),
        ("write_views", "CountedViews"),
    ] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("helper wrapper should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(
            matches!(
                &expr.kind,
                CoreExprKind::Call {
                    target: CoreCallTarget::SchemaDecode(name)
                        | CoreCallTarget::SchemaEncode(name),
                    ..
                } if name == target_name
            ),
            "{function_name} should call {target_name}"
        );
    }

    let ir = lowered.ir.expect("typed IR should be built");
    let counted_items = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "CountedItems")
        .expect("counted nested schema should be emitted");
    let nested_repeat = counted_items.fields[2]
        .repeat
        .as_ref()
        .expect("nested items should carry repeat metadata");
    assert_eq!(nested_repeat.count_field, "row_count * column_count");
    assert_eq!(
        nested_repeat
            .payload_schema
            .as_ref()
            .map(|schema| schema.schema_name.as_str()),
        Some("ItemRecord")
    );

    let counted_views = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "CountedViews")
        .expect("counted byte-view schema should be emitted");
    let byte_view_repeat = counted_views.fields[3]
        .repeat
        .as_ref()
        .expect("byte-view items should carry repeat metadata");
    assert_eq!(byte_view_repeat.count_field, "row_count * column_count");
    assert_eq!(
        byte_view_repeat.byte_view_length_field.as_deref(),
        Some("item_length")
    );
    assert!(byte_view_repeat.payload_schema.is_none());
}

#[test]
fn generated_schema_encode_helpers_resolve_length_bounded_byte_view_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, payload: ByteView}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let write = core
        .functions
        .iter()
        .find(|function| function.name == "write")
        .expect("write should be lowered");
    let CoreStmtKind::Return { expr } = &write.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "PacketWire"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "PacketWire");
    assert_eq!(schema.fields[1].name, "payload");
    assert_eq!(schema.fields[1].length_field.as_deref(), Some("length"));
}

#[test]
fn generated_schema_encode_helpers_resolve_for_exact_width_binary_schemas() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema WritePacket\n",
            "  format binary\n",
            "\n",
            "  short_value: UInt16be\n",
            "  little_length: UInt24le\n",
            "  stream_id: UInt31be\n",
            "  little_stream_id: UInt31le\n",
            "  little_wide: UInt32le\n",
            "  wide_value: UInt32be\n",
            "  trace_id: UInt40be\n",
            "  little_trace_id: UInt40le\n",
            "  extended_value: UInt48be\n",
            "  little_extended: UInt48le\n",
            "  seven_byte_value: UInt56be\n",
            "  little_seven_byte: UInt56le\n",
            "  massive_value: UInt64be\n",
            "  little_massive: UInt64le\n",
            "end\n",
            "\n",
            "pub fn main(packet: {short_value: Int, little_length: Int, stream_id: Int, little_stream_id: Int, little_wide: Int, wide_value: Int, trace_id: Int, little_trace_id: Int, extended_value: Int, little_extended: Int, seven_byte_value: Int, little_seven_byte: Int, massive_value: Int, little_massive: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_write_packet(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "WritePacket"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "WritePacket");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| {
                (
                    field.name.as_str(),
                    field.width,
                    field.max_value,
                    field.little_endian,
                )
            })
            .collect::<Vec<_>>(),
        vec![
            ("short_value", 2, 0xffff, false),
            ("little_length", 3, 0xffffff, true),
            ("stream_id", 4, 0x7fffffff, false),
            ("little_stream_id", 4, 0x7fffffff, true),
            ("little_wide", 4, 0xffffffff, true),
            ("wide_value", 4, 0xffffffff, false),
            ("trace_id", 5, 0xffffffffff, false),
            ("little_trace_id", 5, 0xffffffffff, true),
            ("extended_value", 6, 0xffffffffffff, false),
            ("little_extended", 6, 0xffffffffffff, true),
            ("seven_byte_value", 7, 0xffffffffffffff, false),
            ("little_seven_byte", 7, 0xffffffffffffff, true),
            ("massive_value", 8, i64::MAX, false),
            ("little_massive", 8, i64::MAX, true),
        ]
    );
}

#[test]
fn generated_schema_encode_helpers_omit_reserved_bits_from_value_record() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ReservedStreamIdentifier\n",
            "  format binary\n",
            "\n",
            "  length: UInt24be\n",
            "  stream_reserved: ReservedBits(1, 0)\n",
            "  stream_id: UInt31be\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int, stream_id: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_reserved_stream_identifier(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "ReservedStreamIdentifier"
    ));

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
            ("length", 3, 0xffffff, None),
            ("stream_reserved", 0, 0, Some((1, 0))),
            ("stream_id", 4, 0x7fffffff, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_byte_aligned_reserved_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ReservedPaddedHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: UInt8\n",
            "  padding: ReservedBits(16, 43981)\n",
            "  kind: UInt8\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{prefix: Int, kind: Int}, String>\n",
            "  byte_decode_reserved_padded_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {prefix: Int, kind: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_reserved_padded_header(packet)\n",
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
            ("padding", 0, 0, Some((16, 43981))),
            ("kind", 1, 0xff, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_one_byte_packed_reserved_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PackedHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: UInt8\n",
            "  control_reserved: ReservedBits(3, 5)\n",
            "  control: UInt5\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{prefix: Int, control: Int}, String>\n",
            "  byte_decode_packed_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {prefix: Int, control: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packed_header(packet)\n",
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
            ("control_reserved", 0, 0, Some((3, 5))),
            ("control", 1, 0x1f, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_reserved_byte_prefix_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ReservedBytePrefixHeader\n",
            "  format binary\n",
            "\n",
            "  guard: ReservedBits(2, 0)\n",
            "  payload: UInt8\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{payload: Int}, String>\n",
            "  byte_decode_reserved_byte_prefix_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {payload: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_reserved_byte_prefix_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "reserved byte prefix bits should be accepted: {:#?}",
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
        vec![("guard", 0, 0, Some((2, 0))), ("payload", 1, 0xff, None),]
    );
}

#[test]
fn generated_schema_helpers_accept_reserved_nine_bit_prefix_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ReservedNineBitPrefixHeader\n",
            "  format binary\n",
            "\n",
            "  guard: ReservedBits(9, 0)\n",
            "  payload: UInt8\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{payload: Int}, String>\n",
            "  byte_decode_reserved_nine_bit_prefix_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {payload: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_reserved_nine_bit_prefix_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "reserved nine-bit prefix bits should be accepted: {:#?}",
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
        vec![("guard", 0, 0, Some((9, 0))), ("payload", 1, 0xff, None),]
    );
}

#[test]
fn generated_schema_helpers_accept_general_reserved_byte_prefix_boundaries() {
    for (bit_width, expected_value) in [(3, 5), (7, 85), (9, 341), (55, (1_i64 << 55) - 1)] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                "schema GeneralReservedBytePrefix\n\
                 \tformat binary\n\
                 \n\
                 \tguard: ReservedBits({bit_width}, {expected_value})\n\
                 \tpayload: UInt8\n\
                 end\n\
                 \n\
                 pub fn read_header(view: ByteView) -> Result<{{payload: Int}}, String>\n\
                 \tbyte_decode_general_reserved_byte_prefix(view)\n\
                 end\n\
                 \n\
                 pub fn write_header(packet: {{payload: Int}}) -> Result<ByteChunk, EncodeError>\n\
                 \tbyte_encode_general_reserved_byte_prefix(packet)\n\
                 end\n"
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let lowered = lower_checked_surface_module(&module);

        assert!(
            lowered.diagnostics.is_empty(),
            "reserved byte prefix width {bit_width} should be accepted: {:#?}",
            lowered.diagnostics
        );
        let ir = lowered.ir.expect("typed IR should be built");
        assert_eq!(ir.schema_decoders.len(), 1, "width {bit_width}");
        assert_eq!(
            ir.schema_decoders[0].fields[0]
                .reserved_bits
                .as_ref()
                .map(|reserved| (reserved.bit_width, reserved.expected_value)),
            Some((bit_width as u8, expected_value)),
            "width {bit_width}"
        );
    }
}
