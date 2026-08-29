use super::*;

#[cfg(unix)]
#[test]
fn exit_code_from_status_maps_success_failure_and_signal_statuses() {
    use std::os::unix::process::ExitStatusExt;

    assert_eq!(
        exit_code_from_status(ExitStatus::from_raw(0)),
        ExitCode::from(0)
    );
    assert_eq!(
        exit_code_from_status(ExitStatus::from_raw(42 << 8)),
        ExitCode::from(42)
    );
    assert_eq!(
        exit_code_from_status(ExitStatus::from_raw(9)),
        ExitCode::from(1)
    );
}
