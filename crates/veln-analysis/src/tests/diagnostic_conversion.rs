use super::*;

#[test]
fn parse_diagnostic_conversion_preserves_complete_json_shape() {
    let diagnostic = ParseDiagnostic {
        id: "parse.expected_item",
        message: "expected a function or test declaration".to_string(),
        span: Some(span("main.veln", 2, 3, 4, 2, 4, 5)),
        parser_context: "module",
        unexpected: UnexpectedToken {
            kind: "At".to_string(),
            text: "@".to_string(),
        },
        expected: vec!["fn", "test"],
        recovery: Recovery {
            strategy: RecoveryStrategy::SynchronizeToAnchor,
            anchor: Some("fn".to_string()),
            dropped_token_count: 2,
        },
        repair_candidates: vec![ParseRepairCandidate {
            candidate_id: "parse.expected_item.replace".to_string(),
            name: "Replace token".to_string(),
            application_policy: "manual_review".to_string(),
            application_status: "available".to_string(),
            edit_summary: "Replace `@` with `fn`".to_string(),
            edits: vec![ParseRepairEdit {
                span: span("main.veln", 2, 3, 4, 2, 4, 5),
                replacement: "fn".to_string(),
            }],
        }],
    };

    let converted = parse_diagnostic_to_envelope(&diagnostic);

    assert_eq!(converted.kind, DiagnosticKind::Parse);
    assert_eq!(converted.severity, Severity::Error);
    assert_eq!(
        veln_diagnostics::diagnostic_to_json(&converted).to_json(),
        concat!(
            "{\"id\":\"parse.expected_item\",\"severity\":\"error\",\"kind\":\"parse\",",
            "\"message\":\"expected a function or test declaration\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":2,\"column\":3,\"offset\":4},",
            "\"end\":{\"line\":2,\"column\":4,\"offset\":5}},",
            "\"details\":{\"phase\":\"parse\",\"node_id\":null,\"parser_context\":\"module\",",
            "\"unexpected\":{\"kind\":\"At\",\"text\":\"@\"},\"expected\":[\"fn\",\"test\"],",
            "\"recovery\":{\"strategy\":\"synchronize_to_anchor\",\"anchor\":\"fn\",",
            "\"dropped_token_count\":2},\"candidate_queries\":[{\"query_id\":\"parse.expected_item\",",
            "\"candidates\":[{\"candidate_id\":\"parse.expected_item.replace\",",
            "\"name\":\"Replace token\",\"application_policy\":\"manual_review\",",
            "\"application_status\":\"available\",\"edit_summary\":\"Replace `@` with `fn`\",",
            "\"edits\":[{\"kind\":\"replace\",\"span\":{\"file\":\"main.veln\",",
            "\"start\":{\"line\":2,\"column\":3,\"offset\":4},",
            "\"end\":{\"line\":2,\"column\":4,\"offset\":5}},\"replacement\":\"fn\"}]}]}]},",
            "\"related\":[]}"
        )
    );
}

#[test]
fn parse_diagnostic_conversion_preserves_contract_classification() {
    let diagnostic = diagnostic("contract_predicate", vec!["contract predicate"]);

    let converted = parse_diagnostic_to_envelope(&diagnostic);

    assert_eq!(converted.kind, DiagnosticKind::Contract);
    assert_eq!(
        converted.details.to_json(),
        concat!(
            "{\"phase\":\"parse\",\"node_id\":null,\"parser_context\":\"contract_predicate\",",
            "\"unexpected\":{\"kind\":\"Invalid\",\"text\":\"?\"},",
            "\"expected\":[\"contract predicate\"],",
            "\"recovery\":{\"strategy\":\"none\",\"anchor\":null,\"dropped_token_count\":0}}"
        )
    );
}

#[test]
fn integer_literal_conversion_preserves_related_note() {
    let diagnostic = diagnostic("integer_literal", vec!["decimal integer"]);

    let converted = parse_diagnostic_to_envelope(&diagnostic);

    assert_eq!(
        JsonValue::array(converted.related).to_json(),
        "[{\"message\":\"Accepted integer form: decimal integer.\"}]"
    );
}
