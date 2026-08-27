use super::*;

#[test]
fn protocol_result_failure_diagnostic_projects_invalid_frame_kind_context() {
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
        ("stream_id", JsonValue::Number(0)),
        ("stream_ref", JsonValue::string("connection")),
        ("expected_frame_kind", JsonValue::Number(4)),
        (
            "byte_preview",
            byte_preview_with_counts("0000000000000000", 9, true),
        ),
        ("active_state", JsonValue::string("connection-control")),
        (
            "rule_provenance",
            JsonValue::string("connection_frames_require_settings"),
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
            .contains("Frame kind 0 on connection 0")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("expected frame kind 4")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("00 00 00 00 00 00 00 00")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("connection-control")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("connection_frames_require_settings")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_initial_peer_settings_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.initial_peer_settings_required"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(24)),
            ]),
        ),
        ("actual_frame_kind", JsonValue::Number(6)),
        ("actual_flags", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(0)),
        ("stream_ref", JsonValue::string("connection")),
        ("endpoint_role", JsonValue::string("server")),
        (
            "byte_preview",
            byte_preview_with_counts("0000000601000000", 9, true),
        ),
        (
            "active_state",
            JsonValue::string("expect-initial-peer-settings"),
        ),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_initial_peer_frame_requires_non_ack_settings"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 initial peer frame must be non-ACK SETTINGS at byte offset 24".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic = protocol_result_failure_diagnostic(&failure)
        .expect("initial peer SETTINGS diagnostic should project");

    assert_eq!(
        diagnostic.id,
        "http2.protocol.initial_peer_settings_required"
    );
    assert_eq!(
        diagnostic.message,
        "initial peer frame must be non-ACK SETTINGS at byte offset 24"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(diagnostic.related[0].to_json().contains("flags 1"));
    assert!(diagnostic.related[0].to_json().contains("server endpoint"));
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("expect-initial-peer-settings")
    );
}

#[test]
fn protocol_result_failure_diagnostic_rejects_incomplete_frame_identity_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.initial_peer_settings_required"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(24)),
            ]),
        ),
        ("actual_frame_kind", JsonValue::Number(6)),
        ("actual_flags", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(0)),
        ("stream_ref", JsonValue::string("connection")),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 initial peer frame context is incomplete".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    assert!(protocol_result_failure_diagnostic(&failure).is_none());
}

#[test]
fn protocol_result_failure_diagnostic_projects_invalid_stream_id_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        ("id", JsonValue::string("http2.protocol.invalid_stream_id")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(2)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "required_stream_id_domain",
            JsonValue::string("nonzero client-initiated stream id"),
        ),
        ("endpoint_role", JsonValue::string("server")),
        (
            "byte_preview",
            byte_preview_with_counts("0000000104000000", 9, true),
        ),
        ("active_state", JsonValue::string("stream-id-domain")),
        (
            "rule_provenance",
            JsonValue::string("server_receives_client_initiated_streams"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 invalid stream id at byte offset 0".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.invalid_stream_id");
    assert_eq!(diagnostic.message, "invalid stream id at byte offset 0");
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("Frame kind 1 on stream 2")
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("nonzero client-initiated stream id")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("00 00 00 01 04 00 00 00")
    );
    assert!(diagnostic.related[2].to_json().contains("stream-id-domain"));
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("server_receives_client_initiated_streams")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_peer_stream_ordering_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.peer_stream_id_not_increasing"),
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
        ("previous_peer_stream_id", JsonValue::Number(5)),
        ("endpoint_role", JsonValue::string("server")),
        (
            "byte_preview",
            byte_preview_with_counts("0000000104000000", 9, true),
        ),
        ("active_state", JsonValue::string("idle-stream")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_peer_stream_ids_increase"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 peer-created stream id is not increasing".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.id,
        "http2.protocol.peer_stream_id_not_increasing"
    );
    assert_eq!(
        diagnostic.message,
        "peer-created stream id 3 is not greater than 5 at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 5);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("server endpoint attempted to create idle stream 3")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("00 00 00 01 04 00 00 00")
    );
    assert!(diagnostic.related[2].to_json().contains("idle-stream"));
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_peer_stream_ids_increase")
    );
    assert!(diagnostic.related[4].to_json().contains("greater than 5"));
}

#[test]
fn protocol_result_failure_diagnostic_projects_unexpected_settings_ack_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.unexpected_settings_ack"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(4)),
        ("stream_id", JsonValue::Number(0)),
        ("stream_ref", JsonValue::string("connection")),
        (
            "byte_preview",
            byte_preview_with_counts("0000000401000000", 9, true),
        ),
        ("active_state", JsonValue::string("connection-control")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_settings_ack_requires_outstanding_local_settings"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 unexpected SETTINGS ACK at byte offset 0".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.unexpected_settings_ack");
    assert_eq!(
        diagnostic.message,
        "unexpected SETTINGS ACK at byte offset 0"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("no local SETTINGS batch is outstanding")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("00 00 00 04 01 00 00 00")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("connection-control")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_settings_ack_requires_outstanding_local_settings")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_settings_endpoint_role_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.settings_not_allowed_for_endpoint"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(15)),
            ]),
        ),
        ("setting_identifier", JsonValue::Number(2)),
        ("setting_name", JsonValue::string("SETTINGS_ENABLE_PUSH")),
        ("endpoint_role", JsonValue::string("client")),
        ("frame_kind", JsonValue::Number(4)),
        ("stream_id", JsonValue::Number(0)),
        ("stream_ref", JsonValue::string("connection")),
        ("byte_preview", byte_preview("000200000001")),
        ("active_state", JsonValue::string("peer-settings")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_client_must_not_receive_settings_enable_push"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 SETTINGS item is not allowed for endpoint role at byte offset 15".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.id,
        "http2.protocol.settings_not_allowed_for_endpoint"
    );
    assert_eq!(
        diagnostic.message,
        "SETTINGS_ENABLE_PUSH is not allowed for client endpoints at byte offset 15"
    );
    assert_eq!(diagnostic.related.len(), 5);
    assert!(diagnostic.related[0].to_json().contains("(2)"));
    assert!(diagnostic.related[0].to_json().contains("frame kind 4"));
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("00 02 00 00 00 01")
    );
    assert!(diagnostic.related[2].to_json().contains("client"));
    assert!(diagnostic.related[3].to_json().contains("peer-settings"));
    assert!(
        diagnostic.related[4]
            .to_json()
            .contains("rfc9113_client_must_not_receive_settings_enable_push")
    );
}
