use super::*;

#[test]
fn imported_public_codec_decode_resolves_through_qualified_module_path() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  wire::decode_packet(view, base)\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "\n",
            "pub fn decode_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  NeedMore(NeedEnd)\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::Function(name),
            ..
        } if name == "decode_packet"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::Function(name),
            ..
        } if name == "decode_packet"
    ));
}

#[test]
fn imported_public_codec_encode_resolves_through_qualified_module_path() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(packet: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode wire::PacketWire from packet\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "pub schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "\n",
            "fn encode_packet(packet: {length: Int}) -> EncodeStep<String>\n",
            "  Encoded(list_nil())\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "PacketWire"
    ));

    let ir = lowered.ir.expect("typed IR should be built");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::SchemaEncode(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn imported_public_derived_codec_decode_resolves_through_qualified_module_path() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{wire_length: Int}>\n",
            "  decode wire::PacketWire from view at base\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "pub schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  wire_length: UInt8\n",
            "end\n",
            "\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: wire.types,
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaDecodeStep(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn imported_public_derived_codec_encode_resolves_through_qualified_module_path() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(packet: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
            "  encode wire::PacketWire from packet\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "pub schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: app.functions,
        invalid_names: Vec::new(),
    };

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::SchemaEncode(name),
            ..
        } if name == "PacketWire"
    ));
}

#[test]
fn imported_codec_private_implementation_items_do_not_resolve_as_calls() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn call_helper(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  wire::decode_packet(view, base)\n",
            "end\n",
            "\n",
            "pub fn call_schema(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  wire::PacketWire(view, base)\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "\n",
            "fn decode_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  NeedMore(NeedEnd)\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `wire::PacketWire`"
    }));
}

#[test]
fn imported_codec_decode_does_not_resolve_as_bare_call() {
    let app_source = SourceFile::new(
        "app.veln",
        concat!(
            "mod app\n",
            "use wire\n",
            "\n",
            "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  decode_packet(view, base)\n",
            "end\n",
        ),
    );
    let wire_source = SourceFile::new(
        "wire.veln",
        concat!(
            "mod wire\n",
            "\n",
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  length: UInt8\n",
            "end\n",
            "\n",
            "\n",
            "fn decode_packet(input: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
            "  NeedMore(NeedEnd)\n",
            "end\n",
        ),
    );
    let app = lower_surface_ast(&parse(&app_source).tree);
    let wire = lower_surface_ast(&parse(&wire_source).tree);
    let module = SurfaceModule {
        module: app.module,
        uses: app.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: wire.schemas,
        codecs: wire.codecs,
        functions: [app.functions, wire.functions].concat(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `decode_packet`"
    }));
}

#[test]
fn infers_prelude_helper_calls_from_expected_types() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type List<A>\n",
            "  Nil\n",
            "  Cons(head: A, tail: List<A>)\n",
            "end\n",
            "pub fn main(items: Vec<Int>, other: Vec<Int>, table: Dict<String, Int>, ",
            "list: List<Int>, one_byte: Byte, chunk: ByteChunk, other_chunk: ByteChunk, ",
            "view: ByteView, count: ByteCount, offset: ByteOffset, ",
            "mapper: fn(Int) -> String, keep: fn(Int) -> Bool, folder: fn(String, Int) -> String, ",
            "dict_mapper: fn(String, Int) -> String, dict_keep: fn(String, Int) -> Bool, ",
            "dict_folder: fn(String, String, Int) -> String, ",
            "dict_fallible: fn(String, Int) -> Result<String, AppError>, ",
            "dict_mapper_with: fn(String, String, Int) -> String, ",
            "dict_keep_with: fn(Int, String, Int) -> Bool, ",
            "dict_folder_with: fn(String, String, String, Int) -> String, ",
            "dict_fallible_with: fn(String, String, Int) -> Result<String, AppError>, ",
            "fallible: fn(Int) -> Result<String, AppError>, opt: Option<Int>, ",
            "fallible_with: fn(String, Int) -> Result<String, AppError>, ",
            "opt_map: fn(Int) -> String, opt_next: fn(Int) -> Option<String>, ",
            "res: Result<Int, AppError>, err_map: fn(AppError) -> String, ",
            "res_next: fn(Int) -> Result<String, AppError>) -> {",
            "count: Int, empty: Bool, byte_value: Result<Byte, String>, byte_int: Int, ",
            "chunk_value: ByteChunk, chunk_count: ByteCount, appended: ByteChunk, ",
            "hex_chunk: Result<ByteChunk, String>, ascii_text: Result<String, String>, ",
            "ascii_chunk: Result<ByteChunk, String>, ",
            "taken: Result<ByteChunk, String>, dropped: Result<ByteChunk, String>, ",
            "view_value: Result<ByteView, String>, view_chunk: ByteChunk, view_count: ByteCount, ",
            "view_taken: Result<ByteView, String>, view_dropped: Result<ByteView, String>, ",
            "view_slice: Result<ByteView, String>, empty_chunks: List<ByteChunk>, ",
            "one_chunk: List<ByteChunk>, appended_chunks: List<ByteChunk>, ",
            "produced_chunks: {chunks: List<ByteChunk>, produced: ByteCount, remaining: List<ByteChunk>}, ",
            "read_u8: Result<Int, String>, expect_u8: Result<Int, String>, ",
            "decoded_frame: Result<{length: Int, kind: Int, flags: Int, stream_id: Int, payload: ByteView}, String>, ",
            "decoded_widths: Result<{short_value: Int, wide_value: Int}, String>, ",
            "decoded_validation: Result<{length: Int, padding_length: Int}, String>, ",
            "closed_http2: Result<(), RuntimeDiagnostic>, partial_preface_http2: Result<(), RuntimeDiagnostic>, ",
            "invalid_preface_http2: Result<(), RuntimeDiagnostic>, continuation_http2: Result<(), RuntimeDiagnostic>, ",
            "initial_peer_settings_http2: Result<(), RuntimeDiagnostic>, ",
            "invalid_kind_http2: Result<(), RuntimeDiagnostic>, invalid_stream_http2: Result<(), RuntimeDiagnostic>, ",
            "invalid_payload_http2: Result<(), RuntimeDiagnostic>, invalid_payload_chunk_http2: Result<(), RuntimeDiagnostic>, ",
            "invalid_window_update_increment_http2: Result<(), RuntimeDiagnostic>, ",
            "invalid_data_padding_http2: Result<(), RuntimeDiagnostic>, content_length_http2: Result<(), RuntimeDiagnostic>, ",
            "invalid_request_headers_http2: Result<(), RuntimeDiagnostic>, ",
            "invalid_response_headers_http2: Result<(), RuntimeDiagnostic>, ",
            "unexpected_settings_ack_http2: Result<(), RuntimeDiagnostic>, ",
            "settings_endpoint_role_http2: Result<(), RuntimeDiagnostic>, ",
            "invalid_priority_dependency_http2: Result<(), RuntimeDiagnostic>, ",
            "stream_after_goaway_http2: Result<(), RuntimeDiagnostic>, ",
            "frame_size_http2: Result<(), RuntimeDiagnostic>, header_list_http2: Result<(), RuntimeDiagnostic>, ",
            "header_table_http2: Result<(), RuntimeDiagnostic>, ",
            "flow_control_http2: Result<(), RuntimeDiagnostic>, ",
            "concurrent_streams_http2: Result<(), RuntimeDiagnostic>, ",
            "settings_value_http2: Result<(), RuntimeDiagnostic>, ",
            "read_u16: Result<Int, String>, read_u24: Result<Int, String>, ",
            "read_u31: Result<Int, String>, read_u32: Result<Int, String>, ",
            "read_u16_le: Result<Int, String>, read_u24_le: Result<Int, String>, ",
            "read_u31_le: Result<Int, String>, read_u32_le: Result<Int, String>, ",
            "write_u8: Result<ByteChunk, String>, write_u16: Result<ByteChunk, String>, ",
            "write_u24: Result<ByteChunk, String>, write_u31: Result<ByteChunk, String>, ",
            "write_u32: Result<ByteChunk, String>, ",
            "write_u16_le: Result<ByteChunk, String>, write_u24_le: Result<ByteChunk, String>, ",
            "write_u31_le: Result<ByteChunk, String>, write_u32_le: Result<ByteChunk, String>, ",
            "count_value: Result<ByteCount, String>, count_int: Int, ",
            "offset_value: Result<ByteOffset, String>, offset_int: Int, ",
            "pushed: Vec<Int>, joined: Vec<Int>, mapped: Vec<String>, ",
            "filtered: Vec<Int>, folded: String, tried: Result<Vec<String>, AppError>, ",
            "tried_with: Result<Vec<String>, AppError>, split: Option<{left: String, right: String}>, ",
            "parsed: Result<Int, String>, rendered: String, ",
            "list_nil: List<Int>, list_cons: List<Int>, list_empty: Bool, list_folded: String, ",
            "list_reversed: List<Int>, list_mapped: List<String>, list_filtered: List<Int>, ",
            "list_tried: Result<List<String>, AppError>, ",
            "found: Option<Int>, has_key: Bool, inserted: Dict<String, Int>, removed: Dict<String, Int>, ",
            "dict_mapped: Dict<String, String>, dict_filtered: Dict<String, Int>, ",
            "dict_folded: String, dict_tried: Result<Dict<String, String>, AppError>, ",
            "dict_mapped_with: Dict<String, String>, dict_filtered_with: Dict<String, Int>, ",
            "dict_folded_with: String, dict_tried_with: Result<Dict<String, String>, AppError>, ",
            "opt_mapped: Option<String>, opt_nexted: Option<String>, opt_value: Int, ",
            "res_mapped: Result<String, AppError>, res_err: Result<Int, String>, ",
            "res_nexted: Result<String, AppError>}\n",
            "  {count: vec_len(items), empty: vec_is_empty(items), ",
            "byte_value: byte(1), byte_int: byte_to_int(one_byte), ",
            "chunk_value: byte_chunk([one_byte]), chunk_count: byte_chunk_count(chunk), ",
            "appended: byte_append(chunk, other_chunk), hex_chunk: byte_chunk_from_hex(\"00 ff\"), ",
            "ascii_text: byte_chunk_to_visible_ascii_string(chunk), ascii_chunk: byte_chunk_from_visible_ascii_string(\"A\"), ",
            "taken: byte_take(chunk, count), ",
            "dropped: byte_drop(chunk, count), view_value: byte_view(chunk, offset, count), ",
            "view_chunk: byte_view_to_chunk(view), view_count: byte_view_count(view), ",
            "view_taken: byte_view_take(view, count), view_dropped: byte_view_drop(view, count), ",
            "view_slice: byte_view_slice(view, count, count), empty_chunks: byte_chunks_empty(), ",
            "one_chunk: byte_chunks_one(chunk), appended_chunks: byte_chunks_append(byte_chunks_one(chunk), byte_chunks_one(other_chunk)), ",
            "produced_chunks: byte_chunks_produce(byte_chunks_append(byte_chunks_one(chunk), byte_chunks_one(other_chunk)), count), ",
            "read_u8: byte_read_u8_be(view), ",
            "expect_u8: byte_expect_fixed_u8_be(view, 1, \"DemoPacket\", \"kind\"), ",
            "decoded_frame: prelude_builtin::byte_decode_http2_frame(view), ",
            "decoded_widths: byte_decode_schema_width_sample(view), ",
            "decoded_validation: byte_decode_schema_validation_sample(view), ",
            "closed_http2: prelude_builtin::http2_protocol_closed_with_pending(0, 4, \"none\", 0, 0, 0, 0, \"none\", view), ",
            "partial_preface_http2: prelude_builtin::http2_protocol_partial_preface(0, 6, view), ",
            "invalid_preface_http2: prelude_builtin::http2_protocol_invalid_preface(4, 42, 43, 4, view), ",
            "initial_peer_settings_http2: prelude_builtin::http2_protocol_initial_peer_settings_required(24, 6, 1, 0, \"server\", \"expect-initial-peer-settings\", \"rfc9113_initial_peer_frame_requires_non_ack_settings\", view), ",
            "continuation_http2: prelude_builtin::http2_protocol_continuation_expected(9, 0, 1, 1, 1, 0, \"headers\", 3, \"rfc9113_continuation_sequence\", view), ",
            "invalid_kind_http2: prelude_builtin::http2_protocol_invalid_frame_kind(0, 0, 0, 4, \"connection-control\", \"connection_frames_require_settings\", view), ",
            "invalid_stream_http2: prelude_builtin::http2_protocol_invalid_stream_id(0, 1, 2, \"nonzero client-initiated stream id\", \"server\", \"stream-id-domain\", \"server_receives_client_initiated_streams\", view), ",
            "invalid_payload_http2: prelude_builtin::http2_protocol_invalid_payload_length(0, 6, 0, 7, 8, \"connection-control\", \"rfc9113_ping_payload_length\", view), ",
            "invalid_payload_chunk_http2: prelude_builtin::http2_protocol_invalid_payload_length_chunk(0, 6, 0, 7, 8, \"connection-control\", \"rfc9113_ping_payload_length\", chunk), ",
            "invalid_window_update_increment_http2: prelude_builtin::http2_protocol_invalid_window_update_increment(0, 0, 0, 1, 2147483647, \"connection-flow-control\", \"window_update_increment_nonzero\", view), ",
            "invalid_data_padding_http2: prelude_builtin::http2_protocol_invalid_data_padding(9, 1, 2, 0, \"open-stream\", \"rfc9113_data_padding\", view), ",
            "content_length_http2: prelude_builtin::http2_protocol_content_length_mismatch(9, 0, 1, 5, 3, \"open-stream\", \"rfc9113_content_length_body\", view), ",
            "invalid_request_headers_http2: prelude_builtin::http2_protocol_invalid_request_header_list(12, 9, 1, \"missing_required_pseudo_header\", \":method\", \":scheme,:path\", \"request-headers\", \"rfc9113_request_pseudo_headers\", view), ",
            "invalid_response_headers_http2: prelude_builtin::http2_protocol_invalid_response_header_list(12, 9, 1, \"missing_required_pseudo_header\", \":status\", \"server\", \"response-headers\", \"rfc9113_response_pseudo_headers\", view), ",
            "unexpected_settings_ack_http2: prelude_builtin::http2_protocol_unexpected_settings_ack(0, \"connection-control\", \"rfc9113_settings_ack_requires_outstanding_local_settings\", view), ",
            "settings_endpoint_role_http2: prelude_builtin::http2_protocol_settings_not_allowed_for_endpoint(15, 2, \"SETTINGS_ENABLE_PUSH\", \"client\", 4, \"peer-settings\", \"rfc9113_client_must_not_receive_settings_enable_push\", view), ",
            "invalid_priority_dependency_http2: prelude_builtin::http2_protocol_invalid_priority_dependency(0, 1, 1, \"stream-control\", \"rfc9113_priority_dependency\", view), ",
            "stream_after_goaway_http2: prelude_builtin::http2_protocol_stream_after_goaway(9, 7, 5, \"graceful_shutdown\", \"server\", \"goaway_last_stream_id\", view), ",
            "frame_size_http2: prelude_builtin::http2_peer_limit_frame_size_exceeded(0, 16385, 16384, 0, 3, \"protocol_default\", view), ",
            "header_list_http2: prelude_builtin::http2_peer_limit_header_list_size_exceeded(12, 10, 9, 9, 1, \"local_configuration\", \"header_list_receive_limit\", view), ",
            "header_table_http2: prelude_builtin::http2_peer_limit_header_table_size_exceeded(35, 289, 160, 9, 1, \"local_configuration\", \"hpack_dynamic_table_size_update\", view), ",
            "flow_control_http2: prelude_builtin::http2_peer_limit_flow_control_window_exceeded(0, 4, 3, 0, 1, \"open-stream\", \"stream_receive_window\", view), ",
            "concurrent_streams_http2: prelude_builtin::http2_peer_limit_concurrent_streams_exceeded(9, 3, 2, 1, \"server\", \"open-stream\", \"local_configuration\", \"peer_created_stream_receive_limit\", view), ",
            "settings_value_http2: prelude_builtin::http2_peer_limit_settings_value_out_of_range(9, 5, \"SETTINGS_MAX_FRAME_SIZE\", 16383, 16384, 16777215, \"peer_settings\", view), ",
            "hpack_fixture: prelude_builtin::hpack_fixture_unsupported_header_block(27, 1, 255, \"fixture header block\", \"hpack_fixture\", view), ",
            "hpack_static_index: prelude_builtin::hpack_fixture_unsupported_static_index(27, 1, 128, \"fixture HPACK static indexed header\", \"hpack_fixture\", view), ",
            "hpack_string_length: prelude_builtin::hpack_fixture_malformed_string_length(27, 2, 4, \"fixture HPACK string length\", \"hpack_fixture\", view), ",
            "hpack_raw_string: prelude_builtin::hpack_fixture_malformed_raw_string_value(27, 5, 8, \"fixture HPACK raw string value\", \"hpack_fixture\", view), ",
            "hpack_padding: prelude_builtin::hpack_fixture_malformed_huffman_padding(27, 3, 4, \"fixture HPACK Huffman padding\", \"hpack_fixture\", view), ",
            "hpack_eos: prelude_builtin::hpack_fixture_huffman_eos_symbol(27, 6, 4, \"fixture HPACK Huffman data symbol instead of EOS\", \"hpack_fixture\", view), ",
            "hpack_visible: prelude_builtin::hpack_fixture_huffman_non_visible_value(27, 4, 4, \"fixture HPACK Huffman visible ASCII header value\", \"hpack_fixture\", view), ",
            "hpack_table_update_malformed: prelude_builtin::hpack_fixture_table_size_update_malformed(27, 2, 63, \"fixture HPACK malformed table-size update integer\", \"hpack_fixture\", view), ",
            "hpack_dynamic_index: prelude_builtin::hpack_fixture_dynamic_index_out_of_range(27, 1, 190, 0, 0, \"fixture dynamic indexed header\", \"hpack_fixture\", view), ",
            "hpack_dynamic_name_missing: prelude_builtin::hpack_fixture_dynamic_name_continuation_missing(27, 8, 127, 1, 0, \"fixture dynamic-name continuation entry\", \"hpack_fixture\", view), ",
            "hpack_dynamic_name_malformed: prelude_builtin::hpack_fixture_dynamic_name_continuation_malformed(27, 2, 127, -1, 3, \"fixture dynamic-name continuation integer\", \"hpack_fixture\", view), ",
            "hpack_dynamic_name_out_of_range: prelude_builtin::hpack_fixture_dynamic_name_continuation_out_of_range(27, 8, 127, 3, 3, \"fixture dynamic-name continuation range\", \"hpack_fixture\", view), ",
            "hpack_table_update_placement: prelude_builtin::hpack_fixture_table_size_update_not_at_start(10, 2, 62, 30, 1, 1, \"hpack-fixture\", \"fixture HPACK table-size update at header block start\", \"hpack_fixture\", view), ",
            "hpack_table_update_trailing: prelude_builtin::hpack_fixture_table_size_update_trailing_bytes(10, 3, 63, 33, 1, 1, \"hpack-fixture\", \"fixture HPACK table-size update without trailing bytes\", \"hpack_fixture\", view), ",
            "read_u16: byte_read_u16_be(view), read_u24: byte_read_u24_be(view), ",
            "read_u31: byte_read_u31_be(view), read_u32: byte_read_u32_be(view), ",
            "read_u40: byte_read_u40_be(view), read_u48: byte_read_u48_be(view), ",
            "read_u56: byte_read_u56_be(view), read_u64: byte_read_u64_be(view), ",
            "read_u16_le: byte_read_u16_le(view), read_u24_le: byte_read_u24_le(view), ",
            "read_u31_le: byte_read_u31_le(view), read_u32_le: byte_read_u32_le(view), ",
            "read_u40_le: byte_read_u40_le(view), read_u48_le: byte_read_u48_le(view), ",
            "read_u56_le: byte_read_u56_le(view), read_u64_le: byte_read_u64_le(view), ",
            "write_u8: byte_write_u8_be(1), write_u16: byte_write_u16_be(1), ",
            "write_u24: byte_write_u24_be(1), write_u31: byte_write_u31_be(1), ",
            "write_u32: byte_write_u32_be(1), write_u40: byte_write_u40_be(1), ",
            "write_u48: byte_write_u48_be(1), write_u56: byte_write_u56_be(1), ",
            "write_u64: byte_write_u64_be(1), ",
            "write_u16_le: byte_write_u16_le(1), ",
            "write_u24_le: byte_write_u24_le(1), write_u31_le: byte_write_u31_le(1), ",
            "write_u32_le: byte_write_u32_le(1), write_u40_le: byte_write_u40_le(1), ",
            "write_u48_le: byte_write_u48_le(1), write_u56_le: byte_write_u56_le(1), ",
            "write_u64_le: byte_write_u64_le(1), ",
            "count_value: byte_count(1), ",
            "count_int: byte_count_to_int(count), offset_value: byte_offset(1), ",
            "offset_int: byte_offset_to_int(offset), ",
            "pushed: vec_push(items, 1), joined: vec_concat(items, other), ",
            "mapped: vec_map(items, mapper), filtered: vec_filter(items, keep), ",
            "folded: vec_fold(items, \"\", folder), tried: vec_try_map(items, fallible), ",
            "tried_with: vec_try_map_with(\"prefix\", items, fallible_with), ",
            "split: string_split_once(\"sku,2\", \",\"), parsed: string_parse_int(\"2\"), ",
            "rendered: int_to_string(2), ",
            "list_nil: list_nil(), list_cons: list_cons(1, list_nil()), ",
            "list_empty: list_is_empty(list), list_folded: list_fold(list, \"\", folder), ",
            "list_reversed: list_reverse(list), list_mapped: list_map(list, mapper), ",
            "list_filtered: list_filter(list, keep), list_tried: list_try_map(list, fallible), ",
            "found: dict_get(table, \"a\"), has_key: dict_contains(table, \"a\"), ",
            "inserted: dict_insert(table, \"b\", 2), removed: dict_remove(table, \"b\"), ",
            "dict_mapped: dict_map(table, dict_mapper), dict_filtered: dict_filter(table, dict_keep), ",
            "dict_folded: dict_fold(table, \"\", dict_folder), dict_tried: dict_try_map(table, dict_fallible), ",
            "dict_mapped_with: dict_map_with(\"ctx\", table, dict_mapper_with), ",
            "dict_filtered_with: dict_filter_with(2, table, dict_keep_with), ",
            "dict_folded_with: dict_fold_with(\"ctx\", table, \"\", dict_folder_with), ",
            "dict_tried_with: dict_try_map_with(\"ctx\", table, dict_fallible_with), ",
            "opt_mapped: option_map(opt, opt_map), opt_nexted: option_and_then(opt, opt_next), ",
            "opt_value: option_unwrap_or(opt, 0), res_mapped: result_map(res, opt_map), ",
            "res_err: result_map_err(res, err_map), res_nexted: result_and_then(res, res_next)}\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Record(fields) = &expr.kind else {
        panic!("prelude results should be returned in a record");
    };
    let first = fields
        .first()
        .expect("record should contain prelude result fields");
    assert!(matches!(
        &first.expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "vec_len"
    ));
    assert!(matches!(first.expr.ty, CoreType::Named { ref name, .. } if name == "Int"));
    let compiler_adapter_names = crate::standard_symbols::compiler_adapter_names()
        .filter(|name| *name != "stream_adapter_drain_actions")
        .filter(|name| *name != "stream_adapter_accept_loop")
        .filter(|name| *name != "stream_adapter_drain_actions_until_cancellable")
        .collect::<Vec<_>>();
    let core_prelude_calls = fields
        .iter()
        .filter_map(|field| match &field.expr.kind {
            CoreExprKind::Call {
                target: CoreCallTarget::PreludeBuiltin(name),
                ..
            } => Some(name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for name in &compiler_adapter_names {
        assert!(
            core_prelude_calls.contains(name),
            "{name} should keep prelude core lowering"
        );
    }
    let ir = lowered
        .ir
        .expect("complete prelude core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    let IrExprKind::Record(fields) = &value.kind else {
        panic!("prelude record should lower to IR");
    };
    assert!(matches!(
        &fields[0].value.kind,
        IrExprKind::Call {
            target: IrCallTarget::PreludeBuiltin(name),
            ..
        } if name == "vec_len"
    ));
    let ir_prelude_calls = fields
        .iter()
        .filter_map(|field| match &field.value.kind {
            IrExprKind::Call {
                target: IrCallTarget::PreludeBuiltin(name),
                ..
            } => Some(name.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    for name in &compiler_adapter_names {
        assert!(
            ir_prelude_calls.contains(name),
            "{name} should keep prelude IR lowering"
        );
    }
}
