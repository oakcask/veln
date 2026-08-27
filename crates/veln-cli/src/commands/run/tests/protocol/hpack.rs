use super::*;

#[test]
fn protocol_result_failure_diagnostic_projects_hpack_preview_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("hpack.fixture.unsupported_header_block"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(27)),
            ]),
        ),
        ("observed_header_block_size", JsonValue::Number(1)),
        ("observed_first_byte", JsonValue::Number(255)),
        (
            "expected_fixture",
            JsonValue::string("fixture header block"),
        ),
        ("codec_module", JsonValue::string("hpack_fixture")),
        ("byte_preview", byte_preview("ff")),
    ]);
    let failure = TestFailure::result_with_details(
        "HPACK fixture unsupported header block at byte offset 27".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "hpack.fixture.unsupported_header_block");
    assert_eq!(
        diagnostic.message,
        "unsupported HPACK fixture header block at byte offset 27"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("ff (showing 1 of 1 byte(s), complete)")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_hpack_string_length_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("hpack.fixture.malformed_string_length"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("observed_header_block_size", JsonValue::Number(2)),
        ("observed_first_byte", JsonValue::Number(4)),
        (
            "expected_fixture",
            JsonValue::string("fixture HPACK string length"),
        ),
        ("codec_module", JsonValue::string("hpack_fixture")),
        ("byte_preview", byte_preview("04ff")),
    ]);
    let failure = TestFailure::result_with_details(
        "HPACK fixture malformed string length at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "hpack.fixture.malformed_string_length");
    assert_eq!(
        diagnostic.message,
        "malformed HPACK string length at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("04 ff (showing 2 of 2 byte(s), complete)")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_hpack_table_size_malformed_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("hpack.fixture.table_size_update_malformed"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(77)),
            ]),
        ),
        ("observed_header_block_size", JsonValue::Number(2)),
        ("observed_first_byte", JsonValue::Number(63)),
        (
            "expected_fixture",
            JsonValue::string("fixture HPACK malformed table-size update integer"),
        ),
        ("codec_module", JsonValue::string("hpack_fixture")),
        ("byte_preview", byte_preview("3f80")),
    ]);
    let failure = TestFailure::result_with_details(
        "HPACK fixture malformed table-size update integer at byte offset 77".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "hpack.fixture.table_size_update_malformed");
    assert_eq!(
        diagnostic.message,
        "malformed HPACK table-size update integer at byte offset 77"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("3f 80 (showing 2 of 2 byte(s), complete)")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_hpack_raw_string_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("hpack.fixture.malformed_raw_string_value"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("observed_header_block_size", JsonValue::Number(5)),
        ("observed_first_byte", JsonValue::Number(8)),
        (
            "expected_fixture",
            JsonValue::string("fixture HPACK raw string value"),
        ),
        ("codec_module", JsonValue::string("hpack_fixture")),
        ("byte_preview", byte_preview("0803321f30")),
    ]);
    let failure = TestFailure::result_with_details(
        "HPACK fixture malformed raw string value at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "hpack.fixture.malformed_raw_string_value");
    assert_eq!(
        diagnostic.message,
        "malformed HPACK raw string value at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("08 03 32 1f 30 (showing 5 of 5 byte(s), complete)")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_hpack_huffman_padding_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("hpack.fixture.malformed_huffman_padding"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("observed_header_block_size", JsonValue::Number(3)),
        ("observed_first_byte", JsonValue::Number(4)),
        (
            "expected_fixture",
            JsonValue::string("fixture HPACK Huffman padding"),
        ),
        ("codec_module", JsonValue::string("hpack_fixture")),
        ("byte_preview", byte_preview("048100")),
    ]);
    let failure = TestFailure::result_with_details(
        "HPACK fixture malformed Huffman padding at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "hpack.fixture.malformed_huffman_padding");
    assert_eq!(
        diagnostic.message,
        "malformed HPACK Huffman padding at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("header block size 3")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("04 81 00 (showing 3 of 3 byte(s), complete)")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_hpack_huffman_eos_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        ("id", JsonValue::string("hpack.fixture.huffman_eos_symbol")),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("observed_header_block_size", JsonValue::Number(6)),
        ("observed_first_byte", JsonValue::Number(4)),
        (
            "expected_fixture",
            JsonValue::string("fixture HPACK Huffman data symbol instead of EOS"),
        ),
        ("codec_module", JsonValue::string("hpack_fixture")),
        ("byte_preview", byte_preview("0484ffffffff")),
    ]);
    let failure = TestFailure::result_with_details(
        "HPACK fixture Huffman EOS symbol at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "hpack.fixture.huffman_eos_symbol");
    assert_eq!(
        diagnostic.message,
        "HPACK Huffman EOS used as decoded symbol at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("04 84 ff ff ff ff (showing 6 of 6 byte(s), complete)")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_hpack_huffman_non_visible_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("hpack.fixture.huffman_non_visible_value"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(9)),
            ]),
        ),
        ("observed_header_block_size", JsonValue::Number(4)),
        ("observed_first_byte", JsonValue::Number(4)),
        (
            "expected_fixture",
            JsonValue::string("fixture HPACK Huffman visible ASCII header value"),
        ),
        ("codec_module", JsonValue::string("hpack_fixture")),
        ("byte_preview", byte_preview("0482ffc7")),
    ]);
    let failure = TestFailure::result_with_details(
        "HPACK fixture Huffman non-visible value at byte offset 9".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "hpack.fixture.huffman_non_visible_value");
    assert_eq!(
        diagnostic.message,
        "HPACK Huffman decoded non-visible header value at byte offset 9"
    );
    assert_eq!(diagnostic.related.len(), 3);
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("04 82 ff c7 (showing 4 of 4 byte(s), complete)")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_hpack_dynamic_index_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("hpack.fixture.dynamic_index_out_of_range"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(27)),
            ]),
        ),
        ("observed_header_block_size", JsonValue::Number(1)),
        ("observed_first_byte", JsonValue::Number(190)),
        ("requested_dynamic_index", JsonValue::Number(0)),
        ("dynamic_table_entry_count", JsonValue::Number(0)),
        (
            "expected_fixture",
            JsonValue::string("fixture dynamic indexed header"),
        ),
        ("codec_module", JsonValue::string("hpack_fixture")),
        ("byte_preview", byte_preview("be")),
    ]);
    let failure = TestFailure::result_with_details(
        "HPACK dynamic index out of range at byte offset 27".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(diagnostic.id, "hpack.fixture.dynamic_index_out_of_range");
    assert_eq!(
        diagnostic.message,
        "HPACK dynamic index out of range at byte offset 27"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(diagnostic.related[0].to_json().contains("dynamic index 0"));
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("header block size 1")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("be (showing 1 of 1 byte(s), complete)")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_hpack_dynamic_name_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("hpack.fixture.dynamic_name_continuation_out_of_range"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(98)),
            ]),
        ),
        ("observed_header_block_size", JsonValue::Number(8)),
        ("observed_first_byte", JsonValue::Number(127)),
        ("requested_dynamic_index", JsonValue::Number(3)),
        ("dynamic_table_entry_count", JsonValue::Number(3)),
        (
            "expected_fixture",
            JsonValue::string("fixture dynamic-name continuation range"),
        ),
        ("codec_module", JsonValue::string("hpack_fixture")),
        ("byte_preview", byte_preview("7f02055041544348")),
    ]);
    let failure = TestFailure::result_with_details(
        "HPACK dynamic-name continuation out of range at byte offset 98".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.id,
        "hpack.fixture.dynamic_name_continuation_out_of_range"
    );
    assert_eq!(
        diagnostic.message,
        "HPACK dynamic-name continuation is out of range at byte offset 98"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(diagnostic.related[0].to_json().contains("dynamic index 3"));
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("7f 02 05 50 41 54 43 48")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_hpack_table_size_update_placement_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("hpack.fixture.table_size_update_not_at_start"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(10)),
            ]),
        ),
        ("observed_header_block_size", JsonValue::Number(2)),
        ("observed_first_byte", JsonValue::Number(62)),
        ("observed_header_table_size", JsonValue::Number(30)),
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        ("active_state", JsonValue::string("hpack-fixture")),
        (
            "expected_fixture",
            JsonValue::string("fixture HPACK table-size update at header block start"),
        ),
        ("codec_module", JsonValue::string("hpack_fixture")),
        ("byte_preview", byte_preview("823e")),
    ]);
    let failure = TestFailure::result_with_details(
        "HPACK fixture table-size update after header field at byte offset 10".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.id,
        "hpack.fixture.table_size_update_not_at_start"
    );
    assert_eq!(
        diagnostic.message,
        "HPACK table-size update appears after a header field at byte offset 10"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("requested HPACK header table size 30")
    );
    assert!(
        diagnostic.related[1]
            .to_json()
            .contains("active state hpack-fixture")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("82 3e (showing 2 of 2 byte(s), complete)")
    );
    assert!(
        diagnostic.related[3]
            .to_json()
            .contains("fixture HPACK table-size update at header block start")
    );
}

#[test]
fn protocol_result_failure_diagnostic_projects_hpack_table_size_trailing_context() {
    let protocol_diagnostic = JsonValue::object([
        ("kind", JsonValue::string("protocol_diagnostic")),
        (
            "id",
            JsonValue::string("hpack.fixture.table_size_update_trailing_bytes"),
        ),
        (
            "byte_offset",
            JsonValue::object([
                ("kind", JsonValue::string("ByteOffset")),
                ("value", JsonValue::Number(80)),
            ]),
        ),
        ("observed_header_block_size", JsonValue::Number(3)),
        ("observed_first_byte", JsonValue::Number(63)),
        ("observed_header_table_size", JsonValue::Number(33)),
        ("frame_kind", JsonValue::Number(1)),
        ("stream_id", JsonValue::Number(1)),
        ("stream_ref", JsonValue::string("stream")),
        ("active_state", JsonValue::string("hpack-fixture")),
        (
            "expected_fixture",
            JsonValue::string("fixture HPACK table-size update without trailing bytes"),
        ),
        ("codec_module", JsonValue::string("hpack_fixture")),
        ("byte_preview", byte_preview("3f0200")),
    ]);
    let failure = TestFailure::result_with_details(
        "HPACK fixture table-size update has trailing bytes at byte offset 80".to_string(),
        None,
        None,
        Some(protocol_diagnostic),
    );

    let diagnostic =
        protocol_result_failure_diagnostic(&failure).expect("protocol diagnostic should project");

    assert_eq!(
        diagnostic.id,
        "hpack.fixture.table_size_update_trailing_bytes"
    );
    assert_eq!(
        diagnostic.message,
        "HPACK table-size update leaves trailing bytes at byte offset 80"
    );
    assert_eq!(diagnostic.related.len(), 4);
    assert!(
        diagnostic.related[0]
            .to_json()
            .contains("before unexpected trailing header-block bytes")
    );
    assert!(
        diagnostic.related[2]
            .to_json()
            .contains("3f 02 00 (showing 3 of 3 byte(s), complete)")
    );
}
