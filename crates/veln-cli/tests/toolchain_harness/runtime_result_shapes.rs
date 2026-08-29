use super::*;

#[test]
pub(super) fn result_value_parser_exposes_http2_preface_runtime_diagnostics() {
    let partial = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.partial_preface, HTTP/2 input ended with partial client connection preface at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPartialPrefaceDiagnostic(0, 12, 24, connection-preface, rfc9113_client_connection_preface, ByteChunk([Byte(80), Byte(82), Byte(73), Byte(32), Byte(42), Byte(32)]))))",
    )
    .expect("partial preface runtime diagnostic value should parse");
    let invalid = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_preface, HTTP/2 invalid client connection preface at byte offset 4, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidPrefaceDiagnostic(4, 42, 43, 4, 24, connection-preface, rfc9113_client_connection_preface, ByteChunk([Byte(80), Byte(82), Byte(73), Byte(32), Byte(43)]))))",
    )
    .expect("invalid preface runtime diagnostic value should parse");

    assert_eq!(
        json_path(&partial, "value.detail.detail.pending_count"),
        Some(&JsonValue::Number(12))
    );
    assert_eq!(
        json_path(&partial, "value.detail.detail.active_state"),
        Some(&JsonValue::String("connection-preface".to_string()))
    );
    assert_eq!(
        json_path(&invalid, "value.detail.detail.expected_byte"),
        Some(&JsonValue::Number(42))
    );
    assert_eq!(
        json_path(&invalid, "value.detail.detail.actual_byte"),
        Some(&JsonValue::Number(43))
    );
    assert_eq!(
        json_path(&invalid, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
}

#[test]
pub(super) fn result_value_parser_exposes_http2_control_runtime_diagnostics() {
    let closed = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.closed_with_pending, HTTP/2 input ended with 4 pending byte(s) at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolClosedWithPendingDiagnostic(0, 4, none, 0, 0, 0, 0, none, ByteChunk([Byte(1), Byte(2), Byte(3), Byte(4)]))))",
    )
    .expect("closed-input runtime diagnostic value should parse");
    let continuation = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.continuation_expected, HTTP/2 expected CONTINUATION frame at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolContinuationExpectedDiagnostic(9, 0, 1, 1, 1, 0, headers, 3, rfc9113_continuation_sequence, ByteChunk([Byte(0), Byte(0), Byte(0)]))))",
    )
    .expect("continuation runtime diagnostic value should parse");
    let frame_kind = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_frame_kind, HTTP/2 invalid frame kind at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidFrameKindDiagnostic(0, 0, 1, 1, idle-stream, idle_streams_require_headers, ByteChunk([Byte(0)]))))",
    )
    .expect("invalid frame-kind runtime diagnostic value should parse");
    let stream_id = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_stream_id, HTTP/2 invalid stream id at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidStreamIdDiagnostic(0, 1, 2, nonzero client-initiated stream id, server, stream-id-domain, server_receives_client_initiated_streams, ByteChunk([Byte(0)]))))",
    )
    .expect("invalid stream-id runtime diagnostic value should parse");

    assert_eq!(
        json_path(&closed, "value.detail.detail.pending_count"),
        Some(&JsonValue::Number(4))
    );
    assert_eq!(
        json_path(&closed, "value.detail.detail.active_continuation"),
        Some(&JsonValue::String("none".to_string()))
    );
    assert_eq!(
        json_path(&closed, "value.detail.detail.expected_stream_id"),
        Some(&JsonValue::Number(0))
    );
    assert_eq!(
        json_path(&closed, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String("none".to_string()))
    );
    assert_eq!(
        json_path(&continuation, "value.detail.detail.expected_stream_id"),
        Some(&JsonValue::Number(1))
    );
    assert_eq!(
        json_path(&continuation, "value.detail.detail.active_continuation"),
        Some(&JsonValue::String("headers".to_string()))
    );
    assert_eq!(
        json_path(
            &continuation,
            "value.detail.detail.accumulated_header_block_bytes"
        ),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(&continuation, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String(
            "rfc9113_continuation_sequence".to_string()
        ))
    );
    assert_eq!(
        json_path(&frame_kind, "value.detail.detail.expected_frame_kind"),
        Some(&JsonValue::Number(1))
    );
    assert_eq!(
        json_path(&frame_kind, "value.detail.detail.active_state"),
        Some(&JsonValue::String("idle-stream".to_string()))
    );
    assert_eq!(
        json_path(&stream_id, "value.detail.detail.required_stream_id_domain"),
        Some(&JsonValue::String(
            "nonzero client-initiated stream id".to_string()
        ))
    );
    assert_eq!(
        json_path(&stream_id, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String(
            "server_receives_client_initiated_streams".to_string()
        ))
    );
}

#[test]
pub(super) fn result_value_parser_exposes_http2_limit_and_shutdown_runtime_diagnostics() {
    let payload_length = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_payload_length, HTTP/2 invalid payload length at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidPayloadLengthDiagnostic(9, 8, 0, 3, 4, connection-flow-control, rfc9113_window_update_payload_length, ByteChunk([Byte(1), Byte(2), Byte(3)]))))",
    )
    .expect("invalid payload-length runtime diagnostic value should parse");
    let settings_value = parse_result_value(
        "RuntimeDiagnostic(http2.peer_limit.settings_value_out_of_range, HTTP/2 SETTINGS value outside accepted range at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitSettingsValueDiagnostic(9, 5, SETTINGS_MAX_FRAME_SIZE, 16383, 16384, 16777215, peer_settings, ByteChunk([Byte(0), Byte(5)]))))",
    )
    .expect("settings value runtime diagnostic value should parse");
    let window_update = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_window_update_increment, HTTP/2 invalid WINDOW_UPDATE increment at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolInvalidWindowUpdateIncrementDiagnostic(0, 0, 0, 1, 2147483647, connection-flow-control, window_update_increment_nonzero, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(0)]))))",
    )
    .expect("window-update runtime diagnostic value should parse");
    let priority = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.invalid_priority_dependency, HTTP/2 invalid PRIORITY dependency at byte offset 0, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolPriorityDependencyDiagnostic(0, 1, 1, stream-control, rfc9113_priority_dependency, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(1), Byte(15)]))))",
    )
    .expect("priority runtime diagnostic value should parse");
    let goaway = parse_result_value(
        "RuntimeDiagnostic(http2.protocol.stream_after_goaway, HTTP/2 stream opened after graceful shutdown at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2ProtocolStreamAfterGoawayDiagnostic(9, 7, 5, graceful_shutdown, server, goaway_last_stream_id, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(1), Byte(4), Byte(0), Byte(0), Byte(0), Byte(7)]))))",
    )
    .expect("stream-after-GOAWAY runtime diagnostic value should parse");

    assert_eq!(
        json_path(
            &payload_length,
            "value.detail.detail.observed_payload_length"
        ),
        Some(&JsonValue::Number(3))
    );
    assert_eq!(
        json_path(
            &payload_length,
            "value.detail.detail.expected_payload_length"
        ),
        Some(&JsonValue::Number(4))
    );
    assert_eq!(
        json_path(&settings_value, "value.detail.detail.setting_name"),
        Some(&JsonValue::String("SETTINGS_MAX_FRAME_SIZE".to_string()))
    );
    assert_eq!(
        json_path(&settings_value, "value.detail.detail.peer_limit_provenance"),
        Some(&JsonValue::String("peer_settings".to_string()))
    );
    assert_eq!(
        json_path(
            &window_update,
            "value.detail.detail.accepted_max_window_increment"
        ),
        Some(&JsonValue::Number(2147483647))
    );
    assert_eq!(
        json_path(&priority, "value.detail.detail.dependency_stream_id"),
        Some(&JsonValue::Number(1))
    );
    assert_eq!(
        json_path(&priority, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
    assert_eq!(
        json_path(&goaway, "value.detail.detail.shutdown_state"),
        Some(&JsonValue::String("graceful_shutdown".to_string()))
    );
    assert_eq!(
        json_path(&goaway, "value.detail.detail.rule_provenance"),
        Some(&JsonValue::String("goaway_last_stream_id".to_string()))
    );
    assert_eq!(
        json_path(&goaway, "value.detail.detail.preview.constructor"),
        Some(&JsonValue::String("ByteChunk".to_string()))
    );
}

#[test]
pub(super) fn manifest_output_chunk_lists_parse_ordered_hex_chunks() {
    let manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[output_chunk_list]]
name = "protocol-output"
chunks = ["0001ff", "00040000000f000001"]

[[output_chunk_list]]
name = "empty-chunk"
chunks = [""]

[[output_chunk_list]]
name = "no-output"
chunks = []
"#,
    );

    assert!(manifest.expectations.needs_stdout_json());
    let chunk_lists = &manifest.expectations.output_chunk_lists;
    assert_eq!(chunk_lists.len(), 3);
    assert_eq!(chunk_lists[0].name, "protocol-output");
    assert_eq!(
        chunk_lists[0].chunks.as_ref().unwrap()[0].bytes,
        [0, 1, 255]
    );
    assert_eq!(
        expected_output_chunk_list_lines(&chunk_lists[0]),
        [
            "output_chunk_list protocol-output count 2",
            "output_chunk protocol-output index 0 hex \"0001ff\" count 3",
            "output_chunk protocol-output index 1 hex \"00040000000f000001\" count 9",
        ]
    );
    assert_eq!(
        expected_output_chunk_list_lines(&chunk_lists[1]),
        [
            "output_chunk_list empty-chunk count 1",
            "output_chunk empty-chunk index 0 hex \"\" count 0",
        ]
    );
    assert_eq!(
        expected_output_chunk_list_lines(&chunk_lists[2]),
        ["output_chunk_list no-output count 0"]
    );
}

#[test]
#[should_panic(expected = "expected lowercase hex")]
pub(super) fn manifest_output_chunk_lists_reject_uppercase_hex() {
    let _manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[output_chunk_list]]
name = "protocol-output"
chunks = ["00FF"]
"#,
    );
}

#[test]
#[should_panic(expected = "expected complete lowercase hex byte pairs")]
pub(super) fn manifest_output_chunk_lists_reject_odd_length_hex() {
    let _manifest = parse_manifest(
        Path::new("case.toml"),
        r#"
command = ["run", "--json", "main", "main.veln"]
exit = 0

[[output_chunk_list]]
name = "protocol-output"
chunks = ["001"]
"#,
    );
}

#[test]
pub(super) fn missing_tool_setup_leaves_isolated_tool_path_empty() {
    let root = test_temp_root("missing-tool");
    setup_tool(&root, "java", ToolAvailability::Missing);

    let entries = fs::read_dir(&root)
        .expect("tool root should be readable")
        .collect::<Result<Vec<_>, _>>()
        .expect("tool entries should be readable");
    assert!(entries.is_empty());

    fs::remove_dir_all(root).expect("test root should be removed");
}

#[test]
pub(super) fn fake_success_tool_setup_installs_success_launcher() {
    let root = test_temp_root("fake-tool");
    setup_tool(&root, "java", ToolAvailability::FakeSuccess);

    let output = Command::new(fake_tool_path(&root, "java"))
        .output()
        .expect("fake tool should run");
    assert!(output.status.success());
    assert_eq!(output.stdout, b"");
    assert_eq!(output.stderr, b"");

    fs::remove_dir_all(root).expect("test root should be removed");
}
