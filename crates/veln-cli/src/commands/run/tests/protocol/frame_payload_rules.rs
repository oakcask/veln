use super::*;

#[test]
fn protocol_result_failure_diagnostic_projects_invalid_payload_length_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.invalid_payload_length"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(6)),
        ("stream_id", JsonValue::Number(0)),
        ("stream_ref", JsonValue::string("connection")),
        ("observed_payload_length", JsonValue::Number(7)),
        ("expected_payload_length", JsonValue::Number(8)),
        ("byte_preview", byte_preview("01020304050607")),
        ("active_state", JsonValue::string("connection-control")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_ping_payload_length"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 invalid payload length at byte offset 0".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.invalid_payload_length");
    assert_eq!(
        diagnostic.message,
        "invalid payload length at byte offset 0"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("Frame kind 6 on connection 0")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("expected 8 byte(s)")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("01 02 03 04 05 06 07 (showing 7 of 7 byte(s), complete)")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("connection-control")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_ping_payload_length")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_invalid_window_update_increment_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.invalid_window_update_increment"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(8)),
        ("stream_id", JsonValue::Number(0)),
        ("stream_ref", JsonValue::string("connection")),
        ("observed_window_increment", JsonValue::Number(0)),
        ("accepted_min_window_increment", JsonValue::Number(1)),
        (
            "accepted_max_window_increment",
            JsonValue::Number(2_147_483_647),
        ),
        ("byte_preview", byte_preview("00000000")),
        ("active_state", JsonValue::string("connection-flow-control")),
        (
            "rule_provenance",
            JsonValue::string("window_update_increment_nonzero"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 invalid WINDOW_UPDATE increment at byte offset 0".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.id,
        "http2.protocol.invalid_window_update_increment"
    );
    assert_eq!(
        diagnostic.message,
        "invalid WINDOW_UPDATE increment at byte offset 0"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("WINDOW_UPDATE increment 0")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("accepted range is 1..2147483647")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("00 00 00 00 (showing 4 of 4 byte(s), complete)")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("connection-flow-control")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("window_update_increment_nonzero")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_invalid_data_padding_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.invalid_data_padding"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(0)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        ("pad_length", JsonValue::Number(2)),
        ("remaining_payload_length", JsonValue::Number(0)),
        ("byte_preview", byte_preview("02")),
        ("active_state", JsonValue::string("open-stream")),
        ("rule_provenance", JsonValue::string("rfc9113_data_padding")),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 invalid DATA padding at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.invalid_data_padding");
    assert_eq!(diagnostic.message, "invalid DATA padding at byte offset 9");
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("pad length 2 byte(s)")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("remaining payload length is 0 byte(s)")
    );
    assert!(diagnostic.related[1].to_json().contains("02"));
    assert!(diagnostic.related[2].to_json().contains("open-stream"));
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_data_padding")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_content_length_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.content_length_mismatch"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(0)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        ("expected_content_length", JsonValue::Number(5)),
        ("observed_body_length", JsonValue::Number(3)),
        ("byte_preview", byte_preview("aabbcc")),
        ("active_state", JsonValue::string("open-stream")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_content_length_body"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 content-length body length mismatch at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.content_length_mismatch");
    assert_eq!(
        diagnostic.message,
        "content-length body length mismatch at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("observed 3 DATA application byte(s)")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("accepted content-length is 5 byte(s)")
    );
    assert!(diagnostic.related[1].to_json().contains("aa bb cc"));
    assert!(diagnostic.related[2].to_json().contains("open-stream"));
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_content_length_body")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_no_content_response_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.content_length_mismatch"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(0)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        ("expected_content_length", JsonValue::Number(0)),
        ("observed_body_length", JsonValue::Number(3)),
        ("byte_preview", byte_preview("aabbcc")),
        ("active_state", JsonValue::string("no-content-response-204")),
        (
            "rule_provenance",
            JsonValue::string("rfc9110_no_content_response_body"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 response status 204 prohibits nonempty DATA at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.content_length_mismatch");
    assert_eq!(
        diagnostic.message,
        "response status 204 prohibits nonempty DATA at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("response status 204 permits no application content")
    );
    assert!(diagnostic.related[1].to_json().contains("aa bb cc"));
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("no-content-response-204")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9110_no_content_response_body")
    );
}
