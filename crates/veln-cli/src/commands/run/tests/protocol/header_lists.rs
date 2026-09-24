use super::*;
use crate::commands::run::protocol_diagnostics::protocol_header_list_message;

const HEADER_LIST_MESSAGE_CASES: &[(&str, &str)] = &[
    (
        "protocol_on_non_connect_request",
        "request header list contains :protocol on a non-CONNECT request at byte offset 17",
    ),
    (
        "duplicate_protocol_pseudo_header",
        "request header list contains duplicate :protocol at byte offset 17",
    ),
    (
        "protocol_value_empty",
        "request header list contains empty :protocol at byte offset 17",
    ),
    (
        "extended_connect_scheme_missing",
        "request header list is missing required extended CONNECT :scheme at byte offset 17",
    ),
    (
        "extended_connect_path_missing",
        "request header list is missing required extended CONNECT :path at byte offset 17",
    ),
    (
        "extended_connect_authority_missing",
        "request header list is missing required extended CONNECT :authority at byte offset 17",
    ),
    (
        "extended_connect_not_negotiated",
        "request header list uses extended CONNECT before negotiation at byte offset 17",
    ),
    (
        "connect_authority_missing",
        "request header list is missing required CONNECT :authority at byte offset 17",
    ),
    (
        "connect_authority_empty",
        "request header list contains empty CONNECT :authority at byte offset 17",
    ),
    (
        "connect_scheme_present",
        "request header list contains forbidden CONNECT :scheme at byte offset 17",
    ),
    (
        "connect_path_present",
        "request header list contains forbidden CONNECT :path at byte offset 17",
    ),
    (
        "missing_required_pseudo_header",
        "request header list is missing :name at byte offset 17",
    ),
    (
        "response_only_pseudo_header",
        "request header list contains response-only :name at byte offset 17",
    ),
    (
        "request_only_pseudo_header",
        "request header list contains request-only :name at byte offset 17",
    ),
    (
        "duplicate_pseudo_header",
        "request header list contains duplicate :name at byte offset 17",
    ),
    (
        "trailer_pseudo_header",
        "request header list contains pseudo-header :name at byte offset 17",
    ),
    (
        "pseudo_header_after_regular_header",
        "request header list places :name after a regular header at byte offset 17",
    ),
    (
        "ordinary_header_name_not_lowercase",
        "request header list contains uppercase ordinary header :name at byte offset 17",
    ),
    (
        "ordinary_header_name_invalid_token",
        "request header list contains invalid ordinary header name :name at byte offset 17",
    ),
    (
        "connection_specific_header",
        "request header list contains connection-specific header :name at byte offset 17",
    ),
    (
        "te_header_value_not_trailers",
        "request header list contains te value other than trailers at byte offset 17",
    ),
    (
        "method_value_empty",
        "request header list contains empty :method at byte offset 17",
    ),
    (
        "scheme_value_not_http_or_https",
        "request header list contains :scheme value other than http or https at byte offset 17",
    ),
    (
        "path_value_empty",
        "request header list contains empty :path at byte offset 17",
    ),
    (
        "authority_value_invalid",
        "request header list contains invalid :authority at byte offset 17",
    ),
    (
        "content_length_invalid",
        "request header list contains invalid content-length at byte offset 17",
    ),
    (
        "content_length_mismatch",
        "request header list contains mismatched content-length values at byte offset 17",
    ),
    (
        "switching_protocols_status_forbidden",
        "request header list uses switching protocols status at byte offset 17",
    ),
    (
        "informational_response_end_stream",
        "informational response ended the stream at byte offset 17",
    ),
    (
        "unknown_fact",
        "invalid request header list at byte offset 17",
    ),
];

#[test]
fn protocol_header_list_messages_preserve_every_known_fact_and_fallback() {
    for (failed_fact, expected) in HEADER_LIST_MESSAGE_CASES {
        assert_eq!(
            protocol_header_list_message("request header list", failed_fact, ":name", 17),
            *expected,
            "failed fact {failed_fact}"
        );
    }
}

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
