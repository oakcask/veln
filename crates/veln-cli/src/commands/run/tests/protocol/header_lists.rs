use super::*;

#[test]
fn protocol_result_failure_diagnostic_projects_response_header_list_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.invalid_response_header_list"),
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
        ("header_name", JsonValue::string(":status")),
        ("decoded_header_names", JsonValue::string("server")),
        ("byte_preview", byte_preview("88")),
        ("active_state", JsonValue::string("response-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_response_pseudo_headers"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 response header list is missing :status at byte offset 12".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.invalid_response_header_list");
    assert_eq!(
        diagnostic.message,
        "response header list is missing :status at byte offset 12"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("Frame kind 9 on stream 1")
    );
    assert!(diagnostic.related[0].to_json().contains("server"));
    assert!(diagnostic.related[1].to_json().contains("88"));
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_response_pseudo_headers")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_response_trailer_list_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.invalid_response_header_list"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(12)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("trailer_pseudo_header"),
        ),
        ("header_name", JsonValue::string(":status")),
        ("decoded_header_names", JsonValue::string(":status")),
        ("byte_preview", byte_preview("88")),
        ("active_state", JsonValue::string("response-trailers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_trailer_pseudo_headers"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 response trailer list contains pseudo-header :status at byte offset 12".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "http2.protocol.invalid_response_header_list");
    assert_eq!(
        diagnostic.message,
        "response trailer list contains pseudo-header :status at byte offset 12"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("decoded response trailer names: :status")
    );
    assert!(diagnostic.related[1].to_json().contains("88"));
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("response-trailers")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("rfc9113_trailer_pseudo_headers")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_response_ordinary_header_name_facts() {
    let uppercase_protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.invalid_response_header_list"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(12)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("ordinary_header_name_not_lowercase"),
        ),
        ("header_name", JsonValue::string("Server")),
        ("decoded_header_names", JsonValue::string(":status,Server")),
        ("active_state", JsonValue::string("response-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_field_name_lowercase"),
        ),
    ]);
    let uppercase_failure = TestFailure::result_with_details(
        "HTTP/2 response header list contains uppercase ordinary header Server at byte offset 12"
            .to_string(),
        None,
        None,
        Some(uppercase_protocol_diagnostic),
    );

    let uppercase_diagnostic = protocol_result_failure_diagnostic(&uppercase_failure)
        .expect("protocol diagnostic should project");

    assert_eq!(
        uppercase_diagnostic.message,
        "response header list contains uppercase ordinary header Server at byte offset 12"
    );
    assert!(
        uppercase_diagnostic.related[0]
            .to_json()
            .contains(":status,Server")
    );
    assert!(
        uppercase_diagnostic.related[2]
            .to_json()
            .contains("rfc9113_field_name_lowercase")
    );

    let token_protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.invalid_response_header_list"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(12)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("ordinary_header_name_invalid_token"),
        ),
        ("header_name", JsonValue::string("bad header")),
        (
            "decoded_header_names",
            JsonValue::string(":status,bad header"),
        ),
        ("active_state", JsonValue::string("response-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9110_field_name_token"),
        ),
    ]);
    let token_failure = TestFailure::result_with_details(
        "HTTP/2 response header list contains invalid ordinary header name bad header at byte offset 12"
            .to_string(),
        None,
        None,
        Some(token_protocol_diagnostic),
    );

    let token_diagnostic = protocol_result_failure_diagnostic(&token_failure)
        .expect("protocol diagnostic should project");

    assert_eq!(
        token_diagnostic.message,
        "response header list contains invalid ordinary header name bad header at byte offset 12"
    );
    assert!(
        token_diagnostic.related[2]
            .to_json()
            .contains("rfc9110_field_name_token")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_response_te_header_value() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("http2.protocol.invalid_response_header_list"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(12)),
            ]),
        ),
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("te_header_value_not_trailers"),
        ),
        ("header_name", JsonValue::string("te")),
        ("decoded_header_names", JsonValue::string(":status,te")),
        ("active_state", JsonValue::string("response-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_te_trailers_only"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 response header list contains te value other than trailers at byte offset 12"
            .to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.message,
        "response header list contains te value other than trailers at byte offset 12"
    );
    assert!(diagnostic.related[0].to_json().contains(":status,te"));
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("rfc9113_te_trailers_only")
    );
}
