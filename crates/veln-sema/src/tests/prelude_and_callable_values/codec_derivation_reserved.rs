use super::*;

#[test]
fn codec_decode_with_resolves_as_named_decode_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "\n",
            "fn decode_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  NeedMore(NeedEnd)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode PacketWire from view at base\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
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
            target: CoreCallTarget::SchemaDecodeStep(name),
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
            target: IrCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn codec_encode_with_resolves_as_named_encode_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "\n",
            "fn encode_packet(packet: {length: Int}) -> EncodeStep<String>\n",
            "  Encoded(list_nil())\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int}) -> EncodeStep<String>\n",
            "  encode_packet(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
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
            target: CoreCallTarget::Function(name),
            ..
        } if name == "encode_packet"
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
            target: IrCallTarget::Function(name),
            ..
        } if name == "encode_packet"
    ));
}

#[test]
fn bidirectional_codec_call_uses_expected_return_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "\n",
            "fn decode_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  NeedMore(NeedEnd)\n",
            "end\n",
            "\n",
            "fn encode_packet(packet: {length: Int}) -> EncodeStep<String>\n",
            "  Encoded(list_nil())\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int}) -> EncodeStep<String>\n",
            "  encode_packet(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
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
            target: CoreCallTarget::Function(name),
            ..
        } if name == "encode_packet"
    ));
}

#[test]
fn codec_derive_decode_resolves_as_schema_decode_step_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "end\n",
            "\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{wire_length: Int}>\n",
            "  decode PacketWire from view at base\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
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
            target: CoreCallTarget::SchemaDecodeStep(name),
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
            target: IrCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn codec_derive_decode_resolves_added_and_subtracted_byte_view_schema_decode_step_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema AddPacketWire\n",
            "  format binary\n",
            "\n",
            "  header_length: UInt8\n",
            "  body_length: UInt8\n",
            "  payload: ByteView(header_length + body_length)\n",
            "end\n",
            "\n",
            "schema SubtractPacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8\n",
            "  payload: ByteView(length - padding_length)\n",
            "end\n",
            "\n",
            "\n",
            "\n",
            "pub fn read_add(view: ByteView, base: ByteOffset) -> DecodeStep<{header_length: Int, body_length: Int, payload: ByteView}>\n",
            "  decode AddPacketWire from view at base\n",
            "end\n",
            "\n",
            "pub fn read_subtract(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int, padding_length: Int, payload: ByteView}>\n",
            "  decode SubtractPacketWire from view at base\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for (function_name, schema_name) in [
        ("read_add", "AddPacketWire"),
        ("read_subtract", "SubtractPacketWire"),
    ] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("schema decode wrapper should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(matches!(
            &expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaDecodeStep(name),
                ..
            } if name == schema_name
        ));
    }

    let ir = lowered.ir.expect("typed IR should be built");
    for (function_name, schema_name) in [
        ("read_add", "AddPacketWire"),
        ("read_subtract", "SubtractPacketWire"),
    ] {
        let function = ir
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("schema decode wrapper should be in IR");
        let IrStmtKind::Return { value } = &function.body[0].kind else {
            panic!("tail expression should lower as IR return");
        };
        assert!(matches!(
            &value.kind,
            IrExprKind::Call {
                target: IrCallTarget::SchemaDecodeStep(name),
                ..
            } if name == schema_name
        ));
    }
}

#[test]
fn codec_derive_encode_resolves_added_and_subtracted_byte_view_schema_encode_step_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema AddPacketWire\n",
            "  format binary\n",
            "\n",
            "  header_length: UInt8\n",
            "  body_length: UInt8\n",
            "  payload: ByteView(header_length + body_length)\n",
            "end\n",
            "\n",
            "schema SubtractPacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8\n",
            "  payload: ByteView(length - padding_length)\n",
            "end\n",
            "\n",
            "\n",
            "\n",
            "pub fn write_add(packet: {header_length: Int, body_length: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  encode AddPacketWire from packet\n",
            "end\n",
            "\n",
            "pub fn write_subtract(packet: {length: Int, padding_length: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  encode SubtractPacketWire from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for (function_name, schema_name) in [
        ("write_add", "AddPacketWire"),
        ("write_subtract", "SubtractPacketWire"),
    ] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("schema encode wrapper should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(matches!(
            &expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaEncode(name),
                ..
            } if name == schema_name
        ));
    }

    let ir = lowered.ir.expect("typed IR should be built");
    for (function_name, schema_name) in [
        ("write_add", "AddPacketWire"),
        ("write_subtract", "SubtractPacketWire"),
    ] {
        let function = ir
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("schema encode wrapper should be in IR");
        let IrStmtKind::Return { value } = &function.body[0].kind else {
            panic!("tail expression should lower as IR return");
        };
        assert!(matches!(
            &value.kind,
            IrExprKind::Call {
                target: IrCallTarget::SchemaEncode(name),
                ..
            } if name == schema_name
        ));
    }
}

#[test]
fn codec_derive_decode_resolves_quotient_byte_view_schema_decode_step_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  total_length: UInt8\n",
            "  chunk_count: UInt8\n",
            "  payload: ByteView(total_length / chunk_count)\n",
            "end\n",
            "\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{total_length: Int, chunk_count: Int, payload: ByteView}>\n",
            "  decode PacketWire from view at base\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
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
            target: CoreCallTarget::SchemaDecodeStep(name),
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
            target: IrCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn codec_derive_encode_resolves_quotient_byte_view_schema_encode_step_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  total_length: UInt8\n",
            "  chunk_count: UInt8\n",
            "  payload: ByteView(total_length / chunk_count)\n",
            "end\n",
            "\n",
            "\n",
            "pub fn main(packet: {total_length: Int, chunk_count: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  encode PacketWire from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
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
fn codec_derive_decode_resolves_middle_reserved_schema_decode_step_boundary() {
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
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  decode MiddleReservedHeader from view at base\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
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
            target: CoreCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "MiddleReservedHeader"
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
            target: IrCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "MiddleReservedHeader"
    ));
}

#[test]
fn codec_derive_resolves_byte_interleaved_middle_reserved_boundaries() {
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
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, middle: Int, low: Int}>\n",
            "  decode ByteInterleavedMiddleReservedHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, middle: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode ByteInterleavedMiddleReservedHeader from packet\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let read_header = core
        .functions
        .iter()
        .find(|function| function.name == "read_header")
        .expect("read_header should be lowered");
    let CoreStmtKind::Return { expr } = &read_header.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "ByteInterleavedMiddleReservedHeader"
    ));

    let write_header = core
        .functions
        .iter()
        .find(|function| function.name == "write_header")
        .expect("write_header should be lowered");
    let CoreStmtKind::Return { expr } = &write_header.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "ByteInterleavedMiddleReservedHeader"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let read_header = ir
        .functions
        .iter()
        .find(|function| function.name == "read_header")
        .expect("read_header should be in IR");
    let IrStmtKind::Return { value } = &read_header.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "ByteInterleavedMiddleReservedHeader"
    ));

    let write_header = ir
        .functions
        .iter()
        .find(|function| function.name == "write_header")
        .expect("write_header should be in IR");
    let IrStmtKind::Return { value } = &write_header.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaEncode(name),
            ..
        } if name == "ByteInterleavedMiddleReservedHeader"
    ));
}
