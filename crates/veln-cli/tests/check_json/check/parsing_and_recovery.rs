use super::*;

#[test]
fn check_json_reports_recovery_with_required_details() {
    let project = TestProject::new("recovery");
    project.write("main.veln", "garbage\nfn ok() -> ()\n  ()\nend\n");

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"parse.expected_item\"",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":1,\"column\":1,\"offset\":0},\"end\":{\"line\":1,\"column\":8,\"offset\":7}}",
            "\"details\":{\"phase\":\"parse\",\"node_id\":null,\"parser_context\":\"module\"",
            "\"unexpected\":{\"kind\":\"identifier\",\"text\":\"garbage\"}",
            "\"expected\":[\"pub\",\"fn\",\"test\",\"type\",\"effect\",\"handler\",\"schema\"]",
            "\"recovery\":{\"strategy\":\"synchronize_to_anchor\",\"anchor\":\"fn\",\"dropped_token_count\":2}",
        ],
    );
}

#[test]
fn check_json_reports_missing_end_at_eof_span() {
    let project = TestProject::new("missing-end");
    project.write("main.veln", "fn broken() -> ()\n  _\n");

    let output = project.check_json(&["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        concat!(
            "{\"schema_version\":1,",
            "\"tool\":{\"name\":\"veln\",\"version\":\"0.1.0\"},",
            "\"status\":\"error\",",
            "\"diagnostics\":[{",
            "\"id\":\"parse.expected_end\",",
            "\"severity\":\"error\",",
            "\"kind\":\"parse\",",
            "\"message\":\"expected `end` to close function declaration\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":3,\"column\":1,\"offset\":22},\"end\":{\"line\":3,\"column\":1,\"offset\":22}},",
            "\"details\":{\"phase\":\"parse\",\"node_id\":null,\"parser_context\":\"function_body\",",
            "\"unexpected\":{\"kind\":\"end of file\",\"text\":\"\"},",
            "\"expected\":[\"end\"],",
            "\"recovery\":{\"strategy\":\"close_block\",\"anchor\":\"end\",\"dropped_token_count\":0}},",
            "\"related\":[]}],",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"parse\":1}}}\n"
        )
    );
}

#[test]
fn check_json_reports_malformed_declaration() {
    let project = TestProject::new("malformed-declaration");
    project.write("main.veln", "pub main() -> ()\n  ()\nend\n");

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"parse.expected_token\"",
            "\"message\":\"expected fn\"",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":1,\"column\":5,\"offset\":4},\"end\":{\"line\":1,\"column\":9,\"offset\":8}}",
            "\"parser_context\":\"function_declaration\"",
            "\"unexpected\":{\"kind\":\"identifier\",\"text\":\"main\"}",
            "\"expected\":[\"fn\"]",
            "\"recovery\":{\"strategy\":\"insert_token\",\"anchor\":null,\"dropped_token_count\":0}",
        ],
    );
}

#[test]
fn check_json_reports_contract_predicate_parse_errors_as_contract_kind() {
    let project = TestProject::new("contract-predicate-parse");
    project.write(
        "main.veln",
        "fn bad(value: Int) -> Int\nrequire _missing\n  value\nend\n",
    );

    let output = project.check_json(&["main.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_contains_all(
        stdout,
        &[
            "\"id\":\"parse.contract_predicate\"",
            "\"kind\":\"contract\"",
            "\"message\":\"hole syntax is not allowed in a contract predicate\"",
            "\"parser_context\":\"contract_predicate\"",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"contract\":1}}",
        ],
    );
}

#[test]
fn check_json_reports_invalid_tokens() {
    let project = TestProject::new("invalid-token");
    project.write("main.veln", "fn bad() -> ()\n  @\nend\n");

    let output = project.check_json(&["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        stdout(&output),
        concat!(
            "{\"schema_version\":1,",
            "\"tool\":{\"name\":\"veln\",\"version\":\"0.1.0\"},",
            "\"status\":\"error\",",
            "\"diagnostics\":[{",
            "\"id\":\"parse.invalid_token\",",
            "\"severity\":\"error\",",
            "\"kind\":\"parse\",",
            "\"message\":\"invalid token in expression\",",
            "\"span\":{\"file\":\"main.veln\",\"start\":{\"line\":2,\"column\":3,\"offset\":17},\"end\":{\"line\":2,\"column\":4,\"offset\":18}},",
            "\"details\":{\"phase\":\"parse\",\"node_id\":null,\"parser_context\":\"expression_line\",",
            "\"unexpected\":{\"kind\":\"invalid token\",\"text\":\"@\"},",
            "\"expected\":[\"expression\"],",
            "\"recovery\":{\"strategy\":\"skip_token\",\"anchor\":\"newline\",\"dropped_token_count\":1}},",
            "\"related\":[]}],",
            "\"summary\":{\"diagnostic_count\":1,\"by_severity\":{\"error\":1},\"by_kind\":{\"parse\":1}}}\n"
        )
    );
}

#[test]
fn check_json_orders_diagnostics_by_source_discovery_order() {
    let project = TestProject::new("ordering");
    project.write("b.veln", "fn b() -> ()\n  _\n");
    project.write("a.veln", "fn a() -> ()\n  @\nend\n");

    let output = project.check_json(&["b.veln", "a.veln"]);
    let stdout = stdout(&output);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    let a_index = stdout
        .find("\"file\":\"a.veln\"")
        .expect("a.veln diagnostic should be present");
    let b_index = stdout
        .find("\"file\":\"b.veln\"")
        .expect("b.veln diagnostic should be present");
    assert!(a_index < b_index, "{stdout}");
}
