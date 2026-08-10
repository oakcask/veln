use veln_language_service::{
    PackageDocDoctest, PackageDocExpectedOutput, PackageDocResult, PackageDocResultKind,
};

#[test]
fn package_documentation_expected_output_type_is_public() {
    let output = PackageDocExpectedOutput {
        stream: "stdout".to_string(),
        lines: vec!["ok".to_string()],
    };
    let doctest = PackageDocDoctest {
        kind: "veln".to_string(),
        code: "1".to_string(),
        expected_error: None,
        should_fail: false,
        expected_output: vec![output],
    };

    assert_eq!(doctest.expected_output[0].stream, "stdout");
    let _result_type: Option<PackageDocResult> = None;
    let _kind_type: Option<PackageDocResultKind> = None;
}
