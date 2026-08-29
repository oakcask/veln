use super::*;

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
            "end\n",
            "\n",
            "pub fn direct(packet: {length: Int, padding_length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_padded_header(packet)\n",
            "end\n",
            "\n",
            "pub fn mapped(packet: {wire_length: Int, wire_padding_length: Int}) -> Result<ByteChunk, EncodeError>\n",
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
fn generated_schema_helpers_resolve_added_repeated_byte_view_lengths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema CountedViews\n",
            "  format binary\n",
            "\n",
            "  count: UInt8\n",
            "  left_length: UInt8\n",
            "  right_length: UInt8\n",
            "  items: Repeat(count, ByteView(left_length + right_length))\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{count: Int, left_length: Int, right_length: Int, items: List<ByteView>}, String>\n",
            "  byte_decode_counted_views(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {count: Int, left_length: Int, right_length: Int, items: List<ByteView>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_counted_views(packet)\n",
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
        .find(|schema| schema.schema_name == "CountedViews")
        .expect("counted schema should be emitted");
    let repeat = schema.fields[3]
        .repeat
        .as_ref()
        .expect("items should carry repeat metadata");
    assert_eq!(repeat.count_field, "count");
    assert_eq!(
        repeat.byte_view_length_field.as_deref(),
        Some("left_length + right_length")
    );
    assert_eq!(repeat.width, 0);
    assert!(repeat.payload_schema.is_none());
}

#[test]
fn generated_schema_helpers_resolve_subtracted_repeated_byte_view_lengths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema CountedViews\n",
            "  format binary\n",
            "\n",
            "  count: UInt8\n",
            "  total_length: UInt8\n",
            "  padding_length: UInt8\n",
            "  items: Repeat(count, ByteView(total_length - padding_length))\n",
            "end\n",
            "\n",
            "pub fn read(view: ByteView) -> Result<{count: Int, total_length: Int, padding_length: Int, items: List<ByteView>}, String>\n",
            "  byte_decode_counted_views(view)\n",
            "end\n",
            "\n",
            "pub fn write(packet: {count: Int, total_length: Int, padding_length: Int, items: List<ByteView>}) -> Result<ByteChunk, EncodeError>\n",
            "  byte_encode_counted_views(packet)\n",
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
        .find(|schema| schema.schema_name == "CountedViews")
        .expect("counted schema should be emitted");
    let repeat = schema.fields[3]
        .repeat
        .as_ref()
        .expect("items should carry repeat metadata");
    assert_eq!(repeat.count_field, "count");
    assert_eq!(
        repeat.byte_view_length_field.as_deref(),
        Some("total_length - padding_length")
    );
    assert_eq!(repeat.width, 0);
    assert!(repeat.payload_schema.is_none());
}
