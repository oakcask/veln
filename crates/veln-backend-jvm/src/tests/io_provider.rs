use super::*;

#[test]
fn read_file_adapter_captures_without_dispatch_rejects_duplicates_and_retains_identity() {
    let ir = lower_project_entry_to_ir(
        include_str!("../../test-support/io_provider_registration.veln"),
        "main",
    );
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("io-provider-read-file-registration", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "capture, publication, duplicate, and retained identity invariants held\n"
    );
}
