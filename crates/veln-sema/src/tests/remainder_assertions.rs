use super::*;

pub(super) fn assert_operand_mismatch<'a>(
    diagnostics: &'a [Diagnostic],
    actual_type: &str,
) -> &'a Diagnostic {
    let mismatch = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.mismatch")
        .unwrap_or_else(|| panic!("missing operand mismatch: {diagnostics:#?}"));
    assert_eq!(
        mismatch.message,
        format!("expected `Int`, but found `{actual_type}`")
    );
    let details = mismatch.details.to_json();
    for expected in [
        "\"phase\":\"type\"",
        "\"expected_type\":\"Int\"",
        &format!("\"actual_type\":\"{actual_type}\""),
        "\"expected_type_source\":\"inferred_expression\"",
        "\"actual_type_source\":\"inferred_expression\"",
        "\"constraint\":\"operator_operand\"",
    ] {
        assert!(details.contains(expected), "{details}");
    }
    mismatch
}
