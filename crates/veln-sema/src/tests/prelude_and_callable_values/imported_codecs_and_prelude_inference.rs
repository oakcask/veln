use super::*;

#[test]
fn codec_derive_resolves_two_byte_prefix_reserved_group_boundaries() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema TwoBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(10, 682)\n",
            "  high: UInt3\n",
            "  low: UInt3\n",
            "end\n",
            "\n",
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  decode TwoBytePrefixReservedGroupHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode TwoBytePrefixReservedGroupHeader from packet\n",
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
        } if name == "TwoBytePrefixReservedGroupHeader"
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
        } if name == "TwoBytePrefixReservedGroupHeader"
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
        } if name == "TwoBytePrefixReservedGroupHeader"
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
        } if name == "TwoBytePrefixReservedGroupHeader"
    ));
}

#[test]
fn codec_derive_resolves_three_byte_prefix_reserved_group_boundaries() {
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
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  decode ThreeBytePrefixReservedGroupHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode ThreeBytePrefixReservedGroupHeader from packet\n",
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
        } if name == "ThreeBytePrefixReservedGroupHeader"
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
        } if name == "ThreeBytePrefixReservedGroupHeader"
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
        } if name == "ThreeBytePrefixReservedGroupHeader"
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
        } if name == "ThreeBytePrefixReservedGroupHeader"
    ));
}

#[test]
fn codec_derive_resolves_four_byte_prefix_reserved_group_boundaries() {
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
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  decode FourBytePrefixReservedGroupHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode FourBytePrefixReservedGroupHeader from packet\n",
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
        } if name == "FourBytePrefixReservedGroupHeader"
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
        } if name == "FourBytePrefixReservedGroupHeader"
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
        } if name == "FourBytePrefixReservedGroupHeader"
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
        } if name == "FourBytePrefixReservedGroupHeader"
    ));
}

#[test]
fn codec_derive_resolves_five_byte_prefix_reserved_group_boundaries() {
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
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  decode FiveBytePrefixReservedGroupHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode FiveBytePrefixReservedGroupHeader from packet\n",
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
        } if name == "FiveBytePrefixReservedGroupHeader"
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
        } if name == "FiveBytePrefixReservedGroupHeader"
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
        } if name == "FiveBytePrefixReservedGroupHeader"
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
        } if name == "FiveBytePrefixReservedGroupHeader"
    ));
}

#[test]
fn codec_derive_resolves_six_byte_prefix_reserved_group_boundaries() {
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
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  decode SixBytePrefixReservedGroupHeader from view at base\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode SixBytePrefixReservedGroupHeader from packet\n",
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
        } if name == "SixBytePrefixReservedGroupHeader"
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
        } if name == "SixBytePrefixReservedGroupHeader"
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
        } if name == "SixBytePrefixReservedGroupHeader"
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
        } if name == "SixBytePrefixReservedGroupHeader"
    ));
}

#[test]
fn codec_derive_decode_resolves_nested_dispatch_schema_decode_step_boundary() {
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
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{kind: Int, payload: {code: Int, value: Int}}>\n",
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
