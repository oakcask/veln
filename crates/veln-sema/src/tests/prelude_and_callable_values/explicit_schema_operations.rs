use super::*;

#[test]
fn generated_format_neutral_schema_decode_helpers_reject_functions_inside_record_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type LocalPayload\n",
            "  LocalPayload(value: Int)\n",
            "end\n",
            "\n",
            "schema BadPacket\n",
            "  metadata: {payload: LocalPayload, callback: fn(Int) -> String}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "schema.format_neutral_decode_helper")
        .expect("unsupported function field should be reported");
    assert_eq!(
        diagnostic.message,
        format!(
            "format-neutral schema field `metadata` cannot expose a generated decode helper because `{{ payload : LocalPayload, callback : fn(Int) -> String }}` is not a {FORMAT_NEUTRAL_HELPER_SUPPORTED}"
        )
    );
}

#[test]
fn explicit_schema_decode_expression_resolves_as_schema_decode_step_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
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
            target: CoreCallTarget::SchemaDecodeStep(name),
            args,
        } if name == "PacketWire" && args.len() == 2
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
            args,
        } if name == "PacketWire" && args.len() == 2
    ));
}

#[test]
fn explicit_schema_encode_expression_resolves_as_schema_encode_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
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
            args,
        } if name == "PacketWire" && args.len() == 1
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
            args,
        } if name == "PacketWire" && args.len() == 1
    ));
}

#[test]
fn explicit_schema_decode_expression_resolves_qualified_public_schema_path() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{wire_length: Int}>\n",
            "  decode wire::PacketWire from view at base\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "pub schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: wire.types,
        schemas: wire.schemas,
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
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
fn explicit_schema_operations_resolve_public_schema_alias_to_target_schema() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use facade\n",
            "\n",
            "pub fn decode_alias(view: ByteView, base: ByteOffset) -> DecodeStep<{wire_length: Int}>\n",
            "  decode facade::AliasPacket from view at base\n",
            "end\n",
            "\n",
            "pub fn encode_alias(packet: {wire_length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode facade::AliasPacket from packet\n",
            "end\n",
        ),
    );
    let facade_source = SourceFile::new(
        "facade.veln",
        concat!(
            "mod facade\n",
            "use wire\n",
            "\n",
            "pub schema AliasPacket = wire::PublicPacket\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "pub schema PublicPacket\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let facade = lower_surface_ast(&parse(&facade_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: [app.uses, facade.uses].concat(),
        aliases: facade.aliases,
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: wire.schemas,
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let decode_alias = core
        .functions
        .iter()
        .find(|function| function.name == "decode_alias")
        .expect("decode_alias should be lowered");
    let CoreStmtKind::Return { expr } = &decode_alias.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "PublicPacket"
    ));
    let encode_alias = core
        .functions
        .iter()
        .find(|function| function.name == "encode_alias")
        .expect("encode_alias should be lowered");
    let CoreStmtKind::Return { expr } = &encode_alias.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "PublicPacket"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let decode_alias = ir
        .functions
        .iter()
        .find(|function| function.name == "decode_alias")
        .expect("decode_alias should be in IR");
    let IrStmtKind::Return { value } = &decode_alias.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "PublicPacket"
    ));
    let encode_alias = ir
        .functions
        .iter()
        .find(|function| function.name == "encode_alias")
        .expect("encode_alias should be in IR");
    let IrStmtKind::Return { value } = &encode_alias.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaEncode(name),
            ..
        } if name == "PublicPacket"
    ));
}

#[test]
fn explicit_schema_decode_expression_reports_unresolved_private_and_wrong_kind_schema_paths() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "fn missing(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode MissingPacket from view at base\n",
            "end\n",
            "\n",
            "fn private_schema(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode wire::PrivatePacket from view at base\n",
            "end\n",
            "\n",
            "fn wrong_type(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode wire::PacketShape from view at base\n",
            "end\n",
            "\n",
            "fn wrong_function(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode wire::make_packet from view at base\n",
            "end\n",
            "\n",
            "fn wrong_codec(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode wire::PacketCodec from view at base\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "pub schema PublicPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "schema PrivatePacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "pub fn make_packet() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "pub type PacketShape\n",
            "  Box\n",
            "end\n",
            "\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: [app.types, wire.types].concat(),
        schemas: wire.schemas,
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    let messages = lowered
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "schema.decode_expression")
        .map(|diagnostic| diagnostic.message.as_str())
        .collect::<Vec<_>>();
    assert!(
        messages.contains(
            &"schema decode expression cannot resolve `MissingPacket` as an eligible binary schema",
        ),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        messages.contains(&"schema decode expression schema `wire::PrivatePacket` is private"),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        messages.contains(
            &"schema decode expression target `wire::PacketShape` is a type, not a schema"
        ),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        messages.contains(
            &"schema decode expression target `wire::make_packet` is a function, not a schema",
        ),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        messages.contains(
            &"schema decode expression cannot resolve `wire::PacketCodec` as an eligible binary schema",
        ),
        "{:#?}",
        lowered.diagnostics
    );
    for (schema_path, reason) in [
        ("MissingPacket", "unresolved_schema"),
        ("wire::PrivatePacket", "private_schema"),
        ("wire::PacketShape", "wrong_kind"),
        ("wire::make_packet", "wrong_kind"),
        ("wire::PacketCodec", "unresolved_schema"),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.decode_expression"
                    && matches!(
                        &diagnostic.details,
                        veln_diagnostics::JsonValue::Object(entries)
                            if entries.iter().any(|(key, value)| {
                                key == "schema_path"
                                    && value
                                        == &veln_diagnostics::JsonValue::string(schema_path)
                            }) && entries.iter().any(|(key, value)| {
                                key == "reason"
                                    && value == &veln_diagnostics::JsonValue::string(reason)
                            })
                    )
            }),
            "{:#?}",
            lowered.diagnostics
        );
    }
}

#[test]
fn matching_companion_resolves_private_target_schema_operations_and_composition() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "schema Wrapped\n",
            "  format binary\n",
            "\n",
            "  packet: math::Packet\n",
            "end\n",
            "fn companion_uses_private_schema(view: ByteView, offset: ByteOffset, value: {length: Int}) -> ()\n",
            "  let decoded: DecodeStep<{length: Int}> = decode math::Packet from view at offset\n",
            "  let encoded: Result<ByteChunk, EncodeError> = encode math::Packet from value\n",
            "  ()\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "schema Packet\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
        ),
    );
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let module = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: companion
            .schemas
            .into_iter()
            .chain(target.schemas)
            .collect(),
        functions: companion.functions,
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
}

#[test]
fn companion_private_target_schema_access_preserves_boundaries() {
    for (companion_path, companion_text, expected_message) in [
        (
            "math.test.veln",
            concat!(
                "mod math__test_companion\n",
                "fn missing_import(view: ByteView, offset: ByteOffset) -> ()\n",
                "  let decoded: DecodeStep<{length: Int}> = decode math::Packet from view at offset\n",
                "  ()\n",
                "end\n",
            ),
            "schema decode expression cannot resolve `math::Packet` as an eligible binary schema",
        ),
        (
            "math.test.veln",
            concat!(
                "mod math__test_companion\n",
                "use math\n",
                "fn bare_name(view: ByteView, offset: ByteOffset) -> ()\n",
                "  let decoded: DecodeStep<{length: Int}> = decode Packet from view at offset\n",
                "  ()\n",
                "end\n",
            ),
            "schema decode expression cannot resolve `Packet` as an eligible binary schema",
        ),
        (
            "other.test.veln",
            concat!(
                "mod other__test_companion\n",
                "use math\n",
                "fn wrong_target(view: ByteView, offset: ByteOffset) -> ()\n",
                "  let decoded: DecodeStep<{length: Int}> = decode math::Packet from view at offset\n",
                "  ()\n",
                "end\n",
            ),
            "schema decode expression schema `math::Packet` is private",
        ),
        (
            "math_test.veln",
            concat!(
                "mod math_test\n",
                "use math\n",
                "fn integration(view: ByteView, offset: ByteOffset) -> ()\n",
                "  let decoded: DecodeStep<{length: Int}> = decode math::Packet from view at offset\n",
                "  ()\n",
                "end\n",
            ),
            "schema decode expression schema `math::Packet` is private",
        ),
        (
            "math.test.veln",
            concat!(
                "mod math__test_companion\n",
                "use support\n",
                "fn non_transitive(view: ByteView, offset: ByteOffset) -> ()\n",
                "  let decoded: DecodeStep<{length: Int}> = decode support::Packet from view at offset\n",
                "  ()\n",
                "end\n",
            ),
            "schema decode expression schema `support::Packet` is private",
        ),
    ] {
        let companion_source = SourceFile::new(companion_path, companion_text);
        let target_source = SourceFile::new(
            "math.veln",
            concat!(
                "mod math\n",
                "use support\n",
                "schema Packet\n",
                "  format binary\n",
                "\n",
                "  length: UInt8\n",
                "end\n",
            ),
        );
        let support_source = SourceFile::new(
            "support.veln",
            concat!(
                "mod support\n",
                "schema Packet\n",
                "  format binary\n",
                "\n",
                "  length: UInt8\n",
                "end\n",
            ),
        );
        let companion = lower_surface_ast(&parse(&companion_source).tree);
        let target = lower_surface_ast(&parse(&target_source).tree);
        let support = lower_surface_ast(&parse(&support_source).tree);
        let module = SurfaceModule {
            module: companion.module,
            uses: companion.uses.into_iter().chain(target.uses).collect(),
            aliases: Vec::new(),
            effects: Vec::new(),
            handlers: Vec::new(),
            types: Vec::new(),
            schemas: target.schemas.into_iter().chain(support.schemas).collect(),
            functions: companion.functions,
            invalid_names: Vec::new(),
        };

        let lowered = lower_checked_surface_module(&module);

        let diagnostic = lowered
            .diagnostics
            .iter()
            .find(|diagnostic| {
                diagnostic.id == "schema.decode_expression"
                    && diagnostic.message == expected_message
            })
            .unwrap_or_else(|| panic!("{:#?}", lowered.diagnostics));
        if matches!(companion_path, "other.test.veln" | "math.test.veln")
            && expected_message.contains(" is private")
        {
            let details = diagnostic.details.to_json();
            let expected_target = if companion_path == "other.test.veln" {
                "other"
            } else {
                "math"
            };
            assert!(
                details.contains(&format!(
                    "\"companion_target_module\":\"{expected_target}\""
                )),
                "{details}"
            );
            assert!(diagnostic.related.iter().any(|related| {
                let related = related.to_json();
                related.contains("\"kind\":\"companion_target\"")
                    && related.contains(&format!("`{expected_target}`"))
            }));
        }
    }
}
