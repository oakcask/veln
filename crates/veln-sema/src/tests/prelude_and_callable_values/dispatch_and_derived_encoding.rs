use super::*;

#[test]
fn generated_schema_helpers_accept_standalone_sub_byte_primitives() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema LooseBits\n",
            "  format binary\n",
            "\n",
            "  first: UInt1\n",
            "  middle: UInt5\n",
            "  last: UInt7\n",
            "end\n",
            "\n",
            "\n",
            "pub fn read_bits(view: ByteView, base: ByteOffset) -> DecodeStep<{first: Int, middle: Int, last: Int}>\n",
            "  decode LooseBits from view at base\n",
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
    assert_eq!(schema.schema_name, "LooseBits");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.max_value))
            .collect::<Vec<_>>(),
        vec![("first", 1, 0x1), ("middle", 1, 0x1f), ("last", 1, 0x7f)]
    );
}

#[test]
fn generated_schema_helpers_accept_three_byte_packed_visible_primitives() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PackedVisibleThreeByteHeader\n",
            "  format binary\n",
            "\n",
            "  high: UInt5\n",
            "  upper: UInt7\n",
            "  middle: UInt5\n",
            "  lower: UInt2\n",
            "  tail: UInt5\n",
            "end\n",
            "\n",
            "\n",
            "pub fn direct(view: ByteView) -> Result<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int}, String>\n",
            "  byte_decode_packed_visible_three_byte_header(view)\n",
            "end\n",
            "\n",
            "pub fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int}>\n",
            "  byte_decode_step_packed_visible_three_byte_header(view, base)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {high: Int, upper: Int, middle: Int, lower: Int, tail: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packed_visible_three_byte_header(packet)\n",
            "end\n",
            "\n",
            "pub fn item_decode(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int}>\n",
            "  decode PackedVisibleThreeByteHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn item_encode(packet: {high: Int, upper: Int, middle: Int, lower: Int, tail: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PackedVisibleThreeByteHeader from packet\n",
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
    assert_eq!(schema.schema_name, "PackedVisibleThreeByteHeader");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.max_value))
            .collect::<Vec<_>>(),
        vec![
            ("high", 1, 0x1f),
            ("upper", 1, 0x7f),
            ("middle", 1, 0x1f),
            ("lower", 1, 0x3),
            ("tail", 1, 0x1f)
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_four_byte_packed_visible_primitives() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PackedVisibleFourByteHeader\n",
            "  format binary\n",
            "\n",
            "  high: UInt5\n",
            "  upper: UInt7\n",
            "  middle: UInt6\n",
            "  lower: UInt5\n",
            "  tail: UInt4\n",
            "  flag: UInt5\n",
            "end\n",
            "\n",
            "\n",
            "pub fn direct(view: ByteView) -> Result<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int}, String>\n",
            "  byte_decode_packed_visible_four_byte_header(view)\n",
            "end\n",
            "\n",
            "pub fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int}>\n",
            "  byte_decode_step_packed_visible_four_byte_header(view, base)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packed_visible_four_byte_header(packet)\n",
            "end\n",
            "\n",
            "pub fn item_decode(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int}>\n",
            "  decode PackedVisibleFourByteHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn item_encode(packet: {high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PackedVisibleFourByteHeader from packet\n",
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
    assert_eq!(schema.schema_name, "PackedVisibleFourByteHeader");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.max_value))
            .collect::<Vec<_>>(),
        vec![
            ("high", 1, 0x1f),
            ("upper", 1, 0x7f),
            ("middle", 1, 0x3f),
            ("lower", 1, 0x1f),
            ("tail", 1, 0xf),
            ("flag", 1, 0x1f)
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_five_byte_packed_visible_primitives() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PackedVisibleFiveByteHeader\n",
            "  format binary\n",
            "\n",
            "  high: UInt5\n",
            "  upper: UInt7\n",
            "  middle: UInt6\n",
            "  lower: UInt5\n",
            "  tail: UInt4\n",
            "  flag: UInt6\n",
            "  code: UInt7\n",
            "end\n",
            "\n",
            "\n",
            "pub fn direct(view: ByteView) -> Result<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int}, String>\n",
            "  byte_decode_packed_visible_five_byte_header(view)\n",
            "end\n",
            "\n",
            "pub fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int}>\n",
            "  byte_decode_step_packed_visible_five_byte_header(view, base)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packed_visible_five_byte_header(packet)\n",
            "end\n",
            "\n",
            "pub fn item_decode(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int}>\n",
            "  decode PackedVisibleFiveByteHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn item_encode(packet: {high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PackedVisibleFiveByteHeader from packet\n",
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
    assert_eq!(schema.schema_name, "PackedVisibleFiveByteHeader");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.max_value))
            .collect::<Vec<_>>(),
        vec![
            ("high", 1, 0x1f),
            ("upper", 1, 0x7f),
            ("middle", 1, 0x3f),
            ("lower", 1, 0x1f),
            ("tail", 1, 0xf),
            ("flag", 1, 0x3f),
            ("code", 1, 0x7f)
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_six_byte_packed_visible_primitives() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PackedVisibleSixByteHeader\n",
            "  format binary\n",
            "\n",
            "  high: UInt2\n",
            "  upper: UInt7\n",
            "  middle: UInt4\n",
            "  lower: UInt7\n",
            "  tail: UInt7\n",
            "  flag: UInt7\n",
            "  code: UInt7\n",
            "  route: UInt7\n",
            "end\n",
            "\n",
            "\n",
            "pub fn direct(view: ByteView) -> Result<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int}, String>\n",
            "  byte_decode_packed_visible_six_byte_header(view)\n",
            "end\n",
            "\n",
            "pub fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int}>\n",
            "  byte_decode_step_packed_visible_six_byte_header(view, base)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packed_visible_six_byte_header(packet)\n",
            "end\n",
            "\n",
            "pub fn item_decode(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int}>\n",
            "  decode PackedVisibleSixByteHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn item_encode(packet: {high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PackedVisibleSixByteHeader from packet\n",
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
    assert_eq!(schema.schema_name, "PackedVisibleSixByteHeader");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.max_value))
            .collect::<Vec<_>>(),
        vec![
            ("high", 1, 0x3),
            ("upper", 1, 0x7f),
            ("middle", 1, 0xf),
            ("lower", 1, 0x7f),
            ("tail", 1, 0x7f),
            ("flag", 1, 0x7f),
            ("code", 1, 0x7f),
            ("route", 1, 0x7f)
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_seven_byte_packed_visible_primitives() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PackedVisibleSevenByteHeader\n",
            "  format binary\n",
            "\n",
            "  high: UInt1\n",
            "  upper: UInt7\n",
            "  middle: UInt7\n",
            "  lower: UInt7\n",
            "  tail: UInt7\n",
            "  flag: UInt7\n",
            "  code: UInt7\n",
            "  route: UInt7\n",
            "  marker: UInt6\n",
            "end\n",
            "\n",
            "\n",
            "pub fn direct(view: ByteView) -> Result<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int, marker: Int}, String>\n",
            "  byte_decode_packed_visible_seven_byte_header(view)\n",
            "end\n",
            "\n",
            "pub fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int, marker: Int}>\n",
            "  byte_decode_step_packed_visible_seven_byte_header(view, base)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int, marker: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packed_visible_seven_byte_header(packet)\n",
            "end\n",
            "\n",
            "pub fn item_decode(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int, marker: Int}>\n",
            "  decode PackedVisibleSevenByteHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn item_encode(packet: {high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int, marker: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PackedVisibleSevenByteHeader from packet\n",
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
    assert_eq!(schema.schema_name, "PackedVisibleSevenByteHeader");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.max_value))
            .collect::<Vec<_>>(),
        vec![
            ("high", 1, 0x1),
            ("upper", 1, 0x7f),
            ("middle", 1, 0x7f),
            ("lower", 1, 0x7f),
            ("tail", 1, 0x7f),
            ("flag", 1, 0x7f),
            ("code", 1, 0x7f),
            ("route", 1, 0x7f),
            ("marker", 1, 0x3f)
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_eight_byte_packed_visible_primitives() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PackedVisibleEightByteHeader\n",
            "  format binary\n",
            "\n",
            "  marker: UInt1\n",
            "  high: UInt7\n",
            "  upper: UInt7\n",
            "  middle: UInt7\n",
            "  lower: UInt7\n",
            "  tail: UInt7\n",
            "  flag: UInt7\n",
            "  code: UInt7\n",
            "  route: UInt7\n",
            "  checksum: UInt7\n",
            "end\n",
            "\n",
            "\n",
            "pub fn direct(view: ByteView) -> Result<{marker: Int, high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int, checksum: Int}, String>\n",
            "  byte_decode_packed_visible_eight_byte_header(view)\n",
            "end\n",
            "\n",
            "pub fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{marker: Int, high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int, checksum: Int}>\n",
            "  byte_decode_step_packed_visible_eight_byte_header(view, base)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {marker: Int, high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int, checksum: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packed_visible_eight_byte_header(packet)\n",
            "end\n",
            "\n",
            "pub fn item_decode(view: ByteView, base: ByteOffset) -> DecodeStep<{marker: Int, high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int, checksum: Int}>\n",
            "  decode PackedVisibleEightByteHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn item_encode(packet: {marker: Int, high: Int, upper: Int, middle: Int, lower: Int, tail: Int, flag: Int, code: Int, route: Int, checksum: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PackedVisibleEightByteHeader from packet\n",
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
    assert_eq!(schema.schema_name, "PackedVisibleEightByteHeader");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.max_value))
            .collect::<Vec<_>>(),
        vec![
            ("marker", 1, 0x1),
            ("high", 1, 0x7f),
            ("upper", 1, 0x7f),
            ("middle", 1, 0x7f),
            ("lower", 1, 0x7f),
            ("tail", 1, 0x7f),
            ("flag", 1, 0x7f),
            ("code", 1, 0x7f),
            ("route", 1, 0x7f),
            ("checksum", 1, 0x7f)
        ]
    );
}

#[test]
fn generated_schema_encode_helpers_resolve_for_closed_dispatch_binary_schemas() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ClosedDispatchWritePacket\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => UInt8, 2 => UInt16be, 3 => UInt24le, 4 => UInt32le)\n",
            "end\n",
            "\n",
            "pub fn main(packet: {kind: Int, payload: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_closed_dispatch_write_packet(packet)\n",
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
        } if name == "ClosedDispatchWritePacket"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "ClosedDispatchWritePacket");
    assert_eq!(schema.fields[0].name, "kind");
    assert_eq!(schema.fields[0].width, 1);
    assert_eq!(schema.fields[1].name, "payload");
    let dispatch = schema.fields[1]
        .dispatch
        .as_ref()
        .expect("payload should carry dispatch metadata");
    assert_eq!(dispatch.tag_field, "kind");
    assert_eq!(dispatch.length_field, None);
    assert_eq!(
        dispatch
            .cases
            .iter()
            .map(|case| (case.tag, case.width, case.little_endian))
            .collect::<Vec<_>>(),
        vec![(1, 1, false), (2, 2, false), (3, 3, true), (4, 4, true)]
    );
}

#[test]
fn generated_schema_encode_helpers_skip_length_bounded_closed_dispatch_binary_schemas() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema LengthBoundedClosedDispatchWritePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, length, 1 => UInt8)\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int, kind: Int, payload: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_length_bounded_closed_dispatch_write_packet(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("byte_encode_length_bounded_closed_dispatch_write_packet")),
        "{:#?}",
        lowered.diagnostics
    );
}

#[test]
fn generated_schema_encode_helpers_resolve_for_extension_dispatch_binary_schemas() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ExtensionDispatchWritePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 1 => UInt24le, 2 => UInt32le)\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int, kind: Int, payload: SchemaDispatchPayload<Int>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_extension_dispatch_write_packet(packet)\n",
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
        } if name == "ExtensionDispatchWritePacket"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "ExtensionDispatchWritePacket");
    assert_eq!(schema.fields[0].name, "length");
    assert_eq!(schema.fields[1].name, "kind");
    assert_eq!(schema.fields[2].name, "payload");
    let dispatch = schema.fields[2]
        .dispatch
        .as_ref()
        .expect("payload should carry extension dispatch metadata");
    assert_eq!(dispatch.tag_field, "kind");
    assert_eq!(dispatch.length_field.as_deref(), Some("length"));
    assert_eq!(
        dispatch
            .cases
            .iter()
            .map(|case| (case.tag, case.width, case.little_endian))
            .collect::<Vec<_>>(),
        vec![(1, 3, true), (2, 4, true)]
    );
}
