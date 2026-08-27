use super::*;

#[test]
fn byte_result_failure_diagnostic_projects_unsupported_feature_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.unsupported_feature")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(27)),
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
                    ("name", JsonValue::string("extension")),
                ]),
            ]),
        ),
        (
            "unsupported_feature",
            JsonValue::string("dynamic_table_size_update"),
        ),
        (
            "reason",
            JsonValue::string("dynamic table size updates are disabled for this profile"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.extension"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.unsupported_feature, ByteOffset(27), ManualPacketWire.extension, feature=dynamic_table_size_update; reason=dynamic table size updates are disabled for this profile)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.unsupported_feature");
    assert_eq!(
        diagnostic.message,
        "unsupported feature failed at byte offset 27"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Unsupported feature: `dynamic_table_size_update`.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Unsupported feature reason: dynamic table size updates are disabled for this profile.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.unsupported_feature, ByteOffset(27), ManualPacketWire.extension, feature=dynamic_table_size_update; reason=dynamic table size updates are disabled for this profile).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_trailing_input_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.trailing_input")),
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
                    ("name", JsonValue::string("ManualPacketWire")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("payload")),
                ]),
            ]),
        ),
        ("consumed_count", JsonValue::Number(5)),
        ("available_count", JsonValue::Number(8)),
        ("remaining_count", JsonValue::Number(3)),
        (
            "reason",
            JsonValue::string("packet decoder completed before the bounded input ended"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.payload"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.trailing_input, ByteOffset(5), ManualPacketWire.payload, consumed_count=5; available_count=8; remaining_count=3; reason=packet decoder completed before the bounded input ended)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.trailing_input");
    assert_eq!(diagnostic.message, "trailing input at byte offset 5");
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Consumed 5 of 8 available bytes; 3 bytes remain.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Trailing input reason: packet decoder completed before the bounded input ended.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.trailing_input, ByteOffset(5), ManualPacketWire.payload, consumed_count=5; available_count=8; remaining_count=3; reason=packet decoder completed before the bounded input ended).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_preserves_plain_trailing_input_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.trailing_input")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(7)),
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
            JsonValue::string("bounded input has trailing bytes"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.payload"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.trailing_input, ByteOffset(7), ManualPacketWire.payload, bounded input has trailing bytes)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.trailing_input");
    assert_eq!(diagnostic.message, "trailing input at byte offset 7");
    assert_eq!(diagnostic.related.len(), 3);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Trailing input reason: bounded input has trailing bytes.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.trailing_input, ByteOffset(7), ManualPacketWire.payload, bounded input has trailing bytes).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_version_mismatch_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.version_mismatch")),
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
                    ("name", JsonValue::string("ManualPacketWire")),
                ]),
                JsonValue::object([
                    ("kind", JsonValue::string("field")),
                    ("name", JsonValue::string("version")),
                ]),
            ]),
        ),
        ("expected_version", JsonValue::string("2")),
        ("actual_version", JsonValue::string("1")),
        (
            "reason",
            JsonValue::string("codec version is not supported"),
        ),
        (
            "field_path_display",
            JsonValue::string("ManualPacketWire.version"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.version_mismatch, ByteOffset(3), ManualPacketWire.version, expected_version=2; actual_version=1; reason=codec version is not supported)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.version_mismatch");
    assert_eq!(diagnostic.message, "version mismatch at byte offset 3");
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Expected version `2`; actual version was `1`.\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Version mismatch reason: codec version is not supported.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.version_mismatch, ByteOffset(3), ManualPacketWire.version, expected_version=2; actual_version=1; reason=codec version is not supported).\"}"
    );
}

#[test]
fn byte_result_failure_diagnostic_projects_consumed_count_invalid_reason() {
    let byte_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("byte_diagnostic")),
        ("id", JsonValue::string("codec.consumed_count_invalid")),
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
                    ("name", JsonValue::string("count")),
                ]),
            ]),
        ),
        ("available_count", JsonValue::Number(3)),
        ("actual_consumed_count", JsonValue::Number(5)),
        (
            "reason",
            JsonValue::string("decoder consumed beyond the supplied view"),
        ),
        ("local_byte_offset", JsonValue::Number(2)),
        ("expected_count", JsonValue::Number(4)),
        ("byte_preview", byte_preview("aabbcc")),
        (
            "field_path_display",
            JsonValue::string("ignored.fallback.path"),
        ),
    ]);
    let failure = TestFailure::result_with_details(
        "DecodeErrorWithReason(codec.consumed_count_invalid, ByteOffset(21), ManualPacketWire.count, available_count=3; actual_consumed_count=5; reason=decoder consumed beyond the supplied view)".to_string(),
        None,
        Some(byte_diagnostic),
        None,
    );

    let diagnostic =
        byte_result_failure_diagnostic(&failure).expect("byte diagnostic should project");

    assert_eq!(diagnostic.id, "codec.consumed_count_invalid");
    assert_eq!(
        diagnostic.message,
        "invalid decoded consumed count at byte offset 21"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert_eq!(
        diagnostic.related[0].to_json(),
        "{\"message\":\"Field path: schema `ManualPacketWire` / field `count`.\"}"
    );
    assert_eq!(
        diagnostic.related[1].to_json(),
        "{\"message\":\"Decoder consumed 5 byte(s); supplied view length was 3 byte(s).\"}"
    );
    assert_eq!(
        diagnostic.related[2].to_json(),
        "{\"message\":\"Consumed count reason: decoder consumed beyond the supplied view.\"}"
    );
    assert_eq!(
        diagnostic.related[3].to_json(),
        "{\"message\":\"DecodeError value: DecodeErrorWithReason(codec.consumed_count_invalid, ByteOffset(21), ManualPacketWire.count, available_count=3; actual_consumed_count=5; reason=decoder consumed beyond the supplied view).\"}"
    );
}
