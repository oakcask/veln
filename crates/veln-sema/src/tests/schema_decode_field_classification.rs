use super::*;
use crate::semantic_model::Type;
use crate::types::schema_types::schema_decode_record_fields;

#[test]
fn mixed_binary_schema_fields_keep_their_decoded_record_shape() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Payload\n",
            "  format binary\n",
            "  value: UInt16be\n",
            "end\n",
            "\n",
            "schema Packet\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  length: UInt16be\n",
            "  reserved: ReservedBits(8, 0)\n",
            "  bytes: ByteView(length)\n",
            "  count: UInt8\n",
            "  values: Repeat(count, UInt16le)\n",
            "  header: {code: UInt8}\n",
            "  nested: Payload\n",
            "  choice: Dispatch(kind, 1 => UInt8)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);
    let packet = module
        .schemas
        .iter()
        .find(|schema| schema.name.as_deref() == Some("Packet"))
        .expect("packet schema should be lowered");

    let fields = schema_decode_record_fields(&module, packet)
        .expect("mixed schema should have a decoded record shape");

    assert_eq!(
        fields,
        vec![
            ("kind".to_string(), Type::int(), 1),
            ("length".to_string(), Type::int(), 2),
            ("bytes".to_string(), Type::named("ByteView", Vec::new()), 0),
            ("count".to_string(), Type::int(), 1),
            (
                "values".to_string(),
                Type::named("List", vec![Type::int()]),
                0,
            ),
            (
                "header".to_string(),
                Type::Record(vec![("code".to_string(), Type::int())]),
                0,
            ),
            (
                "nested".to_string(),
                Type::Record(vec![("value".to_string(), Type::int())]),
                0,
            ),
            ("choice".to_string(), Type::int(), 0),
        ]
    );
}

#[test]
fn dispatch_field_classification_preserves_payload_wrapping_and_rejects_mixed_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "schema Payload\n",
            "  format binary\n",
            "  code: UInt8\n",
            "end\n",
            "\n",
            "schema ClosedPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => UInt8, 2 => UInt16be)\n",
            "end\n",
            "\n",
            "schema ExtensionPacket\n",
            "  format binary\n",
            "  length: UInt8\n",
            "  kind: UInt8\n",
            "  payload: ExtensionDispatch(kind, length, 1 => UInt8)\n",
            "end\n",
            "\n",
            "schema MixedPacket\n",
            "  format binary\n",
            "  kind: UInt8\n",
            "  payload: Dispatch(kind, 1 => UInt8, 2 => Payload)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let closed = module
        .schemas
        .iter()
        .find(|schema| schema.name.as_deref() == Some("ClosedPacket"))
        .expect("closed schema should be lowered");
    let extension = module
        .schemas
        .iter()
        .find(|schema| schema.name.as_deref() == Some("ExtensionPacket"))
        .expect("extension schema should be lowered");
    let mixed = module
        .schemas
        .iter()
        .find(|schema| schema.name.as_deref() == Some("MixedPacket"))
        .expect("mixed schema should be lowered");

    assert_eq!(
        schema_decode_record_fields(&module, closed),
        Some(vec![
            ("kind".to_string(), Type::int(), 1),
            ("payload".to_string(), Type::int(), 0),
        ])
    );
    assert_eq!(
        schema_decode_record_fields(&module, extension),
        Some(vec![
            ("length".to_string(), Type::int(), 1),
            ("kind".to_string(), Type::int(), 1),
            (
                "payload".to_string(),
                Type::named("SchemaDispatchPayload", vec![Type::int()]),
                0,
            ),
        ])
    );
    assert_eq!(schema_decode_record_fields(&module, mixed), None);

    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "schema.dispatch_payload"
            && diagnostic
                .details
                .to_json()
                .contains("\"reason\":\"incompatible_payload_type\"")
    }));
}
