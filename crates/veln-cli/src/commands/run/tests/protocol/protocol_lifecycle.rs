use super::*;

#[test]
fn protocol_result_failure_diagnostic_projects_closed_input_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.closed_with_pending"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("pending_count", JsonValue::Number(4)),
        ("input_event", JsonValue::string("end")),
        ("active_continuation", JsonValue::string("none")),
        ("expected_stream_id", JsonValue::Number(0)),
        ("started_frame_kind", JsonValue::Number(0)),
        ("started_byte_offset", JsonValue::Number(0)),
        ("accumulated_header_block_bytes", JsonValue::Number(0)),
        ("rule_provenance", JsonValue::string("none")),
        ("byte_preview", byte_preview("01020304")),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 input ended with 4 pending byte(s) at byte offset 0".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.closed_with_pending");
    assert_eq!(
        diagnostic.message,
        "input ended with pending bytes at byte offset 0"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("4 byte(s) remained undecoded")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("01 02 03 04 (showing 4 of 4 byte(s), complete)")
    );
    assert!(diagnostic.related[2].to_json().contains("none"));
}

#[test]
fn protocol_result_failure_diagnostic_projects_partial_preface_context() {
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
        "HTTP/2 input ended with partial client connection preface at byte offset 0".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.partial_preface");
    assert_eq!(
        diagnostic.message,
        "input ended with partial client connection preface at byte offset 0"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("6 of 24 preface byte(s)")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("50 52 49 20 2a 20")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("showing 6 of 6 byte(s), complete")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("connection-preface")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_client_connection_preface")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_invalid_preface_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        ("id", JsonValue::string("http2.protocol.invalid_preface")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(4)),
            ]),
        ),
        ("expected_byte", JsonValue::Number(42)),
        ("actual_byte", JsonValue::Number(43)),
        ("matched_prefix_count", JsonValue::Number(4)),
        ("expected_count", JsonValue::Number(24)),
        ("byte_preview", byte_preview("505249202b")),
        ("active_state", JsonValue::string("connection-preface")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_client_connection_preface"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 invalid client connection preface at byte offset 4".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.invalid_preface");
    assert_eq!(
        diagnostic.message,
        "invalid client connection preface at byte offset 4"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("Observed byte 43; expected byte 42")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("4 of 24 preface byte(s)")
    );
    assert!(diagnostic.related[1].to_json().contains("50 52 49 20 2b"));
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("showing 5 of 5 byte(s), complete")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("connection-preface")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_client_connection_preface")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_continuation_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.continuation_expected"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("actual_frame_kind", JsonValue::Number(0)),
        ("actual_stream_id", JsonValue::Number(1)),
        ("expected_stream_id", JsonValue::Number(1)),
        ("started_frame_kind", JsonValue::Number(1)),
        ("started_byte_offset", JsonValue::Number(0)),
        ("active_continuation", JsonValue::string("headers")),
        ("accumulated_header_block_bytes", JsonValue::Number(3)),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_continuation_sequence"),
        ),
        (
            "byte_preview",
            byte_preview_with_counts("0000000000000000", 9, true),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 expected CONTINUATION frame at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.continuation_expected");
    assert_eq!(
        diagnostic.message,
        "expected CONTINUATION frame at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("frame kind 0 on stream 1")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("frame kind 1 at byte offset 0")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("00 00 00 00 00 00 00 00 (showing 8 of 9 byte(s), truncated)")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_continuation_sequence")
    );
}
