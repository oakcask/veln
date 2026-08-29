use super::*;

#[test]
fn expected_output_mismatch_marks_case_failed() {
    let source_file = SourceFile::new(
        "main.veln#doctest-1_test.veln",
        "test doctest_1() -> () effects [stdio]\n  ()\nend\n",
    );
    let mut case = TestCase {
        id: "case-1".to_string(),
        name: "doctest_1".to_string(),
        kind: "doctest".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main.veln#doctest-1_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: source_file.span(TextRange::new(0, source_file.len())),
        },
        reason: None,
        failure: None,
        expected_output: Some(ExpectedOutput {
            stdout: Some("ready".to_string()),
            stderr: None,
            ..ExpectedOutput::default()
        }),
        expected_runtime_failure: None,
        events: vec![stdio_event(
            "stdout",
            "println",
            "waiting",
            "newline",
            1,
            "call-1",
            &source_file.span(TextRange::new(0, source_file.len())),
        )],
        diagnostics: Vec::new(),
    };

    compare_expected_output(&mut case);

    assert_eq!(case.status, TestCaseStatus::Failed);
    assert_eq!(case.reason.as_deref(), Some("expected_output"));
    let failure = case.failure.expect("mismatch should create failure");
    assert_eq!(failure.kind, "output");
    assert_eq!(failure.message, "expected stdout output did not match");
    let failure_json = failure.to_json().to_json();
    assert!(failure_json.contains("\"actual\":\"waiting\\n\""));
    assert!(failure_json.contains(
        "\"first_difference\":{\"line\":1,\"expected\":\"ready\",\"actual\":\"waiting\"}"
    ));
    assert!(failure_json.contains("\"actual_events\":[{\"kind\":\"stdio\""));
}

#[test]
fn expected_output_match_normalizes_line_endings_and_keeps_case_passed() {
    let source_file = SourceFile::new(
        "main.veln#doctest-1_test.veln",
        "test doctest_1() -> () effects [stdio]\n  ()\nend\n",
    );
    let mut case = TestCase {
        id: "case-1".to_string(),
        name: "doctest_1".to_string(),
        kind: "doctest".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main.veln#doctest-1_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: source_file.span(TextRange::new(0, source_file.len())),
        },
        reason: None,
        failure: None,
        expected_output: Some(ExpectedOutput {
            stdout: Some("ready\nnext".to_string()),
            stderr: Some("warn".to_string()),
            ..ExpectedOutput::default()
        }),
        expected_runtime_failure: None,
        events: vec![
            stdio_event(
                "stdout",
                "print",
                "ready\r\nnext\n",
                "none",
                1,
                "call-1",
                &source_file.span(TextRange::new(0, source_file.len())),
            ),
            stdio_event(
                "stderr",
                "eprint",
                "warn\r\n",
                "none",
                2,
                "call-2",
                &source_file.span(TextRange::new(0, source_file.len())),
            ),
        ],
        diagnostics: Vec::new(),
    };

    compare_expected_output(&mut case);

    assert_eq!(case.status, TestCaseStatus::Passed);
    assert!(case.reason.is_none());
    assert!(case.failure.is_none());
}

#[test]
fn expected_output_mismatch_reports_missing_actual_line() {
    let source_file = SourceFile::new(
        "main.veln#doctest-1_test.veln",
        "test doctest_1() -> () effects [stdio]\n  ()\nend\n",
    );
    let mut case = TestCase {
        id: "case-1".to_string(),
        name: "doctest_1".to_string(),
        kind: "doctest".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main.veln#doctest-1_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: source_file.span(TextRange::new(0, source_file.len())),
        },
        reason: None,
        failure: None,
        expected_output: Some(ExpectedOutput {
            stdout: Some("ready\nnext".to_string()),
            stderr: None,
            ..ExpectedOutput::default()
        }),
        expected_runtime_failure: None,
        events: vec![stdio_event(
            "stdout",
            "println",
            "ready",
            "newline",
            1,
            "call-1",
            &source_file.span(TextRange::new(0, source_file.len())),
        )],
        diagnostics: Vec::new(),
    };

    compare_expected_output(&mut case);

    assert_eq!(case.status, TestCaseStatus::Failed);
    let failure = case.failure.expect("mismatch should create failure");
    assert!(
        failure
            .to_json()
            .to_json()
            .contains("\"first_difference\":{\"line\":2,\"expected\":\"next\",\"actual\":null}")
    );
}

#[test]
fn expected_output_mismatch_reports_extra_actual_line_and_expected_span() {
    let source_file = SourceFile::new(
        "main.veln#doctest-1_test.veln",
        "test doctest_1() -> () effects [stdio]\n  ()\nend\n",
    );
    let span = source_file.span(TextRange::new(0, source_file.len()));
    let mut case = TestCase {
        id: "case-1".to_string(),
        name: "doctest_1".to_string(),
        kind: "doctest".to_string(),
        status: TestCaseStatus::Passed,
        source: TestCaseSource {
            file: "main.veln#doctest-1_test.veln".to_string(),
            node_id: "test-1".to_string(),
            span: span.clone(),
        },
        reason: None,
        failure: None,
        expected_output: Some(ExpectedOutput {
            stdout: Some("ready".to_string()),
            stdout_span: Some(span.clone()),
            stderr: None,
            ..ExpectedOutput::default()
        }),
        expected_runtime_failure: None,
        events: vec![
            stdio_event("stdout", "println", "ready", "newline", 1, "call-1", &span),
            stdio_event("stdout", "println", "next", "newline", 2, "call-2", &span),
        ],
        diagnostics: Vec::new(),
    };

    compare_expected_output(&mut case);

    assert_eq!(case.status, TestCaseStatus::Failed);
    let failure = case.failure.expect("mismatch should create failure");
    let failure_json = failure.to_json().to_json();
    assert!(
        failure_json
            .contains("\"first_difference\":{\"line\":2,\"expected\":null,\"actual\":\"next\"}")
    );
    assert!(failure_json.contains("\"expected_span\""));
}

#[test]
fn stdio_trace_events_preserve_operation_terminator_and_call_span() {
    let module = module(concat!(
        "test first() -> () effects [stdio]\n",
        "  stdio::println(\"out\")\n",
        "  stdio::eprint(\"err\")\n",
        "  ()\n",
        "end\n",
    ));
    let call_spans = stdio_call_spans(&module);
    let call_keys = call_spans.keys().cloned().collect::<Vec<_>>();
    let call_ids = call_keys
        .iter()
        .map(|(_, node_id)| node_id.clone())
        .collect::<Vec<_>>();
    assert_eq!(call_ids.len(), 2);
    let source = TestCaseSource {
        file: "main_test.veln".to_string(),
        node_id: "test-1".to_string(),
        span: module.functions[0].span.clone(),
    };
    let trace = format!(
        "1\tstdout\tprintln\tnewline\t{}\t{}\t6f7574\n2\tstderr\teprint\tnone\t{}\t{}\t657272\n",
        call_keys[0].1, call_keys[0].0, call_keys[1].1, call_keys[1].0
    );

    let events = stdio_events_from_trace(&trace, &call_spans, &source);

    assert_eq!(events.len(), 2);
    let first_event = events[0].to_json();
    assert!(first_event.contains("\"operation\":\"println\""));
    assert!(first_event.contains("\"text\":\"out\""));
    assert!(first_event.contains("\"terminator\":\"newline\""));
    assert!(first_event.contains(&format!("\"node_id\":\"{}\"", call_ids[0])));
    assert!(first_event.contains("\"file\":\"main_test.veln\""));
    assert!(first_event.contains("\"start\":{\"line\":2,\"column\":3"));
    assert!(events[1].to_json().contains("\"operation\":\"eprint\""));
    assert!(events[1].to_json().contains("\"terminator\":\"none\""));
}

#[test]
fn stdio_call_spans_include_type_applied_stdio_calls() {
    let module = module(concat!(
        "test first() -> () effects [stdio]\n",
        "  stdio::println<String>(\"out\")\n",
        "  ()\n",
        "end\n",
    ));

    let call_spans = stdio_call_spans(&module);

    assert_eq!(call_spans.len(), 1);
    let ((file, node_id), span) = call_spans
        .iter()
        .next()
        .expect("typed stdio call span should be recorded");
    assert_eq!(file, "main_test.veln");
    assert!(node_id.starts_with("call-"));
    assert_eq!(span.start.line, 2);
    assert_eq!(span.start.column, 3);
}

#[test]
fn stdio_call_spans_include_nested_aggregate_and_match_calls() {
    let module = module(concat!(
        "test first() -> () effects [stdio]\n",
        "  let record = {out: stdio::println(\"record\")}\n",
        "  let list = [stdio::println(\"list\")]\n",
        "  let dict = {\"out\": stdio::println(\"dict\")}\n",
        "  match true\n",
        "    true => stdio::println(\"match\")\n",
        "    false => ()\n",
        "  end\n",
        "end\n",
    ));

    let call_spans = stdio_call_spans(&module);
    let mut lines = call_spans
        .values()
        .map(|span| span.start.line)
        .collect::<Vec<_>>();
    lines.sort();

    assert_eq!(call_spans.len(), 4);
    assert_eq!(lines, vec![2, 3, 4, 6]);
    assert!(
        call_spans
            .keys()
            .all(|(file, node_id)| file == "main_test.veln" && node_id.starts_with("call-"))
    );
}

#[test]
fn stdio_trace_skips_malformed_lines() {
    let source_file = SourceFile::new("main_test.veln", "test first() -> ()\n  ()\nend\n");
    let source = TestCaseSource {
        file: "main_test.veln".to_string(),
        node_id: "test-1".to_string(),
        span: source_file.span(TextRange::new(0, source_file.len())),
    };

    let events = stdio_events_from_trace(
        concat!(
            "not-a-sequence\tstdout\tprint\tnone\t\t\t7265616479\n",
            "1\tstdout\tprint\tnone\t\t\tinvalid-hex\n",
            "2\tstdout\tprint\tnone\t\t\t6f6b\n",
        ),
        &BTreeMap::new(),
        &source,
    );

    assert_eq!(events.len(), 1);
    assert!(events[0].to_json().contains("\"sequence\":2"));
    assert!(events[0].to_json().contains("\"text\":\"ok\""));
}

#[test]
fn stdio_trace_decodes_uppercase_hex_text() {
    let source_file = SourceFile::new("main_test.veln", "test first() -> ()\n  ()\nend\n");
    let source = TestCaseSource {
        file: "main_test.veln".to_string(),
        node_id: "test-1".to_string(),
        span: source_file.span(TextRange::new(0, source_file.len())),
    };

    let events = stdio_events_from_trace(
        "1\tstdout\tprint\tnone\t\t\t4F4B\n",
        &BTreeMap::new(),
        &source,
    );

    assert_eq!(events.len(), 1);
    assert!(events[0].to_json().contains("\"text\":\"OK\""));
}

#[test]
fn contract_trace_becomes_structured_test_failure() {
    let trace = "contract\trequire\t66616c7365\t72656a65637473\tcaller\t636f6e74726163742d32\t6d61696e5f746573742e76656c6e\t2\t1\t2\t14\n";

    let failure = contract_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "contract");
    assert_eq!(
        failure.message,
        "contract failure: require `false` in `rejects` blame caller"
    );
    assert_eq!(
        failure.to_json().to_json(),
        concat!(
            "{\"kind\":\"contract\",\"message\":\"contract failure: require `false` in `rejects` blame caller\",",
            "\"expected\":null,\"actual\":null,\"span\":null,",
            "\"details\":{\"kind\":\"contract\",\"phase\":\"runtime\",",
            "\"clause\":\"require\",\"predicate\":\"false\",\"function\":\"rejects\",",
            "\"blame\":\"caller\",\"node_id\":\"contract-2\",",
            "\"span\":{\"file\":\"main_test.veln\",",
            "\"start\":{\"line\":2,\"column\":1,\"offset\":0},",
            "\"end\":{\"line\":2,\"column\":14,\"offset\":0}}}}"
        )
    );
}

#[test]
fn result_trace_becomes_structured_test_failure() {
    let trace = "result\t626164\n";

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(failure.message, "runtime result failure: Err(bad)");
    assert_eq!(
        failure.to_json().to_json(),
        concat!(
            "{\"kind\":\"result\",\"message\":\"runtime result failure: Err(bad)\",",
            "\"expected\":null,\"actual\":null,\"span\":null,",
            "\"details\":{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"bad\"}}"
        )
    );
}

#[test]
fn fixture_hex_result_trace_keeps_structured_details() {
    let trace = concat!(
        "result\t",
        "666978747572652e6865782e696e76616c69645f6368617261637465723a20",
        "657870656374656420415343494920686578206469676974206174206279746520",
        "6f666673657420312068696768206e6962626c65",
        "\tfixture_hex\tfixture.hex.invalid_character\t2\t3\t1\thigh\t0\t5\t30305f3031\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.message,
        "runtime result failure: Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 1 high nibble)"
    );
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"fixture.hex.invalid_character: expected ASCII hex digit at byte offset 1 high nibble\",",
            "\"fixture_hex\":{\"kind\":\"fixture_hex\",",
            "\"id\":\"fixture.hex.invalid_character\",",
            "\"fixture_text_span\":{\"start\":2,\"end\":3},",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":1},",
            "\"nibble_position\":\"high\",",
            "\"nearby_context\":{\"start\":0,\"end\":5,\"text\":\"00_01\"}}}"
        )
    );
}

#[test]
fn byte_diagnostic_result_trace_keeps_structured_details() {
    let trace = concat!(
        "result\t",
        "6279746520726561642072657175697265732033206279746573206275742076696577206861732032",
        "\tbyte_diagnostic\tcodec.incomplete_input\t2\t0\t3\t2\tneed_bytes\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.message,
        "runtime result failure: Err(byte read requires 3 bytes but view has 2)"
    );
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"byte read requires 3 bytes but view has 2\",",
            "\"byte_diagnostic\":{\"kind\":\"byte_diagnostic\",",
            "\"id\":\"codec.incomplete_input\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":2},",
            "\"field_path\":[],",
            "\"expected_count\":3,",
            "\"available_count\":2,",
            "\"readiness\":\"need_bytes\"}}"
        )
    );
}

#[test]
fn byte_diagnostic_result_trace_keeps_field_path_segments() {
    let trace = concat!(
        "result\t",
        "6279746520726561642072657175697265732033206279746573206275742076696577206861732032",
        "\tbyte_diagnostic\tcodec.incomplete_input\t2",
        "\t2\tschema\t48747470324672616d65486561646572\tfield\t6c656e677468",
        "\t3\t2\tneed_bytes\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"byte read requires 3 bytes but view has 2\",",
            "\"byte_diagnostic\":{\"kind\":\"byte_diagnostic\",",
            "\"id\":\"codec.incomplete_input\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":2},",
            "\"field_path\":[{\"kind\":\"schema\",\"name\":\"Http2FrameHeader\"},",
            "{\"kind\":\"field\",\"name\":\"length\"}],",
            "\"expected_count\":3,",
            "\"available_count\":2,",
            "\"readiness\":\"need_bytes\"}}"
        )
    );
}

#[test]
fn byte_diagnostic_v2_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "6669786564206669656c64206d69736d617463682061742062797465206f66667365742030",
        "\tbyte_diagnostic_v2\tschema.fixed_field_mismatch\t0",
        "\t2\tschema\t44656d6f5061636b6574\tfield\t6b696e64",
        "\t3\texpected_value\tnumber\t1",
        "\tactual_value\tnumber\t255",
        "\tbyte_preview\tbyte_preview\t666630303031\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"fixed field mismatch at byte offset 0\",",
            "\"byte_diagnostic\":{\"kind\":\"byte_diagnostic\",",
            "\"id\":\"schema.fixed_field_mismatch\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
            "\"field_path\":[{\"kind\":\"schema\",\"name\":\"DemoPacket\"},",
            "{\"kind\":\"field\",\"name\":\"kind\"}],",
            "\"expected_value\":1,",
            "\"actual_value\":255,",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"ff0001\",",
            "\"preview_byte_count\":3,",
            "\"total_byte_count\":3,",
            "\"truncated\":false}}}"
        )
    );
}

#[test]
fn byte_diagnostic_v2_result_trace_decodes_preview_counts() {
    let trace = concat!(
        "result\t",
        "6669786564206669656c64206d69736d617463682061742062797465206f66667365742030",
        "\tbyte_diagnostic_v2\tschema.fixed_field_mismatch\t0",
        "\t2\tschema\t44656d6f5061636b6574\tfield\t6b696e64",
        "\t3\texpected_value\tnumber\t1",
        "\tactual_value\tnumber\t255",
        "\tbyte_preview\tbyte_preview_v2\t666630303031:3:7:true\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"fixed field mismatch at byte offset 0\",",
            "\"byte_diagnostic\":{\"kind\":\"byte_diagnostic\",",
            "\"id\":\"schema.fixed_field_mismatch\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":0},",
            "\"field_path\":[{\"kind\":\"schema\",\"name\":\"DemoPacket\"},",
            "{\"kind\":\"field\",\"name\":\"kind\"}],",
            "\"expected_value\":1,",
            "\"actual_value\":255,",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"ff0001\",",
            "\"preview_byte_count\":3,",
            "\"total_byte_count\":7,",
            "\"truncated\":true}}}"
        )
    );
}

#[test]
fn byte_diagnostic_v2_result_trace_keeps_range_details() {
    let trace = concat!(
        "result\t",
        "6279746520766965772072616e67652065786365656473206368756e6b206c656e677468",
        "\tbyte_diagnostic_v2\tcodec.byte_range_out_of_bounds\t2",
        "\t0",
        "\t3\trequested_count\tnumber\t2",
        "\tavailable_count\tnumber\t1",
        "\tbyte_preview\tbyte_preview_v2\t3032:1:1:false\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"byte view range exceeds chunk length\",",
            "\"byte_diagnostic\":{\"kind\":\"byte_diagnostic\",",
            "\"id\":\"codec.byte_range_out_of_bounds\",",
            "\"byte_offset\":{\"kind\":\"ByteOffset\",\"value\":2},",
            "\"field_path\":[],",
            "\"requested_count\":2,",
            "\"available_count\":1,",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"02\",",
            "\"preview_byte_count\":1,",
            "\"total_byte_count\":1,",
            "\"truncated\":false}}}"
        )
    );
}

#[test]
fn value_diagnostic_result_trace_keeps_value_details() {
    let trace = concat!(
        "result\t",
        "736368656d612076616c75652076616c69646174696f6e206661696c656420666f72206669656c64206070616464696e675f6c656e67746860",
        "\tvalue_diagnostic\tschema.validation_failed",
        "\t2\tschema\t4f7264696e6172795061636b6574\tfield\t70616464696e675f6c656e677468",
        "\t5\tpredicate\tstring\t70616464696e675f6c656e677468203c3d206c656e677468",
        "\tfield_value\tnumber\t6",
        "\tsupplied_values\tstring\t6c656e6774683d352c2070616464696e675f6c656e6774683d36",
        "\tlength\tnumber\t5",
        "\tpadding_length\tnumber\t6\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(failure.kind, "result");
    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"schema value validation failed for field `padding_length`\",",
            "\"value_diagnostic\":{\"kind\":\"value_diagnostic\",",
            "\"id\":\"schema.validation_failed\",",
            "\"field_path\":[{\"kind\":\"schema\",\"name\":\"OrdinaryPacket\"},",
            "{\"kind\":\"field\",\"name\":\"padding_length\"}],",
            "\"predicate\":\"padding_length <= length\",",
            "\"field_value\":6,",
            "\"supplied_values\":\"length=5, padding_length=6\",",
            "\"length\":5,",
            "\"padding_length\":6}}"
        )
    );
}

#[test]
fn value_diagnostic_result_trace_decodes_byte_preview_details() {
    let trace = concat!(
        "result\t",
        "456e636f64654572726f7228736368656d612e656e636f64655f76616c75655f756e726570726573656e7461626c652c205061636b6574576972652e7061796c6f61642c2062797465207669657720636f756e74203320646f6573206e6f74206d61746368206c656e677468206669656c6420606c656e677468602076616c7565203229",
        "\tvalue_diagnostic\tschema.encode_value_unrepresentable",
        "\t2\tschema\t5061636b657457697265\tfield\t7061796c6f6164",
        "\t7\treason\tstring\t62797465207669657720636f756e74203320646f6573206e6f74206d61746368206c656e677468206669656c6420606c656e677468602076616c75652032",
        "\tfield_path_display\tstring\t5061636b6574576972652e7061796c6f6164",
        "\texpected_count\tnumber\t2",
        "\tactual_count\tnumber\t3",
        "\tlength_expression\tstring\t6c656e677468",
        "\tbyte_offset\tnumber\t0",
        "\tbyte_preview\tbyte_preview_v2\t616162626363:3:3:false\n",
    );

    let failure = result_failure_from_trace(trace).expect("trace should decode");

    assert_eq!(
        failure.details.to_json(),
        concat!(
            "{\"kind\":\"result\",\"phase\":\"runtime\",",
            "\"value\":\"EncodeError(schema.encode_value_unrepresentable, PacketWire.payload, byte view count 3 does not match length field `length` value 2)\",",
            "\"value_diagnostic\":{\"kind\":\"value_diagnostic\",",
            "\"id\":\"schema.encode_value_unrepresentable\",",
            "\"field_path\":[{\"kind\":\"schema\",\"name\":\"PacketWire\"},",
            "{\"kind\":\"field\",\"name\":\"payload\"}],",
            "\"reason\":\"byte view count 3 does not match length field `length` value 2\",",
            "\"field_path_display\":\"PacketWire.payload\",",
            "\"expected_count\":2,",
            "\"actual_count\":3,",
            "\"length_expression\":\"length\",",
            "\"byte_offset\":0,",
            "\"byte_preview\":{\"encoding\":\"hex\",",
            "\"data\":\"aabbcc\",",
            "\"preview_byte_count\":3,",
            "\"total_byte_count\":3,",
            "\"truncated\":false}}}"
        )
    );
}
