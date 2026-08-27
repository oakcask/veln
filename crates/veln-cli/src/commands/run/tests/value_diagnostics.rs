use super::*;

#[test]
fn value_result_failure_diagnostic_projects_byte_write_context() {
    let value_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("value_diagnostic")),
        (
            "id",
            JsonValue::string("codec.byte_write_value_unrepresentable"),
        ),
        ("field_path", JsonValue::array([])),
        ("helper_name", JsonValue::string("byte_write_u31_be")),
        ("supplied_value", JsonValue::Number(2147483648)),
        ("min_value", JsonValue::Number(0)),
        ("max_value", JsonValue::Number(2147483647)),
        ("width", JsonValue::Number(4)),
        ("byte_order", JsonValue::string("big_endian")),
    ]);
    let failure = TestFailure {
        kind: "result".to_string(),
        message:
            "runtime result failure: Err(byte_write_u31_be value must be between 0 and 2147483647)"
                .to_string(),
        details: JsonValue::object([
            ("kind", JsonValue::string("result")),
            ("phase", JsonValue::string("runtime")),
            (
                "value",
                JsonValue::string("byte_write_u31_be value must be between 0 and 2147483647"),
            ),
            ("value_diagnostic", value_diagnostic),
        ]),
    };

    let diagnostic =
        value_result_failure_diagnostic(&failure).expect("value diagnostic should project");

    assert_eq!(diagnostic.id, "codec.byte_write_value_unrepresentable");
    assert_eq!(diagnostic.kind, DiagnosticKind::Runtime);
    assert_eq!(diagnostic.message, "byte write value is unrepresentable");
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("`byte_write_u31_be` received value 2147483648")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("Accepted range is 0..2147483647")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("4 byte(s) with `big_endian` byte order")
    );
}

#[test]
fn value_result_failure_diagnostic_projects_byte_view_encode_context() {
    let value_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("value_diagnostic")),
        (
            "id",
            JsonValue::string("schema.encode_value_unrepresentable"),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("PacketWire")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("payload")),
                ]),
            ]),
        ),
        (
            "field_path_display",
            JsonValue::string("PacketWire.payload"),
        ),
        (
            "reason",
            JsonValue::string("byte view count 3 does not match length field `length` value 2"),
        ),
        ("expected_count", JsonValue::Number(2)),
        ("actual_count", JsonValue::Number(3)),
        ("length_expression", JsonValue::string("length")),
        ("byte_offset", JsonValue::Number(0)),
        ("byte_preview", byte_preview_with_counts("aabbcc", 3, false)),
    ]);
    let failure = TestFailure {
        kind: "result".to_string(),
        message: "runtime result failure: Err(EncodeError(schema.encode_value_unrepresentable, PacketWire.payload, byte view count 3 does not match length field `length` value 2))".to_string(),
        details: JsonValue::object([
            ("kind", JsonValue::string("result")),
            ("phase", JsonValue::string("runtime")),
            (
                "value",
                JsonValue::string("EncodeError(schema.encode_value_unrepresentable, PacketWire.payload, byte view count 3 does not match length field `length` value 2)"),
            ),
            ("value_diagnostic", value_diagnostic),
        ]),
    };

    let diagnostic =
        value_result_failure_diagnostic(&failure).expect("value diagnostic should project");

    assert_eq!(diagnostic.id, "schema.encode_value_unrepresentable");
    assert_eq!(diagnostic.message, "encode value is unrepresentable");
    assert_eq!(diagnostic.related.len(), 6);
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("Expected 2 byte(s); supplied ByteView has 3 byte(s)")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("Supplied ByteView starts at byte offset 0")
    );
    assert!(
        diagnostic.related[4]
            .to_json()
            .contains("aa bb cc (showing 3 of 3 byte(s), complete)")
    );
}
