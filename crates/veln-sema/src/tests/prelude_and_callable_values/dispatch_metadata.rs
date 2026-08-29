use super::*;

#[test]
fn generated_schema_decode_helpers_reject_added_repeat_byte_view_length_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  count: UInt8\n",
            "  left_length: UInt8\n",
            "  items: Repeat(count, ByteView(left_length + right_length))\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  count: UInt8\n",
            "  left_length: UInt8\n",
            "  items: Repeat(count, ByteView(left_length + right_length))\n",
            "  right_length: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  count: UInt8\n",
            "  left_length: UInt8\n",
            "  flags: ByteView(left_length)\n",
            "  items: Repeat(count, ByteView(left_length + flags))\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "repeat ByteView length operand `right_length` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "repeat ByteView length operand `right_length` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "repeat ByteView length operand `flags` decodes as `ByteView`, not `Int`",
        ),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.byte_view_reference"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{:#?}",
            lowered.diagnostics
        );
    }
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing Repeat ByteView length expression should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_reject_product_repeat_count_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  row_count: UInt8\n",
            "  items: Repeat(row_count * column_count, UInt16be)\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  row_count: UInt8\n",
            "  items: Repeat(row_count * column_count, UInt16be)\n",
            "  column_count: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  row_count: UInt8\n",
            "  flags: ByteView(row_count)\n",
            "  items: Repeat(row_count * flags, UInt16be)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "repeat count operand `column_count` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "repeat count operand `column_count` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "repeat count operand `flags` decodes as `ByteView`, not `Int`",
        ),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.repeat_reference"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{:#?}",
            lowered.diagnostics
        );
    }
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing Repeat count expression should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_reject_quotient_repeat_count_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  items: Repeat(length / chunk_count, UInt16be)\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  items: Repeat(length / chunk_count, UInt16be)\n",
            "  chunk_count: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  flags: ByteView(length)\n",
            "  items: Repeat(length / flags, UInt16be)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "repeat count operand `chunk_count` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "repeat count operand `chunk_count` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "repeat count operand `flags` decodes as `ByteView`, not `Int`",
        ),
    ] {
        assert!(
            lowered.diagnostics.iter().any(|diagnostic| {
                diagnostic.id == "schema.repeat_reference"
                    && diagnostic.message == message
                    && diagnostic
                        .details
                        .to_json()
                        .contains(&format!("\"reason\":\"{reason}\""))
            }),
            "{:#?}",
            lowered.diagnostics
        );
    }
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing Repeat count expression should not emit typed IR"
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
            "  payload: Dispatch(kind, 1 => UInt24le, 2 => UInt32le)\n",
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
            .map(|case| (case.tag, case.width, case.little_endian))
            .collect::<Vec<_>>(),
        vec![(1, 3, true), (2, 4, true)]
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
            "  payload: ExtensionDispatch(kind, length, 1 => UInt24le, 2 => UInt32le)\n",
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
            .map(|case| (case.tag, case.width, case.little_endian))
            .collect::<Vec<_>>(),
        vec![(1, 3, true), (2, 4, true)]
    );
}

#[test]
fn generated_schema_decode_helpers_keep_nested_dispatch_schema_metadata() {
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
            "schema ExtensionNestedPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 1 => SettingsPayload)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{length: Int, kind: Int, payload: SchemaDispatchPayload<{code: Int, value: Int}>}, String>\n",
            "  byte_decode_extension_nested_packet(view)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "ExtensionNestedPacket")
        .expect("nested packet decoder should be emitted");
    let dispatch = schema.fields[2]
        .dispatch
        .as_ref()
        .expect("payload should carry dispatch metadata");
    assert_eq!(dispatch.length_field.as_deref(), Some("length"));
    assert_eq!(dispatch.cases[0].tag, 1);
    assert_eq!(dispatch.cases[0].width, 0);
    let nested = dispatch.cases[0]
        .payload_schema
        .as_ref()
        .expect("dispatch case should carry nested schema metadata");
    assert_eq!(nested.schema_name, "SettingsPayload");
    assert_eq!(
        nested
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.width))
            .collect::<Vec<_>>(),
        vec![("code", 1), ("value", 2)]
    );
}

#[test]
fn generated_schema_decode_helpers_keep_imported_dispatch_schema_metadata() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app.main\n",
            "use app.wire\n",
            "\n",
            "schema ClosedImportedPacket\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::SettingsPayload)\n",
            "end\n",
            "\n",
            "schema ExtensionImportedPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 1 => wire::SettingsPayload)\n",
            "end\n",
            "\n",
            "pub fn closed(view: ByteView) -> Result<{kind: Int, payload: {code: Int, value: Int}}, String>\n",
            "  byte_decode_closed_imported_packet(view)\n",
            "end\n",
            "\n",
            "pub fn extension(view: ByteView) -> Result<{length: Int, kind: Int, payload: SchemaDispatchPayload<{code: Int, value: Int}>}, String>\n",
            "  byte_decode_extension_imported_packet(view)\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod app.wire\n",
            "\n",
            "pub schema SettingsPayload\n",
            "  format binary\n",
            "\n",
            "  code: UInt8\n",
            "  value: UInt16be\n",
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
        types: [app.types, wire.types].concat(),
        schemas: [app.schemas, wire.schemas].concat(),
        codecs: Vec::new(),
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    for (schema_name, field_index) in [("ClosedImportedPacket", 1), ("ExtensionImportedPacket", 2)]
    {
        let schema = ir
            .schema_decoders
            .iter()
            .find(|schema| schema.schema_name == schema_name)
            .expect("imported dispatch packet decoder should be emitted");
        let dispatch = schema.fields[field_index]
            .dispatch
            .as_ref()
            .expect("payload should carry dispatch metadata");
        let nested = dispatch.cases[0]
            .payload_schema
            .as_ref()
            .expect("dispatch case should carry imported nested schema metadata");
        assert_eq!(nested.schema_name, "SettingsPayload");
        assert_eq!(
            nested
                .fields
                .iter()
                .map(|field| (field.name.as_str(), field.width))
                .collect::<Vec<_>>(),
            vec![("code", 1), ("value", 2)]
        );
    }
}

#[test]
fn generated_schema_encode_helpers_keep_imported_dispatch_schema_metadata() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app.main\n",
            "use app.wire\n",
            "\n",
            "schema ClosedImportedPacket\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => wire::SettingsPayload)\n",
            "end\n",
            "\n",
            "schema ExtensionImportedPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 1 => wire::SettingsPayload)\n",
            "end\n",
            "\n",
            "pub fn closed(packet: {kind: Int, payload: {code: Int, value: Int}}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_closed_imported_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn extension(packet: {length: Int, kind: Int, payload: SchemaDispatchPayload<{code: Int, value: Int}>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_extension_imported_packet(packet)\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod app.wire\n",
            "\n",
            "pub schema SettingsPayload\n",
            "  format binary\n",
            "\n",
            "  code: UInt8\n",
            "  value: UInt16be\n",
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
        types: [app.types, wire.types].concat(),
        schemas: [app.schemas, wire.schemas].concat(),
        codecs: Vec::new(),
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    for (function_name, target_name) in [
        ("closed", "ClosedImportedPacket"),
        ("extension", "ExtensionImportedPacket"),
    ] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("encode wrapper should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        assert!(matches!(
            &expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::SchemaEncode(name),
                ..
            } if name == target_name
        ));
    }

    let ir = lowered.ir.expect("typed IR should be built");
    for (schema_name, field_index) in [("ClosedImportedPacket", 1), ("ExtensionImportedPacket", 2)]
    {
        let schema = ir
            .schema_decoders
            .iter()
            .find(|schema| schema.schema_name == schema_name)
            .expect("imported dispatch packet encoder metadata should be emitted");
        let dispatch = schema.fields[field_index]
            .dispatch
            .as_ref()
            .expect("payload should carry dispatch metadata");
        let nested = dispatch.cases[0]
            .payload_schema
            .as_ref()
            .expect("dispatch case should carry imported nested schema metadata");
        assert_eq!(nested.schema_name, "SettingsPayload");
        assert_eq!(
            nested
                .fields
                .iter()
                .map(|field| (field.name.as_str(), field.width))
                .collect::<Vec<_>>(),
            vec![("code", 1), ("value", 2)]
        );
    }
}
