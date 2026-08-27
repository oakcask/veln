use super::*;

#[test]
fn runtime_result_failure_diagnostic_falls_through_to_protocol_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.incomplete_input")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
    ]);
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        ("id", JsonValue::string("http2.protocol.partial_preface")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("pending_count", JsonValue::Number(6)),
        ("expected_count", JsonValue::Number(24)),
        ("byte_preview", byte_preview("505249202a20")),
        ("active_state", JsonValue::string("connection-preface")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_client_connection_preface"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "partial protocol input".to_string(),
        None,
        Some(byte_diagnostic),
        Some(protocol_diagnostic),
    );

    let diagnostic = runtime_result_failure_diagnostic(&failure)
        .expect("valid protocol context should project after incomplete byte context");

    assert_eq!(diagnostic.id, "http2.protocol.partial_preface");
    assert_eq!(
        diagnostic.message,
        "input ended with partial client connection preface at byte offset 0"
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("6 of 24 preface byte(s)")
    );
}

#[test]
fn runtime_result_failure_diagnostic_prefers_byte_context_over_protocol_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.incomplete_input")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(3)),
            ]),
        ),
        ("expected_count", JsonValue::Number(4)),
        ("available_count", JsonValue::Number(1)),
        ("readiness", JsonValue::string("need_bytes")),
    ]);
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        ("id", JsonValue::string("http2.protocol.partial_preface")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("pending_count", JsonValue::Number(6)),
        ("expected_count", JsonValue::Number(24)),
        ("byte_preview", byte_preview("505249202a20")),
        ("active_state", JsonValue::string("connection-preface")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_client_connection_preface"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "incomplete input".to_string(),
        None,
        Some(byte_diagnostic),
        Some(protocol_diagnostic),
    );

    let diagnostic = runtime_result_failure_diagnostic(&failure)
        .expect("specific byte context should project before protocol context");

    assert_eq!(diagnostic.id, "codec.incomplete_input");
    assert_eq!(diagnostic.message, "missing byte at byte offset 3");
}

#[test]
fn stderr_without_result_failure_line_keeps_user_stderr() {
    let failure = TestFailure::result_with_details("short input".to_string(), None, None, None);
    let stderr = b"user warning\nErr(short input)\n";

    assert_eq!(
        stderr_without_result_failure_line(stderr, &failure),
        b"user warning\n"
    );
}
