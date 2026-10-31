use super::{JsonValue, parse_json};

#[test]
fn json_unicode_units_preserve_scalar_and_error_boundaries() {
    assert_eq!(
        parse_json(r#""\u0041\uD83D\uDE42""#).expect("valid Unicode units should parse"),
        JsonValue::String("A🙂".to_string())
    );

    for (input, offset, message) in [
        (r#""\u12G4""#, 3, "invalid unicode escape `12G4` at byte 3"),
        (
            r#""\uD83D\u12G4""#,
            9,
            "invalid unicode escape `12G4` at byte 9",
        ),
        (r#""\uD83D\u12""#, 9, "short unicode escape at byte 9"),
    ] {
        let error = parse_json(input).expect_err("invalid Unicode unit should fail");
        assert_eq!(error.offset, offset, "unexpected offset for {input}");
        assert_eq!(error.to_string(), message, "unexpected error for {input}");
    }
}
