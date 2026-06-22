use super::*;
use crate::types::repeat_schema_primitive;

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
fn generated_schema_decode_helpers_keep_schema_level_validation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema ValidatedPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8\n",
            "  checksum: UInt8\n",
            "\n",
            "  validate length == padding_length + checksum\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    assert_eq!(
        ir.schema_decoders[0].validation.as_deref(),
        Some("length == padding_length + checksum")
    );
}

#[test]
fn generated_schema_value_validation_helpers_resolve_from_binary_schema_declarations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema OrdinaryPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8 where padding_length <= length\n",
            "  payload: ByteView(length - padding_length)\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int, padding_length: Int, payload: ByteView}) -> Result<{length: Int, padding_length: Int, payload: ByteView}, String>\n",
            "  validate_ordinary_packet(packet)\n",
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
            target: CoreCallTarget::SchemaValidate(name),
            ..
        } if name == "OrdinaryPacket"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "OrdinaryPacket");
    assert_eq!(
        schema
            .fields
            .iter()
            .map(|field| (field.name.as_str(), field.predicate.as_deref()))
            .collect::<Vec<_>>(),
        vec![
            ("length", None),
            ("padding_length", Some("padding_length <= length")),
            ("payload", None),
        ]
    );
}

#[test]
fn generated_schema_encode_helpers_resolve_with_field_local_predicates() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {length: Int, padding_length: Int}\n",
            "end\n",
            "\n",
            "schema PaddedHeader\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8 where padding_length <= length\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "  wire_padding_length: UInt8 where wire_padding_length <= wire_length\n",
            "\n",
            "  map to Header\n",
            "    length = wire_length\n",
            "    padding_length = wire_padding_length\n",
            "end\n",
            "\n",
            "pub fn direct(packet: {length: Int, padding_length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_padded_header(packet)\n",
            "end\n",
            "\n",
            "pub fn mapped(packet: {length: Int, padding_length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_header_wire(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    for (function_name, schema_name) in [("direct", "PaddedHeader"), ("mapped", "HeaderWire")] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("function should be lowered");
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
}

#[test]
fn schema_level_validation_rejects_unsupported_references() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema InvalidValidationPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length)\n",
            "\n",
            "  validate length == missing\n",
            "  validate payload == length\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.validation_reference"
            && diagnostic.message
                == "schema validation reference `missing` is not a decoded schema field"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.validation_duplicate"
            && diagnostic.message
                == "schema `InvalidValidationPacket` can declare only one schema-level validation"
    }));
}

#[test]
fn schema_level_validation_rejects_non_int_decoded_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema InvalidValidationPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length)\n",
            "\n",
            "  validate payload == length\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.validation_reference"
            && diagnostic.message
                == "schema validation reference `payload` decodes as `ByteView`, not `Int`"
    }));
}

#[test]
fn generated_schema_helpers_resolve_bounded_repeated_primitive_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema CountedValues\n",
            "  format binary\n",
            "\n",
            "  count: UInt8\n",
            "  items: Repeat(count, UInt16be)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{count: Int, items: List<Int>}, String>\n",
            "  byte_decode_counted_values(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {count: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_counted_values(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let read = core
        .functions
        .iter()
        .find(|function| function.name == "read")
        .expect("read should be lowered");
    let CoreStmtKind::Return { expr } = &read.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecode(name),
            ..
        } if name == "CountedValues"
    ));
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
        } if name == "CountedValues"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    assert_eq!(ir.schema_decoders.len(), 1);
    let schema = &ir.schema_decoders[0];
    assert_eq!(schema.schema_name, "CountedValues");
    assert_eq!(schema.fields[0].name, "count");
    assert_eq!(
        schema.fields[1].repeat.as_ref().map(|repeat| {
            (
                repeat.count_field.as_str(),
                repeat.width,
                repeat.max_value,
                repeat.little_endian,
            )
        }),
        Some(("count", 2, 0xffff, false))
    );
}

#[test]
fn generated_schema_helpers_resolve_bounded_repeated_nested_schema_fields() {
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
            "  count: UInt8\n",
            "  items: Repeat(count, ItemRecord)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{count: Int, items: List<{code: Int, value: Int}>}, String>\n",
            "  byte_decode_counted_items(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {count: Int, items: List<{code: Int, value: Int}>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_counted_items(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    let read = core
        .functions
        .iter()
        .find(|function| function.name == "read")
        .expect("read should be lowered");
    let CoreStmtKind::Return { expr } = &read.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecode(name),
            ..
        } if name == "CountedItems"
    ));
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
        } if name == "CountedItems"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "CountedItems")
        .expect("counted schema should be emitted");
    let repeat = schema.fields[1]
        .repeat
        .as_ref()
        .expect("items should carry repeat metadata");
    assert_eq!(repeat.count_field, "count");
    assert_eq!(repeat.width, 0);
    let nested = repeat
        .payload_schema
        .as_ref()
        .expect("nested repeat should carry schema metadata");
    assert_eq!(nested.schema_name, "ItemRecord");
    assert_eq!(nested.fields.len(), 2);
}

#[test]
fn generated_schema_helpers_resolve_bounded_repeated_imported_nested_schema_fields() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app.main\n",
            "use app.wire\n",
            "\n",
            "schema CountedItems\n",
            "  format binary\n",
            "\n",
            "  count: UInt8\n",
            "  items: Repeat(count, wire::ItemRecord)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{count: Int, items: List<{code: Int, value: Int}>}, String>\n",
            "  byte_decode_counted_items(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {count: Int, items: List<{code: Int, value: Int}>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_counted_items(packet)\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod app.wire\n",
            "\n",
            "pub schema ItemRecord\n",
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
        types: [app.types, wire.types].concat(),
        schemas: [app.schemas, wire.schemas].concat(),
        codecs: Vec::new(),
        functions: [app.functions, wire.functions].concat(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.as_ref().expect("checked core should be built");
    for (function_name, target_name, expected_target) in [
        ("read", "CountedItems", "decode"),
        ("write", "CountedItems", "encode"),
    ] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("helper wrapper should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("tail expression should lower as return");
        };
        match (&expr.kind, expected_target) {
            (
                CoreExprKind::Call {
                    target: CoreCallTarget::SchemaDecode(name),
                    ..
                },
                "decode",
            ) => assert_eq!(name, target_name),
            (
                CoreExprKind::Call {
                    target: CoreCallTarget::SchemaEncode(name),
                    ..
                },
                "encode",
            ) => assert_eq!(name, target_name),
            _ => panic!("helper wrapper should lower to a schema helper call"),
        }
    }

    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "CountedItems")
        .expect("counted schema should be emitted");
    let repeat = schema.fields[1]
        .repeat
        .as_ref()
        .expect("items should carry repeat metadata");
    assert_eq!(repeat.count_field, "count");
    assert_eq!(repeat.width, 0);
    let nested = repeat
        .payload_schema
        .as_ref()
        .expect("imported nested repeat should carry schema metadata");
    assert_eq!(nested.schema_name, "ItemRecord");
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
fn generated_schema_helpers_resolve_bounded_repeated_byte_view_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema CountedViews\n",
            "  format binary\n",
            "\n",
            "  count: UInt8\n",
            "  item_length: UInt8\n",
            "  items: Repeat(count, ByteView(item_length))\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{count: Int, item_length: Int, items: List<ByteView>}, String>\n",
            "  byte_decode_counted_views(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {count: Int, item_length: Int, items: List<ByteView>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_counted_views(packet)\n",
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
        } if name == "CountedViews"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "CountedViews")
        .expect("counted schema should be emitted");
    let repeat = schema.fields[2]
        .repeat
        .as_ref()
        .expect("items should carry repeat metadata");
    assert_eq!(repeat.count_field, "count");
    assert_eq!(
        repeat.byte_view_length_field.as_deref(),
        Some("item_length")
    );
    assert_eq!(repeat.width, 0);
    assert!(repeat.payload_schema.is_none());
}

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
fn generated_schema_encode_helpers_accept_mapped_value_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {length: Int, kind: Int}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt16be\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    length = wire_length\n",
            "    kind = wire_kind\n",
            "end\n",
            "\n",
            "pub fn main(header: {length: Int, kind: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_header_wire(header)\n",
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
        } if name == "HeaderWire"
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
        } if name == "HeaderWire"
    ));
}

#[test]
fn generated_schema_encode_helpers_accept_selected_mapped_value_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Packet\n",
            "  Packet {kind: Int, value: Int}\n",
            "end\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  value: UInt8\n",
            "\n",
            "  map to Packet when kind == 1\n",
            "    kind = kind\n",
            "    value = value\n",
            "\n",
            "  map to Packet when kind == 2\n",
            "    kind = kind\n",
            "    value = value\n",
            "end\n",
            "\n",
            "pub fn main(packet: {kind: Int, value: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
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
fn generated_schema_encode_helpers_accept_mixed_dispatch_selected_mapping_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Settings\n",
            "  Settings {code: Int, value: Int}\n",
            "end\n",
            "\n",
            "type PacketPayload\n",
            "  InlineValue(Int)\n",
            "  SettingsValue({code: Int, value: Int})\n",
            "end\n",
            "\n",
            "type Packet\n",
            "  Packet {kind: Int, body: PacketPayload}\n",
            "end\n",
            "\n",
            "schema SettingsPayload\n",
            "  format binary\n",
            "\n",
            "  code: UInt8\n",
            "  value: UInt16be\n",
            "\n",
            "  map to Settings\n",
            "    code = code\n",
            "    value = value\n",
            "end\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => UInt8, 2 => SettingsPayload)\n",
            "\n",
            "  map to Packet when kind == 1\n",
            "    kind = kind\n",
            "    body = InlineValue(payload)\n",
            "\n",
            "  map to Packet when kind == 2\n",
            "    kind = kind\n",
            "    body = SettingsValue(payload)\n",
            "end\n",
            "\n",
            "pub fn main(packet: {kind: Int, body: PacketPayload}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
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
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet encoder metadata should be emitted");
    let dispatch = schema.fields[1]
        .dispatch
        .as_ref()
        .expect("payload should carry dispatch metadata");
    assert_eq!(
        dispatch
            .cases
            .iter()
            .map(|case| (case.tag, case.width, case.payload_schema.is_some()))
            .collect::<Vec<_>>(),
        vec![(1, 1, false), (2, 0, true)]
    );
}

#[test]
fn generated_schema_encode_helpers_accept_mapped_record_expression_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {summary: {value: Int}, kind: Int}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt16be\n",
            "  kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    summary = {value: length}\n",
            "    kind = kind\n",
            "end\n",
            "\n",
            "pub fn main(header: {summary: {value: Int}, kind: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_header_wire(header)\n",
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
        } if name == "HeaderWire"
    ));
}

#[test]
fn generated_schema_encode_helpers_accept_mapped_field_selection_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {code: Int, value: Int}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_code: UInt8\n",
            "  wire_value: UInt16be\n",
            "\n",
            "  map to Header\n",
            "    code = {code: wire_code}.code\n",
            "    value = wire_value\n",
            "end\n",
            "\n",
            "pub fn main(header: {code: Int, value: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_header_wire(header)\n",
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
        } if name == "HeaderWire"
    ));
}

#[test]
fn generated_schema_encode_helpers_accept_flag8_mapped_constructor_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Http2Flags\n",
            "  Http2Flags(Flag8)\n",
            "end\n",
            "\n",
            "type FlagPacket\n",
            "  FlagPacket {flags: Http2Flags}\n",
            "end\n",
            "\n",
            "schema FlagPacketWire\n",
            "  format binary\n",
            "\n",
            "  wire_flags: Flag8\n",
            "\n",
            "  map to FlagPacket\n",
            "    flags = Http2Flags(wire_flags)\n",
            "end\n",
            "\n",
            "pub fn main(packet: {flags: Http2Flags}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_flag_packet_wire(packet)\n",
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
        } if name == "FlagPacketWire"
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
        } if name == "FlagPacketWire"
    ));
}

#[test]
fn generated_schema_encode_helpers_accept_integer_mapped_constructor_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type FrameKind\n",
            "  FrameKind(Int)\n",
            "end\n",
            "\n",
            "type FrameHeader\n",
            "  FrameHeader {kind: FrameKind}\n",
            "end\n",
            "\n",
            "schema FrameHeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to FrameHeader\n",
            "    kind = FrameKind(wire_kind)\n",
            "end\n",
            "\n",
            "pub fn main(header: {kind: FrameKind}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_frame_header_wire(header)\n",
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
        } if name == "FrameHeaderWire"
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
        } if name == "FrameHeaderWire"
    ));
}

#[test]
fn generated_schema_encode_helpers_accept_multi_payload_mapped_constructor_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type FieldRange\n",
            "  Empty\n",
            "  Between(Int, Int)\n",
            "end\n",
            "\n",
            "type RangePacket\n",
            "  RangePacket {range: FieldRange}\n",
            "end\n",
            "\n",
            "schema RangePacketWire\n",
            "  format binary\n",
            "\n",
            "  start: UInt16be\n",
            "  finish: UInt16be\n",
            "\n",
            "  map to RangePacket\n",
            "    range = Between(start, finish)\n",
            "end\n",
            "\n",
            "pub fn main(packet: {range: FieldRange}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_range_packet_wire(packet)\n",
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
        } if name == "RangePacketWire"
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
        } if name == "RangePacketWire"
    ));
}

#[test]
fn generated_schema_encode_helpers_accept_mapped_constructor_field_selection_args() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type FieldRange\n",
            "  Empty\n",
            "  Between(Int, Int)\n",
            "end\n",
            "\n",
            "type RangePacket\n",
            "  RangePacket {range: FieldRange}\n",
            "end\n",
            "\n",
            "schema RangePacketWire\n",
            "  format binary\n",
            "\n",
            "  start: UInt16be\n",
            "  finish: UInt16be\n",
            "\n",
            "  map to RangePacket\n",
            "    range = Between({value: start}.value, finish)\n",
            "end\n",
            "\n",
            "pub fn main(packet: {range: FieldRange}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_range_packet_wire(packet)\n",
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
        } if name == "RangePacketWire"
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
        } if name == "RangePacketWire"
    ));
}

#[test]
fn generated_schema_encode_helpers_accept_record_payload_mapped_constructor_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type PacketPayload\n",
            "  Empty\n",
            "  Known({code: Int, value: Int})\n",
            "end\n",
            "\n",
            "type Packet\n",
            "  Packet {payload: PacketPayload}\n",
            "end\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  code: UInt8\n",
            "  value: UInt16be\n",
            "\n",
            "  map to Packet\n",
            "    payload = PacketPayload::Known({code: code, value: value})\n",
            "end\n",
            "\n",
            "pub fn main(packet: {payload: PacketPayload}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
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
fn generated_schema_encode_helpers_accept_nested_mapped_constructor_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Payload\n",
            "  Payload(Int)\n",
            "end\n",
            "\n",
            "type Envelope\n",
            "  Envelope(Payload)\n",
            "  OtherEnvelope(Payload)\n",
            "end\n",
            "\n",
            "type Header\n",
            "  Header {wrapped: Envelope}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    wrapped = Envelope(Payload(kind))\n",
            "end\n",
            "\n",
            "pub fn main(header: {wrapped: Envelope}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_header_wire(header)\n",
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
        } if name == "HeaderWire"
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
        } if name == "HeaderWire"
    ));
}

#[test]
fn generated_schema_encode_helpers_reject_multi_variant_mapped_constructor_records() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Http2Flags\n",
            "  Http2Flags(Flag8)\n",
            "  OtherFlags(Flag8)\n",
            "end\n",
            "\n",
            "type FlagPacket\n",
            "  FlagPacket {flags: Http2Flags}\n",
            "end\n",
            "\n",
            "schema FlagPacketWire\n",
            "  format binary\n",
            "\n",
            "  wire_flags: Flag8\n",
            "\n",
            "  map to FlagPacket\n",
            "    flags = Http2Flags(wire_flags)\n",
            "end\n",
            "\n",
            "pub fn main(packet: {flags: Http2Flags}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_flag_packet_wire(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved"),
        "{:#?}",
        lowered.diagnostics
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
fn generated_schema_helpers_accept_one_byte_packed_reserved_suffix_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PackedSuffixHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: UInt8\n",
            "  control: UInt3\n",
            "  control_padding: ReservedBits(5, 0)\n",
            "  suffix: UInt8\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{prefix: Int, control: Int, suffix: Int}, String>\n",
            "  byte_decode_packed_suffix_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {prefix: Int, control: Int, suffix: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packed_suffix_header(packet)\n",
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
            ("control", 1, 0x7, None),
            ("control_padding", 0, 0, Some((5, 0))),
            ("suffix", 1, 0xff, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_middle_reserved_bits() {
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
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_middle_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_middle_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "middle reserved bits should be accepted: {:#?}",
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
            ("high", 1, 7, None),
            ("gap", 0, 0, Some((2, 1))),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_byte_interleaved_middle_reserved_bits() {
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
            "pub fn read_header(view: ByteView) -> Result<{high: Int, middle: Int, low: Int}, String>\n",
            "  byte_decode_byte_interleaved_middle_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, middle: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_byte_interleaved_middle_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "byte-interleaved middle reserved bits should be accepted: {:#?}",
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
            ("high", 1, 15, None),
            ("guard", 0, 0, Some((1, 0))),
            ("middle", 1, 255, None),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_prefix_reserved_visible_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(2, 2)\n",
            "  high: UInt3\n",
            "  low: UInt3\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "prefix reserved visible group bits should be accepted: {:#?}",
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
            ("prefix", 0, 0, Some((2, 2))),
            ("high", 1, 7, None),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_two_byte_prefix_reserved_byte_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema BytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  marker: ReservedBits(8, 171)\n",
            "  high: UInt3\n",
            "  low: UInt5\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "two-byte prefix reserved byte group bits should be accepted: {:#?}",
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
            ("marker", 0, 0, Some((8, 171))),
            ("high", 1, 7, None),
            ("low", 1, 31, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_min_width_two_byte_prefix_reserved_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MinWidthPrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(1, 1)\n",
            "  high: UInt7\n",
            "  low: UInt8\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_min_width_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_min_width_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "minimum-width two-byte prefix reserved group bits should be accepted: {:#?}",
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
            ("prefix", 0, 0, Some((1, 1))),
            ("high", 1, 127, None),
            ("low", 1, 255, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_three_byte_prefix_reserved_group_bits() {
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
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_three_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_three_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "three-byte prefix reserved group bits should be accepted: {:#?}",
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
            ("prefix", 0, 0, Some((17, 87381))),
            ("high", 1, 15, None),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_four_byte_prefix_reserved_group_bits() {
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
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_four_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_four_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "four-byte prefix reserved group bits should be accepted: {:#?}",
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
            ("prefix", 0, 0, Some((25, 22369621))),
            ("high", 1, 15, None),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_five_byte_prefix_reserved_group_bits() {
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
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_five_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_five_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "five-byte prefix reserved group bits should be accepted: {:#?}",
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
            ("prefix", 0, 0, Some((33, 5726623061))),
            ("high", 1, 7, None),
            ("low", 1, 15, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_six_byte_prefix_reserved_group_bits() {
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
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_six_byte_prefix_reserved_group_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_six_byte_prefix_reserved_group_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "six-byte prefix reserved group bits should be accepted: {:#?}",
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
            ("prefix", 0, 0, Some((41, 1466015503701))),
            ("high", 1, 7, None),
            ("low", 1, 15, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_reject_malformed_three_byte_prefix_reserved_group_bits() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema TooWideThreeBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(17, 87381)\n",
            "  high: UInt4\n",
            "  low: UInt8\n",
            "end\n",
            "\n",
            "schema TooNarrowThreeBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(17, 87381)\n",
            "  high: UInt4\n",
            "  low: UInt2\n",
            "end\n",
            "\n",
            "schema TooWideVisibleThreeBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(1, 1)\n",
            "  high: UInt16be\n",
            "  low: UInt7\n",
            "end\n",
            "\n",
            "schema LittleEndianThreeBytePrefixReservedGroupHeader\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(1, 1)\n",
            "  high: UInt16le\n",
            "  low: UInt7\n",
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
        unsupported_shapes, 4,
        "malformed three-byte prefix reserved group bits should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "malformed three-byte prefix reserved group bits should not emit typed IR"
    );
}

#[test]
fn generated_schema_helpers_accept_split_reserved_bit_groups() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema SplitReservedHeader\n",
            "  format binary\n",
            "\n",
            "  top: ReservedBits(1, 1)\n",
            "  high: UInt2\n",
            "  gap: ReservedBits(2, 2)\n",
            "  low: UInt3\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{high: Int, low: Int}, String>\n",
            "  byte_decode_split_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_split_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "split reserved bit groups should be accepted: {:#?}",
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
            ("top", 0, 0, Some((1, 1))),
            ("high", 1, 3, None),
            ("gap", 0, 0, Some((2, 2))),
            ("low", 1, 7, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_five_byte_split_reserved_bit_groups() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema FiveByteSplitReservedHeader\n",
            "  format binary\n",
            "\n",
            "  lead: UInt3\n",
            "  guard: ReservedBits(10, 682)\n",
            "  mode: UInt5\n",
            "  gap: ReservedBits(17, 87381)\n",
            "  tail: UInt5\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{lead: Int, mode: Int, tail: Int}, String>\n",
            "  byte_decode_five_byte_split_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {lead: Int, mode: Int, tail: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_five_byte_split_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "five-byte split reserved bit groups should be accepted: {:#?}",
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
            ("lead", 1, 7, None),
            ("guard", 0, 0, Some((10, 682))),
            ("mode", 1, 31, None),
            ("gap", 0, 0, Some((17, 87381))),
            ("tail", 1, 31, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_six_byte_split_reserved_bit_groups() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema SixByteSplitReservedHeader\n",
            "  format binary\n",
            "\n",
            "  lead: UInt4\n",
            "  guard: ReservedBits(12, 2748)\n",
            "  mode: UInt6\n",
            "  gap: ReservedBits(20, 703710)\n",
            "  tail: UInt6\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{lead: Int, mode: Int, tail: Int}, String>\n",
            "  byte_decode_six_byte_split_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {lead: Int, mode: Int, tail: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_six_byte_split_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "six-byte split reserved bit groups should be accepted: {:#?}",
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
            ("lead", 1, 15, None),
            ("guard", 0, 0, Some((12, 2748))),
            ("mode", 1, 63, None),
            ("gap", 0, 0, Some((20, 703710))),
            ("tail", 1, 63, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_seven_byte_split_reserved_bit_groups() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema SevenByteSplitReservedHeader\n",
            "  format binary\n",
            "\n",
            "  lead: UInt5\n",
            "  guard: ReservedBits(14, 10922)\n",
            "  mode: UInt7\n",
            "  gap: ReservedBits(23, 5614165)\n",
            "  tail: UInt7\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{lead: Int, mode: Int, tail: Int}, String>\n",
            "  byte_decode_seven_byte_split_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {lead: Int, mode: Int, tail: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_seven_byte_split_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "seven-byte split reserved bit groups should be accepted: {:#?}",
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
            ("lead", 1, 31, None),
            ("guard", 0, 0, Some((14, 10922))),
            ("mode", 1, 127, None),
            ("gap", 0, 0, Some((23, 5614165))),
            ("tail", 1, 127, None),
        ]
    );
}

#[test]
fn generated_schema_helpers_accept_eight_byte_split_reserved_bit_groups() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema EightByteSplitReservedHeader\n",
            "  format binary\n",
            "\n",
            "  lead: UInt6\n",
            "  guard: ReservedBits(15, 21845)\n",
            "  mode: UInt7\n",
            "  gap: ReservedBits(29, 357913941)\n",
            "  tail: UInt7\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView) -> Result<{lead: Int, mode: Int, tail: Int}, String>\n",
            "  byte_decode_eight_byte_split_reserved_header(view)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {lead: Int, mode: Int, tail: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_eight_byte_split_reserved_header(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "eight-byte split reserved bit groups should be accepted: {:#?}",
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
            ("lead", 1, 63, None),
            ("guard", 0, 0, Some((15, 21845))),
            ("mode", 1, 127, None),
            ("gap", 0, 0, Some((29, 357913941))),
            ("tail", 1, 127, None),
        ]
    );
}

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
        unsupported_shapes, 3,
        "unsupported two-byte packed reserved suffix shapes should be rejected: {:#?}",
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
        unsupported_shapes, 3,
        "unsupported three-byte packed reserved suffix shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported three-byte packed reserved suffix shapes should not emit typed IR"
    );
}

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
        unsupported_shapes, 3,
        "unsupported four-byte packed reserved suffix shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported four-byte packed reserved suffix shapes should not emit typed IR"
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
            "schema TooWidePackedHeader\n",
            "  format binary\n",
            "\n",
            "  control_reserved: ReservedBits(17, 0)\n",
            "  control: UInt8\n",
            "end\n",
            "\n",
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
        unsupported_shapes, 3,
        "unsupported three-byte packed reserved shapes should be rejected: {:#?}",
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
            "schema TooWidePackedHeader\n",
            "  format binary\n",
            "\n",
            "  control_reserved: ReservedBits(25, 0)\n",
            "  control: UInt8\n",
            "end\n",
            "\n",
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
        unsupported_shapes, 3,
        "unsupported four-byte packed reserved shapes should be rejected: {:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "unsupported four-byte packed reserved shapes should not emit typed IR"
    );
}

#[test]
fn generated_schema_helpers_accept_standalone_sub_byte_primitives() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type BitRecord\n",
            "  BitRecord {one: Int, five: Int, seven: Int}\n",
            "end\n",
            "\n",
            "schema LooseBits\n",
            "  format binary\n",
            "\n",
            "  first: UInt1\n",
            "  middle: UInt5\n",
            "  last: UInt7\n",
            "\n",
            "  map to BitRecord\n",
            "    one = first\n",
            "    five = middle\n",
            "    seven = last\n",
            "end\n",
            "\n",
            "codec LooseCodec for LooseBits decode\n",
            "  derive decode\n",
            "end\n",
            "\n",
            "pub fn read_bits(view: ByteView, base: ByteOffset) -> DecodeStep<{one: Int, five: Int, seven: Int}>\n",
            "  LooseCodec(view, base)\n",
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
fn generated_schema_encode_helpers_resolve_for_recursive_closed_dispatch_binary_schemas() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type NodePayload\n",
            "  Leaf(Int)\n",
            "  Branch({length: Int, kind: Int, payload: NodePayload})\n",
            "end\n",
            "\n",
            "type Node\n",
            "  Node {length: Int, kind: Int, payload: NodePayload}\n",
            "end\n",
            "\n",
            "schema RecursiveNode\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, length, 0 => UInt8, 1 => RecursiveNode)\n",
            "\n",
            "  map to Node when kind == 0\n",
            "    length = length\n",
            "    kind = kind\n",
            "    payload = NodePayload::Leaf(payload)\n",
            "\n",
            "  map to Node when kind == 1\n",
            "    length = length\n",
            "    kind = kind\n",
            "    payload = NodePayload::Branch(payload)\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int, kind: Int, payload: NodePayload}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_recursive_node(packet)\n",
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
        } if name == "RecursiveNode"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "RecursiveNode")
        .expect("recursive node encoder metadata should be emitted");
    let dispatch = schema.fields[2]
        .dispatch
        .as_ref()
        .expect("payload should carry dispatch metadata");
    assert_eq!(dispatch.length_field.as_deref(), Some("length"));
    assert!(!dispatch.preserves_unknown);
    assert_eq!(dispatch.cases[0].tag, 0);
    assert_eq!(dispatch.cases[0].width, 1);
    assert_eq!(dispatch.cases[1].tag, 1);
    assert_eq!(
        dispatch.cases[1].payload_schema_name.as_deref(),
        Some("RecursiveNode")
    );
}

#[test]
fn generated_schema_encode_helpers_resolve_for_recursive_extension_dispatch_binary_schemas() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type NodePayload\n",
            "  Leaf(Int)\n",
            "  Branch({length: Int, kind: Int, payload: SchemaDispatchPayload<NodePayload>})\n",
            "end\n",
            "\n",
            "type Node\n",
            "  Node {length: Int, kind: Int, payload: SchemaDispatchPayload<NodePayload>}\n",
            "end\n",
            "\n",
            "schema RecursiveExtensionNode\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 0 => UInt8, 1 => RecursiveExtensionNode)\n",
            "\n",
            "  map to Node when kind == 0\n",
            "    length = length\n",
            "    kind = kind\n",
            "    payload = Known(NodePayload::Leaf(payload))\n",
            "\n",
            "  map to Node when kind == 1\n",
            "    length = length\n",
            "    kind = kind\n",
            "    payload = Known(NodePayload::Branch(payload))\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int, kind: Int, payload: SchemaDispatchPayload<NodePayload>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_recursive_extension_node(packet)\n",
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
        .find(|schema| schema.schema_name == "RecursiveExtensionNode")
        .expect("recursive extension node encoder metadata should be emitted");
    let dispatch = schema.fields[2]
        .dispatch
        .as_ref()
        .expect("payload should carry extension dispatch metadata");
    assert_eq!(dispatch.length_field.as_deref(), Some("length"));
    assert!(dispatch.preserves_unknown);
    assert_eq!(dispatch.cases[0].tag, 0);
    assert_eq!(dispatch.cases[0].width, 1);
    assert_eq!(dispatch.cases[1].tag, 1);
    assert_eq!(
        dispatch.cases[1].payload_schema_name.as_deref(),
        Some("RecursiveExtensionNode")
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
            "codec PacketCodec for PacketWire encode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int, kind: Int}) -> EncodeStep<()>\n",
            "  PacketCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn derived_codec_encode_resolves_mapped_schema_encode_step_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {length: Int, kind: Int}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt16be\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    length = wire_length\n",
            "    kind = wire_kind\n",
            "end\n",
            "\n",
            "codec HeaderCodec for HeaderWire encode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn main(header: {length: Int, kind: Int}) -> EncodeStep<()>\n",
            "  HeaderCodec(header)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
            ..
        } if name == "HeaderWire"
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
            target: IrCallTarget::SchemaEncodeStep(name),
            ..
        } if name == "HeaderWire"
    ));
}

#[test]
fn derived_codec_encode_resolves_selected_mapped_schema_encode_step_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Packet\n",
            "  Packet {kind: Int, value: Int}\n",
            "end\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  value: UInt8\n",
            "\n",
            "  map to Packet when kind == 1\n",
            "    kind = kind\n",
            "    value = value\n",
            "\n",
            "  map to Packet when kind == 2\n",
            "    kind = kind\n",
            "    value = value\n",
            "end\n",
            "\n",
            "codec PacketCodec for PacketWire encode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn main(packet: {kind: Int, value: Int}) -> EncodeStep<()>\n",
            "  PacketCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn derived_codec_encode_resolves_mixed_dispatch_selected_mapping_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Settings\n",
            "  Settings {code: Int, value: Int}\n",
            "end\n",
            "\n",
            "type PacketPayload\n",
            "  InlineValue(Int)\n",
            "  SettingsValue({code: Int, value: Int})\n",
            "end\n",
            "\n",
            "type Packet\n",
            "  Packet {kind: Int, body: PacketPayload}\n",
            "end\n",
            "\n",
            "schema SettingsPayload\n",
            "  format binary\n",
            "\n",
            "  code: UInt8\n",
            "  value: UInt16be\n",
            "\n",
            "  map to Settings\n",
            "    code = code\n",
            "    value = value\n",
            "end\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => UInt8, 2 => SettingsPayload)\n",
            "\n",
            "  map to Packet when kind == 1\n",
            "    kind = kind\n",
            "    body = InlineValue(payload)\n",
            "\n",
            "  map to Packet when kind == 2\n",
            "    kind = kind\n",
            "    body = SettingsValue(payload)\n",
            "end\n",
            "\n",
            "codec PacketCodec for PacketWire encode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn main(packet: {kind: Int, body: PacketPayload}) -> EncodeStep<()>\n",
            "  PacketCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            "codec PacketCodec for PacketWire encode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int, payload: ByteView}) -> EncodeStep<()>\n",
            "  PacketCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
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
            "codec PacketCodec for PacketWire encode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn main(packet: {kind: Int, payload: {code: Int, value: Int}}) -> EncodeStep<()>\n",
            "  PacketCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn derived_codec_encode_resolves_budgeted_schema_encode_step_boundary() {
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
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn main(packet: {length: Int}, budget: ByteCount) -> EncodeStep<{length: Int, encoded_offset: ByteCount}>\n",
            "  PacketCodec(packet, budget)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
            args,
        } if name == "PacketWire" && args.len() == 2
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
            "  flags: Flag8\n",
            "  sample_count: UInt8\n",
            "  samples: Repeat(sample_count, UInt16be)\n",
            "  padding: ReservedBits(8, 0)\n",
            "  metadata: ByteView(section_length - payload_length)\n",
            "  payload: ExtensionDispatch(kind, payload_length, 1 => TelemetryPayload)\n",
            "end\n",
            "\n",
            "codec TelemetryCodec for TelemetryEnvelope decode encode\n",
            "  derive decode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn decode_main(view: ByteView, base: ByteOffset) -> DecodeStep<{section_length: Int, payload_length: Int, kind: Int, flags: Flag8, sample_count: Int, samples: List<Int>, metadata: ByteView, payload: SchemaDispatchPayload<{channel: Int, reading: Int}>}>\n",
            "  TelemetryCodec(view, base)\n",
            "end\n",
            "\n",
            "pub fn encode_main(packet: {section_length: Int, payload_length: Int, kind: Int, flags: Flag8, sample_count: Int, samples: List<Int>, metadata: ByteView, payload: SchemaDispatchPayload<{channel: Int, reading: Int}>}) -> EncodeStep<()>\n",
            "  TelemetryCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
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
            "codec CountedCodec for CountedValues decode encode\n",
            "  derive decode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn decode_main(view: ByteView, base: ByteOffset) -> DecodeStep<{left_count: Int, right_count: Int, items: List<Int>}>\n",
            "  CountedCodec(view, base)\n",
            "end\n",
            "\n",
            "pub fn encode_main(packet: {left_count: Int, right_count: Int, items: List<Int>}) -> EncodeStep<()>\n",
            "  CountedCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
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
            "codec CountedCodec for CountedValues decode encode\n",
            "  derive decode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn decode_main(view: ByteView, base: ByteOffset) -> DecodeStep<{row_count: Int, column_count: Int, items: List<Int>}>\n",
            "  CountedCodec(view, base)\n",
            "end\n",
            "\n",
            "pub fn encode_main(packet: {row_count: Int, column_count: Int, items: List<Int>}) -> EncodeStep<()>\n",
            "  CountedCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
            ..
        } if name == "CountedValues"
    ));
}

#[test]
fn generated_schema_helpers_resolve_reserved_payload_dispatch_binary_schemas() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PackedReservedPayload\n",
            "  format binary\n",
            "\n",
            "  prefix: ReservedBits(2, 1)\n",
            "  value: UInt6\n",
            "end\n",
            "\n",
            "schema ByteReservedPayload\n",
            "  format binary\n",
            "\n",
            "  marker: ReservedBits(8, 171)\n",
            "  code: UInt8\n",
            "end\n",
            "\n",
            "schema ClosedReservedPacket\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => PackedReservedPayload)\n",
            "end\n",
            "\n",
            "schema ExtensionReservedPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 1 => ByteReservedPayload)\n",
            "end\n",
            "\n",
            "pub fn read_closed(view: ByteView) -> Result<{kind: Int, payload: {value: Int}}, String>\n",
            "  byte_decode_closed_reserved_packet(view)\n",
            "end\n",
            "\n",
            "pub fn write_closed(packet: {kind: Int, payload: {value: Int}}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_closed_reserved_packet(packet)\n",
            "end\n",
            "\n",
            "pub fn read_extension(view: ByteView) -> Result<{length: Int, kind: Int, payload: SchemaDispatchPayload<{code: Int}>}, String>\n",
            "  byte_decode_extension_reserved_packet(view)\n",
            "end\n",
            "\n",
            "pub fn write_extension(packet: {length: Int, kind: Int, payload: SchemaDispatchPayload<{code: Int}>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_extension_reserved_packet(packet)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let closed = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "ClosedReservedPacket")
        .expect("closed dispatch metadata should be emitted");
    let closed_dispatch = closed.fields[1]
        .dispatch
        .as_ref()
        .expect("closed payload should carry dispatch metadata");
    assert_eq!(
        closed_dispatch.cases[0]
            .payload_schema
            .as_ref()
            .expect("closed case should carry nested schema metadata")
            .schema_name,
        "PackedReservedPayload"
    );

    let extension = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "ExtensionReservedPacket")
        .expect("extension dispatch metadata should be emitted");
    let extension_dispatch = extension.fields[2]
        .dispatch
        .as_ref()
        .expect("extension payload should carry dispatch metadata");
    assert_eq!(extension_dispatch.length_field.as_deref(), Some("length"));
    assert_eq!(
        extension_dispatch.cases[0]
            .payload_schema
            .as_ref()
            .expect("extension case should carry nested schema metadata")
            .schema_name,
        "ByteReservedPayload"
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
fn generated_schema_decode_helpers_return_mapped_byte_view_field_shape() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Packet\n",
            "  Packet {length: Int, body: ByteView}\n",
            "end\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "  payload: ByteView(wire_length)\n",
            "\n",
            "  map to Packet\n",
            "    length = wire_length\n",
            "    body = payload\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{length: Int, body: ByteView}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn step(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int, body: ByteView}>\n",
            "  byte_decode_step_packet_wire(view, base)\n",
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
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet decoder should be emitted");
    assert_eq!(schema.fields[1].name, "payload");
    assert_eq!(
        schema.fields[1].length_field.as_deref(),
        Some("wire_length")
    );
    assert_eq!(
        schema
            .mapping
            .iter()
            .map(|field| (field.target.as_str(), field.source.as_str()))
            .collect::<Vec<_>>(),
        vec![("length", "wire_length"), ("body", "payload")]
    );
}

#[test]
fn generated_schema_decode_helpers_keep_structural_mapping_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type FrameKind\n",
            "  FrameKind(Int)\n",
            "end\n",
            "\n",
            "type Header\n",
            "  Header {summary: {value: Int}, kind: FrameKind}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt16be\n",
            "  kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    summary = {value: length}\n",
            "    kind = FrameKind(kind)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{summary: {value: Int}, kind: FrameKind}, String>\n",
            "  byte_decode_header_wire(view)\n",
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
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let summary = schema
        .mapping
        .iter()
        .find(|field| field.target == "summary")
        .expect("summary mapping should be emitted");
    assert!(matches!(
        &summary.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Record(fields)
            if fields.len() == 1
                && fields[0].name == "value"
                && matches!(
                    fields[0].expr,
                    veln_ir::IrSchemaDecodeMappingExpr::Field(ref name) if name == "length"
                )
    ));
    let kind = schema
        .mapping
        .iter()
        .find(|field| field.target == "kind")
        .expect("kind mapping should be emitted");
    assert!(matches!(
        &kind.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Constructor { name, args }
            if name == &vec!["FrameKind".to_string(), "FrameKind".to_string()]
                && args.len() == 1
                && matches!(
                    args[0],
                    veln_ir::IrSchemaDecodeMappingExpr::Field(ref field) if field == "kind"
                )
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_nested_constructor_mapping_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Payload\n",
            "  Payload(Int)\n",
            "end\n",
            "\n",
            "type Envelope\n",
            "  Envelope(Payload)\n",
            "end\n",
            "\n",
            "type Header\n",
            "  Header {wrapped: Envelope}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    wrapped = Envelope(Payload(kind))\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{wrapped: Envelope}, String>\n",
            "  byte_decode_header_wire(view)\n",
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
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let wrapped = schema
        .mapping
        .iter()
        .find(|field| field.target == "wrapped")
        .expect("wrapped mapping should be emitted");
    assert!(matches!(
        &wrapped.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Constructor { name, args }
            if name == &vec!["Envelope".to_string(), "Envelope".to_string()]
                && args.len() == 1
                && matches!(
                    &args[0],
                    veln_ir::IrSchemaDecodeMappingExpr::Constructor {
                        name: nested_name,
                        args: nested_args,
                    } if nested_name == &vec!["Payload".to_string(), "Payload".to_string()]
                        && nested_args.len() == 1
                        && matches!(
                            &nested_args[0],
                            veln_ir::IrSchemaDecodeMappingExpr::Field(field)
                                if field == "kind"
                        )
                )
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_field_selection_mapping_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {summary: Int, converted: Int, wrapped: FrameKind}\n",
            "end\n",
            "\n",
            "type FrameKind\n",
            "  FrameKind(Int)\n",
            "end\n",
            "\n",
            "fn wrap(input: Int) -> {code: Int}\n",
            "  {code: input}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    summary = {code: kind}.code\n",
            "    converted = wrap(kind).code\n",
            "    wrapped = FrameKind({code: kind}.code)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{summary: Int, converted: Int, wrapped: FrameKind}, String>\n",
            "  byte_decode_header_wire(view)\n",
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
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let summary = schema
        .mapping
        .iter()
        .find(|field| field.target == "summary")
        .expect("summary mapping should be emitted");
    assert!(matches!(
        &summary.expr,
        veln_ir::IrSchemaDecodeMappingExpr::FieldAccess { base, field }
            if field == "code"
                && matches!(
                    base.as_ref(),
                    veln_ir::IrSchemaDecodeMappingExpr::Record(fields)
                        if fields.len() == 1
                            && fields[0].name == "code"
                            && matches!(
                                fields[0].expr,
                                veln_ir::IrSchemaDecodeMappingExpr::Field(ref name)
                                    if name == "kind"
                            )
                )
    ));
    let converted = schema
        .mapping
        .iter()
        .find(|field| field.target == "converted")
        .expect("converted mapping should be emitted");
    assert!(matches!(
        &converted.expr,
        veln_ir::IrSchemaDecodeMappingExpr::FieldAccess { base, field }
            if field == "code"
                && matches!(
                    base.as_ref(),
                    veln_ir::IrSchemaDecodeMappingExpr::Converter { function, .. }
                        if function == "wrap"
                )
    ));
    let wrapped = schema
        .mapping
        .iter()
        .find(|field| field.target == "wrapped")
        .expect("wrapped mapping should be emitted");
    assert!(matches!(
        &wrapped.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Constructor { args, .. }
            if args.len() == 1
                && matches!(
                    &args[0],
                    veln_ir::IrSchemaDecodeMappingExpr::FieldAccess { base, field }
                        if field == "code"
                            && matches!(
                                base.as_ref(),
                                veln_ir::IrSchemaDecodeMappingExpr::Record(fields)
                                    if fields.len() == 1 && fields[0].name == "code"
                            )
                )
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_integer_arithmetic_mapping_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {body_length: Int, scaled_length: Int, converted_length: Int}\n",
            "end\n",
            "\n",
            "fn bump(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding: UInt8\n",
            "  checksum: UInt8\n",
            "\n",
            "  map to Header\n",
            "    body_length = (length - 9) + checksum\n",
            "    scaled_length = (length + padding) * 2\n",
            "    converted_length = bump(length) + checksum\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{body_length: Int, scaled_length: Int, converted_length: Int}, String>\n",
            "  byte_decode_header_wire(view)\n",
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
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let body_length = schema
        .mapping
        .iter()
        .find(|field| field.target == "body_length")
        .expect("body_length mapping should be emitted");
    assert!(matches!(
        &body_length.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Binary {
            op: veln_ast::BinaryOp::Add,
            left,
            right,
        } if matches!(
                left.as_ref(),
                veln_ir::IrSchemaDecodeMappingExpr::Binary {
                    op: veln_ast::BinaryOp::Subtract,
                    left,
                    right,
                } if matches!(
                        left.as_ref(),
                        veln_ir::IrSchemaDecodeMappingExpr::Field(field) if field == "length"
                    )
                    && matches!(
                        right.as_ref(),
                        veln_ir::IrSchemaDecodeMappingExpr::Literal(9)
                    )
            )
            && matches!(
                right.as_ref(),
                veln_ir::IrSchemaDecodeMappingExpr::Field(field) if field == "checksum"
            )
    ));
    let scaled_length = schema
        .mapping
        .iter()
        .find(|field| field.target == "scaled_length")
        .expect("scaled_length mapping should be emitted");
    assert!(matches!(
        &scaled_length.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Binary {
            op: veln_ast::BinaryOp::Multiply,
            left,
            right,
        } if matches!(
                left.as_ref(),
                veln_ir::IrSchemaDecodeMappingExpr::Binary {
                    op: veln_ast::BinaryOp::Add,
                    left,
                    right,
                } if matches!(
                        left.as_ref(),
                        veln_ir::IrSchemaDecodeMappingExpr::Field(field) if field == "length"
                    )
                    && matches!(
                        right.as_ref(),
                        veln_ir::IrSchemaDecodeMappingExpr::Field(field) if field == "padding"
                    )
            )
            && matches!(right.as_ref(), veln_ir::IrSchemaDecodeMappingExpr::Literal(2))
    ));
    let converted_length = schema
        .mapping
        .iter()
        .find(|field| field.target == "converted_length")
        .expect("converted_length mapping should be emitted");
    assert!(matches!(
        &converted_length.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Binary {
            op: veln_ast::BinaryOp::Add,
            left,
            right,
        } if matches!(
                left.as_ref(),
                veln_ir::IrSchemaDecodeMappingExpr::Converter { function, args, .. }
                    if function == "bump"
                        && matches!(&args[0], veln_ir::IrSchemaDecodeMappingExpr::Field(field) if field == "length")
            )
            && matches!(
                right.as_ref(),
                veln_ir::IrSchemaDecodeMappingExpr::Field(field) if field == "checksum"
            )
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_bool_composition_mapping_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Packet\n",
            "  Packet {active: Bool, safe_length: Bool, not_kind_two: Bool}\n",
            "end\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  flags: UInt8\n",
            "  length: UInt8\n",
            "  copy_length: UInt8\n",
            "\n",
            "  map to Packet\n",
            "    active = kind == 1 and flags != 0\n",
            "    safe_length = length == copy_length or not (flags != 0)\n",
            "    not_kind_two = not (kind == 2)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{active: Bool, safe_length: Bool, not_kind_two: Bool}, String>\n",
            "  byte_decode_packet_wire(view)\n",
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
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet decoder should be emitted");
    let active = schema
        .mapping
        .iter()
        .find(|field| field.target == "active")
        .expect("active mapping should be emitted");
    assert!(matches!(
        &active.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Binary {
            op: veln_ast::BinaryOp::And,
            left,
            right,
        } if matches!(
                left.as_ref(),
                veln_ir::IrSchemaDecodeMappingExpr::Binary {
                    op: veln_ast::BinaryOp::Equal,
                    ..
                }
            )
            && matches!(
                right.as_ref(),
                veln_ir::IrSchemaDecodeMappingExpr::Binary {
                    op: veln_ast::BinaryOp::NotEqual,
                    ..
                }
            )
    ));
    let safe_length = schema
        .mapping
        .iter()
        .find(|field| field.target == "safe_length")
        .expect("safe_length mapping should be emitted");
    assert!(matches!(
        &safe_length.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Binary {
            op: veln_ast::BinaryOp::Or,
            left,
            right,
        } if matches!(
                left.as_ref(),
                veln_ir::IrSchemaDecodeMappingExpr::Binary {
                    op: veln_ast::BinaryOp::Equal,
                    ..
                }
            )
            && matches!(
                right.as_ref(),
                veln_ir::IrSchemaDecodeMappingExpr::Prefix {
                    op: veln_ast::PrefixOp::Not,
                    ..
                }
            )
    ));
    let not_kind_two = schema
        .mapping
        .iter()
        .find(|field| field.target == "not_kind_two")
        .expect("not_kind_two mapping should be emitted");
    assert!(matches!(
        &not_kind_two.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Prefix {
            op: veln_ast::PrefixOp::Not,
            expr,
        } if matches!(
            expr.as_ref(),
            veln_ir::IrSchemaDecodeMappingExpr::Binary {
                op: veln_ast::BinaryOp::Equal,
                ..
            }
        )
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_imported_converter_mapping_expressions() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use helpers\n",
            "\n",
            "type Header\n",
            "  Header {kind: Int}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    kind = helpers::next_kind(wire_kind)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{kind: Int}, String>\n",
            "  byte_decode_header_wire(view)\n",
            "end\n",
        ),
    );
    let helpers_source = SourceFile::new(
        "helpers.veln",
        concat!(
            "mod helpers\n",
            "pub fn next_kind(kind: Int) -> Int\n",
            "  kind + 1\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let helpers = lower_surface_ast(&parse(&helpers_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        schemas: app.schemas,
        codecs: Vec::new(),
        types: app.types,
        functions: app.functions.into_iter().chain(helpers.functions).collect(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let kind = schema
        .mapping
        .iter()
        .find(|field| field.target == "kind")
        .expect("kind mapping should be emitted");
    assert!(matches!(
        &kind.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Converter { function, args, .. }
            if function == "next_kind"
                && args.len() == 1
                && matches!(&args[0], veln_ir::IrSchemaDecodeMappingExpr::Field(field) if field == "wire_kind")
    ));
}

#[test]
fn generated_schema_encode_helpers_keep_imported_converter_inverse_mapping_expressions() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use helpers\n",
            "\n",
            "type Header\n",
            "  Header {kind: Int}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    kind = helpers::next_kind(wire_kind) inverse helpers::previous_kind\n",
            "end\n",
            "\n",
            "pub fn main(header: {kind: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_header_wire(header)\n",
            "end\n",
        ),
    );
    let helpers_source = SourceFile::new(
        "helpers.veln",
        concat!(
            "mod helpers\n",
            "pub fn next_kind(kind: Int) -> Int\n",
            "  kind + 1\n",
            "end\n",
            "\n",
            "pub fn previous_kind(kind: Int) -> Int\n",
            "  kind - 1\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let helpers = lower_surface_ast(&parse(&helpers_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        schemas: app.schemas,
        codecs: Vec::new(),
        types: app.types,
        functions: app.functions.into_iter().chain(helpers.functions).collect(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let kind = schema
        .mapping
        .iter()
        .find(|field| field.target == "kind")
        .expect("kind mapping should be emitted");
    assert!(matches!(
        &kind.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Converter {
            function,
            inverse_function,
            args,
            ..
        } if function == "next_kind"
            && inverse_function.as_deref() == Some("previous_kind")
            && args.len() == 1
            && matches!(&args[0], veln_ir::IrSchemaDecodeMappingExpr::Field(field) if field == "wire_kind")
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_structural_converter_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {kind: Int}\n",
            "end\n",
            "\n",
            "fn next_kind(input: {value: Int}) -> Int\n",
            "  input.value + 1\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    kind = next_kind({value: wire_kind})\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{kind: Int}, String>\n",
            "  byte_decode_header_wire(view)\n",
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
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let kind = schema
        .mapping
        .iter()
        .find(|field| field.target == "kind")
        .expect("kind mapping should be emitted");
    assert!(matches!(
        &kind.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Converter { function, args, .. }
            if function == "next_kind"
                && args.len() == 1
                && matches!(
                    &args[0],
                    veln_ir::IrSchemaDecodeMappingExpr::Record(fields)
                        if fields.len() == 1
                            && fields[0].name == "value"
                            && matches!(
                                fields[0].expr,
                                veln_ir::IrSchemaDecodeMappingExpr::Field(ref field)
                                    if field == "wire_kind"
                            )
                )
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_two_argument_converter_mapping_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {summary: Int}\n",
            "end\n",
            "\n",
            "fn combine(input: {value: Int}, extra: Int) -> Int\n",
            "  input.value + extra\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt16be\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    summary = combine({value: wire_kind}, wire_length - 1)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{summary: Int}, String>\n",
            "  byte_decode_header_wire(view)\n",
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
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let summary = schema
        .mapping
        .iter()
        .find(|field| field.target == "summary")
        .expect("summary mapping should be emitted");
    assert!(matches!(
        &summary.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Converter { function, args, .. }
            if function == "combine"
                && args.len() == 2
                && matches!(
                    &args[0],
                    veln_ir::IrSchemaDecodeMappingExpr::Record(fields)
                        if fields.len() == 1
                            && fields[0].name == "value"
                            && matches!(
                                fields[0].expr,
                                veln_ir::IrSchemaDecodeMappingExpr::Field(ref field)
                                    if field == "wire_kind"
                            )
                )
                && matches!(
                    &args[1],
                    veln_ir::IrSchemaDecodeMappingExpr::Binary {
                        op: veln_ast::BinaryOp::Subtract,
                        left,
                        right,
                    } if matches!(
                            left.as_ref(),
                            veln_ir::IrSchemaDecodeMappingExpr::Field(field)
                                if field == "wire_length"
                        )
                        && matches!(
                            right.as_ref(),
                            veln_ir::IrSchemaDecodeMappingExpr::Literal(1)
                        )
                )
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_three_argument_converter_mapping_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {summary: Int}\n",
            "end\n",
            "\n",
            "fn combine(input: {value: Int}, extra: Int, another: Int) -> Int\n",
            "  input.value + extra + another\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt16be\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    summary = combine({value: wire_kind}, wire_length - 1, wire_kind + 1)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{summary: Int}, String>\n",
            "  byte_decode_header_wire(view)\n",
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
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let summary = schema
        .mapping
        .iter()
        .find(|field| field.target == "summary")
        .expect("summary mapping should be emitted");
    assert!(matches!(
        &summary.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Converter { function, args, .. }
            if function == "combine"
                && args.len() == 3
                && matches!(
                    &args[0],
                    veln_ir::IrSchemaDecodeMappingExpr::Record(fields)
                        if fields.len() == 1
                            && fields[0].name == "value"
                            && matches!(
                                fields[0].expr,
                                veln_ir::IrSchemaDecodeMappingExpr::Field(ref field)
                                    if field == "wire_kind"
                            )
                )
                && matches!(
                    &args[1],
                    veln_ir::IrSchemaDecodeMappingExpr::Binary {
                        op: veln_ast::BinaryOp::Subtract,
                        left,
                        right,
                    } if matches!(
                            left.as_ref(),
                            veln_ir::IrSchemaDecodeMappingExpr::Field(field)
                                if field == "wire_length"
                        )
                        && matches!(
                            right.as_ref(),
                            veln_ir::IrSchemaDecodeMappingExpr::Literal(1)
                        )
                )
                && matches!(
                    &args[2],
                    veln_ir::IrSchemaDecodeMappingExpr::Binary {
                        op: veln_ast::BinaryOp::Add,
                        left,
                        right,
                    } if matches!(
                            left.as_ref(),
                            veln_ir::IrSchemaDecodeMappingExpr::Field(field)
                                if field == "wire_kind"
                        )
                        && matches!(
                            right.as_ref(),
                            veln_ir::IrSchemaDecodeMappingExpr::Literal(1)
                        )
                )
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_four_argument_converter_mapping_expressions() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Header\n",
            "  Header {summary: Int}\n",
            "end\n",
            "\n",
            "fn combine(input: {value: Int}, extra: Int, another: Int, final: Int) -> Int\n",
            "  input.value + extra + another + final\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt16be\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    summary = combine({value: wire_kind}, wire_length - 1, wire_kind + 1, wire_length + wire_kind)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{summary: Int}, String>\n",
            "  byte_decode_header_wire(view)\n",
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
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let summary = schema
        .mapping
        .iter()
        .find(|field| field.target == "summary")
        .expect("summary mapping should be emitted");
    assert!(matches!(
        &summary.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Converter { function, args, .. }
            if function == "combine"
                && args.len() == 4
                && matches!(
                    &args[0],
                    veln_ir::IrSchemaDecodeMappingExpr::Record(fields)
                        if fields.len() == 1
                            && fields[0].name == "value"
                            && matches!(
                                fields[0].expr,
                                veln_ir::IrSchemaDecodeMappingExpr::Field(ref field)
                                    if field == "wire_kind"
                            )
                )
                && matches!(
                    &args[1],
                    veln_ir::IrSchemaDecodeMappingExpr::Binary {
                        op: veln_ast::BinaryOp::Subtract,
                        ..
                    }
                )
                && matches!(
                    &args[2],
                    veln_ir::IrSchemaDecodeMappingExpr::Binary {
                        op: veln_ast::BinaryOp::Add,
                        ..
                    }
                )
                && matches!(
                    &args[3],
                    veln_ir::IrSchemaDecodeMappingExpr::Binary {
                        op: veln_ast::BinaryOp::Add,
                        ..
                    }
                )
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_imported_two_argument_converter_mapping_expressions() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use helpers\n",
            "\n",
            "type Header\n",
            "  Header {summary: Int}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt16be\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    summary = helpers::combine({value: wire_kind}, wire_length)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{summary: Int}, String>\n",
            "  byte_decode_header_wire(view)\n",
            "end\n",
        ),
    );
    let helpers_source = SourceFile::new(
        "helpers.veln",
        concat!(
            "mod helpers\n",
            "pub fn combine(input: {value: Int}, extra: Int) -> Int\n",
            "  input.value + extra\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let helpers = lower_surface_ast(&parse(&helpers_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        schemas: app.schemas,
        codecs: Vec::new(),
        types: app.types,
        functions: app.functions.into_iter().chain(helpers.functions).collect(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let summary = schema
        .mapping
        .iter()
        .find(|field| field.target == "summary")
        .expect("summary mapping should be emitted");
    assert!(matches!(
        &summary.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Converter { function, args, .. }
            if function == "combine"
                && args.len() == 2
                && matches!(
                    &args[0],
                    veln_ir::IrSchemaDecodeMappingExpr::Record(fields)
                        if fields.len() == 1 && fields[0].name == "value"
                )
                && matches!(
                    &args[1],
                    veln_ir::IrSchemaDecodeMappingExpr::Field(field) if field == "wire_length"
                )
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_imported_three_argument_converter_mapping_expressions() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use helpers\n",
            "\n",
            "type Header\n",
            "  Header {summary: Int}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt16be\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    summary = helpers::combine({value: wire_kind}, wire_length, wire_kind + 1)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{summary: Int}, String>\n",
            "  byte_decode_header_wire(view)\n",
            "end\n",
        ),
    );
    let helpers_source = SourceFile::new(
        "helpers.veln",
        concat!(
            "mod helpers\n",
            "pub fn combine(input: {value: Int}, extra: Int, another: Int) -> Int\n",
            "  input.value + extra + another\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let helpers = lower_surface_ast(&parse(&helpers_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        schemas: app.schemas,
        codecs: Vec::new(),
        types: app.types,
        functions: app.functions.into_iter().chain(helpers.functions).collect(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let summary = schema
        .mapping
        .iter()
        .find(|field| field.target == "summary")
        .expect("summary mapping should be emitted");
    assert!(matches!(
        &summary.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Converter { function, args, .. }
            if function == "combine"
                && args.len() == 3
                && matches!(
                    &args[0],
                    veln_ir::IrSchemaDecodeMappingExpr::Record(fields)
                        if fields.len() == 1 && fields[0].name == "value"
                )
                && matches!(
                    &args[1],
                    veln_ir::IrSchemaDecodeMappingExpr::Field(field) if field == "wire_length"
                )
                && matches!(
                    &args[2],
                    veln_ir::IrSchemaDecodeMappingExpr::Binary {
                        op: veln_ast::BinaryOp::Add,
                        ..
                    }
                )
    ));
}

#[test]
fn generated_schema_decode_helpers_keep_imported_four_argument_converter_mapping_expressions() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use helpers\n",
            "\n",
            "type Header\n",
            "  Header {summary: Int}\n",
            "end\n",
            "\n",
            "schema HeaderWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt16be\n",
            "  wire_kind: UInt8\n",
            "\n",
            "  map to Header\n",
            "    summary = helpers::combine({value: wire_kind}, wire_length, wire_kind + 1, wire_length + wire_kind)\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{summary: Int}, String>\n",
            "  byte_decode_header_wire(view)\n",
            "end\n",
        ),
    );
    let helpers_source = SourceFile::new(
        "helpers.veln",
        concat!(
            "mod helpers\n",
            "pub fn combine(input: {value: Int}, extra: Int, another: Int, final: Int) -> Int\n",
            "  input.value + extra + another + final\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let helpers = lower_surface_ast(&parse(&helpers_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        schemas: app.schemas,
        codecs: Vec::new(),
        types: app.types,
        functions: app.functions.into_iter().chain(helpers.functions).collect(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered.ir.expect("typed IR should be built");
    let schema = ir
        .schema_decoders
        .iter()
        .find(|schema| schema.schema_name == "HeaderWire")
        .expect("header decoder should be emitted");
    let summary = schema
        .mapping
        .iter()
        .find(|field| field.target == "summary")
        .expect("summary mapping should be emitted");
    assert!(matches!(
        &summary.expr,
        veln_ir::IrSchemaDecodeMappingExpr::Converter { function, args, .. }
            if function == "combine"
                && args.len() == 4
                && matches!(
                    &args[0],
                    veln_ir::IrSchemaDecodeMappingExpr::Record(fields)
                        if fields.len() == 1 && fields[0].name == "value"
                )
                && matches!(
                    &args[1],
                    veln_ir::IrSchemaDecodeMappingExpr::Field(field) if field == "wire_length"
                )
                && matches!(
                    &args[2],
                    veln_ir::IrSchemaDecodeMappingExpr::Binary {
                        op: veln_ast::BinaryOp::Add,
                        ..
                    }
                )
                && matches!(
                    &args[3],
                    veln_ir::IrSchemaDecodeMappingExpr::Binary {
                        op: veln_ast::BinaryOp::Add,
                        ..
                    }
                )
    ));
}

#[test]
fn generated_schema_decode_helpers_require_int_byte_view_length_field() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "  first_payload: ByteView(wire_length)\n",
            "  second_payload: ByteView(first_payload)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.byte_view_reference"
                && diagnostic.message
                    == "ByteView length operand `first_payload` decodes as `ByteView`, not `Int`"
        }),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing ByteView length field should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_subtracted_byte_view_length_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8\n",
            "  payload: ByteView(length - padding_length)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, padding_length: Int, payload: ByteView}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, padding_length: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
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
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "payload");
    assert_eq!(
        schema.fields[2].length_field.as_deref(),
        Some("length - padding_length")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_added_byte_view_length_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8\n",
            "  payload: ByteView(length + padding_length)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, padding_length: Int, payload: ByteView}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, padding_length: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
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
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "payload");
    assert_eq!(
        schema.fields[2].length_field.as_deref(),
        Some("length + padding_length")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_product_byte_view_length_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  row_count: UInt8\n",
            "  column_count: UInt8\n",
            "  payload: ByteView(row_count * column_count)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{row_count: Int, column_count: Int, payload: ByteView}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {row_count: Int, column_count: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
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
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "payload");
    assert_eq!(
        schema.fields[2].length_field.as_deref(),
        Some("row_count * column_count")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_quotient_byte_view_length_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  chunk_count: UInt8\n",
            "  payload: ByteView(length / chunk_count)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, chunk_count: Int, payload: ByteView}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, chunk_count: Int, payload: ByteView}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
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
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "payload");
    assert_eq!(
        schema.fields[2].length_field.as_deref(),
        Some("length / chunk_count")
    );
}

#[test]
fn repeat_count_expressions_accept_product_lengths() {
    let repeat = repeat_schema_primitive("Repeat(row_count * column_count, UInt16be)")
        .expect("product repeat count should parse");

    assert_eq!(repeat.count_field, "row_count * column_count");
}

#[test]
fn repeat_count_expressions_accept_quotient_lengths() {
    let repeat = repeat_schema_primitive("Repeat(length / chunk_count, UInt16be)")
        .expect("quotient repeat count should parse");

    assert_eq!(repeat.count_field, "length / chunk_count");
}

#[test]
fn generated_schema_decode_helpers_resolve_subtracted_repeat_count_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8\n",
            "  items: Repeat(length - padding_length, UInt16be)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, padding_length: Int, items: List<Int>}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, padding_length: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
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
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "items");
    assert_eq!(
        schema.fields[2]
            .repeat
            .as_ref()
            .map(|repeat| repeat.count_field.as_str()),
        Some("length - padding_length")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_added_repeat_count_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  padding_length: UInt8\n",
            "  items: Repeat(length + padding_length, UInt16be)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, padding_length: Int, items: List<Int>}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, padding_length: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
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
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "items");
    assert_eq!(
        schema.fields[2]
            .repeat
            .as_ref()
            .map(|repeat| repeat.count_field.as_str()),
        Some("length + padding_length")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_product_repeat_count_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  row_count: UInt8\n",
            "  column_count: UInt8\n",
            "  items: Repeat(row_count * column_count, UInt16be)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{row_count: Int, column_count: Int, items: List<Int>}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {row_count: Int, column_count: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
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
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "items");
    assert_eq!(
        schema.fields[2]
            .repeat
            .as_ref()
            .map(|repeat| repeat.count_field.as_str()),
        Some("row_count * column_count")
    );
}

#[test]
fn generated_schema_decode_helpers_resolve_quotient_repeat_count_fields() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  chunk_count: UInt8\n",
            "  items: Repeat(length / chunk_count, UInt16be)\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{length: Int, chunk_count: Int, items: List<Int>}, String>\n",
            "  byte_decode_packet_wire(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {length: Int, chunk_count: Int, items: List<Int>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_packet_wire(packet)\n",
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
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet schema should be emitted");
    assert_eq!(schema.fields[2].name, "items");
    assert_eq!(
        schema.fields[2]
            .repeat
            .as_ref()
            .map(|repeat| repeat.count_field.as_str()),
        Some("length / chunk_count")
    );
}

#[test]
fn generated_schema_decode_helpers_reject_forward_subtracted_byte_view_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length - padding_length)\n",
            "  padding_length: UInt8\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "schema.byte_view_reference"
                && diagnostic.message
                    == "ByteView length operand `padding_length` must be an earlier decoded `Int` field"
        }),
        "{:#?}",
        lowered.diagnostics
    );
    assert!(
        lowered.ir.is_none(),
        "diagnostic-bearing ByteView length expression should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_reject_added_byte_view_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length + padding_length)\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length + padding_length)\n",
            "  padding_length: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  flags: Flag8\n",
            "  payload: ByteView(length + flags)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "ByteView length operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "ByteView length operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "ByteView length operand `flags` decodes as `Flag8`, not `Int`",
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
        "diagnostic-bearing ByteView length expression should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_reject_quotient_byte_view_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length / chunk_count)\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  payload: ByteView(length / chunk_count)\n",
            "  chunk_count: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  flags: Flag8\n",
            "  payload: ByteView(length / flags)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "ByteView length operand `chunk_count` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "ByteView length operand `chunk_count` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "ByteView length operand `flags` decodes as `Flag8`, not `Int`",
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
        "diagnostic-bearing ByteView length expression should not emit typed IR"
    );
}

#[test]
fn generated_schema_decode_helpers_reject_subtracted_repeat_count_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  items: Repeat(length - padding_length, UInt16be)\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  items: Repeat(length - padding_length, UInt16be)\n",
            "  padding_length: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  flags: Flag8\n",
            "  items: Repeat(length - flags, UInt16be)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "repeat count operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "repeat count operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "repeat count operand `flags` decodes as `Flag8`, not `Int`",
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
fn generated_schema_decode_helpers_reject_added_repeat_count_operands() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema MissingOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  items: Repeat(length + padding_length, UInt16be)\n",
            "end\n",
            "\n",
            "schema ForwardOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  items: Repeat(length + padding_length, UInt16be)\n",
            "  padding_length: UInt8\n",
            "end\n",
            "\n",
            "schema WrongKindOperandPacket\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  flags: Flag8\n",
            "  items: Repeat(length + flags, UInt16be)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    for (reason, message) in [
        (
            "unknown_field_reference",
            "repeat count operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "forward_field_reference",
            "repeat count operand `padding_length` must be an earlier decoded `Int` field",
        ),
        (
            "incompatible_field_reference",
            "repeat count operand `flags` decodes as `Flag8`, not `Int`",
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
            "  flags: Flag8\n",
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
            "repeat count operand `flags` decodes as `Flag8`, not `Int`",
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
            "  flags: Flag8\n",
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
            "repeat count operand `flags` decodes as `Flag8`, not `Int`",
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
fn generated_schema_decode_helpers_return_mapped_nested_dispatch_field_shape() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Settings\n",
            "  Settings {code: Int, value: Int}\n",
            "end\n",
            "\n",
            "type Packet\n",
            "  Packet {kind: Int, body: {code: Int, value: Int}}\n",
            "end\n",
            "\n",
            "schema SettingsPayload\n",
            "  format binary\n",
            "\n",
            "  wire_code: UInt8\n",
            "  wire_value: UInt16be\n",
            "\n",
            "  map to Settings\n",
            "    code = wire_code\n",
            "    value = wire_value\n",
            "end\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => SettingsPayload)\n",
            "\n",
            "  map to Packet\n",
            "    kind = kind\n",
            "    body = payload\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{kind: Int, body: {code: Int, value: Int}}, String>\n",
            "  byte_decode_packet_wire(view)\n",
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
        .find(|schema| schema.schema_name == "PacketWire")
        .expect("packet decoder should be emitted");
    assert_eq!(
        schema
            .mapping
            .iter()
            .map(|field| (field.target.as_str(), field.source.as_str()))
            .collect::<Vec<_>>(),
        vec![("kind", "kind"), ("body", "payload")]
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
fn generated_schema_decode_helpers_keep_length_bounded_recursive_dispatch_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Node\n",
            "  Node {kind: Int, value: Int, child: Option<Node>}\n",
            "end\n",
            "\n",
            "schema RecursiveNode\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, length, 0 => UInt8, 1 => RecursiveNode)\n",
            "\n",
            "  map to Node when kind == 0\n",
            "    kind = kind\n",
            "    value = payload\n",
            "    child = None\n",
            "\n",
            "  map to Node when kind == 1\n",
            "    kind = kind\n",
            "    value = payload.value\n",
            "    child = Some(Node(payload.kind, payload.value, payload.child))\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{kind: Int, value: Int, child: Option<Node>}, String>\n",
            "  byte_decode_recursive_node(view)\n",
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
        .find(|schema| schema.schema_name == "RecursiveNode")
        .expect("recursive node decoder should be emitted");
    let dispatch = schema.fields[2]
        .dispatch
        .as_ref()
        .expect("payload should carry dispatch metadata");
    assert_eq!(dispatch.length_field.as_deref(), Some("length"));
    assert!(!dispatch.preserves_unknown);
    assert_eq!(dispatch.cases[0].tag, 0);
    assert_eq!(dispatch.cases[0].width, 1);
    assert_eq!(dispatch.cases[1].tag, 1);
    assert_eq!(
        dispatch.cases[1].payload_schema_name.as_deref(),
        Some("RecursiveNode")
    );
}

#[test]
fn generated_schema_decode_helpers_keep_recursive_extension_dispatch_metadata() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type NodePayload\n",
            "  Leaf(Int)\n",
            "  Branch({length: Int, kind: Int, payload: SchemaDispatchPayload<NodePayload>})\n",
            "end\n",
            "\n",
            "type Node\n",
            "  Node {length: Int, kind: Int, payload: SchemaDispatchPayload<NodePayload>}\n",
            "end\n",
            "\n",
            "schema RecursiveExtensionNode\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 0 => UInt8, 1 => RecursiveExtensionNode)\n",
            "\n",
            "  map to Node when kind == 0\n",
            "    length = length\n",
            "    kind = kind\n",
            "    payload = Known(NodePayload::Leaf(payload))\n",
            "\n",
            "  map to Node when kind == 1\n",
            "    length = length\n",
            "    kind = kind\n",
            "    payload = Known(NodePayload::Branch(payload))\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView) -> Result<{length: Int, kind: Int, payload: SchemaDispatchPayload<NodePayload>}, String>\n",
            "  byte_decode_recursive_extension_node(view)\n",
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
        .find(|schema| schema.schema_name == "RecursiveExtensionNode")
        .expect("recursive extension node decoder metadata should be emitted");
    let dispatch = schema.fields[2]
        .dispatch
        .as_ref()
        .expect("payload should carry extension dispatch metadata");
    assert_eq!(dispatch.length_field.as_deref(), Some("length"));
    assert!(dispatch.preserves_unknown);
    assert_eq!(dispatch.cases[0].tag, 0);
    assert_eq!(dispatch.cases[0].width, 1);
    assert_eq!(dispatch.cases[1].tag, 1);
    assert_eq!(
        dispatch.cases[1].payload_schema_name.as_deref(),
        Some("RecursiveExtensionNode")
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
        types: [app.types, wire.types].concat(),
        schemas: [app.schemas, wire.schemas].concat(),
        codecs: Vec::new(),
        functions: [app.functions, wire.functions].concat(),
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
        types: [app.types, wire.types].concat(),
        schemas: [app.schemas, wire.schemas].concat(),
        codecs: Vec::new(),
        functions: [app.functions, wire.functions].concat(),
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
            target: CoreCallTarget::CodecDecode { function: name, codec },
            ..
        } if name == "decode_packet" && codec == "PacketCodec"
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
            target: IrCallTarget::CodecDecode { function: name, codec },
            ..
        } if name == "decode_packet" && codec == "PacketCodec"
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
            "codec MiddleReservedCodec for MiddleReservedHeader decode\n",
            "  derive decode\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  MiddleReservedCodec(view, base)\n",
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
            "codec ByteInterleavedMiddleReservedCodec for ByteInterleavedMiddleReservedHeader decode encode\n",
            "  derive decode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, middle: Int, low: Int}>\n",
            "  ByteInterleavedMiddleReservedCodec(view, base)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, middle: Int, low: Int}) -> EncodeStep<()>\n",
            "  ByteInterleavedMiddleReservedCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
            ..
        } if name == "ByteInterleavedMiddleReservedHeader"
    ));
}

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
            "codec TwoBytePrefixReservedCodec for TwoBytePrefixReservedGroupHeader decode encode\n",
            "  derive decode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  TwoBytePrefixReservedCodec(view, base)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> EncodeStep<()>\n",
            "  TwoBytePrefixReservedCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
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
            "codec ThreeBytePrefixReservedCodec for ThreeBytePrefixReservedGroupHeader decode encode\n",
            "  derive decode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  ThreeBytePrefixReservedCodec(view, base)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> EncodeStep<()>\n",
            "  ThreeBytePrefixReservedCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
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
            "codec FourBytePrefixReservedCodec for FourBytePrefixReservedGroupHeader decode encode\n",
            "  derive decode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  FourBytePrefixReservedCodec(view, base)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> EncodeStep<()>\n",
            "  FourBytePrefixReservedCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
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
            "codec FiveBytePrefixReservedCodec for FiveBytePrefixReservedGroupHeader decode encode\n",
            "  derive decode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  FiveBytePrefixReservedCodec(view, base)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> EncodeStep<()>\n",
            "  FiveBytePrefixReservedCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
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
            "codec SixBytePrefixReservedCodec for SixBytePrefixReservedGroupHeader decode encode\n",
            "  derive decode\n",
            "  derive encode\n",
            "end\n",
            "\n",
            "pub fn read_header(view: ByteView, base: ByteOffset) -> DecodeStep<{high: Int, low: Int}>\n",
            "  SixBytePrefixReservedCodec(view, base)\n",
            "end\n",
            "\n",
            "pub fn write_header(packet: {high: Int, low: Int}) -> EncodeStep<()>\n",
            "  SixBytePrefixReservedCodec(packet)\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            target: IrCallTarget::SchemaEncodeStep(name),
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
            "codec PacketCodec for PacketWire decode\n",
            "  derive decode\n",
            "end\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{kind: Int, payload: {code: Int, value: Int}}>\n",
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
            target: CoreCallTarget::CodecDecode { function: name, codec },
            ..
        } if name == "decode_packet" && codec == "PacketCodec"
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
fn imported_public_derived_codec_encode_resolves_through_qualified_module_path() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(packet: {length: Int}) -> EncodeStep<()>\n",
            "  wire::PacketCodec(packet)\n",
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
            "  length: UInt8\n",
            "end\n",
            "\n",
            "pub codec PacketCodec for PacketWire encode\n",
            "  derive encode\n",
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
            target: CoreCallTarget::SchemaEncodeStep(name),
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
            "view: ByteView, count: ByteCount, offset: ByteOffset, flags: Flag8, flags16: Flag16be, ",
            "flags16le: Flag16le, flags24: Flag24be, flags24le: Flag24le, ",
            "flags32: Flag32be, flags32le: Flag32le, ",
            "flags40: Flag40be, flags40le: Flag40le, ",
            "flags48: Flag48be, flags48le: Flag48le, ",
            "flags56: Flag56be, flags56le: Flag56le, ",
            "flags64: Flag64be, flags64le: Flag64le, ",
            "mapper: fn(Int) -> String, keep: fn(Int) -> Bool, folder: fn(String, Int) -> String, ",
            "fallible: fn(Int) -> Result<String, AppError>, opt: Option<Int>, ",
            "fallible_with: fn(String, Int) -> Result<String, AppError>, ",
            "opt_map: fn(Int) -> String, opt_next: fn(Int) -> Option<String>, ",
            "res: Result<Int, AppError>, err_map: fn(AppError) -> String, ",
            "res_next: fn(Int) -> Result<String, AppError>) -> {",
            "count: Int, empty: Bool, byte_value: Result<Byte, String>, byte_int: Int, ",
            "flag_is_set: Result<Bool, String>, flag_set: Result<Flag8, String>, ",
            "flag_bits: Int, flag_from_bits: Result<Flag8, String>, ",
            "flag16_is_set: Result<Bool, String>, flag16_set: Result<Flag16be, String>, ",
            "flag16_bits: Int, flag16_from_bits: Result<Flag16be, String>, ",
            "flag16le_is_set: Result<Bool, String>, flag16le_set: Result<Flag16le, String>, ",
            "flag16le_bits: Int, flag16le_from_bits: Result<Flag16le, String>, ",
            "flag24_is_set: Result<Bool, String>, flag24_set: Result<Flag24be, String>, ",
            "flag24_bits: Int, flag24_from_bits: Result<Flag24be, String>, ",
            "flag24le_is_set: Result<Bool, String>, flag24le_set: Result<Flag24le, String>, ",
            "flag24le_bits: Int, flag24le_from_bits: Result<Flag24le, String>, ",
            "flag32_is_set: Result<Bool, String>, flag32_set: Result<Flag32be, String>, ",
            "flag32_bits: Int, flag32_from_bits: Result<Flag32be, String>, ",
            "flag32le_is_set: Result<Bool, String>, flag32le_set: Result<Flag32le, String>, ",
            "flag32le_bits: Int, flag32le_from_bits: Result<Flag32le, String>, ",
            "flag40_is_set: Result<Bool, String>, flag40_set: Result<Flag40be, String>, ",
            "flag40_bits: Int, flag40_from_bits: Result<Flag40be, String>, ",
            "flag40le_is_set: Result<Bool, String>, flag40le_set: Result<Flag40le, String>, ",
            "flag40le_bits: Int, flag40le_from_bits: Result<Flag40le, String>, ",
            "flag48_is_set: Result<Bool, String>, flag48_set: Result<Flag48be, String>, ",
            "flag48_bits: Int, flag48_from_bits: Result<Flag48be, String>, ",
            "flag48le_is_set: Result<Bool, String>, flag48le_set: Result<Flag48le, String>, ",
            "flag48le_bits: Int, flag48le_from_bits: Result<Flag48le, String>, ",
            "flag56_is_set: Result<Bool, String>, flag56_set: Result<Flag56be, String>, ",
            "flag56_bits: Int, flag56_from_bits: Result<Flag56be, String>, ",
            "flag56le_is_set: Result<Bool, String>, flag56le_set: Result<Flag56le, String>, ",
            "flag56le_bits: Int, flag56le_from_bits: Result<Flag56le, String>, ",
            "flag64_is_set: Result<Bool, String>, flag64_set: Result<Flag64be, String>, ",
            "flag64_bits: Int, flag64_from_bits: Result<Flag64be, String>, ",
            "flag64le_is_set: Result<Bool, String>, flag64le_set: Result<Flag64le, String>, ",
            "flag64le_bits: Int, flag64le_from_bits: Result<Flag64le, String>, ",
            "chunk_value: ByteChunk, chunk_count: ByteCount, appended: ByteChunk, ",
            "hex_chunk: Result<ByteChunk, String>, ascii_text: Result<String, String>, ",
            "ascii_chunk: Result<ByteChunk, String>, ",
            "taken: Result<ByteChunk, String>, dropped: Result<ByteChunk, String>, ",
            "view_value: Result<ByteView, String>, view_chunk: ByteChunk, view_count: ByteCount, ",
            "view_taken: Result<ByteView, String>, view_dropped: Result<ByteView, String>, ",
            "view_slice: Result<ByteView, String>, empty_chunks: List<ByteChunk>, ",
            "one_chunk: List<ByteChunk>, appended_chunks: List<ByteChunk>, ",
            "read_u8: Result<Int, String>, expect_u8: Result<Int, String>, ",
            "decoded_frame: Result<{length: Int, kind: Int, flags: Int, stream_id: Int, payload: ByteView}, String>, ",
            "decoded_widths: Result<{short_value: Int, wide_value: Int}, String>, ",
            "decoded_validation: Result<{length: Int, padding_length: Int}, String>, ",
            "closed_http2: Result<(), String>, partial_preface_http2: Result<(), String>, ",
            "invalid_preface_http2: Result<(), String>, continuation_http2: Result<(), String>, ",
            "invalid_kind_http2: Result<(), String>, invalid_stream_http2: Result<(), String>, ",
            "invalid_payload_http2: Result<(), String>, invalid_window_update_increment_http2: Result<(), String>, ",
            "invalid_data_padding_http2: Result<(), String>, ",
            "invalid_request_headers_http2: Result<(), String>, unexpected_settings_ack_http2: Result<(), String>, ",
            "invalid_priority_dependency_http2: Result<(), String>, ",
            "stream_after_goaway_http2: Result<(), String>, ",
            "frame_size_http2: Result<(), String>, header_list_http2: Result<(), String>, ",
            "flow_control_http2: Result<(), String>, ",
            "concurrent_streams_http2: Result<(), String>, ",
            "settings_value_http2: Result<(), String>, ",
            "read_u16: Result<Int, String>, read_u24: Result<Int, String>, ",
            "read_u31: Result<Int, String>, read_u32: Result<Int, String>, ",
            "read_u16_le: Result<Int, String>, read_u24_le: Result<Int, String>, ",
            "read_u31_le: Result<Int, String>, read_u32_le: Result<Int, String>, ",
            "write_u8: Result<ByteChunk, String>, write_u16: Result<ByteChunk, String>, ",
            "write_u24: Result<ByteChunk, String>, write_u31: Result<ByteChunk, String>, ",
            "write_u32: Result<ByteChunk, String>, ",
            "write_u16_le: Result<ByteChunk, String>, write_u24_le: Result<ByteChunk, String>, ",
            "write_u31_le: Result<ByteChunk, String>, write_u32_le: Result<ByteChunk, String>, ",
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
            "flag_is_set: flag8_is_set(flags, 3), flag_set: flag8_set(flags, 5), ",
            "flag_bits: flag8_bits(flags), flag_from_bits: flag8_from_bits(40), ",
            "flag16_is_set: flag16be_is_set(flags16, 11), flag16_set: flag16be_set(flags16, 15), ",
            "flag16_bits: flag16be_bits(flags16), flag16_from_bits: flag16be_from_bits(32769), ",
            "flag16le_is_set: flag16le_is_set(flags16le, 11), flag16le_set: flag16le_set(flags16le, 15), ",
            "flag16le_bits: flag16le_bits(flags16le), flag16le_from_bits: flag16le_from_bits(32769), ",
            "flag24_is_set: flag24be_is_set(flags24, 23), flag24_set: flag24be_set(flags24, 0), ",
            "flag24_bits: flag24be_bits(flags24), flag24_from_bits: flag24be_from_bits(8388609), ",
            "flag24le_is_set: flag24le_is_set(flags24le, 23), flag24le_set: flag24le_set(flags24le, 0), ",
            "flag24le_bits: flag24le_bits(flags24le), flag24le_from_bits: flag24le_from_bits(8388609), ",
            "flag32_is_set: flag32be_is_set(flags32, 31), flag32_set: flag32be_set(flags32, 0), ",
            "flag32_bits: flag32be_bits(flags32), flag32_from_bits: flag32be_from_bits(2147483649), ",
            "flag32le_is_set: flag32le_is_set(flags32le, 31), flag32le_set: flag32le_set(flags32le, 0), ",
            "flag32le_bits: flag32le_bits(flags32le), flag32le_from_bits: flag32le_from_bits(2147483649), ",
            "flag40_is_set: flag40be_is_set(flags40, 39), flag40_set: flag40be_set(flags40, 0), ",
            "flag40_bits: flag40be_bits(flags40), flag40_from_bits: flag40be_from_bits(549755813889), ",
            "flag40le_is_set: flag40le_is_set(flags40le, 39), flag40le_set: flag40le_set(flags40le, 0), ",
            "flag40le_bits: flag40le_bits(flags40le), flag40le_from_bits: flag40le_from_bits(549755813889), ",
            "flag48_is_set: flag48be_is_set(flags48, 47), flag48_set: flag48be_set(flags48, 0), ",
            "flag48_bits: flag48be_bits(flags48), flag48_from_bits: flag48be_from_bits(140737488355329), ",
            "flag48le_is_set: flag48le_is_set(flags48le, 47), flag48le_set: flag48le_set(flags48le, 0), ",
            "flag48le_bits: flag48le_bits(flags48le), flag48le_from_bits: flag48le_from_bits(140737488355329), ",
            "flag56_is_set: flag56be_is_set(flags56, 55), flag56_set: flag56be_set(flags56, 0), ",
            "flag56_bits: flag56be_bits(flags56), flag56_from_bits: flag56be_from_bits(36028797018963969), ",
            "flag56le_is_set: flag56le_is_set(flags56le, 55), flag56le_set: flag56le_set(flags56le, 0), ",
            "flag56le_bits: flag56le_bits(flags56le), flag56le_from_bits: flag56le_from_bits(36028797018963969), ",
            "flag64_is_set: flag64be_is_set(flags64, 63), flag64_set: flag64be_set(flags64, 62), ",
            "flag64_bits: flag64be_bits(flags64), flag64_from_bits: flag64be_from_bits(4611686018427387904), ",
            "flag64le_is_set: flag64le_is_set(flags64le, 63), flag64le_set: flag64le_set(flags64le, 62), ",
            "flag64le_bits: flag64le_bits(flags64le), flag64le_from_bits: flag64le_from_bits(4611686018427387904), ",
            "chunk_value: byte_chunk([one_byte]), chunk_count: byte_chunk_count(chunk), ",
            "appended: byte_append(chunk, other_chunk), hex_chunk: byte_chunk_from_hex(\"00 ff\"), ",
            "ascii_text: byte_chunk_to_visible_ascii_string(chunk), ascii_chunk: byte_chunk_from_visible_ascii_string(\"A\"), ",
            "taken: byte_take(chunk, count), ",
            "dropped: byte_drop(chunk, count), view_value: byte_view(chunk, offset, count), ",
            "view_chunk: byte_view_to_chunk(view), view_count: byte_view_count(view), ",
            "view_taken: byte_view_take(view, count), view_dropped: byte_view_drop(view, count), ",
            "view_slice: byte_view_slice(view, count, count), empty_chunks: byte_chunks_empty(), ",
            "one_chunk: byte_chunks_one(chunk), appended_chunks: byte_chunks_append(byte_chunks_one(chunk), byte_chunks_one(other_chunk)), ",
            "read_u8: byte_read_u8_be(view), ",
            "expect_u8: byte_expect_fixed_u8_be(view, 1, \"DemoPacket\", \"kind\"), ",
            "decoded_frame: byte_decode_http2_frame(view), ",
            "decoded_widths: byte_decode_schema_width_sample(view), ",
            "decoded_validation: byte_decode_schema_validation_sample(view), ",
            "closed_http2: http2_protocol_closed_with_pending(0, 4, \"none\", view), ",
            "partial_preface_http2: http2_protocol_partial_preface(0, 6, view), ",
            "invalid_preface_http2: http2_protocol_invalid_preface(4, 42, 43, 4, view), ",
            "continuation_http2: http2_protocol_continuation_expected(9, 0, 1, 1, 1, 0, \"headers\", view), ",
            "invalid_kind_http2: http2_protocol_invalid_frame_kind(0, 0, 0, 4, \"connection-control\", \"connection_frames_require_settings\", view), ",
            "invalid_stream_http2: http2_protocol_invalid_stream_id(0, 1, 2, \"nonzero client-initiated stream id\", \"server\", \"stream-id-domain\", \"server_receives_client_initiated_streams\", view), ",
            "invalid_payload_http2: http2_protocol_invalid_payload_length(0, 6, 0, 7, 8, \"connection-control\", \"rfc9113_ping_payload_length\", view), ",
            "invalid_window_update_increment_http2: http2_protocol_invalid_window_update_increment(0, 0, 0, 1, 2147483647, \"connection-flow-control\", \"window_update_increment_nonzero\", view), ",
            "invalid_data_padding_http2: http2_protocol_invalid_data_padding(9, 1, 2, 0, \"open-stream\", \"rfc9113_data_padding\", view), ",
            "invalid_request_headers_http2: http2_protocol_invalid_request_header_list(12, 9, 1, \"missing_required_pseudo_header\", \":method\", \":scheme,:path\", \"request-headers\", \"rfc9113_request_pseudo_headers\"), ",
            "invalid_response_headers_http2: http2_protocol_invalid_response_header_list(12, 9, 1, \"missing_required_pseudo_header\", \":status\", \"server\", \"response-headers\", \"rfc9113_response_pseudo_headers\"), ",
            "unexpected_settings_ack_http2: http2_protocol_unexpected_settings_ack(0, \"connection-control\", \"rfc9113_settings_ack_requires_outstanding_local_settings\", view), ",
            "invalid_priority_dependency_http2: http2_protocol_invalid_priority_dependency(0, 1, 1, \"stream-control\", \"rfc9113_priority_dependency\", view), ",
            "stream_after_goaway_http2: http2_protocol_stream_after_goaway(9, 7, 5, \"graceful_shutdown\", \"server\", \"goaway_last_stream_id\"), ",
            "frame_size_http2: http2_peer_limit_frame_size_exceeded(0, 16385, 16384, 0, 3, \"protocol_default\"), ",
            "header_list_http2: http2_peer_limit_header_list_size_exceeded(12, 10, 9, 9, 1, \"local_configuration\", \"header_list_receive_limit\", view), ",
            "header_table_http2: http2_peer_limit_header_table_size_exceeded(35, 289, 160, 9, 1, \"local_configuration\", \"hpack_dynamic_table_size_update\", view), ",
            "flow_control_http2: http2_peer_limit_flow_control_window_exceeded(0, 4, 3, 0, 1, \"open-stream\", \"stream_receive_window\", view), ",
            "concurrent_streams_http2: http2_peer_limit_concurrent_streams_exceeded(9, 3, 2, 1, \"server\", \"open-stream\", \"local_configuration\", \"peer_created_stream_receive_limit\"), ",
            "settings_value_http2: http2_peer_limit_settings_value_out_of_range(9, 5, \"SETTINGS_MAX_FRAME_SIZE\", 16383, 16384, 16777215, \"peer_settings\", view), ",
            "hpack_fixture: hpack_fixture_unsupported_header_block(27, 1, 255, \"fixture header block\", \"hpack_fixture\", view), ",
            "hpack_string_length: hpack_fixture_malformed_string_length(27, 2, 4, \"fixture HPACK string length\", \"hpack_fixture\", view), ",
            "hpack_raw_string: hpack_fixture_malformed_raw_string_value(27, 5, 8, \"fixture HPACK raw string value\", \"hpack_fixture\", view), ",
            "hpack_padding: hpack_fixture_malformed_huffman_padding(27, 3, 4, \"fixture HPACK Huffman padding\", \"hpack_fixture\", view), ",
            "hpack_eos: hpack_fixture_huffman_eos_symbol(27, 6, 4, \"fixture HPACK Huffman data symbol instead of EOS\", \"hpack_fixture\", view), ",
            "hpack_visible: hpack_fixture_huffman_non_visible_value(27, 4, 4, \"fixture HPACK Huffman visible ASCII header value\", \"hpack_fixture\", view), ",
            "hpack_table_update_placement: hpack_fixture_table_size_update_not_at_start(10, 2, 62, 30, 1, 1, \"hpack-fixture\", \"fixture HPACK table-size update at header block start\", \"hpack_fixture\", view), ",
            "read_u16: byte_read_u16_be(view), read_u24: byte_read_u24_be(view), ",
            "read_u31: byte_read_u31_be(view), read_u32: byte_read_u32_be(view), ",
            "read_u40: byte_read_u40_be(view), read_u48: byte_read_u48_be(view), ",
            "read_u64: byte_read_u64_be(view), ",
            "read_u16_le: byte_read_u16_le(view), read_u24_le: byte_read_u24_le(view), ",
            "read_u31_le: byte_read_u31_le(view), read_u32_le: byte_read_u32_le(view), ",
            "read_u40_le: byte_read_u40_le(view), read_u48_le: byte_read_u48_le(view), ",
            "read_u64_le: byte_read_u64_le(view), ",
            "write_u8: byte_write_u8_be(1), write_u16: byte_write_u16_be(1), ",
            "write_u24: byte_write_u24_be(1), write_u31: byte_write_u31_be(1), ",
            "write_u32: byte_write_u32_be(1), write_u40: byte_write_u40_be(1), ",
            "write_u48: byte_write_u48_be(1), write_u64: byte_write_u64_be(1), ",
            "write_u16_le: byte_write_u16_le(1), ",
            "write_u24_le: byte_write_u24_le(1), write_u31_le: byte_write_u31_le(1), ",
            "write_u32_le: byte_write_u32_le(1), write_u40_le: byte_write_u40_le(1), ",
            "write_u48_le: byte_write_u48_le(1), write_u64_le: byte_write_u64_le(1), ",
            "count_value: byte_count(1), ",
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
            "    prelude::DecodeStep::Invalid(DecodeErrorWithReason(id, _, _, _)) => id\n",
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
            variadic: None,
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
            variadic: None,
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
