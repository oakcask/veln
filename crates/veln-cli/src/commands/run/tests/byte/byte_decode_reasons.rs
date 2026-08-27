use super::*;

#[test]
fn byte_result_failure_diagnostic_projects_decode_error_reason() {
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
                    ("name", JsonValue::string("kind")),
                ]),
            ]),
        ),
        (
            "reason",
            JsonValue::string("kind value exceeds declared length"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.kind"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.invalid_input, ByteOffset(62), ManualPacketWire.kind, kind value exceeds declared length)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.invalid_input");
    assert_eq!(diagnostic.message, "decode error at byte offset 62");
    assert_eq!(diagnostic.related.len(), 3);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Decode failure reason: kind value exceeds declared length.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.invalid_input, ByteOffset(62), ManualPacketWire.kind, kind value exceeds declared length).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_checksum_mismatch_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.checksum_mismatch")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(12)),
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
                    ("name", JsonValue::string("checksum")),
                ]),
            ]),
        ),
        ("expected_checksum", JsonValue::string("0xabcd")),
        ("actual_checksum", JsonValue::string("0x1234")),
        (
            "reason",
            JsonValue::string("payload checksum did not match header checksum"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.checksum"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.checksum_mismatch, ByteOffset(12), ManualPacketWire.checksum, expected_checksum=0xabcd; actual_checksum=0x1234; reason=payload checksum did not match header checksum)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.checksum_mismatch");
    assert_eq!(diagnostic.message, "checksum mismatch at byte offset 12");
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Expected checksum `0xabcd`; actual checksum was `0x1234`.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Checksum failure reason: payload checksum did not match header checksum.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.checksum_mismatch, ByteOffset(12), ManualPacketWire.checksum, expected_checksum=0xabcd; actual_checksum=0x1234; reason=payload checksum did not match header checksum).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_length_mismatch_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.length_mismatch")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
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
        ("expected_length", JsonValue::Number(4)),
        ("actual_length", JsonValue::Number(3)),
        (
            "reason",
            JsonValue::string("payload length did not match header length"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.payload"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.length_mismatch, ByteOffset(9), ManualPacketWire.payload, expected_length=4; actual_length=3; reason=payload length did not match header length)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.length_mismatch");
    assert_eq!(diagnostic.message, "length mismatch at byte offset 9");
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Expected length 4; actual length was 3.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Length mismatch reason: payload length did not match header length.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.length_mismatch, ByteOffset(9), ManualPacketWire.payload, expected_length=4; actual_length=3; reason=payload length did not match header length).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_payload_length_mismatch_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.payload_length_mismatch")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(21)),
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
        ("expected_payload_length", JsonValue::Number(8)),
        ("actual_payload_length", JsonValue::Number(5)),
        (
            "reason",
            JsonValue::string("payload length did not match frame header"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.payload"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.payload_length_mismatch, ByteOffset(21), ManualPacketWire.payload, expected_payload_length=8; actual_payload_length=5; reason=payload length did not match frame header)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.payload_length_mismatch");
    assert_eq!(
        diagnostic.message,
        "payload length mismatch at byte offset 21"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Expected payload length 8; actual payload length was 5.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Payload length mismatch reason: payload length did not match frame header.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.payload_length_mismatch, ByteOffset(21), ManualPacketWire.payload, expected_payload_length=8; actual_payload_length=5; reason=payload length did not match frame header).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_padding_mismatch_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.padding_mismatch")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(24)),
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
                    ("name", JsonValue::string("padding")),
                ]),
            ]),
        ),
        ("expected_padding_length", JsonValue::Number(2)),
        ("actual_padding_length", JsonValue::Number(5)),
        (
            "reason",
            JsonValue::string("DATA padding did not match payload boundary"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.padding"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.padding_mismatch, ByteOffset(24), ManualPacketWire.padding, expected_padding_length=2; actual_padding_length=5; reason=DATA padding did not match payload boundary)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.padding_mismatch");
    assert_eq!(diagnostic.message, "padding mismatch at byte offset 24");
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Expected padding length 2; actual padding length was 5.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Padding mismatch reason: DATA padding did not match payload boundary.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.padding_mismatch, ByteOffset(24), ManualPacketWire.padding, expected_padding_length=2; actual_padding_length=5; reason=DATA padding did not match payload boundary).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_integer_out_of_range_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.integer_out_of_range")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(17)),
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
                    ("name", JsonValue::string("stream_id")),
                ]),
            ]),
        ),
        ("byte_width", JsonValue::Number(4)),
        ("min_value", JsonValue::Number(0)),
        ("max_value", JsonValue::Number(2147483647)),
        ("actual_value", JsonValue::Number(2147483648)),
        (
            "reason",
            JsonValue::string("decoded value exceeds signed integer range"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.stream_id"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.integer_out_of_range, ByteOffset(17), ManualPacketWire.stream_id, byte_width=4; min_value=0; max_value=2147483647; actual_value=2147483648; reason=decoded value exceeds signed integer range)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.integer_out_of_range");
    assert_eq!(diagnostic.message, "integer out of range at byte offset 17");
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"4-byte integer expected value between 0 and 2147483647; actual value was 2147483648.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Integer conversion reason: decoded value exceeds signed integer range.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.integer_out_of_range, ByteOffset(17), ManualPacketWire.stream_id, byte_width=4; min_value=0; max_value=2147483647; actual_value=2147483648; reason=decoded value exceeds signed integer range).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_sequence_mismatch_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.sequence_mismatch")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(13)),
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
                    ("name", JsonValue::string("sequence")),
                ]),
            ]),
        ),
        (
            "expected_sequence",
            JsonValue::string("client_preface,settings"),
        ),
        ("actual_sequence", JsonValue::string("settings")),
        (
            "reason",
            JsonValue::string("frame sequence violated protocol state"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.sequence"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.sequence_mismatch, ByteOffset(13), ManualPacketWire.sequence, expected_sequence=client_preface,settings; actual_sequence=settings; reason=frame sequence violated protocol state)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.sequence_mismatch");
    assert_eq!(diagnostic.message, "sequence mismatch at byte offset 13");
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Expected sequence `client_preface,settings`; actual sequence was `settings`.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Sequence mismatch reason: frame sequence violated protocol state.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.sequence_mismatch, ByteOffset(13), ManualPacketWire.sequence, expected_sequence=client_preface,settings; actual_sequence=settings; reason=frame sequence violated protocol state).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_tag_mismatch_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.tag_mismatch")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(14)),
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
                    ("name", JsonValue::string("kind")),
                ]),
            ]),
        ),
        ("expected_tag", JsonValue::string("DATA")),
        ("actual_tag", JsonValue::string("HEADERS")),
        (
            "reason",
            JsonValue::string("dispatch tag did not match selected payload"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.kind"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.tag_mismatch, ByteOffset(14), ManualPacketWire.kind, expected_tag=DATA; actual_tag=HEADERS; reason=dispatch tag did not match selected payload)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.tag_mismatch");
    assert_eq!(diagnostic.message, "tag mismatch at byte offset 14");
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Expected tag `DATA`; actual tag was `HEADERS`.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Tag mismatch reason: dispatch tag did not match selected payload.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.tag_mismatch, ByteOffset(14), ManualPacketWire.kind, expected_tag=DATA; actual_tag=HEADERS; reason=dispatch tag did not match selected payload).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_magic_mismatch_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.magic_mismatch")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(18)),
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
                    ("name", JsonValue::string("magic")),
                ]),
            ]),
        ),
        ("expected_magic", JsonValue::string("VELN")),
        ("actual_magic", JsonValue::string("VEIN")),
        (
            "reason",
            JsonValue::string("file magic did not match expected signature"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.magic"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.magic_mismatch, ByteOffset(18), ManualPacketWire.magic, expected_magic=VELN; actual_magic=VEIN; reason=file magic did not match expected signature)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.magic_mismatch");
    assert_eq!(diagnostic.message, "magic mismatch at byte offset 18");
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Expected magic `VELN`; actual magic was `VEIN`.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Magic mismatch reason: file magic did not match expected signature.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.magic_mismatch, ByteOffset(18), ManualPacketWire.magic, expected_magic=VELN; actual_magic=VEIN; reason=file magic did not match expected signature).\"}"
    );
}
