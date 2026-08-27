use super::*;

#[test]
fn protocol_result_failure_diagnostic_projects_invalid_priority_dependency_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.invalid_priority_dependency"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(2)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        ("dependency_stream_id", JsonValue::Number(1)),
        ("byte_preview", byte_preview("000000010f")),
        ("active_state", JsonValue::string("stream-control")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_priority_dependency"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 invalid PRIORITY dependency at byte offset 0".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.invalid_priority_dependency");
    assert_eq!(
        diagnostic.message,
        "invalid PRIORITY dependency at byte offset 0"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("declared itself as dependency stream 1")
    );
    assert!(diagnostic.related[2].to_json().contains("stream-control"));
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("00 00 00 01 0f (showing 5 of 5 byte(s), complete)")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_priority_dependency")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_stream_after_goaway_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.stream_after_goaway"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("stream_id", JsonValue::Number(7)),
        ("stream_ref", JsonValue::string("stream")),
        ("last_stream_id", JsonValue::Number(5)),
        ("shutdown_state", JsonValue::string("graceful_shutdown")),
        ("endpoint_role", JsonValue::string("server")),
        (
            "byte_preview",
            byte_preview_with_counts("0000000104000000", 9, true),
        ),
        ("active_state", JsonValue::string("graceful_shutdown")),
        (
            "rule_provenance",
            JsonValue::string("goaway_last_stream_id"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 stream opened after graceful shutdown at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.stream_after_goaway");
    assert_eq!(
        diagnostic.message,
        "stream opened after graceful shutdown at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 6);
    assert!(diagnostic.related[0].to_json().contains("stream 7"));
    assert!(diagnostic.related[0].to_json().contains("last stream id 5"));
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("graceful_shutdown")
    );
    assert!(diagnostic.related[2].to_json().contains("server"));
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("00 00 00 01 04 00 00 00 (showing 8 of 9 byte(s), truncated)")
    );
    assert!(
        diagnostic.related[5]
            .to_json()
            .contains("goaway_last_stream_id")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_stream_invalid_frame_kind_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        ("id", JsonValue::string("http2.protocol.invalid_frame_kind")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("actual_frame_kind", JsonValue::Number(0)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        ("expected_frame_kind", JsonValue::Number(1)),
        (
            "byte_preview",
            byte_preview_with_counts("0000000000000000", 9, true),
        ),
        ("active_state", JsonValue::string("idle-stream")),
        (
            "rule_provenance",
            JsonValue::string("idle_streams_require_headers"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 invalid frame kind at byte offset 0".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.invalid_frame_kind");
    assert_eq!(diagnostic.message, "invalid frame kind at byte offset 0");
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("Frame kind 0 on stream 1")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("expected frame kind 1")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("00 00 00 00 00 00 00 00")
    );
    assert!(diagnostic.related[2].to_json().contains("idle-stream"));
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("idle_streams_require_headers")
    );
}
