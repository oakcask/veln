use super::*;

#[test]
fn protocol_result_failure_diagnostic_projects_duplicate_request_pseudo_header() {
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
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("duplicate_pseudo_header"),
        ),
        ("header_name", JsonValue::string(":method")),
        (
            "decoded_header_names",
            JsonValue::string(":method,:method,:scheme,:path"),
        ),
        ("active_state", JsonValue::string("request-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_request_pseudo_headers"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 request header list contains duplicate :method at byte offset 12".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.message,
        "request header list contains duplicate :method at byte offset 12"
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains(":method,:method,:scheme,:path")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_request_pseudo_header_order() {
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
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("pseudo_header_after_regular_header"),
        ),
        ("header_name", JsonValue::string(":method")),
        (
            "decoded_header_names",
            JsonValue::string("host,:method,:scheme,:path"),
        ),
        ("active_state", JsonValue::string("request-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_request_pseudo_headers"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 request header list places :method after a regular header at byte offset 12"
            .to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.message,
        "request header list places :method after a regular header at byte offset 12"
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("host,:method,:scheme,:path")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_uppercase_request_header_name() {
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
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("ordinary_header_name_not_lowercase"),
        ),
        ("header_name", JsonValue::string("Host")),
        (
            "decoded_header_names",
            JsonValue::string(":method,:scheme,:path,Host"),
        ),
        ("active_state", JsonValue::string("request-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_field_name_lowercase"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 request header list contains uppercase ordinary header Host at byte offset 12"
            .to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.message,
        "request header list contains uppercase ordinary header Host at byte offset 12"
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains(":method,:scheme,:path,Host")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("rfc9113_field_name_lowercase")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_request_connection_specific_header() {
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
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("connection_specific_header"),
        ),
        ("header_name", JsonValue::string("connection")),
        (
            "decoded_header_names",
            JsonValue::string(":method,:scheme,:path,connection"),
        ),
        ("active_state", JsonValue::string("request-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_connection_specific_header_fields"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 request header list contains connection-specific header connection at byte offset 12"
            .to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.message,
        "request header list contains connection-specific header connection at byte offset 12"
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains(":method,:scheme,:path,connection")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("rfc9113_connection_specific_header_fields")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_request_te_header_value() {
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
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("te_header_value_not_trailers"),
        ),
        ("header_name", JsonValue::string("te")),
        (
            "decoded_header_names",
            JsonValue::string(":method,:scheme,:path,te"),
        ),
        ("active_state", JsonValue::string("request-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_te_trailers_only"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 request header list contains te value other than trailers at byte offset 12"
            .to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.message,
        "request header list contains te value other than trailers at byte offset 12"
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains(":method,:scheme,:path,te")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("rfc9113_te_trailers_only")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_request_scheme_value() {
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
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("scheme_value_not_http_or_https"),
        ),
        ("header_name", JsonValue::string(":scheme")),
        (
            "decoded_header_names",
            JsonValue::string(":method,:scheme,:path"),
        ),
        ("active_state", JsonValue::string("request-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_request_scheme"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 request header list contains :scheme value other than http or https at byte offset 12"
            .to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.message,
        "request header list contains :scheme value other than http or https at byte offset 12"
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains(":method,:scheme,:path")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("rfc9113_request_scheme")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_empty_request_path_value() {
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
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        ("failed_header_fact", JsonValue::string("path_value_empty")),
        ("header_name", JsonValue::string(":path")),
        (
            "decoded_header_names",
            JsonValue::string(":method,:scheme,:path"),
        ),
        ("active_state", JsonValue::string("request-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_request_pseudo_headers"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 request header list contains empty :path at byte offset 12".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.message,
        "request header list contains empty :path at byte offset 12"
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains(":method,:scheme,:path")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("rfc9113_request_pseudo_headers")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_empty_request_method_value() {
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
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("method_value_empty"),
        ),
        ("header_name", JsonValue::string(":method")),
        (
            "decoded_header_names",
            JsonValue::string(":method,:scheme,:path"),
        ),
        ("active_state", JsonValue::string("request-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_request_pseudo_headers"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 request header list contains empty :method at byte offset 12".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.message,
        "request header list contains empty :method at byte offset 12"
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains(":method,:scheme,:path")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("rfc9113_request_pseudo_headers")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_request_authority_value() {
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
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        (
            "failed_header_fact",
            JsonValue::string("authority_value_invalid"),
        ),
        ("header_name", JsonValue::string(":authority")),
        (
            "decoded_header_names",
            JsonValue::string(":method,:scheme,:path,:authority"),
        ),
        ("active_state", JsonValue::string("request-headers")),
        (
            "rule_provenance",
            JsonValue::string("rfc9113_request_authority"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "HTTP/2 request header list contains invalid :authority at byte offset 12".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.message,
        "request header list contains invalid :authority at byte offset 12"
    );
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains(":method,:scheme,:path,:authority")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("rfc9113_request_authority")
    );
}
