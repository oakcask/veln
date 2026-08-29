use super::*;

#[test]
fn generated_schema_encode_helpers_resolve_for_nested_dispatch_binary_schemas() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema SettingsPayload\n",
            "  format binary\n",
            "\n",
            "  code: UInt8\n",
            "  value: UInt16be\n",
            "end\n",
            "\n",
            "schema ExtensionNestedWritePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 1 => SettingsPayload)\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int, kind: Int, payload: SchemaDispatchPayload<{code: Int, value: Int}>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_extension_nested_write_packet(packet)\n",
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
        } if name == "ExtensionNestedWritePacket"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "ExtensionNestedWritePacket")
        .expect("nested packet encoder metadata should be emitted");
    let dispatch = schema.fields[2]
        .dispatch
        .as_ref()
        .expect("payload should carry extension dispatch metadata");
    assert_eq!(dispatch.length_field.as_deref(), Some("length"));
    assert_eq!(dispatch.cases[0].tag, 1);
    assert_eq!(dispatch.cases[0].width, 0);
    assert_eq!(
        dispatch.cases[0]
            .payload_schema
            .as_ref()
            .expect("case should carry nested schema metadata")
            .schema_name,
        "SettingsPayload"
    );
}

#[test]
fn derived_codec_encode_resolves_to_schema_encode_step_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt16be\n",
            "  kind: UInt8\n",
            "end\n",
            "\n",
            "\n",
            "pub fn main(packet: {length: Int, kind: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PacketWire from packet\n",
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
        } if name == "PacketWire"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaEncode(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn derived_codec_encode_resolves_length_bounded_byte_view_schema_boundary() {
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
            "\n",
            "pub fn main(packet: {length: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PacketWire from packet\n",
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
        } if name == "PacketWire"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaEncode(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn derived_codec_encode_resolves_nested_dispatch_schema_encode_step_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema SettingsPayload\n",
            "  format binary\n",
            "\n",
            "  code: UInt8\n",
            "  value: UInt16be\n",
            "end\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => SettingsPayload)\n",
            "end\n",
            "\n",
            "\n",
            "pub fn main(packet: {kind: Int, payload: {code: Int, value: Int}}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PacketWire from packet\n",
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
        } if name == "PacketWire"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaEncode(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn derived_codec_resolves_combined_binary_schema_helper_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema TelemetryPayload\n",
            "  format binary\n",
            "\n",
            "  channel_reserved: ReservedBits(3, 5)\n",
            "  channel: UInt5\n",
            "  reading: UInt16le\n",
            "end\n",
            "\n",
            "schema TelemetryEnvelope\n",
            "  format binary\n",
            "\n",
            "  section_length: UInt8\n",
            "  payload_length: UInt8\n",
            "  kind: UInt8\n",
            "  flags: UInt8\n",
            "  sample_count: UInt8\n",
            "  samples: Repeat(sample_count, UInt16be)\n",
            "  padding: ReservedBits(8, 0)\n",
            "  metadata: ByteView(section_length - payload_length)\n",
            "  payload: ExtensionDispatch(kind, payload_length, 1 => TelemetryPayload)\n",
            "end\n",
            "\n",
            "\n",
            "pub fn decode_main(view: ByteView, base: ByteOffset) -> DecodeStep<{section_length: Int, payload_length: Int, kind: Int, flags: Int, sample_count: Int, samples: List<Int>, metadata: ByteView, payload: SchemaDispatchPayload<{channel: Int, reading: Int}>}>\n",
            "  decode TelemetryEnvelope from view at base\n",
            "end\n",
            "\n",
            "pub fn encode_main(packet: {section_length: Int, payload_length: Int, kind: Int, flags: Int, sample_count: Int, samples: List<Int>, metadata: ByteView, payload: SchemaDispatchPayload<{channel: Int, reading: Int}>}) -> Result<ByteChunk, EncodeError>\n",
            "  encode TelemetryEnvelope from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let decode_main = core
        .functions
        .iter()
        .find(|function| function.name == "decode_main")
        .expect("decode_main should be lowered");
    let CoreStmtKind::Return { expr } = &decode_main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "TelemetryEnvelope"
    ));

    let encode_main = core
        .functions
        .iter()
        .find(|function| function.name == "encode_main")
        .expect("encode_main should be lowered");
    let CoreStmtKind::Return { expr } = &encode_main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "TelemetryEnvelope"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let decode_main = ir
        .functions
        .iter()
        .find(|function| function.name == "decode_main")
        .expect("decode_main should be in IR");
    let IrStmtKind::Return { value } = &decode_main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "TelemetryEnvelope"
    ));

    let encode_main = ir
        .functions
        .iter()
        .find(|function| function.name == "encode_main")
        .expect("encode_main should be in IR");
    let IrStmtKind::Return { value } = &encode_main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaEncode(name),
            ..
        } if name == "TelemetryEnvelope"
    ));
}

#[test]
fn derived_codec_resolves_added_repeat_count_helper_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema CountedValues\n",
            "  format binary\n",
            "\n",
            "  left_count: UInt8\n",
            "  right_count: UInt8\n",
            "  items: Repeat(left_count + right_count, UInt16be)\n",
            "end\n",
            "\n",
            "\n",
            "pub fn decode_main(view: ByteView, base: ByteOffset) -> DecodeStep<{left_count: Int, right_count: Int, items: List<Int>}>\n",
            "  decode CountedValues from view at base\n",
            "end\n",
            "\n",
            "pub fn encode_main(packet: {left_count: Int, right_count: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
            "  encode CountedValues from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let decode_main = core
        .functions
        .iter()
        .find(|function| function.name == "decode_main")
        .expect("decode_main should be lowered");
    let CoreStmtKind::Return { expr } = &decode_main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "CountedValues"
    ));

    let encode_main = core
        .functions
        .iter()
        .find(|function| function.name == "encode_main")
        .expect("encode_main should be lowered");
    let CoreStmtKind::Return { expr } = &encode_main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "CountedValues"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let decode_main = ir
        .functions
        .iter()
        .find(|function| function.name == "decode_main")
        .expect("decode_main should be in IR");
    let IrStmtKind::Return { value } = &decode_main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "CountedValues"
    ));

    let encode_main = ir
        .functions
        .iter()
        .find(|function| function.name == "encode_main")
        .expect("encode_main should be in IR");
    let IrStmtKind::Return { value } = &encode_main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaEncode(name),
            ..
        } if name == "CountedValues"
    ));
}

#[test]
fn derived_codec_resolves_product_repeat_count_helper_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema CountedValues\n",
            "  format binary\n",
            "\n",
            "  row_count: UInt8\n",
            "  column_count: UInt8\n",
            "  items: Repeat(row_count * column_count, UInt16be)\n",
            "end\n",
            "\n",
            "\n",
            "pub fn decode_main(view: ByteView, base: ByteOffset) -> DecodeStep<{row_count: Int, column_count: Int, items: List<Int>}>\n",
            "  decode CountedValues from view at base\n",
            "end\n",
            "\n",
            "pub fn encode_main(packet: {row_count: Int, column_count: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
            "  encode CountedValues from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let decode_main = core
        .functions
        .iter()
        .find(|function| function.name == "decode_main")
        .expect("decode_main should be lowered");
    let CoreStmtKind::Return { expr } = &decode_main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "CountedValues"
    ));

    let encode_main = core
        .functions
        .iter()
        .find(|function| function.name == "encode_main")
        .expect("encode_main should be lowered");
    let CoreStmtKind::Return { expr } = &encode_main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "CountedValues"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let decode_main = ir
        .functions
        .iter()
        .find(|function| function.name == "decode_main")
        .expect("decode_main should be in IR");
    let IrStmtKind::Return { value } = &decode_main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "CountedValues"
    ));

    let encode_main = ir
        .functions
        .iter()
        .find(|function| function.name == "encode_main")
        .expect("encode_main should be in IR");
    let IrStmtKind::Return { value } = &encode_main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaEncode(name),
            ..
        } if name == "CountedValues"
    ));
}

#[test]
fn derived_codec_resolves_quotient_repeat_count_helper_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema CountedValues\n",
            "  format binary\n",
            "\n",
            "  total_count: UInt8\n",
            "  group_count: UInt8\n",
            "  items: Repeat(total_count / group_count, UInt16be)\n",
            "end\n",
            "\n",
            "\n",
            "pub fn decode_main(view: ByteView, base: ByteOffset) -> DecodeStep<{total_count: Int, group_count: Int, items: List<Int>}>\n",
            "  decode CountedValues from view at base\n",
            "end\n",
            "\n",
            "pub fn encode_main(packet: {total_count: Int, group_count: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
            "  encode CountedValues from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let decode_main = core
        .functions
        .iter()
        .find(|function| function.name == "decode_main")
        .expect("decode_main should be lowered");
    let CoreStmtKind::Return { expr } = &decode_main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "CountedValues"
    ));

    let encode_main = core
        .functions
        .iter()
        .find(|function| function.name == "encode_main")
        .expect("encode_main should be lowered");
    let CoreStmtKind::Return { expr } = &encode_main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "CountedValues"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let decode_main = ir
        .functions
        .iter()
        .find(|function| function.name == "decode_main")
        .expect("decode_main should be in IR");
    let IrStmtKind::Return { value } = &decode_main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "CountedValues"
    ));

    let encode_main = ir
        .functions
        .iter()
        .find(|function| function.name == "encode_main")
        .expect("encode_main should be in IR");
    let IrStmtKind::Return { value } = &encode_main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaEncode(name),
            ..
        } if name == "CountedValues"
    ));
}
