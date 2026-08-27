use super::*;

#[test]
fn byte_result_failure_diagnostic_projects_decode_error_byte_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.invalid_input")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(62)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("ManualPacketWire")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("payload")),
                ]),
            ]),
        ),
        (
            "reason",
            JsonValue::string("byte view range exceeds view length"),
        ),
        ("local_byte_offset", JsonValue::Number(2)),
        ("expected_count", JsonValue::Number(4)),
        ("available_count", JsonValue::Number(1)),
        ("byte_preview", byte_preview("07")),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.payload"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.invalid_input, ByteOffset(62), ManualPacketWire.payload, byte view range exceeds view length)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.invalid_input");
    assert_eq!(diagnostic.message, "decode error at byte offset 62");
    assert_eq!(diagnostic.related.len(), 6);
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Local byte offset: 2.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"Decoder expected 4 byte(s); 1 byte(s) were available.\"}"
    );
    assert_eq!(
        diagnostic.related[4].to_json(),
        "{\"message\":\"Nearby bytes: 07 (showing 1 of 1 byte(s), complete).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_decode_need_more_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.incomplete_input")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(3)),
            ]),
        ),
        ("field_path", JsonValue::array([])),
        ("readiness", JsonValue::string("need_bytes")),
        ("needed_count", JsonValue::Number(3)),
    ]);
    let failure = TestFailure::result_with_details(
        "NeedMore(NeedBytes(ByteCount(3)))".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.incomplete_input");
    assert_eq!(diagnostic.message, "incomplete input at byte offset 3");
    assert_eq!(diagnostic.related.len(), 3);
    assert_eq!(
        diagnostic.related[0].to_json(),
        "{\"message\":\"Decode readiness is `need_bytes` because input is closed.\"}"
    );
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Decoder needs at least 3 buffered byte(s) before retrying.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"DecodeStep value: NeedMore(NeedBytes(ByteCount(3))).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_decode_need_end_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.incomplete_input")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        ("field_path", JsonValue::array([])),
        ("readiness", JsonValue::string("need_end")),
    ]);
    let failure = TestFailure::result_with_details(
        "NeedMore(NeedEnd)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.incomplete_input");
    assert_eq!(diagnostic.message, "incomplete input at byte offset 0");
    assert_eq!(diagnostic.related.len(), 2);
    assert_eq!(
        diagnostic.related[0].to_json(),
        "{\"message\":\"Decode readiness is `need_end` because input is closed.\"}"
    );
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"DecodeStep value: NeedMore(NeedEnd).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_fixed_field_mismatch_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("schema.fixed_field_mismatch")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("DemoPacket")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("kind")),
                ]),
            ]),
        ),
        ("expected_value", JsonValue::Number(1)),
        ("actual_value", JsonValue::Number(255)),
        ("byte_preview", byte_preview("ff0001")),
    ]);
    let failure = TestFailure::result_with_details(
        "fixed field mismatch at byte offset 0".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "schema.fixed_field_mismatch");
    assert_eq!(diagnostic.message, "fixed field mismatch at byte offset 0");
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("expected value 1; actual value was 255")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("ff 00 01 (showing 3 of 3 byte(s), complete)")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("schema `DemoPacket` / field `kind`")
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_truncated_schema_field_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("schema.truncated_field")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(6)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("Http2FrameHeader")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("stream_id")),
                ]),
            ]),
        ),
        ("expected_count", JsonValue::Number(4)),
        ("available_count", JsonValue::Number(1)),
        ("readiness", JsonValue::string("need_bytes")),
        ("byte_preview", byte_preview("000005010400")),
    ]);
    let failure = TestFailure::result_with_details(
        "truncated schema field `stream_id` at byte offset 6".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "schema.truncated_field");
    assert_eq!(
        diagnostic.message,
        "truncated schema field at byte offset 6"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(diagnostic.related[0].to_json().contains("need_bytes"));
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("expected 4 byte(s); 1 byte(s) were available")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("schema `Http2FrameHeader` / field `stream_id`")
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_reserved_bits_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("schema.reserved_bits_mismatch")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(5)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("Http2FrameHeader")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("stream_reserved")),
                ]),
            ]),
        ),
        ("bit_width", JsonValue::Number(1)),
        ("expected_value", JsonValue::Number(0)),
        ("actual_value", JsonValue::Number(1)),
        ("byte_preview", byte_preview("000005010480000001")),
    ]);
    let failure = TestFailure::result_with_details(
        "reserved bits mismatch at byte offset 5".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "schema.reserved_bits_mismatch");
    assert_eq!(
        diagnostic.message,
        "reserved bits mismatch at byte offset 5"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("ReservedBits(1, 0) expected value 0; actual value was 1")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("schema `Http2FrameHeader` / field `stream_reserved`")
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_payload_length_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("schema.length_out_of_bounds")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(11)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("Http2FrameHeader")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("payload")),
                ]),
            ]),
        ),
        ("expected_count", JsonValue::Number(5)),
        ("available_count", JsonValue::Number(2)),
        ("byte_preview", byte_preview("000005010400000001aabb")),
    ]);
    let failure = TestFailure::result_with_details(
        "payload length out of bounds at byte offset 11".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "schema.length_out_of_bounds");
    assert_eq!(
        diagnostic.message,
        "payload length out of bounds at byte offset 11"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("expected 5 byte(s); 2 byte(s) were available")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("schema `Http2FrameHeader` / field `payload`")
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_integer_range_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("schema.integer_out_of_range")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(0)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("StreamIdentifierSample")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("stream_id")),
                ]),
            ]),
        ),
        ("byte_width", JsonValue::Number(4)),
        ("min_value", JsonValue::Number(0)),
        ("max_value", JsonValue::Number(2147483647)),
        ("actual_value", JsonValue::Number(2147483648)),
        ("byte_preview", byte_preview("80000000")),
    ]);
    let failure = TestFailure::result_with_details(
        "schema integer out of range at byte offset 0".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "schema.integer_out_of_range");
    assert_eq!(
        diagnostic.message,
        "schema integer out of range at byte offset 0"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("expected value between 0 and 2147483647; actual value was 2147483648")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("schema `StreamIdentifierSample` / field `stream_id`")
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_validation_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("schema.validation_failed")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(3)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("SchemaValidationSample")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("padding_length")),
                ]),
            ]),
        ),
        ("predicate", JsonValue::string("padding_length <= length")),
        ("field_value", JsonValue::Number(6)),
        (
            "decoded_values",
            JsonValue::string("length=5, padding_length=6"),
        ),
        ("length", JsonValue::Number(5)),
        ("padding_length", JsonValue::Number(6)),
        ("byte_preview", byte_preview("00000506")),
    ]);
    let failure = TestFailure::result_with_details(
        "schema validation failed at byte offset 3".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "schema.validation_failed");
    assert_eq!(
        diagnostic.message,
        "schema validation failed at byte offset 3"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("padding_length <= length")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("length=5, padding_length=6")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("00 00 05 06 (showing 4 of 4 byte(s), complete)")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("schema `SchemaValidationSample` / field `padding_length`")
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_schema_level_validation_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("schema.validation_failed")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(3)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([JsonValue::object([
                ("kind", JsonValue::string("schema")),
                ("name", JsonValue::string("SchemaLevelValidationSample")),
            ])]),
        ),
        (
            "predicate",
            JsonValue::string("length == padding_length + checksum"),
        ),
        (
            "decoded_values",
            JsonValue::string("length=5, padding_length=2, checksum=4"),
        ),
        ("length", JsonValue::Number(5)),
        ("padding_length", JsonValue::Number(2)),
        ("checksum", JsonValue::Number(4)),
        ("byte_preview", byte_preview("050204")),
    ]);
    let failure = TestFailure::result_with_details(
        "schema validation failed at byte offset 3".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "schema.validation_failed");
    assert_eq!(
        diagnostic.message,
        "schema validation failed at byte offset 3"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("Schema predicate `length == padding_length + checksum` failed")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("schema `SchemaLevelValidationSample`")
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_length_division_by_zero_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("schema.length_division_by_zero")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(2)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("PacketWire")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("payload")),
                ]),
            ]),
        ),
        ("length_expression", JsonValue::string("length / divisor")),
        ("divisor_operand", JsonValue::string("divisor")),
        ("operator", JsonValue::string("/")),
        ("byte_preview", byte_preview("0800")),
    ]);
    let failure = TestFailure::result_with_details(
        "schema length division by zero at byte offset 2".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "schema.length_division_by_zero");
    assert_eq!(
        diagnostic.message,
        "schema length division by zero at byte offset 2"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("Length expression `length / divisor`")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("08 00 (showing 2 of 2 byte(s), complete)")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("schema `PacketWire` / field `payload`")
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_length_multiple_mismatch_context() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("schema.length_multiple_mismatch")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(2)),
            ]),
        ),
        (
            "field_path",
            JsonValue::array([
                JsonValue::object([
                    ("kind", JsonValue::string("schema")),
                    ("name", JsonValue::string("PacketWire")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("payload")),
                ]),
            ]),
        ),
        ("observed_count", JsonValue::Number(5)),
        ("required_multiple", JsonValue::Number(2)),
        ("multiple_operand", JsonValue::string("frame_count")),
        ("byte_preview", byte_preview("0502aabbccddee")),
    ]);
    let failure = TestFailure::result_with_details(
        "payload length multiple mismatch at byte offset 2".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "schema.length_multiple_mismatch");
    assert_eq!(
        diagnostic.message,
        "payload length multiple mismatch at byte offset 2"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("Payload count 5 must be a multiple of `frame_count` value 2")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("05 02 aa bb cc dd ee (showing 7 of 7 byte(s), complete)")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("schema `PacketWire` / field `payload`")
    );
}
