use super::*;

#[test]
fn generated_schema_decode_helpers_resolve_from_binary_schema_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ArithmeticPacket\n",
            "  format binary\n",
            "\n",
            "  width: UInt8\n",
            "  item_count: UInt8\n",
            "  payload_length: UInt16be where not (payload_length != width * item_count) and true\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{width: Int, item_count: Int, payload_length: Int}, String>\n",
            "  byte_decode_arithmetic_packet(view)\n",
            "end\n",
            "\n",
            "pub fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{width: Int, item_count: Int, payload_length: Int}>\n",
            "  byte_decode_step_arithmetic_packet(view, base)\n",
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
            target: CoreCallTarget::SchemaDecode(name),
            ..
        } if name == "ArithmeticPacket"
    ));
    let step = core
        .functions
        .iter()
        .find(|function| function.name == "step")
        .expect("step should be lowered");
    let CoreStmtKind::Return { expr } = &step.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "ArithmeticPacket"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "ArithmeticPacket");
    assert_eq!(schema.function_name, "byte_decode_arithmetic_packet");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width, field.predicate.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            ("width", 1, None),
            ("item_count", 1, None),
            (
                "payload_length",
                2,
                Some("not(payload_length != width * item_count) and true"),
            ),
        ]
    );
    assert!(schema.mapping.is_empty());
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
            "  stream_id: UInt31be\n",
            "  wide_value: UInt32be\n",
            "end\n",
            "\n",
            "pub fn main(packet: {short_value: Int, stream_id: Int, wide_value: Int}) -> Result<ByteChunk, EncodeError>\n",
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
            .map(|field| (field.name.as_str(), field.width, field.max_value))
            .collect::<Vec<_>>(),
        vec![
            ("short_value", 2, 0xffff),
            ("stream_id", 4, 0x7fffffff),
            ("wide_value", 4, 0xffffffff),
        ]
    );
}

#[test]
fn generated_schema_decode_helpers_return_mapped_record_shape() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type FrameHeader\n",
            "  FrameHeader {kind: Int, length: Int}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt16be\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to FrameHeader\n",
            "    length = wire_length\n",
            "    kind = wire_kind\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{kind: Int, length: Int}, String>\n",
            "  byte_decode_header_wire(view)\n",
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
    assert_eq!(schema.schema_name, "HeaderWire");
    assert_eq!(schema.function_name, "byte_decode_header_wire");
    assert_eq!(
        schema
            .mapping
            .iter()
            .map(|field| (field.target.as_str(), field.source.as_str()))
            .collect::<Vec<_>>(),
        vec![("kind", "wire_kind"), ("length", "wire_length")]
    );
}

#[test]
fn generated_schema_decode_helpers_keep_closed_dispatch_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ClosedDispatchPacket\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => UInt16be, 2 => UInt32be)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{kind: Int, payload: Int}, String>\n",
            "  byte_decode_closed_dispatch_packet(view)\n",
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
    assert_eq!(schema.schema_name, "ClosedDispatchPacket");
    assert_eq!(schema.function_name, "byte_decode_closed_dispatch_packet");
    assert_eq!(schema.fields[0].name, "kind");
    assert_eq!(schema.fields[0].width, 1);
    assert!(schema.fields[0].dispatch.is_none());
    assert_eq!(schema.fields[1].name, "payload");
    assert_eq!(schema.fields[1].width, 0);
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
            .map(|case| (case.tag, case.width))
            .collect::<Vec<_>>(),
        vec![(1, 2), (2, 4)]
    );
}

#[test]
fn generated_schema_decode_helpers_keep_extension_dispatch_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ExtensionDispatchPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 1 => UInt16be, 2 => UInt32be)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{length: Int, kind: Int, payload: SchemaDispatchPayload<Int>}, String>\n",
            "  byte_decode_extension_dispatch_packet(view)\n",
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
    assert_eq!(schema.schema_name, "ExtensionDispatchPacket");
    assert_eq!(
        schema.function_name,
        "byte_decode_extension_dispatch_packet"
    );
    assert_eq!(schema.fields[2].name, "payload");
    assert_eq!(schema.fields[2].width, 0);
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
            .map(|case| (case.tag, case.width))
            .collect::<Vec<_>>(),
        vec![(1, 2), (2, 4)]
    );
}

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
            "codec PacketCodec for PacketWire decode\n",
            "  decode with decode_packet\n",
            "end\n",
            "\n",
            "fn decode_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  NeedMore(NeedEnd)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  PacketCodec(view, base)\n",
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
        } if name == "decode_packet"
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
        } if name == "decode_packet"
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
            "codec PacketCodec for PacketWire encode\n",
            "  encode with encode_packet\n",
            "end\n",
            "\n",
            "fn encode_packet(packet: {length: Int}) -> EncodeStep<String>\n",
            "  Encoded(list_nil())\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int}) -> EncodeStep<String>\n",
            "  PacketCodec(packet)\n",
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
            "codec PacketCodec for PacketWire decode encode\n",
            "  decode with decode_packet\n",
            "  encode with encode_packet\n",
            "end\n",
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
            "  PacketCodec(packet)\n",
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
            "type Packet\n",
            "  Packet {length: Int}\n",
            "end\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "\n",
            "  map to Packet\n",
            "    length = wire_length\n",
            "end\n",
            "\n",
            "codec PacketCodec for PacketWire decode\n",
            "  derive decode\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  PacketCodec(view, base)\n",
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
fn imported_public_codec_decode_resolves_through_qualified_module_path() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  wire::PacketCodec(view, base)\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "pub codec PacketCodec for PacketWire decode\n",
            "  decode with decode_packet\n",
            "end\n",
            "\n",
            "fn decode_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  NeedMore(NeedEnd)\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        types: Vec::new(),
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: [app.functions, wire.functions].concat(),
    };

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
        } if name == "decode_packet"
    ));
}

#[test]
fn imported_public_codec_encode_resolves_through_qualified_module_path() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(packet: {length: Int}) -> EncodeStep<String>\n",
            "  wire::PacketCodec(packet)\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "pub codec PacketCodec for PacketWire encode\n",
            "  encode with encode_packet\n",
            "end\n",
            "\n",
            "fn encode_packet(packet: {length: Int}) -> EncodeStep<String>\n",
            "  Encoded(list_nil())\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        types: Vec::new(),
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: [app.functions, wire.functions].concat(),
    };

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
fn imported_public_derived_codec_decode_resolves_through_qualified_module_path() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  wire::PacketCodec(view, base)\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "type Packet\n",
            "  Packet {length: Int}\n",
            "end\n",
            "\n",
            "pub schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "\n",
            "  map to Packet\n",
            "    length = wire_length\n",
            "end\n",
            "\n",
            "pub codec PacketCodec for PacketWire decode\n",
            "  derive decode\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        types: wire.types,
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: app.functions,
    };

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
}

#[test]
fn imported_codec_decode_does_not_resolve_as_bare_call() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  PacketCodec(view, base)\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "pub codec PacketCodec for PacketWire decode\n",
            "  decode with decode_packet\n",
            "end\n",
            "\n",
            "fn decode_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  NeedMore(NeedEnd)\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        types: Vec::new(),
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: [app.functions, wire.functions].concat(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `PacketCodec`"
    }));
}

#[test]
fn infers_prelude_helper_calls_from_expected_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "pub fn main(items: Vec<Int>, other: Vec<Int>, table: Dict<String, Int>, ",
            "list: List<Int>, one_byte: Byte, chunk: ByteChunk, other_chunk: ByteChunk, ",
            "view: ByteView, count: ByteCount, offset: ByteOffset, ",
            "mapper: fn(Int) -> String, keep: fn(Int) -> Bool, folder: fn(String, Int) -> String, ",
            "fallible: fn(Int) -> Result<String, AppError>, opt: Option<Int>, ",
            "fallible_with: fn(String, Int) -> Result<String, AppError>, ",
            "opt_map: fn(Int) -> String, opt_next: fn(Int) -> Option<String>, ",
            "res: Result<Int, AppError>, err_map: fn(AppError) -> String, ",
            "res_next: fn(Int) -> Result<String, AppError>) -> {",
            "count: Int, empty: Bool, byte_value: Result<Byte, String>, byte_int: Int, ",
            "chunk_value: ByteChunk, chunk_count: ByteCount, appended: ByteChunk, ",
            "hex_chunk: Result<ByteChunk, String>, taken: Result<ByteChunk, String>, dropped: Result<ByteChunk, String>, ",
            "view_value: Result<ByteView, String>, view_chunk: ByteChunk, ",
            "read_u8: Result<Int, String>, expect_u8: Result<Int, String>, ",
            "decoded_header: Result<{length: Int, kind: Int, flags: Int, stream_id: Int}, String>, ",
            "decoded_frame: Result<{length: Int, kind: Int, flags: Int, stream_id: Int, payload: ByteView}, String>, ",
            "decoded_widths: Result<{short_value: Int, wide_value: Int}, String>, ",
            "decoded_validation: Result<{length: Int, padding_length: Int}, String>, ",
            "closed_http2: Result<(), String>, continuation_http2: Result<(), String>, ",
            "invalid_kind_http2: Result<(), String>, frame_size_http2: Result<(), String>, ",
            "settings_value_http2: Result<(), String>, ",
            "read_u16: Result<Int, String>, read_u24: Result<Int, String>, ",
            "read_u31: Result<Int, String>, read_u32: Result<Int, String>, ",
            "write_u8: Result<ByteChunk, String>, write_u16: Result<ByteChunk, String>, ",
            "write_u24: Result<ByteChunk, String>, write_u31: Result<ByteChunk, String>, ",
            "write_u32: Result<ByteChunk, String>, ",
            "count_value: Result<ByteCount, String>, count_int: Int, ",
            "offset_value: Result<ByteOffset, String>, offset_int: Int, ",
            "pushed: Vec<Int>, joined: Vec<Int>, mapped: Vec<String>, ",
            "filtered: Vec<Int>, folded: String, tried: Result<Vec<String>, AppError>, ",
            "tried_with: Result<Vec<String>, AppError>, split: Option<{left: String, right: String}>, ",
            "parsed: Result<Int, String>, rendered: String, ",
            "list_nil: List<Int>, list_cons: List<Int>, list_empty: Bool, list_folded: String, ",
            "list_reversed: List<Int>, list_mapped: List<String>, list_filtered: List<Int>, ",
            "list_tried: Result<List<String>, AppError>, ",
            "found: Option<Int>, has_key: Bool, inserted: Dict<String, Int>, removed: Dict<String, Int>, ",
            "opt_mapped: Option<String>, opt_nexted: Option<String>, opt_value: Int, ",
            "res_mapped: Result<String, AppError>, res_err: Result<Int, String>, ",
            "res_nexted: Result<String, AppError>}\n",
            "  {count: vec_len(items), empty: vec_is_empty(items), ",
            "byte_value: byte(1), byte_int: byte_to_int(one_byte), ",
            "chunk_value: byte_chunk([one_byte]), chunk_count: byte_chunk_count(chunk), ",
            "appended: byte_append(chunk, other_chunk), hex_chunk: byte_chunk_from_hex(\"00 ff\"), ",
            "taken: byte_take(chunk, count), ",
            "dropped: byte_drop(chunk, count), view_value: byte_view(chunk, offset, count), ",
            "view_chunk: byte_view_to_chunk(view), read_u8: byte_read_u8_be(view), ",
            "expect_u8: byte_expect_fixed_u8_be(view, 1, \"DemoPacket\", \"kind\"), ",
            "decoded_header: byte_decode_http2_frame_header(view), ",
            "decoded_frame: byte_decode_http2_frame(view), ",
            "decoded_widths: byte_decode_schema_width_sample(view), ",
            "decoded_validation: byte_decode_schema_validation_sample(view), ",
            "closed_http2: http2_protocol_closed_with_pending(0, 4, \"none\"), ",
            "continuation_http2: http2_protocol_continuation_expected(9, 0, 1, 1, 1, 0, \"headers\"), ",
            "invalid_kind_http2: http2_protocol_invalid_frame_kind(0, 0, 0, 4, \"connection-control\", \"connection_frames_require_settings\"), ",
            "frame_size_http2: http2_peer_limit_frame_size_exceeded(0, 16385, 16384, 0, 3, \"protocol_default\"), ",
            "settings_value_http2: http2_peer_limit_settings_value_out_of_range(9, 5, \"SETTINGS_MAX_FRAME_SIZE\", 16383, 16384, 16777215, \"peer_settings\"), ",
            "read_u16: byte_read_u16_be(view), read_u24: byte_read_u24_be(view), ",
            "read_u31: byte_read_u31_be(view), read_u32: byte_read_u32_be(view), ",
            "write_u8: byte_write_u8_be(1), write_u16: byte_write_u16_be(1), ",
            "write_u24: byte_write_u24_be(1), write_u31: byte_write_u31_be(1), ",
            "write_u32: byte_write_u32_be(1), count_value: byte_count(1), ",
            "count_int: byte_count_to_int(count), offset_value: byte_offset(1), ",
            "offset_int: byte_offset_to_int(offset), ",
            "pushed: vec_push(items, 1), joined: vec_concat(items, other), ",
            "mapped: vec_map(items, mapper), filtered: vec_filter(items, keep), ",
            "folded: vec_fold(items, \"\", folder), tried: vec_try_map(items, fallible), ",
            "tried_with: vec_try_map_with(\"prefix\", items, fallible_with), ",
            "split: string_split_once(\"sku,2\", \",\"), parsed: string_parse_int(\"2\"), ",
            "rendered: int_to_string(2), ",
            "list_nil: list_nil(), list_cons: list_cons(1, list_nil()), ",
            "list_empty: list_is_empty(list), list_folded: list_fold(list, \"\", folder), ",
            "list_reversed: list_reverse(list), list_mapped: list_map(list, mapper), ",
            "list_filtered: list_filter(list, keep), list_tried: list_try_map(list, fallible), ",
            "found: dict_get(table, \"a\"), has_key: dict_contains(table, \"a\"), ",
            "inserted: dict_insert(table, \"b\", 2), removed: dict_remove(table, \"b\"), ",
            "opt_mapped: option_map(opt, opt_map), opt_nexted: option_and_then(opt, opt_next), ",
            "opt_value: option_unwrap_or(opt, 0), res_mapped: result_map(res, opt_map), ",
            "res_err: result_map_err(res, err_map), res_nexted: result_and_then(res, res_next)}\n",
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
    let CoreExprKind::Record(fields) = &expr.kind else {
        panic!("prelude results should be returned in a record");
    };
    let first = fields
        .first()
        .expect("record should contain prelude result fields");
    assert!(matches!(
        &first.expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "vec_len"
    ));
    assert!(matches!(first.expr.ty, CoreType::Named { ref name, .. } if name == "Int"));
    let source_backed_prelude_names =
        crate::standard_symbols::source_backed_prelude_names().collect::<Vec<_>>();
    let core_prelude_calls = fields
        .iter()
        .filter_map(|field| match &field.expr.kind {
            CoreExprKind::Call {
                target: CoreCallTarget::PreludeBuiltin(name),
                ..
            } => Some(name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for name in &source_backed_prelude_names {
        assert!(
            core_prelude_calls.contains(name),
            "{name} should keep prelude core lowering"
        );
    }
    let ir = lowered
        .ir
        .expect("complete prelude core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Record(fields) = &value.kind else {
        panic!("prelude record should lower to IR");
    };
    assert!(matches!(
        &fields[0].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "vec_len"
    ));
    let ir_prelude_calls = fields
        .iter()
        .filter_map(|field| match &field.value.kind {
            IrExprKind::Call {
                target: IrCallTarget::PreludeBuiltin(name),
                ..
            } => Some(name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for name in &source_backed_prelude_names {
        assert!(
            ir_prelude_calls.contains(name),
            "{name} should keep prelude IR lowering"
        );
    }
}

#[test]
fn lowers_qualified_prelude_builtin_calls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(items: Vec<Int>) -> Int\n",
            "  prelude_builtin::vec_len(items)\n",
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
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "vec_len"
    ));
}

#[test]
fn lowers_qualified_standard_prelude_calls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(items: Vec<Int>) -> Int\n",
            "  prelude::vec_len(items)\n",
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
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "vec_len"
    ));
}

#[test]
fn stream_input_constructors_resolve_through_standard_prelude_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn bare(chunk: ByteChunk) -> StreamInput\n",
            "  Chunk(chunk)\n",
            "end\n",
            "fn type_qualified(chunk: ByteChunk) -> StreamInput\n",
            "  StreamInput::Chunk(chunk)\n",
            "end\n",
            "fn prelude_qualified(chunk: ByteChunk) -> StreamInput\n",
            "  prelude::Chunk(chunk)\n",
            "end\n",
            "fn prelude_type_qualified(chunk: ByteChunk) -> StreamInput\n",
            "  prelude::StreamInput::Chunk(chunk)\n",
            "end\n",
            "fn done() -> StreamInput\n",
            "  prelude::End\n",
            "end\n",
            "fn decoded(count: ByteCount) -> DecodeStep<Int>\n",
            "  Decoded(7, count)\n",
            "end\n",
            "fn waiting(count: ByteCount) -> DecodeStep<Int>\n",
            "  prelude::DecodeStep::NeedMore(prelude::DecodeReadiness::NeedBytes(count))\n",
            "end\n",
            "fn waiting_for_end() -> DecodeStep<Int>\n",
            "  DecodeStep::NeedMore(NeedEnd)\n",
            "end\n",
            "fn invalid(offset: ByteOffset) -> DecodeStep<Int>\n",
            "  prelude::Invalid(DecodeError(\"codec.invalid\", offset, \"demo.field\"))\n",
            "end\n",
            "fn encoded(chunks: List<ByteChunk>) -> EncodeStep<String>\n",
            "  Encoded(chunks)\n",
            "end\n",
            "fn partial(chunks: List<ByteChunk>, count: ByteCount) -> EncodeStep<String>\n",
            "  prelude::EncodeStep::Partial(chunks, count, \"waiting\")\n",
            "end\n",
            "fn invalid_encode() -> EncodeStep<String>\n",
            "  EncodeStep::Invalid(EncodeError(\"codec.out_of_range\", \"demo.length\", \"too large\"))\n",
            "end\n",
            "fn label(input: StreamInput) -> String\n",
            "  match input\n",
            "    prelude::StreamInput::Chunk(bytes) => int_to_string(byte_count_to_int(byte_chunk_count(bytes)))\n",
            "    prelude::End => \"end\"\n",
            "  end\n",
            "end\n",
            "fn decode_label(step: DecodeStep<Int>) -> String\n",
            "  match step\n",
            "    prelude::DecodeStep::Decoded(value, consumed) => int_to_string(value + byte_count_to_int(consumed))\n",
            "    NeedMore(prelude::DecodeReadiness::NeedBytes(count)) => int_to_string(byte_count_to_int(count))\n",
            "    NeedMore(prelude::NeedEnd) => \"end\"\n",
            "    prelude::DecodeStep::Invalid(DecodeError(id, _, _)) => id\n",
            "  end\n",
            "end\n",
            "fn encode_label(step: EncodeStep<String>) -> String\n",
            "  match step\n",
            "    prelude::EncodeStep::Encoded(chunks) => int_to_string(list_fold(chunks, 0, count_chunk))\n",
            "    Partial(_, _, state) => state\n",
            "    prelude::EncodeStep::Invalid(EncodeError(id, _, _)) => id\n",
            "  end\n",
            "end\n",
            "fn count_chunk(total: Int, chunk: ByteChunk) -> Int\n",
            "  total + byte_count_to_int(byte_chunk_count(chunk))\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for function_name in [
        "bare",
        "type_qualified",
        "prelude_qualified",
        "prelude_type_qualified",
    ] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .unwrap_or_else(|| panic!("{function_name} should be lowered"));
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("{function_name} should return a constructor");
        };
        assert_eq!(expr.ty, CoreType::named("StreamInput", Vec::new()));
        assert!(
            matches!(&expr.kind, CoreExprKind::AdtVariant { name, payloads }
                if name == &vec!["StreamInput".to_string(), "Chunk".to_string()]
                    && payloads.len() == 1),
            "{function_name} should lower to StreamInput::Chunk"
        );
    }
    let done = core
        .functions
        .iter()
        .find(|function| function.name == "done")
        .expect("done should be lowered");
    let CoreStmtKind::Return { expr } = &done.body[0].kind else {
        panic!("done should return a constructor");
    };
    assert_eq!(expr.ty, CoreType::named("StreamInput", Vec::new()));
    assert!(
        matches!(&expr.kind, CoreExprKind::AdtVariant { name, payloads }
            if name == &vec!["StreamInput".to_string(), "End".to_string()]
                && payloads.is_empty())
    );
    for function_name in ["decoded", "waiting", "waiting_for_end", "invalid"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .unwrap_or_else(|| panic!("{function_name} should be lowered"));
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("{function_name} should return a constructor");
        };
        assert_eq!(
            expr.ty,
            CoreType::named("DecodeStep", vec![CoreType::int()])
        );
    }
    for function_name in ["encoded", "partial", "invalid_encode"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .unwrap_or_else(|| panic!("{function_name} should be lowered"));
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("{function_name} should return a constructor");
        };
        assert_eq!(
            expr.ty,
            CoreType::named("EncodeStep", vec![CoreType::string()])
        );
    }
    let label = core
        .functions
        .iter()
        .find(|function| function.name == "label")
        .expect("label should be lowered");
    let CoreStmtKind::Return { expr } = &label.body[0].kind else {
        panic!("label should return a match");
    };
    let CoreExprKind::Match { arms, .. } = &expr.kind else {
        panic!("label should lower to a match");
    };
    assert!(
        matches!(&arms[0].pattern.kind, CorePatternKind::Constructor { name, args }
            if name == &vec!["StreamInput".to_string(), "Chunk".to_string()]
                && args.len() == 1)
    );
    assert!(
        matches!(&arms[1].pattern.kind, CorePatternKind::Constructor { name, args }
            if name == &vec!["StreamInput".to_string(), "End".to_string()]
                && args.is_empty())
    );
}

#[test]
fn source_backed_prelude_helper_source_is_embedded_and_checkable() {
    let mut entries = Vec::new();

    for symbol in crate::standard_symbols::source_backed_symbols() {
        let source = symbol.source.expect("source metadata");
        assert_eq!(symbol.name, source.entry);
        entries.push(source.entry);
        let file = SourceFile::new(source.path, source.text);
        let parsed = parse(&file);
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics for {}: {:#?}",
            source.path,
            parsed.diagnostics
        );

        let module = lower_surface_ast(&parsed.tree);
        let diagnostics = analyze_surface_module(&module);

        assert!(
            diagnostics.is_empty(),
            "unexpected source helper diagnostics for {}: {diagnostics:#?}",
            source.path
        );
        assert!(
            module
                .functions
                .iter()
                .any(|function| function.name.as_deref() == Some(source.entry)),
            "embedded source should define {}",
            source.entry
        );
    }

    let mut expected_entries =
        crate::standard_symbols::source_backed_prelude_names().collect::<Vec<_>>();
    entries.sort_unstable();
    expected_entries.sort_unstable();
    assert_eq!(entries, expected_entries);
}

#[test]
fn imported_public_function_conflicts_with_implicit_prelude_bare_call() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.measure\n",
            "pub fn main(items: Vec<Int>) -> Int\n",
            "  vec_len(items)\n",
            "end\n",
        ),
    );
    let measure_source = SourceFile::new(
        "measure.veln",
        concat!(
            "mod app.measure\n",
            "pub fn vec_len(items: Vec<Int>) -> Int\n",
            "  0\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let measure = lower_surface_ast(&parse(&measure_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: main
            .functions
            .into_iter()
            .chain(measure.functions)
            .collect(),
    };

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "name.ambiguous"
                && diagnostic.message == "ambiguous call_target `vec_len`"
        })
        .expect("prelude conflict should be ambiguous");
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>();
    assert!(
        related
            .iter()
            .any(|note| note.contains("use `measure::vec_len` to select it"))
    );
    assert!(
        related
            .iter()
            .any(|note| note.contains("use `prelude::vec_len` to select it"))
    );
}

#[test]
fn local_declaration_shadows_implicit_prelude_import() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn vec_len(items: String) -> Int\n",
            "  7\n",
            "end\n",
            "pub fn main() -> Int\n",
            "  vec_len(\"local\")\n",
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
    let CoreExprKind::Call { target, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert_eq!(target, &CoreCallTarget::Function("vec_len".to_string()));
}

#[test]
fn non_callable_local_shadow_blocks_implicit_prelude_call_resolution() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(items: Vec<Int>) -> Int\n",
            "  let vec_len: Int = 1\n",
            "  vec_len(items)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `vec_len`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn user_source_cannot_claim_prelude_module_alias() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.prelude\n",
            "pub fn main() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.reserved"
                && diagnostic.message
                    == "import alias `prelude` conflicts with the standard prelude"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn user_source_cannot_claim_prelude_module_identity() {
    let source = SourceFile::new(
        "main.veln",
        concat!("mod prelude\n", "pub fn main() -> Int\n", "  1\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.reserved"
                && diagnostic.message
                    == "module identity `prelude` conflicts with the standard prelude"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn compiler_support_source_loads_text_through_standard_fs_subset() {
    let source = crate::standard_symbols::compiler_support_sources()
        .find(|source| source.entry == "load_source_text")
        .expect("compiler support source should be embedded");
    let file = SourceFile::new(source.path, source.text);
    let parsed = parse(&file);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics for {}: {:#?}",
        source.path,
        parsed.diagnostics
    );

    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "unexpected compiler support diagnostics for {}: {:#?}",
        source.path,
        lowered.diagnostics
    );
    let core = lowered.core.expect("compiler support should lower to core");
    let function = core
        .functions
        .iter()
        .find(|function| function.name == source.entry)
        .expect("compiler support entry should lower");
    let CoreStmtKind::Let { expr, .. } = &function.body[0].kind else {
        panic!("first statement should call fs before wrapping the result");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Try(value) if matches!(
            &value.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::StandardLibraryBuiltin(name),
                ..
            } if name == "fs::read_to_string"
        )
    ));
}

#[test]
fn suggests_vec_try_map_for_result_returning_map_callback() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(value: Int) -> Result<String, AppError>\n",
            "  Ok(\"ok\")\n",
            "end\n",
            "pub fn main(items: Vec<Int>) -> Vec<String>\n",
            "  vec_map(items, parse)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.mismatch")
        .expect("callback type mismatch should be reported");
    assert_eq!(
        diagnostic.message,
        "expected `fn(unknown) -> String`, but found `fn(Int) -> Result<String, AppError>`"
    );
    assert!(
        diagnostic
            .related
            .iter()
            .any(|related| { related.to_json().contains("Use `vec_try_map`") })
    );
}

#[test]
fn lowers_function_declarations_as_callable_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn main(items: Vec<Int>) -> Vec<String>\n",
            "  vec_map(items, stringify)\n",
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
    let CoreExprKind::Call { args, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert!(matches!(
        &args[1].kind,
        CoreExprKind::FunctionValue(name) if name == "stringify"
    ));
    assert_eq!(
        args[1].ty,
        CoreType::Function {
            params: vec![CoreType::int()],
            return_type: Box::new(CoreType::string()),
            effects: Vec::new()
        }
    );

    let ir = lowered.ir.expect("complete core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Call { args, .. } = &value.kind else {
        panic!("tail expression should lower as IR call");
    };
    assert!(matches!(
        &args[1].kind,
        IrExprKind::FunctionValue(name) if name == "stringify"
    ));
}

#[test]
fn lowers_function_return_types_with_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn printer(text: String) -> () effects [stdio]\n",
            "  stdio::println(text)\n",
            "  ()\n",
            "end\n",
            "pub fn callback_factory() -> fn(String) -> () effects [stdio]\n",
            "  printer\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let factory = core
        .functions
        .iter()
        .find(|function| function.name == "callback_factory")
        .expect("factory should be lowered");
    assert_eq!(
        factory.return_type,
        CoreType::Function {
            params: vec![CoreType::string()],
            return_type: Box::new(CoreType::unit()),
            effects: vec!["stdio".to_string()],
        }
    );
    assert_eq!(factory.effects, Vec::<String>::new());
}

#[test]
fn function_return_effects_must_cover_actual_callable_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn printer(text: String) -> () effects [stdio]\n",
            "  stdio::println(text)\n",
            "  ()\n",
            "end\n",
            "pub fn callback_factory() -> fn(String) -> ()\n",
            "  printer\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(
        diagnostics[0].message,
        "expected `fn(String) -> ()`, but found `fn(String) -> () effects [stdio]`"
    );
}

#[test]
fn call_resolution_prefers_local_callable_over_function_declaration() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String\n",
            "  \"function\"\n",
            "end\n",
            "pub fn main(stringify: fn(Int) -> String effects []) -> String\n",
            "  stringify(1)\n",
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
    let CoreExprKind::Call { target, args } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert_eq!(target, &CoreCallTarget::Value("stringify".to_string()));
    assert!(matches!(&args[0].kind, CoreExprKind::IntLiteral(value) if value == "1"));
}

#[test]
fn non_callable_local_shadow_blocks_function_call_resolution() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn stringify(value: Int) -> String\n",
            "  \"function\"\n",
            "end\n",
            "pub fn main(stringify: Int) -> String\n",
            "  stringify(1)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `stringify`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn lowers_record_field_access_through_core_and_ir() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> String\n",
            "  let payload: {name: String, count: Int} = {name: \"veln\", count: 1}\n",
            "  payload.name\n",
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
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::FieldAccess { field, .. } if field == "name"
    ));
    assert_eq!(expr.ty, CoreType::string());

    let ir = lowered.ir.expect("complete core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[1].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::FieldAccess { field, .. } if field == "name"
    ));
}

#[test]
fn reports_missing_record_field_access() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> Int\n",
            "  let payload: {count: Int} = {count: 1}\n",
            "  payload.name\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.field_missing");
    assert_eq!(
        diagnostics[0].message,
        "type `{count: Int}` has no field `name`"
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}

#[test]
fn prelude_helpers_check_direct_expected_return_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(value: Option<Int>) -> Int\n",
            "  option_unwrap_or(value, \"bad\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "type.mismatch");
    assert_eq!(diagnostics[0].message, "expected `Int`, but found `String`");
}

#[test]
fn source_backed_prelude_helpers_report_user_call_site_diagnostics() {
    for (helper, value_type, return_type, expected_callback) in [
        (
            "vec_map",
            "Vec<Int>",
            "Vec<String>",
            "fn(unknown) -> String",
        ),
        ("vec_filter", "Vec<Int>", "Vec<Int>", "fn(Int) -> Bool"),
        (
            "option_map",
            "Option<Int>",
            "Option<String>",
            "fn(unknown) -> String",
        ),
        (
            "option_and_then",
            "Option<Int>",
            "Option<String>",
            "fn(unknown) -> Option<String>",
        ),
        (
            "result_map",
            "Result<Int, String>",
            "Result<String, String>",
            "fn(unknown) -> String",
        ),
        (
            "result_map_err",
            "Result<String, Int>",
            "Result<String, String>",
            "fn(unknown) -> String",
        ),
        (
            "result_and_then",
            "Result<Int, String>",
            "Result<String, String>",
            "fn(unknown) -> Result<String, String>",
        ),
        (
            "vec_try_map",
            "Vec<Int>",
            "Result<Vec<String>, String>",
            "fn(unknown) -> Result<String, String>",
        ),
        (
            "list_map",
            "List<Int>",
            "List<String>",
            "fn(unknown) -> String",
        ),
        ("list_filter", "List<Int>", "List<Int>", "fn(Int) -> Bool"),
        (
            "list_try_map",
            "List<Int>",
            "Result<List<String>, String>",
            "fn(unknown) -> Result<String, String>",
        ),
    ] {
        let source = SourceFile::new(
            "main.veln",
            format!(
                concat!(
                    "type List<A>\n",
                    "  Nil\n",
                    "  Cons(head: A, tail: List<A>)\n",
                    "end\n",
                    "fn to_int(value: Int) -> Int\n",
                    "  value\n",
                    "end\n",
                    "pub fn main(value: {}) -> {}\n",
                    "  {}(value, to_int)\n",
                    "end\n",
                ),
                value_type, return_type, helper
            ),
        );
        let parsed = parse(&source);
        let module = lower_surface_ast(&parsed.tree);

        let diagnostics = analyze_surface_module(&module);

        assert_eq!(diagnostics.len(), 1, "{helper}");
        assert_eq!(diagnostics[0].id, "type.mismatch");
        assert_eq!(
            diagnostics[0].message,
            format!("expected `{expected_callback}`, but found `fn(Int) -> Int`")
        );
        let span = diagnostics[0]
            .span
            .as_ref()
            .expect("diagnostic should point at user source");
        assert_eq!(span.file.as_str(), "main.veln");
    }
}
