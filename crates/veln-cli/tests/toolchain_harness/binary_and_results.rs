use super::*;

#[test]
pub(super) fn binary_fixture_schema_references_resolve_from_command_sources() {
    let root = test_temp_root("fixture-schema-references");
    write_fixture_schema_sources(&root);
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln", "wire.veln", "facade.veln"]
exit = 0

[[binary_fixture]]
name = "local-private"
schema = "LocalPacket"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"LocalPacket"}]

[[binary_fixture]]
name = "imported-public"
schema = "wire::PublicPacket"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"PublicPacket"}]

[[binary_fixture]]
name = "imported-alias"
schema = "facade::AliasPacket"
hex = "00"
diagnostic_id = "schema.invalid_field_value"
byte_offset = 0
field_path = [{"kind":"schema","name":"PublicPacket"}]
"#,
    );

    manifest.validate_fixture_schema_references(&root);
    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn binary_fixture_schema_references_reject_wrong_targets() {
    assert_fixture_schema_error(
        "MissingPacket",
        Some(r#"[{"kind":"schema","name":"MissingPacket"}]"#),
        "unresolved binary_fixture 0 schema reference `MissingPacket`",
    );
    assert_fixture_schema_error(
        "PrivatePacket",
        Some(r#"[{"kind":"schema","name":"PrivatePacket"}]"#),
        "unresolved binary_fixture 0 schema reference `PrivatePacket`",
    );
    assert_fixture_schema_error(
        "wire::PrivatePacket",
        Some(r#"[{"kind":"schema","name":"PrivatePacket"}]"#),
        "binary_fixture 0 schema reference `wire::PrivatePacket` is private",
    );
    assert_fixture_schema_error(
        "wire::make_packet",
        Some(r#"[{"kind":"schema","name":"make_packet"}]"#),
        "binary_fixture 0 schema reference `wire::make_packet` is a function, not a schema",
    );
    assert_fixture_schema_error(
        "wire::PacketShape",
        Some(r#"[{"kind":"schema","name":"PacketShape"}]"#),
        "binary_fixture 0 schema reference `wire::PacketShape` is a type, not a schema",
    );
    assert_fixture_schema_error(
        "wire::PacketCodec",
        Some(r#"[{"kind":"schema","name":"PacketCodec"}]"#),
        "unresolved binary_fixture 0 schema reference `wire::PacketCodec`",
    );
    assert_fixture_schema_error(
        "wire::byte_decode_public_packet",
        Some(r#"[{"kind":"schema","name":"PublicPacket"}]"#),
        "unresolved binary_fixture 0 schema reference `wire::byte_decode_public_packet`",
    );
    assert_fixture_schema_error(
        "other::PublicPacket",
        Some(r#"[{"kind":"schema","name":"PublicPacket"}]"#),
        "unresolved binary_fixture 0 schema reference `other::PublicPacket`",
    );
    assert_fixture_schema_error(
        "wire::PublicPacket",
        Some(r#"[{"kind":"schema","name":"OtherPacket"}]"#),
        "binary_fixture 0 `field_path` first segment must name schema `PublicPacket`",
    );
}

#[test]
pub(super) fn manifest_result_value_assertions_parse_paths() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 1

[[result_value_assert]]
value_path = "error.details.value"
path = "value.id"
equals = "codec.incomplete_input"

[[result_value_assert]]
value_path = "error.details.value"
path = "value.detail.preview"
missing = true
"#,
    );

    assert!(manifest.expectations.needs_stdout_json());
    let assertions = &manifest.expectations.result_value_assertions;
    assert_eq!(assertions.len(), 2);
    assert_eq!(assertions[0].value_path, "error.details.value");
    assert_eq!(assertions[0].path, "value.id");
    assert_eq!(
        assertions[0].operation,
        Some(ValueAssertionOperation::Equals(JsonValue::String(
            "codec.incomplete_input".to_string()
        )))
    );
    assert_eq!(
        assertions[1].operation,
        Some(ValueAssertionOperation::Missing)
    );
}

#[test]
pub(super) fn result_value_parser_exposes_runtime_diagnostic_shape() {
    let parsed = parse_result_value(
        "RuntimeDiagnostic(codec.incomplete_input, byte read requires 3 bytes but view has 2, RuntimeByteDiagnostic(ByteOffset(2), Cons(RuntimeDiagnosticFieldPathSegment(schema, Payload), Cons(RuntimeDiagnosticFieldPathSegment(field, body), Nil)), RuntimeByteCountFacts(ByteCount(3), ByteCount(2), need_bytes), RuntimeBytePreview(0001, ByteCount(2), ByteCount(2), false)))",
    )
    .expect("runtime diagnostic value should parse");

    assert_eq!(
        json_path(&parsed, "constructor"),
        Some(&JsonValue::String("Err".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.constructor"),
        Some(&JsonValue::String("RuntimeDiagnostic".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.field_path.1.name"),
        Some(&JsonValue::String("body".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.facts.expected_count.value"),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.preview.truncated"),
        Some(&JsonValue::Bool(false))
    );
}

#[test]
pub(super) fn result_value_parser_exposes_runtime_value_diagnostic_shape() {
    let parsed = parse_result_value(
        "RuntimeDiagnostic(schema.encode_value_unrepresentable, encode value is unrepresentable, RuntimeValueDiagnostic(Cons(RuntimeDiagnosticFieldPathSegment(schema, RuntimeValuePacket), Cons(RuntimeDiagnosticFieldPathSegment(field, value), Nil)), value must be between 0 and 255))",
    )
    .expect("runtime value diagnostic should parse");

    assert_eq!(
        json_path(&parsed, "value.detail.constructor"),
        Some(&JsonValue::String("RuntimeValueDiagnostic".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.field_path.1.name"),
        Some(&JsonValue::String("value".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "value.detail.reason"),
        Some(&JsonValue::String(
            "value must be between 0 and 255".to_string()
        ))
    );
}

#[test]
pub(super) fn veln_value_parser_preserves_constructor_field_kinds() {
    let parsed = parse_veln_value(
        "RuntimeByteDiagnostic(ByteOffset(4), Cons(RuntimeDiagnosticFieldPathSegment(schema, Packet), Nil), RuntimeByteReasonFacts(invalid byte), RuntimeBytePreview(ff, 1, 3, true))",
    )
    .expect("runtime byte diagnostic should parse");

    assert_eq!(
        json_path(&parsed, "byte_offset.value"),
        Some(&JsonValue::Number(4))
    );
    assert_eq!(
        json_path(&parsed, "field_path.0.name"),
        Some(&JsonValue::String("Packet".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "facts.reason"),
        Some(&JsonValue::String("invalid byte".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "preview.encoding"),
        Some(&JsonValue::String("hex".to_string()))
    );
    assert_eq!(
        json_path(&parsed, "preview.truncated"),
        Some(&JsonValue::Bool(true))
    );
}

#[test]
pub(super) fn result_value_parser_exposes_hpack_fixture_runtime_diagnostics() {
    let fixture = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.malformed_raw_string_value, HPACK fixture malformed raw string value at byte offset 9, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDiagnostic(9, 5, 8, fixture HPACK raw string value, hpack_fixture, ByteChunk([Byte(8), Byte(3), Byte(50), Byte(31), Byte(48)]))))",
    )
    .expect("HPACK fixture runtime diagnostic value should parse");
    let dynamic_index = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.dynamic_index_out_of_range, HPACK dynamic index out of range at byte offset 27, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicIndexDiagnostic(27, 1, 190, 0, 0, fixture dynamic indexed header, hpack_fixture, ByteChunk([Byte(190)]))))",
    )
    .expect("HPACK dynamic-index runtime diagnostic value should parse");
    let dynamic_name = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.dynamic_name_continuation_out_of_range, HPACK dynamic-name continuation out of range at byte offset 98, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDynamicNameDiagnostic(98, 8, 127, 3, 3, fixture dynamic-name continuation range, hpack_fixture, ByteChunk([Byte(127), Byte(2), Byte(5), Byte(80), Byte(65), Byte(84), Byte(67), Byte(72)]))))",
    )
    .expect("HPACK dynamic-name runtime diagnostic value should parse");
    let table_size = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.table_size_update_not_at_start, HPACK fixture table-size update after header field at byte offset 10, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureTableSizeUpdateDiagnostic(10, 2, 62, 30, 1, 1, hpack-fixture, fixture HPACK table-size update at header block start, hpack_fixture, ByteChunk([Byte(130), Byte(62)]))))",
    )
    .expect("HPACK table-size runtime diagnostic value should parse");
    let table_size_malformed = parse_result_value(
        "RuntimeDiagnostic(hpack.fixture.table_size_update_malformed, HPACK fixture malformed table-size update integer at byte offset 77, RuntimeHttp2HpackDiagnostic(RuntimeHpackFixtureDiagnostic(77, 2, 63, fixture HPACK malformed table-size update integer, hpack_fixture, ByteChunk([Byte(63), Byte(128)]))))",
    )
    .expect("HPACK table-size malformed runtime diagnostic value should parse");

    assert_eq!(
        json_path(&fixture, "value.detail.constructor"),
        Some(&JsonValue::String(
            "RuntimeHttp2HpackDiagnostic".to_string()
        ))
    );
    assert_eq!(
        json_path(&fixture, "value.detail.detail.expected_fixture"),
        Some(&JsonValue::String(
            "fixture HPACK raw string value".to_string()
        ))
    );
    assert_eq!(
        json_path(&fixture, "value.detail.detail.preview.bytes.2.value"),
        Some(&JsonValue::Number(50))
    );
    assert_eq!(
        json_path(
            &dynamic_index,
            "value.detail.detail.requested_dynamic_index"
        ),
        Some(&JsonValue::Number(0))
    );
    assert_eq!(
        json_path(&dynamic_index, "value.detail.detail.preview.bytes.0.value"),
        Some(&JsonValue::Number(190))
    );
    assert_eq!(
        json_path(&dynamic_name, "value.detail.detail.requested_dynamic_index"),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(
            &dynamic_name,
            "value.detail.detail.dynamic_table_entry_count"
        ),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(
            &table_size,
            "value.detail.detail.observed_header_table_size"
        ),
        Some(&JsonValue::Number(30))
    );
    assert_eq!(
        json_path(&table_size, "value.detail.detail.active_state"),
        Some(&JsonValue::String("hpack-fixture".to_string()))
    );
    assert_eq!(
        json_path(
            &table_size_malformed,
            "value.detail.detail.expected_fixture"
        ),
        Some(&JsonValue::String(
            "fixture HPACK malformed table-size update integer".to_string()
        ))
    );
    assert_eq!(
        json_path(
            &table_size_malformed,
            "value.detail.detail.preview.bytes.1.value"
        ),
        Some(&JsonValue::Number(128))
    );
}

#[test]
pub(super) fn result_value_parser_exposes_http2_peer_limit_runtime_diagnostics() {
    let header_table = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.header_table_size_exceeded, HTTP/2 header table size exceeds receive maximum at byte offset 35, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic(35, 289, 160, 9, 1, local_configuration, hpack_dynamic_table_size_update, ByteChunk([Byte(63), Byte(129), Byte(1)]))))",
    )
    .expect("header-table runtime diagnostic value should parse");
    let concurrent_streams = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.concurrent_streams_exceeded, HTTP/2 concurrent stream receive limit exceeded at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic(9, 3, 2, 1, server, open-stream, local_configuration, peer_created_stream_receive_limit, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(1), Byte(4), Byte(0), Byte(0), Byte(0), Byte(3)]))))",
    )
    .expect("concurrent-stream runtime diagnostic value should parse");

    assert_eq!(
        json_path(&header_table, "value.detail.constructor"),
        Some(&JsonValue::String("RuntimeHttp2Diagnostic".to_string()))
    );
    assert_eq!(
        json_path(
            &header_table,
            "value.detail.detail.observed_header_table_size"
        ),
        Some(&JsonValue::Number(289))
    );
    assert_eq!(
        json_path(&header_table, "value.detail.detail.preview.bytes.1.value"),
        Some(&JsonValue::Number(129))
    );
    assert_eq!(
        json_path(
            &concurrent_streams,
            "value.detail.detail.attempted_concurrent_stream_count"
        ),
        Some(&JsonValue::Number(2))
    );
    assert_eq!(
        json_path(
            &concurrent_streams,
            "value.detail.detail.receive_limit_provenance"
        ),
        Some(&JsonValue::String("local_configuration".to_string()))
    );
    assert_eq!(
        json_path(
            &concurrent_streams,
            "value.detail.detail.preview.bytes.8.value"
        ),
        Some(&JsonValue::Number(3))
    );
}

#[test]
pub(super) fn result_value_parser_exposes_http2_data_flow_content_length_diagnostics() {
    let data_padding = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_data_padding, HTTP/2 invalid DATA padding at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidDataPaddingDiagnostic(9, 1, 2, 0, open-stream, rfc9113_data_padding, ByteChunk([Byte(2)]))))",
    )
    .expect("DATA padding runtime diagnostic value should parse");
    let flow_control = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.flow_control_window_exceeded, HTTP/2 flow-control window exceeded at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitFlowControlWindowDiagnostic(0, 4, 3, 0, 1, open-stream, stream_receive_window, ByteChunk([Byte(1), Byte(2), Byte(3), Byte(4)]))))",
    )
    .expect("flow-control runtime diagnostic value should parse");
    let content_length = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.content_length_mismatch, HTTP/2 content-length body length mismatch at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContentLengthMismatchDiagnostic(9, 0, 1, 5, 3, open-stream, rfc9113_content_length_body, ByteChunk([Byte(170), Byte(187), Byte(204)]))))",
    )
    .expect("content-length runtime diagnostic value should parse");

    assert_eq!(
        json_path(&data_padding, "value.detail.detail.pad_length"),
        Some(&JsonValue::Number(2))
    );
    assert_eq!(
        json_path(&data_padding, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
    assert_eq!(
        json_path(&flow_control, "value.detail.detail.allowed_window_credit"),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(&flow_control, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String("stream_receive_window".to_string()))
    );
    assert_eq!(
        json_path(
            &content_length,
            "value.detail.detail.expected_content_length"
        ),
        Some(&JsonValue::Number(5))
    );
    assert_eq!(
        json_path(&content_length, "value.detail.detail.observed_body_length"),
        Some(&JsonValue::Number(3))
    );
}

#[test]
pub(super) fn result_value_parser_exposes_http2_header_list_runtime_diagnostics() {
    let request = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_request_header_list, HTTP/2 request header list is missing :method at byte offset 12, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidRequestHeaderListDiagnostic(12, 9, 1, missing_required_pseudo_header, :method, headers, request-headers, rfc9113_request_pseudo_headers, ByteChunk([Byte(130), Byte(132), Byte(134)]))))",
    )
    .expect("request header-list runtime diagnostic value should parse");
    let response = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_response_header_list, HTTP/2 response header list is missing :status at byte offset 12, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic(12, 9, 1, missing_required_pseudo_header, :status, server, response-headers, rfc9113_response_pseudo_headers, ByteChunk([Byte(136)]))))",
    )
    .expect("response header-list runtime diagnostic value should parse");

    assert_eq!(
        json_path(&request, "value.detail.detail.failed_header_fact"),
        Some(&JsonValue::String(
            "missing_required_pseudo_header".to_string()
        ))
    );
    assert_eq!(
        json_path(&request, "value.detail.detail.decoded_header_names"),
        Some(&JsonValue::String("headers".to_string()))
    );
    assert_eq!(
        json_path(&request, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
    assert_eq!(
        json_path(&response, "value.detail.detail.constructor"),
        Some(&JsonValue::String(
            "RuntimeHttp2ProtocolInvalidResponseHeaderListDiagnostic".to_string()
        ))
    );
    assert_eq!(
        json_path(&response, "value.detail.detail.header_name"),
        Some(&JsonValue::String(":status".to_string()))
    );
    assert_eq!(
        json_path(&response, "value.detail.detail.preview.bytes.0.value"),
        Some(&JsonValue::Number(136))
    );
}
