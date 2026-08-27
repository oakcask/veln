use super::*;

#[test]
fn protocol_result_failure_diagnostic_projects_frame_size_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.peer_limit.frame_size_exceeded"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("observed_payload_length", JsonValue::Number(16385)),
        ("allowed_max_frame_size", JsonValue::Number(16384)),
        ("frame_kind", JsonValue::Number(0)),
        ("stream_id", JsonValue::Number(3)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "receive_limit_provenance",
            JsonValue::string("protocol_default"),
        ),
        (
            "byte_preview",
            byte_preview_with_counts("0000000000000000", 9, true),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 frame payload length exceeds receive maximum at byte offset 0".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.peer_limit.frame_size_exceeded");
    assert_eq!(
        diagnostic.message,
        "frame payload length exceeds receive maximum at byte offset 0"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("declared 16385 byte(s)")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("active receive maximum is 16384 byte(s)")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("showing 8 of 9 byte(s), truncated")
    );
    assert!(diagnostic.related[2].to_json().contains("protocol_default"));
}

#[test]
fn protocol_result_failure_diagnostic_projects_header_list_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.peer_limit.header_list_size_exceeded"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(12)),
            ]),
        ),
        ("observed_header_list_size", JsonValue::Number(10)),
        ("allowed_header_list_size", JsonValue::Number(9)),
        ("frame_kind", JsonValue::Number(9)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "receive_limit_provenance",
            JsonValue::string("local_configuration"),
        ),
        (
            "rule_provenance",
            JsonValue::string("header_list_receive_limit"),
        ),
        (
            "byte_preview",
            byte_preview_with_counts("060708090a0b0c0d", 9, true),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 header list size exceeds receive maximum at byte offset 12".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.peer_limit.header_list_size_exceeded");
    assert_eq!(
        diagnostic.message,
        "header list size exceeds receive maximum at byte offset 12"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("decoded header list size 10")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("active receive maximum is 9")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("local_configuration")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("header_list_receive_limit")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("06 07 08 09 0a 0b 0c 0d (showing 8 of 9 byte(s), truncated)")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_header_table_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.peer_limit.header_table_size_exceeded"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(35)),
            ]),
        ),
        ("observed_header_table_size", JsonValue::Number(289)),
        ("allowed_header_table_size", JsonValue::Number(160)),
        ("frame_kind", JsonValue::Number(9)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "receive_limit_provenance",
            JsonValue::string("local_configuration"),
        ),
        (
            "rule_provenance",
            JsonValue::string("hpack_dynamic_table_size_update"),
        ),
        ("byte_preview", byte_preview("3f8101")),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 header table size exceeds receive maximum at byte offset 35".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.peer_limit.header_table_size_exceeded");
    assert_eq!(
        diagnostic.message,
        "header table size exceeds receive maximum at byte offset 35"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("requested HPACK header table size 289")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("active receive maximum is 160")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("local_configuration")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("hpack_dynamic_table_size_update")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("3f 81 01 (showing 3 of 3 byte(s), complete)")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_flow_control_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.peer_limit.flow_control_window_exceeded"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("observed_payload_length", JsonValue::Number(4)),
        ("allowed_window_credit", JsonValue::Number(3)),
        ("frame_kind", JsonValue::Number(0)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        ("active_state", JsonValue::string("open-stream")),
        (
            "rule_provenance",
            JsonValue::string("stream_receive_window"),
        ),
        ("byte_preview", byte_preview("01020304")),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 flow-control window exceeded at byte offset 0".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.id,
        "http2.peer_limit.flow_control_window_exceeded"
    );
    assert_eq!(
        diagnostic.message,
        "flow-control window exceeded at byte offset 0"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("declared 4 byte(s)")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("available receive window credit is 3 byte(s)")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("01 02 03 04 (showing 4 of 4 byte(s), complete)")
    );
    assert!(diagnostic.related[2].to_json().contains("open-stream"));
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("stream_receive_window")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_concurrent_stream_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.peer_limit.concurrent_streams_exceeded"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("stream_id", JsonValue::Number(3)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "current_open_peer_created_stream_count",
            JsonValue::Number(1),
        ),
        ("attempted_concurrent_stream_count", JsonValue::Number(2)),
        ("allowed_concurrent_stream_count", JsonValue::Number(1)),
        ("endpoint_role", JsonValue::string("server")),
        ("active_state", JsonValue::string("open-stream")),
        (
            "receive_limit_provenance",
            JsonValue::string("local_configuration"),
        ),
        (
            "rule_provenance",
            JsonValue::string("peer_created_stream_receive_limit"),
        ),
        (
            "byte_preview",
            byte_preview_with_counts("0000000104000000", 9, true),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 concurrent stream receive limit exceeded at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.id,
        "http2.peer_limit.concurrent_streams_exceeded"
    );
    assert_eq!(
        diagnostic.message,
        "concurrent stream receive limit exceeded at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 6);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("make 2 concurrent peer-created stream(s)")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("1 peer-created stream(s) are currently open")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("active receive limit is 1")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("00 00 00 01 04 00 00 00 (showing 8 of 9 byte(s), truncated)")
    );
    assert!(diagnostic.related[2].to_json().contains("open-stream"));
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("Endpoint role: server")
    );
    assert!(
        diagnostic.related[4]
            .to_json()
            .contains("local_configuration")
    );
    assert!(
        diagnostic.related[5]
            .to_json()
            .contains("peer_created_stream_receive_limit")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_settings_value_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.peer_limit.settings_value_out_of_range"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("setting_identifier", JsonValue::Number(5)),
        ("setting_name", JsonValue::string("SETTINGS_MAX_FRAME_SIZE")),
        ("observed_value", JsonValue::Number(16383)),
        ("accepted_min_value", JsonValue::Number(16384)),
        ("accepted_max_value", JsonValue::Number(16777215)),
        ("peer_limit_provenance", JsonValue::string("peer_settings")),
        ("byte_preview", byte_preview("000500003fff")),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 SETTINGS value outside accepted range at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.id,
        "http2.peer_limit.settings_value_out_of_range"
    );
    assert_eq!(
        diagnostic.message,
        "SETTINGS value outside accepted range at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("SETTINGS_MAX_FRAME_SIZE (5)")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("accepted range is 16384..16777215")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("00 05 00 00 3f ff")
    );
    assert!(diagnostic.related[2].to_json().contains("peer_settings"));
}

#[test]
fn protocol_result_failure_diagnostic_projects_request_header_list_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.invalid_request_header_list"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(12)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(9)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("missing_required_pseudo_header"),
        ),
        ("header_name", JsonValue::string(":method")),
        ("decoded_header_names", JsonValue::string(":scheme,:path")),
        ("byte_preview", byte_preview("828486")),
        ("active_state", JsonValue::string("request-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_request_pseudo_headers"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 request header list is missing :method at byte offset 12".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.invalid_request_header_list");
    assert_eq!(
        diagnostic.message,
        "request header list is missing :method at byte offset 12"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("Frame kind 9 on stream 1")
    );
    assert!(diagnostic.related[0].to_json().contains(":scheme,:path"));
    assert!(diagnostic.related[1].to_json().contains("82 84 86"));
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_request_pseudo_headers")
    );
}
