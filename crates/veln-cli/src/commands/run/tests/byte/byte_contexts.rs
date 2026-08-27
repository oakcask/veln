use super::*;

#[test]
fn byte_result_failure_diagnostic_projects_field_path_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.incomplete_input")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(7)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("DemoPacket")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("payload")),
                ]),
            ]),
        ),
        ("expected_count", JsonValue::Number(4)),
        ("available_count", JsonValue::Number(1)),
        ("readiness", JsonValue::string("need_bytes")),
    ]);
    let failure = TestFailure::result_with_details(
        "short input".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.incomplete_input");
    assert_eq!(diagnostic.kind, DiagnosticKind::Runtime);
    assert_eq!(diagnostic.message, "missing byte at byte offset 7");
    assert_eq!(diagnostic.related.len(), 3);
    assert!(diagnostic.related[0].to_json().contains("need_bytes"));
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("expected 4 byte(s); 1 byte(s) were available")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("schema `DemoPacket` / field `payload`")
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_range_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.byte_range_out_of_bounds")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(2)),
            ]),
        ),
        ("field_path", JsonValue::array([])),
        ("requested_count", JsonValue::Number(2)),
        ("available_count", JsonValue::Number(1)),
        ("byte_preview", byte_preview_with_counts("02", 1, false)),
    ]);
    let failure = TestFailure::result_with_details(
        "byte view range exceeds chunk length".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.byte_range_out_of_bounds");
    assert_eq!(
        diagnostic.message,
        "byte range out of bounds at byte offset 2"
    );
    assert_eq!(diagnostic.related.len(), 2);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("requested 2 byte(s); 1 byte(s) were available")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("02 (showing 1 of 1 byte(s), complete)")
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_decode_error_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.invalid_input")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(5)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("ManualPacketWire")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("kind")),
                ]),
            ]),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.kind"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeError(codec.invalid_input, ByteOffset(5), ManualPacketWire.kind)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.invalid_input");
    assert_eq!(diagnostic.message, "decode error at byte offset 5");
    assert_eq!(diagnostic.related.len(), 2);
    assert_eq!(
        diagnostic.related[0].to_json(),
        "{\"message\":\"Field path: schema `ManualPacketWire` / field `kind`.\"}"
    );
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"DecodeError value: DecodeError(codec.invalid_input, ByteOffset(5), ManualPacketWire.kind).\"}"
    );
}
