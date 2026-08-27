use super::*;

#[test]
fn byte_result_failure_diagnostic_projects_truncated_preview_counts() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("schema.length_out_of_bounds")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(11)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("Http2FrameHeader")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("payload")),
                ]),
            ]),
        ),
        ("expected_count", JsonValue::Number(5)),
        ("available_count", JsonValue::Number(2)),
        (
            "byte_preview",
            byte_preview_with_counts("0000050104000000", 11, true),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "payload length out of bounds at byte offset 11".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("00 00 05 01 04 00 00 00 (showing 8 of 11 byte(s), truncated)")
    );
}

#[test]
fn byte_result_failure_diagnostic_keeps_empty_preview_counts() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("schema.truncated_field")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("EmptyPacket")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("kind")),
                ]),
            ]),
        ),
        ("expected_count", JsonValue::Number(1)),
        ("available_count", JsonValue::Number(0)),
        ("readiness", JsonValue::string("need_bytes")),
        ("byte_preview", byte_preview_with_counts("", 0, false)),
    ]);
    let failure = TestFailure::result_with_details(
        "truncated schema field at byte offset 0".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("<empty> (showing 0 of 0 byte(s), complete)")
    );
}
