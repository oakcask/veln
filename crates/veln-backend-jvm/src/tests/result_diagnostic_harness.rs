use super::*;

#[test]
fn jvm_runtime_decodes_compact_hex_fixtures_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-byte-hex");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("RuntimeByteHexHarness.java"),
        RUNTIME_BYTE_HEX_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeByteHexHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimeByteHexHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "Ok(ByteChunk([Byte(0), Byte(1), Byte(255)]))\n",
            "Ok(ByteChunk([Byte(0), Byte(1), Byte(255), Byte(16)]))\n",
            "Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 0 low nibble)\n",
            "Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 1 high nibble)\n",
            "Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 0 low nibble)\n",
            "Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 1 high nibble)\n",
            "Err(fixture.hex.invalid_character: expected ASCII hex digit at byte offset 1 high nibble)\n",
            "Err(fixture.hex.odd_length: dangling hex nibble at byte offset 1 high nibble)\n",
        )
    );
}

#[test]
fn jvm_runtime_records_result_diagnostics_from_values_when_java_is_available() {
    let Some(trace) = run_result_diagnostic_trace_harness_when_java_is_available() else {
        return;
    };

    assert_result_value_diagnostics(&trace);
    assert_decode_and_length_diagnostics(&trace);
    assert_payload_and_padding_diagnostics(&trace);
    assert_integer_sequence_and_version_diagnostics(&trace);
    assert_tag_and_magic_diagnostics(&trace);
    assert_feature_and_trailing_input_diagnostics(&trace);
    assert_invalid_input_diagnostics(&trace);
    assert_incomplete_input_diagnostics(&trace);
}

fn run_result_diagnostic_trace_harness_when_java_is_available() -> Option<String> {
    if !java_and_javac_are_available() {
        return None;
    }

    let root = write_result_diagnostic_trace_harness();
    compile_result_diagnostic_trace_harness(&root);
    let trace = execute_result_diagnostic_trace_harness(&root);
    let _ = fs::remove_dir_all(root);
    Some(trace)
}

fn java_and_javac_are_available() -> bool {
    Command::new("java").arg("-version").output().is_ok()
        && Command::new("javac").arg("-version").output().is_ok()
}

fn write_result_diagnostic_trace_harness() -> std::path::PathBuf {
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-result-diagnostic-trace");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("RuntimeResultDiagnosticTraceHarness.java"),
        RUNTIME_RESULT_DIAGNOSTIC_TRACE_HARNESS,
    )
    .expect("Java harness should be written");
    root
}

fn compile_result_diagnostic_trace_harness(root: &std::path::Path) {
    let output = Command::new("javac")
        .arg("RuntimeResultDiagnosticTraceHarness.java")
        .current_dir(root)
        .output()
        .expect("javac should run");
    assert_process_succeeded("javac", &output);
}

fn execute_result_diagnostic_trace_harness(root: &std::path::Path) -> String {
    let trace_path = root.join("result-errors.tsv");
    let output = Command::new("java")
        .arg("-cp")
        .arg(root)
        .arg("RuntimeResultDiagnosticTraceHarness")
        .current_dir(root)
        .env("VELN_RESULT_ERRORS", &trace_path)
        .output()
        .expect("java should run");
    assert_process_succeeded("java", &output);
    fs::read_to_string(trace_path).expect("result trace should be written")
}

fn assert_process_succeeded(name: &str, output: &std::process::Output) {
    assert!(
        output.status.success(),
        "{name} failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_result_value_diagnostics(trace: &str) {
    assert!(
        trace.contains("\tvalue_diagnostic\tschema.encode_value_unrepresentable\t"),
        "{trace}"
    );
    assert!(
        trace.contains("\tvalue_diagnostic\tcodec.encode_value_unrepresentable\t"),
        "{trace}"
    );
    assert!(trace.contains("\tschema\t5061636b6574"), "{trace}");
    assert!(trace.contains("\tfield\t76616c7565"), "{trace}");
    assert!(
        trace.contains("\treason\tstring\t746f6f206c61726765"),
        "{trace}"
    );
}

fn assert_decode_and_length_diagnostics(trace: &str) {
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.decode_failed\t7\t"),
        "{trace}"
    );
    assert!(
        trace.contains("\treason\tstring\t706c61696e20726561736f6e"),
        "{trace}"
    );
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.length_mismatch\t9\t"),
        "{trace}"
    );
    assert!(trace.contains("\texpected_length\tnumber\t4"), "{trace}");
    assert!(trace.contains("\tactual_length\tnumber\t3"), "{trace}");
    assert!(
        trace.contains("\treason\tstring\t7061796c6f6164206c656e67746820646964206e6f74206d6174636820686561646572206c656e677468"),
        "{trace}"
    );
    let plain_length_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.length_mismatch\t10\t"))
        .expect("plain length mismatch should be recorded");
    assert!(
        plain_length_line.contains("\treason\tstring\t706c61696e206c656e677468206d69736d61746368"),
        "{plain_length_line}"
    );
    assert!(!plain_length_line.contains("\texpected_length\t"));
    assert!(!plain_length_line.contains("\tactual_length\t"));
}

fn assert_payload_and_padding_diagnostics(trace: &str) {
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.payload_length_mismatch\t21\t"),
        "{trace}"
    );
    assert!(
        trace.contains("\texpected_payload_length\tnumber\t8"),
        "{trace}"
    );
    assert!(
        trace.contains("\tactual_payload_length\tnumber\t5"),
        "{trace}"
    );
    assert!(
        trace.contains("\treason\tstring\t7061796c6f6164206c656e67746820646964206e6f74206d61746368206672616d6520686561646572"),
        "{trace}"
    );
    let plain_payload_length_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.payload_length_mismatch\t22\t"))
        .expect("plain payload length mismatch should be recorded");
    assert!(
        plain_payload_length_line.contains(
            "\treason\tstring\t706c61696e207061796c6f6164206c656e677468206d69736d61746368"
        ),
        "{plain_payload_length_line}"
    );
    assert!(!plain_payload_length_line.contains("\texpected_payload_length\t"));
    assert!(!plain_payload_length_line.contains("\tactual_payload_length\t"));
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.padding_mismatch\t24\t"),
        "{trace}"
    );
    assert!(
        trace.contains("\texpected_padding_length\tnumber\t2"),
        "{trace}"
    );
    assert!(
        trace.contains("\tactual_padding_length\tnumber\t5"),
        "{trace}"
    );
    assert!(
        trace.contains("\treason\tstring\t444154412070616464696e6720646964206e6f74206d61746368207061796c6f616420626f756e64617279"),
        "{trace}"
    );
    let plain_padding_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.padding_mismatch\t25\t"))
        .expect("plain padding mismatch should be recorded");
    assert!(
        plain_padding_line
            .contains("\treason\tstring\t706c61696e2070616464696e67206d69736d61746368"),
        "{plain_padding_line}"
    );
    assert!(!plain_padding_line.contains("\texpected_padding_length\t"));
    assert!(!plain_padding_line.contains("\tactual_padding_length\t"));
}

fn assert_integer_sequence_and_version_diagnostics(trace: &str) {
    let integer_range_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.integer_out_of_range\t17\t"))
        .expect("integer out-of-range should be recorded");
    assert!(
        integer_range_line.contains("\tbyte_width\tnumber\t4"),
        "{integer_range_line}"
    );
    assert!(
        integer_range_line.contains("\tmin_value\tnumber\t0"),
        "{integer_range_line}"
    );
    assert!(
        integer_range_line.contains("\tmax_value\tnumber\t2147483647"),
        "{integer_range_line}"
    );
    assert!(
        integer_range_line.contains("\tactual_value\tnumber\t2147483648"),
        "{integer_range_line}"
    );
    assert!(
        integer_range_line.contains("\treason\tstring\t"),
        "{integer_range_line}"
    );
    let plain_integer_range_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.integer_out_of_range\t18\t"))
        .expect("plain integer out-of-range should be recorded");
    assert!(
        plain_integer_range_line.contains("\treason\tstring\t"),
        "{plain_integer_range_line}"
    );
    assert!(!plain_integer_range_line.contains("\tbyte_width\t"));
    assert!(!plain_integer_range_line.contains("\tmin_value\t"));
    assert!(!plain_integer_range_line.contains("\tmax_value\t"));
    assert!(!plain_integer_range_line.contains("\tactual_value\t"));
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.sequence_mismatch\t13\t"),
        "{trace}"
    );
    assert!(
        trace.contains(
            "\texpected_sequence\tstring\t636c69656e745f707265666163652c73657474696e6773"
        ),
        "{trace}"
    );
    assert!(
        trace.contains("\tactual_sequence\tstring\t73657474696e6773"),
        "{trace}"
    );
    assert!(
        trace.contains("\treason\tstring\t6672616d652073657175656e63652076696f6c617465642070726f746f636f6c207374617465"),
        "{trace}"
    );
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.version_mismatch\t3\t"),
        "{trace}"
    );
    assert!(trace.contains("\texpected_version\tstring\t32"), "{trace}");
    assert!(trace.contains("\tactual_version\tstring\t31"), "{trace}");
    assert!(
        trace.contains(
            "\treason\tstring\t636f6465632076657273696f6e206973206e6f7420737570706f72746564"
        ),
        "{trace}"
    );
}

fn assert_tag_and_magic_diagnostics(trace: &str) {
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.tag_mismatch\t14\t"),
        "{trace}"
    );
    assert!(
        trace.contains("\texpected_tag\tstring\t44415441"),
        "{trace}"
    );
    assert!(
        trace.contains("\tactual_tag\tstring\t48454144455253"),
        "{trace}"
    );
    assert!(
        trace.contains("\treason\tstring\t64697370617463682074616720646964206e6f74206d617463682073656c6563746564207061796c6f6164"),
        "{trace}"
    );
    let plain_tag_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.tag_mismatch\t15\t"))
        .expect("plain tag mismatch should be recorded");
    assert!(
        plain_tag_line.contains("\treason\tstring\t706c61696e20746167206d69736d61746368"),
        "{plain_tag_line}"
    );
    assert!(!plain_tag_line.contains("\texpected_tag\t"));
    assert!(!plain_tag_line.contains("\tactual_tag\t"));
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.magic_mismatch\t18\t"),
        "{trace}"
    );
    assert!(
        trace.contains("\texpected_magic\tstring\t56454c4e"),
        "{trace}"
    );
    assert!(
        trace.contains("\tactual_magic\tstring\t5645494e"),
        "{trace}"
    );
    assert!(
        trace.contains("\treason\tstring\t66696c65206d6167696320646964206e6f74206d61746368206578706563746564207369676e6174757265"),
        "{trace}"
    );
    let plain_magic_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.magic_mismatch\t19\t"))
        .expect("plain magic mismatch should be recorded");
    assert!(
        plain_magic_line.contains("\treason\tstring\t706c61696e206d61676963206d69736d61746368"),
        "{plain_magic_line}"
    );
    assert!(!plain_magic_line.contains("\texpected_magic\t"));
    assert!(!plain_magic_line.contains("\tactual_magic\t"));
}

fn assert_feature_and_trailing_input_diagnostics(trace: &str) {
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.unsupported_feature\t27\t"),
        "{trace}"
    );
    assert!(
        trace.contains(
            "\tunsupported_feature\tstring\t64796e616d69635f7461626c655f73697a655f757064617465"
        ),
        "{trace}"
    );
    assert!(
        trace.contains("\treason\tstring\t64796e616d6963207461626c652073697a652075706461746573206172652064697361626c656420666f7220746869732070726f66696c65"),
        "{trace}"
    );
    let plain_unsupported_feature_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.unsupported_feature\t28\t"))
        .expect("plain unsupported feature should be recorded");
    assert!(
        plain_unsupported_feature_line
            .contains("\treason\tstring\t706c61696e20756e737570706f727465642066656174757265"),
        "{plain_unsupported_feature_line}"
    );
    assert!(!plain_unsupported_feature_line.contains("\tunsupported_feature\t"));
    let trailing_input_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.trailing_input\t5\t"))
        .expect("trailing input should be recorded");
    assert!(
        trailing_input_line.contains("\tconsumed_count\tnumber\t5"),
        "{trailing_input_line}"
    );
    assert!(
        trailing_input_line.contains("\tavailable_count\tnumber\t8"),
        "{trailing_input_line}"
    );
    assert!(
        trailing_input_line.contains("\tremaining_count\tnumber\t3"),
        "{trailing_input_line}"
    );
    let malformed_trailing_input_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.trailing_input\t6\t"))
        .expect("malformed trailing input should be recorded");
    assert!(
        malformed_trailing_input_line.contains("\treason\tstring\t"),
        "{malformed_trailing_input_line}"
    );
    assert!(!malformed_trailing_input_line.contains("\tconsumed_count\t"));
    assert!(!malformed_trailing_input_line.contains("\tavailable_count\t"));
    assert!(!malformed_trailing_input_line.contains("\tremaining_count\t"));
}

fn assert_invalid_input_diagnostics(trace: &str) {
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.invalid_input\t42\t"),
        "{trace}"
    );
    assert!(
        trace.contains("\treason\tstring\t6279746520726561642072657175697265732034206279746573206275742076696577206861732033"),
        "{trace}"
    );
    assert!(trace.contains("\tlocal_byte_offset\tnumber\t5"), "{trace}");
    assert!(trace.contains("\texpected_count\tnumber\t4"), "{trace}");
    assert!(trace.contains("\tavailable_count\tnumber\t3"), "{trace}");
    let plain_consumed_count_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.consumed_count_invalid\t11\t"))
        .expect("plain consumed count diagnostic should be recorded");
    assert!(!plain_consumed_count_line.contains("\tavailable_count\t"));
    assert!(!plain_consumed_count_line.contains("\tactual_consumed_count\t"));
    assert!(!plain_consumed_count_line.contains("\treason\t"));
    let oversized_consumed_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.consumed_count_invalid\t21\t"))
        .expect("oversized consumed count should be recorded");
    assert!(
        oversized_consumed_line.contains("\tavailable_count\tnumber\t3"),
        "{oversized_consumed_line}"
    );
    assert!(
        oversized_consumed_line.contains("\tactual_consumed_count\tnumber\t5"),
        "{oversized_consumed_line}"
    );
    assert!(
        oversized_consumed_line.contains("\treason\tstring\t6465636f64656420636f6e73756d656420636f756e74206973206f75747369646520737570706c696564204279746556696577"),
        "{oversized_consumed_line}"
    );
    let negative_consumed_line = trace
        .lines()
        .find(|line| line.contains("\tbyte_diagnostic_v2\tcodec.consumed_count_invalid\t22\t"))
        .expect("negative consumed count should be recorded");
    assert!(
        negative_consumed_line.contains("\tavailable_count\tnumber\t3"),
        "{negative_consumed_line}"
    );
    assert!(
        negative_consumed_line.contains("\tactual_consumed_count\tnumber\t-1"),
        "{negative_consumed_line}"
    );
}

fn assert_incomplete_input_diagnostics(trace: &str) {
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.incomplete_input\t5\t"),
        "{trace}"
    );
    assert!(
        trace.contains("\treadiness\tstring\t6e6565645f6279746573"),
        "{trace}"
    );
    assert!(trace.contains("\tneeded_count\tnumber\t5"), "{trace}");
    assert!(
        trace.contains("\tbyte_diagnostic_v2\tcodec.incomplete_input\t0\t"),
        "{trace}"
    );
    assert!(
        trace.contains("\treadiness\tstring\t6e6565645f656e64"),
        "{trace}"
    );
}
