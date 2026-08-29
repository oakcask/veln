use super::*;

#[test]
fn protocol_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "485454502f3220657870656374656420434f4e54494e554154494f4e206672616d652061742062797465206f66667365742039",
        "\tprotocol_diagnostic\thttp2.protocol.continuation_expected\t9",
        "\t9\tactual_frame_kind\tnumber\t0",
        "\tactual_stream_id\tnumber\t1",
        "\texpected_stream_id\tnumber\t1",
        "\tstarted_frame_kind\tnumber\t1",
        "\tstarted_byte_offset\tnumber\t0",
        "\tactive_continuation\tstring\t68656164657273",
        "\taccumulated_header_block_bytes\tnumber\t3",
        "\trule_provenance\tstring\t726663393131335f636f6e74696e756174696f6e5f73657175656e6365",
        "\tbyte_preview\tbyte_preview_v2\t30303030303030303030303030303030:8:9:true\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"HTTP/2 expected CONTINUATION frame at byte offset 9\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.protocol.continuation_expected\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":9},",
            "\"actual_frame_kind\":0,",
            "\"actual_stream_id\":1,",
            "\"expected_stream_id\":1,",
            "\"started_frame_kind\":1,",
            "\"started_byte_offset\":0,",
            "\"active_continuation\":\"headers\",",
            "\"accumulated_header_block_bytes\":3,",
            "\"rule_provenance\":\"rfc9113_continuation_sequence\",",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"0000000000000000\",",
            "\"preview_byte_count\":8,",
            "\"total_byte_count\":9,",
            "\"truncated\":true}}}"
        )
    );
}

#[test]
fn protocol_diagnostic_result_trace_decodes_byte_preview_details() {
    let trace = concat!(
        "result\t",
        "485454502f3220696e76616c696420636c69656e7420636f6e6e656374696f6e20707265666163652061742062797465206f66667365742034",
        "\tprotocol_diagnostic\thttp2.protocol.invalid_preface\t4",
        "\t7\texpected_byte\tnumber\t42",
        "\tactual_byte\tnumber\t43",
        "\tmatched_prefix_count\tnumber\t4",
        "\texpected_count\tnumber\t24",
        "\tbyte_preview\tbyte_preview_v2\t35303532343932303262:5:5:false",
        "\tactive_state\tstring\t636f6e6e656374696f6e2d70726566616365",
        "\trule_provenance\tstring\t726663393131335f636c69656e745f636f6e6e656374696f6e5f70726566616365\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"HTTP/2 invalid client connection preface at byte offset 4\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.protocol.invalid_preface\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":4},",
            "\"expected_byte\":42,",
            "\"actual_byte\":43,",
            "\"matched_prefix_count\":4,",
            "\"expected_count\":24,",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"505249202b\",",
            "\"preview_byte_count\":5,",
            "\"total_byte_count\":5,",
            "\"truncated\":false},",
            "\"active_state\":\"connection-preface\",",
            "\"rule_provenance\":\"rfc9113_client_connection_preface\"}}"
        )
    );
}

#[test]
fn peer_limit_protocol_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "485454502f32206672616d65207061796c6f6164206c656e67746820657863656564732072656365697665206d6178696d756d2061742062797465206f66667365742030",
        "\tprotocol_diagnostic\thttp2.peer_limit.frame_size_exceeded\t0",
        "\t7\tobserved_payload_length\tnumber\t16385",
        "\tallowed_max_frame_size\tnumber\t16384",
        "\tframe_kind\tnumber\t0",
        "\tstream_id\tnumber\t3",
        "\tstream_ref\tstring\t73747265616d",
        "\treceive_limit_provenance\tstring\t70726f746f636f6c5f64656661756c74",
        "\tbyte_preview\tbyte_preview_v2\t30303034303130303030303030303030:8:9:true\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"HTTP/2 frame payload length exceeds receive maximum at byte offset 0\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.peer_limit.frame_size_exceeded\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
            "\"observed_payload_length\":16385,",
            "\"allowed_max_frame_size\":16384,",
            "\"frame_kind\":0,",
            "\"stream_id\":3,",
            "\"stream_ref\":\"stream\",",
            "\"receive_limit_provenance\":\"protocol_default\",",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"0004010000000000\",",
            "\"preview_byte_count\":8,",
            "\"total_byte_count\":9,",
            "\"truncated\":true}}}"
        )
    );
}

#[test]
fn header_list_protocol_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "485454502f3220686561646572206c6973742073697a6520657863656564732072656365697665206d6178696d756d2061742062797465206f6666736574203132",
        "\tprotocol_diagnostic\thttp2.peer_limit.header_list_size_exceeded\t12",
        "\t8\tobserved_header_list_size\tnumber\t10",
        "\tallowed_header_list_size\tnumber\t9",
        "\tframe_kind\tnumber\t9",
        "\tstream_id\tnumber\t1",
        "\tstream_ref\tstring\t73747265616d",
        "\treceive_limit_provenance\tstring\t6c6f63616c5f636f6e66696775726174696f6e",
        "\trule_provenance\tstring\t6865616465725f6c6973745f726563656976655f6c696d6974",
        "\tbyte_preview\tbyte_preview_v2\t30363037303830393061306230633064:8:9:true\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"HTTP/2 header list size exceeds receive maximum at byte offset 12\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.peer_limit.header_list_size_exceeded\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":12},",
            "\"observed_header_list_size\":10,",
            "\"allowed_header_list_size\":9,",
            "\"frame_kind\":9,",
            "\"stream_id\":1,",
            "\"stream_ref\":\"stream\",",
            "\"receive_limit_provenance\":\"local_configuration\",",
            "\"rule_provenance\":\"header_list_receive_limit\",",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"060708090a0b0c0d\",",
            "\"preview_byte_count\":8,",
            "\"total_byte_count\":9,",
            "\"truncated\":true}}}"
        )
    );
}

#[test]
fn settings_value_protocol_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "485454502f322053455454494e47532076616c7565206f7574736964652061636365707465642072616e67652061742062797465206f66667365742039",
        "\tprotocol_diagnostic\thttp2.peer_limit.settings_value_out_of_range\t9",
        "\t7\tsetting_identifier\tnumber\t2",
        "\tsetting_name\tstring\t53455454494e47535f454e41424c455f50555348",
        "\tobserved_value\tnumber\t2",
        "\taccepted_min_value\tnumber\t0",
        "\taccepted_max_value\tnumber\t1",
        "\tpeer_limit_provenance\tstring\t706565725f73657474696e6773",
        "\tbyte_preview\tbyte_preview_v2\t303030323030303030303032:6:6:false\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"HTTP/2 SETTINGS value outside accepted range at byte offset 9\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.peer_limit.settings_value_out_of_range\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":9},",
            "\"setting_identifier\":2,",
            "\"setting_name\":\"SETTINGS_ENABLE_PUSH\",",
            "\"observed_value\":2,",
            "\"accepted_min_value\":0,",
            "\"accepted_max_value\":1,",
            "\"peer_limit_provenance\":\"peer_settings\",",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"000200000002\",",
            "\"preview_byte_count\":6,",
            "\"total_byte_count\":6,",
            "\"truncated\":false}}}"
        )
    );
}

#[test]
fn header_table_protocol_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "52756e74696d65446961676e6f737469632868747470322e706565725f6c696d69742e6865616465725f7461626c655f73697a655f65786365656465642c20485454502f3220686561646572207461626c652073697a6520657863656564732072656365697665206d6178696d756d2061742062797465206f66667365742033352c2052756e74696d654874747032446961676e6f737469632852756e74696d654874747032506565724c696d69744865616465725461626c6553697a65446961676e6f737469632833352c203238392c203136302c20392c20312c206c6f63616c5f636f6e66696775726174696f6e2c20687061636b5f64796e616d69635f7461626c655f73697a655f7570646174652c20427974654368756e6b285b42797465283633292c204279746528313239292c20427974652831295d29292929",
        "\tprotocol_diagnostic\thttp2.peer_limit.header_table_size_exceeded\t35",
        "\t8\tobserved_header_table_size\tnumber\t289",
        "\tallowed_header_table_size\tnumber\t160",
        "\tframe_kind\tnumber\t9",
        "\tstream_id\tnumber\t1",
        "\tstream_ref\tstring\t73747265616d",
        "\treceive_limit_provenance\tstring\t6c6f63616c5f636f6e66696775726174696f6e",
        "\trule_provenance\tstring\t687061636b5f64796e616d69635f7461626c655f73697a655f757064617465",
        "\tbyte_preview\tbyte_preview_v2\t336638313031:3:3:false\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"RuntimeDiagnostic(http2.peer_limit.header_table_size_exceeded, HTTP/2 header table size exceeds receive maximum at byte offset 35, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitHeaderTableSizeDiagnostic(35, 289, 160, 9, 1, local_configuration, hpack_dynamic_table_size_update, ByteChunk([Byte(63), Byte(129), Byte(1)]))))\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.peer_limit.header_table_size_exceeded\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":35},",
            "\"observed_header_table_size\":289,",
            "\"allowed_header_table_size\":160,",
            "\"frame_kind\":9,",
            "\"stream_id\":1,",
            "\"stream_ref\":\"stream\",",
            "\"receive_limit_provenance\":\"local_configuration\",",
            "\"rule_provenance\":\"hpack_dynamic_table_size_update\",",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"3f8101\",",
            "\"preview_byte_count\":3,",
            "\"total_byte_count\":3,",
            "\"truncated\":false}}}"
        )
    );
}

#[test]
fn flow_control_protocol_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "485454502f3220666c6f772d636f6e74726f6c2077696e646f772065786365656465642061742062797465206f66667365742030",
        "\tprotocol_diagnostic\thttp2.peer_limit.flow_control_window_exceeded\t0",
        "\t8\tobserved_payload_length\tnumber\t4",
        "\tallowed_window_credit\tnumber\t3",
        "\tframe_kind\tnumber\t0",
        "\tstream_id\tnumber\t1",
        "\tstream_ref\tstring\t73747265616d",
        "\tactive_state\tstring\t6f70656e2d73747265616d",
        "\trule_provenance\tstring\t73747265616d5f726563656976655f77696e646f77",
        "\tbyte_preview\tbyte_preview_v2\t3031303230333034:4:4:false\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"HTTP/2 flow-control window exceeded at byte offset 0\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.peer_limit.flow_control_window_exceeded\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
            "\"observed_payload_length\":4,",
            "\"allowed_window_credit\":3,",
            "\"frame_kind\":0,",
            "\"stream_id\":1,",
            "\"stream_ref\":\"stream\",",
            "\"active_state\":\"open-stream\",",
            "\"rule_provenance\":\"stream_receive_window\",",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"01020304\",",
            "\"preview_byte_count\":4,",
            "\"total_byte_count\":4,",
            "\"truncated\":false}}}"
        )
    );
}

#[test]
fn concurrent_streams_protocol_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "52756e74696d65446961676e6f737469632868747470322e706565725f6c696d69742e636f6e63757272656e745f73747265616d735f65786365656465642c20485454502f3220636f6e63757272656e742073747265616d2072656365697665206c696d69742065786365656465642061742062797465206f666673657420392c2052756e74696d654874747032446961676e6f737469632852756e74696d654874747032506565724c696d6974436f6e63757272656e7453747265616d73446961676e6f7374696328392c20332c20322c20312c207365727665722c206f70656e2d73747265616d2c206c6f63616c5f636f6e66696775726174696f6e2c20706565725f637265617465645f73747265616d5f726563656976655f6c696d69742c20427974654368756e6b285b427974652830292c20427974652830292c20427974652830292c20427974652831292c20427974652834292c20427974652830292c20427974652830292c20427974652830292c20427974652833295d29292929",
        "\tprotocol_diagnostic\thttp2.peer_limit.concurrent_streams_exceeded\t9",
        "\t10\tstream_id\tnumber\t3",
        "\tstream_ref\tstring\t73747265616d",
        "\tcurrent_open_peer_created_stream_count\tnumber\t1",
        "\tattempted_concurrent_stream_count\tnumber\t2",
        "\tallowed_concurrent_stream_count\tnumber\t1",
        "\tendpoint_role\tstring\t736572766572",
        "\tactive_state\tstring\t6f70656e2d73747265616d",
        "\treceive_limit_provenance\tstring\t6c6f63616c5f636f6e66696775726174696f6e",
        "\trule_provenance\tstring\t706565725f637265617465645f73747265616d5f726563656976655f6c696d6974",
        "\tbyte_preview\tbyte_preview_v2\t30303030303030313034303030303030:8:9:true\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"RuntimeDiagnostic(http2.peer_limit.concurrent_streams_exceeded, HTTP/2 concurrent stream receive limit exceeded at byte offset 9, RuntimeHttp2Diagnostic(RuntimeHttp2PeerLimitConcurrentStreamsDiagnostic(9, 3, 2, 1, server, open-stream, local_configuration, peer_created_stream_receive_limit, ByteChunk([Byte(0), Byte(0), Byte(0), Byte(1), Byte(4), Byte(0), Byte(0), Byte(0), Byte(3)]))))\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.peer_limit.concurrent_streams_exceeded\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":9},",
            "\"stream_id\":3,",
            "\"stream_ref\":\"stream\",",
            "\"current_open_peer_created_stream_count\":1,",
            "\"attempted_concurrent_stream_count\":2,",
            "\"allowed_concurrent_stream_count\":1,",
            "\"endpoint_role\":\"server\",",
            "\"active_state\":\"open-stream\",",
            "\"receive_limit_provenance\":\"local_configuration\",",
            "\"rule_provenance\":\"peer_created_stream_receive_limit\",",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"0000000104000000\",",
            "\"preview_byte_count\":8,",
            "\"total_byte_count\":9,",
            "\"truncated\":true}}}"
        )
    );
}

#[test]
fn invalid_frame_kind_protocol_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "485454502f3220696e76616c6964206672616d65206b696e642061742062797465206f66667365742030",
        "\tprotocol_diagnostic\thttp2.protocol.invalid_frame_kind\t0",
        "\t7\tactual_frame_kind\tnumber\t0",
        "\tstream_id\tnumber\t0",
        "\tstream_ref\tstring\t636f6e6e656374696f6e",
        "\texpected_frame_kind\tnumber\t4",
        "\tbyte_preview\tbyte_preview_v2\t30303030303030303030303030303030:8:9:true",
        "\tactive_state\tstring\t636f6e6e656374696f6e2d636f6e74726f6c",
        "\trule_provenance\tstring\t636f6e6e656374696f6e5f6672616d65735f726571756972655f73657474696e6773\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"HTTP/2 invalid frame kind at byte offset 0\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.protocol.invalid_frame_kind\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
            "\"actual_frame_kind\":0,",
            "\"stream_id\":0,",
            "\"stream_ref\":\"connection\",",
            "\"expected_frame_kind\":4,",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"0000000000000000\",",
            "\"preview_byte_count\":8,",
            "\"total_byte_count\":9,",
            "\"truncated\":true},",
            "\"active_state\":\"connection-control\",",
            "\"rule_provenance\":\"connection_frames_require_settings\"}}"
        )
    );
}

#[test]
fn invalid_stream_id_protocol_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "485454502f3220696e76616c69642073747265616d2069642061742062797465206f66667365742030",
        "\tprotocol_diagnostic\thttp2.protocol.invalid_stream_id\t0",
        "\t8\tframe_kind\tnumber\t1",
        "\tstream_id\tnumber\t2",
        "\tstream_ref\tstring\t73747265616d",
        "\trequired_stream_id_domain\tstring\t6e6f6e7a65726f20636c69656e742d696e697469617465642073747265616d206964",
        "\tendpoint_role\tstring\t736572766572",
        "\tbyte_preview\tbyte_preview_v2\t30303030303030313034303030303030:8:9:true",
        "\tactive_state\tstring\t73747265616d2d69642d646f6d61696e",
        "\trule_provenance\tstring\t7365727665725f72656365697665735f636c69656e745f696e697469617465645f73747265616d73\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"HTTP/2 invalid stream id at byte offset 0\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.protocol.invalid_stream_id\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
            "\"frame_kind\":1,",
            "\"stream_id\":2,",
            "\"stream_ref\":\"stream\",",
            "\"required_stream_id_domain\":\"nonzero client-initiated stream id\",",
            "\"endpoint_role\":\"server\",",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"0000000104000000\",",
            "\"preview_byte_count\":8,",
            "\"total_byte_count\":9,",
            "\"truncated\":true},",
            "\"active_state\":\"stream-id-domain\",",
            "\"rule_provenance\":\"server_receives_client_initiated_streams\"}}"
        )
    );
}

#[test]
fn invalid_payload_length_protocol_diagnostic_result_trace_keeps_byte_preview() {
    let trace = concat!(
        "result\t",
        "485454502f3220696e76616c6964207061796c6f6164206c656e6774682061742062797465206f66667365742030",
        "\tprotocol_diagnostic\thttp2.protocol.invalid_payload_length\t0",
        "\t8\tframe_kind\tnumber\t6",
        "\tstream_id\tnumber\t0",
        "\tstream_ref\tstring\t636f6e6e656374696f6e",
        "\tobserved_payload_length\tnumber\t7",
        "\texpected_payload_length\tnumber\t8",
        "\tbyte_preview\tbyte_preview_v2\t3031303230333034303530363037:7:7:false",
        "\tactive_state\tstring\t636f6e6e656374696f6e2d636f6e74726f6c",
        "\trule_provenance\tstring\t726663393131335f70696e675f7061796c6f61645f6c656e677468\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"HTTP/2 invalid payload length at byte offset 0\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.protocol.invalid_payload_length\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
            "\"frame_kind\":6,",
            "\"stream_id\":0,",
            "\"stream_ref\":\"connection\",",
            "\"observed_payload_length\":7,",
            "\"expected_payload_length\":8,",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"01020304050607\",",
            "\"preview_byte_count\":7,",
            "\"total_byte_count\":7,",
            "\"truncated\":false},",
            "\"active_state\":\"connection-control\",",
            "\"rule_provenance\":\"rfc9113_ping_payload_length\"}}"
        )
    );
}

#[test]
fn stream_after_goaway_protocol_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "485454502f322073747265616d206f70656e656420616674657220677261636566756c2073687574646f776e2061742062797465206f66667365742039",
        "\tprotocol_diagnostic\thttp2.protocol.stream_after_goaway\t9",
        "\t8\tstream_id\tnumber\t7",
        "\tstream_ref\tstring\t73747265616d",
        "\tlast_stream_id\tnumber\t5",
        "\tshutdown_state\tstring\t677261636566756c5f73687574646f776e",
        "\tendpoint_role\tstring\t736572766572",
        "\tbyte_preview\tbyte_preview_v2\t30303030303030313034303030303030:8:9:true",
        "\tactive_state\tstring\t677261636566756c5f73687574646f776e",
        "\trule_provenance\tstring\t676f617761795f6c6173745f73747265616d5f6964\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"HTTP/2 stream opened after graceful shutdown at byte offset 9\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.protocol.stream_after_goaway\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":9},",
            "\"stream_id\":7,",
            "\"stream_ref\":\"stream\",",
            "\"last_stream_id\":5,",
            "\"shutdown_state\":\"graceful_shutdown\",",
            "\"endpoint_role\":\"server\",",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"0000000104000000\",",
            "\"preview_byte_count\":8,",
            "\"total_byte_count\":9,",
            "\"truncated\":true},",
            "\"active_state\":\"graceful_shutdown\",",
            "\"rule_provenance\":\"goaway_last_stream_id\"}}"
        )
    );
}

#[test]
fn stream_invalid_frame_kind_protocol_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "485454502f3220696e76616c6964206672616d65206b696e642061742062797465206f66667365742030",
        "\tprotocol_diagnostic\thttp2.protocol.invalid_frame_kind\t0",
        "\t7\tactual_frame_kind\tnumber\t0",
        "\tstream_id\tnumber\t1",
        "\tstream_ref\tstring\t73747265616d",
        "\texpected_frame_kind\tnumber\t1",
        "\tbyte_preview\tbyte_preview_v2\t30303030303030303030303030303030:8:9:true",
        "\tactive_state\tstring\t69646c652d73747265616d",
        "\trule_provenance\tstring\t69646c655f73747265616d735f726571756972655f68656164657273\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"HTTP/2 invalid frame kind at byte offset 0\",",
            "\"protocol_diagnostic\":{\"kind\":\"protocol_diagnostic\",",
            "\"id\":\"http2.protocol.invalid_frame_kind\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
            "\"actual_frame_kind\":0,",
            "\"stream_id\":1,",
            "\"stream_ref\":\"stream\",",
            "\"expected_frame_kind\":1,",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"0000000000000000\",",
            "\"preview_byte_count\":8,",
            "\"total_byte_count\":9,",
            "\"truncated\":true},",
            "\"active_state\":\"idle-stream\",",
            "\"rule_provenance\":\"idle_streams_require_headers\"}}"
        )
    );
}
