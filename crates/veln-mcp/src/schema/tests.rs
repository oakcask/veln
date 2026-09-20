use super::*;

#[test]
fn checked_tool_schemas_are_the_advertised_schemas() {
    let declarations = declarations().as_array().unwrap();
    assert_eq!(declarations.len(), TOOLS.len());
    for (declaration, tool) in declarations.iter().zip(TOOLS) {
        assert_eq!(declaration["name"], tool.name);
        assert_eq!(declaration["inputSchema"], tool.input_schema());
        assert_eq!(declaration["outputSchema"], tool.result_schema());
        assert_eq!(declaration["inputSchema"]["type"], "object");
        assert_eq!(declaration["outputSchema"]["type"], "object");
        if tool.name == "references" {
            assert!(
                declaration["inputSchema"]
                    .get("additionalProperties")
                    .is_none()
            );
        } else {
            assert_eq!(declaration["inputSchema"]["additionalProperties"], false);
        }
    }
}

#[test]
fn advertised_tools_round_trip_through_name_lookup() {
    for advertised in TOOLS {
        let resolved = tool(advertised.name).unwrap();
        assert_eq!(resolved.name, advertised.name);
        assert_eq!(resolved.input_schema(), advertised.input_schema());
        assert_eq!(resolved.result_schema(), advertised.result_schema());
    }
    assert!(tool("unknown").is_none());
}

#[test]
fn refresh_result_schema_accepts_success_and_domain_failure() {
    let schema = tool("refresh_workspace").unwrap().result_schema();
    assert!(matches_schema(
        &schema,
        &serde_json::json!({"generation": 1, "roots": ["alpha", "beta/deep"]})
    ));
    assert!(matches_schema(
        &schema,
        &serde_json::json!({
            "code": "generation_failed",
            "message": "workspace project discovery failed",
            "details": {}
        })
    ));
    assert!(!matches_schema(
        &schema,
        &serde_json::json!({"code": "generation_failed", "message": "missing details"})
    ));
}

#[test]
fn empty_tool_inputs_accept_only_empty_objects() {
    for tool in [
        tool("workspace_projects").unwrap(),
        tool("refresh_workspace").unwrap(),
    ] {
        assert!(tool.accepts_input(&serde_json::json!({})));
        assert!(!tool.accepts_input(&serde_json::json!({"unknown": true})));
        assert!(!tool.accepts_input(&serde_json::json!([])));
        assert!(!tool.accepts_input(&Value::Null));
    }
}

#[test]
fn check_project_input_accepts_only_declared_string_fields() {
    let tool = tool("check_project").unwrap();
    for value in [
        serde_json::json!({}),
        serde_json::json!({"project": "."}),
        serde_json::json!({"source": "main.veln"}),
        serde_json::json!({"project": ".", "source": "main.veln"}),
    ] {
        assert!(tool.accepts_input(&value), "{value}");
    }
    for value in [
        serde_json::json!({"unknown": true}),
        serde_json::json!({"project": null}),
        serde_json::json!({"source": null}),
        serde_json::json!({"project": []}),
        serde_json::json!(null),
    ] {
        assert!(!tool.accepts_input(&value), "{value}");
    }
}

#[test]
fn definition_input_requires_closed_positive_coordinates() {
    let tool = tool("definition").unwrap();
    assert_position_input_schema(tool);
}

#[test]
fn references_input_requires_closed_positive_coordinates() {
    let tool = tool("references").unwrap();
    assert!(tool.input_schema().get("additionalProperties").is_none());
    assert_position_input_schema(tool);
    assert!(tool.accepts_input(&serde_json::json!({
        "source": "main.veln",
        "line": 1,
        "column": 1,
        "page_size": 1
    })));
    assert!(tool.accepts_input(&serde_json::json!({
        "source": "main.veln",
        "line": 1,
        "column": 1,
        "page_size": 1000
    })));
    assert!(tool.accepts_input(&serde_json::json!({
        "source": "main.veln",
        "line": 1,
        "column": 1,
        "include_declaration": true
    })));
    assert_eq!(
        tool.input_schema()["oneOf"][0]["properties"]["include_declaration"],
        serde_json::json!({"type": "boolean"})
    );
    assert!(tool.accepts_input(&serde_json::json!({"cursor": "opaque"})));
    for value in [
        serde_json::json!({"source":"main.veln","line":1,"column":1,"page_size":0}),
        serde_json::json!({"source":"main.veln","line":1,"column":1,"page_size":1001}),
        serde_json::json!({"source":"main.veln","line":1,"column":1,"page_size":null}),
        serde_json::json!({"cursor":"opaque","page_size":1}),
        serde_json::json!({"cursor":"opaque","include_declaration":true}),
        serde_json::json!({"source":"main.veln","line":1,"column":1,"include_declaration":null}),
        serde_json::json!({"source":"main.veln","line":1,"column":1,"include_declaration":"true"}),
        serde_json::json!({"source":"main.veln","line":1,"column":1,"page_size":0,"include_declaration":true}),
        serde_json::json!({"source":"main.veln","line":1,"column":1,"include_declaration":false,"cursor":"opaque"}),
        serde_json::json!({"cursor":""}),
    ] {
        assert!(!tool.accepts_input(&value), "{value}");
    }
}

#[test]
fn rename_input_requires_closed_coordinates_and_a_nonempty_name() {
    let tool = tool("rename").unwrap();
    assert!(tool.accepts_input(&serde_json::json!({
        "source": "main.veln",
        "line": 1.0,
        "column": 1e0,
        "new_name": "next"
    })));
    for value in [
        serde_json::json!({"source":"main.veln","line":1,"column":1}),
        serde_json::json!({"source":"main.veln","line":1,"column":1,"new_name":""}),
        serde_json::json!({"source":"main.veln","line":1,"column":1,"new_name":null}),
        serde_json::json!({"source":"main.veln","line":1,"column":1,"new_name":"next","extra":true}),
        serde_json::json!({"source":"main.veln","line":0,"column":1,"new_name":"next"}),
    ] {
        assert!(!tool.accepts_input(&value), "{value}");
    }
}

#[test]
fn rename_result_schema_closes_success_and_failure_variants() {
    let tool = tool("rename").unwrap();
    let range = serde_json::json!({
        "start": {"line": 1, "column": 4},
        "end": {"line": 1, "column": 8}
    });
    for result in [
        serde_json::json!({"edits":[{"uri":"file:///workspace/main.veln","range":range,"new_text":"next"}]}),
        serde_json::json!({"code":"rename.invalid_name","message":"failed","details":{"requested_name":"not valid"}}),
        serde_json::json!({"code":"rename.invalid_case","message":"failed","details":{"symbol_class":"type","requested_name":"entry","required_initial":"ascii_uppercase"}}),
        serde_json::json!({"code":"rename.conflict","message":"failed","details":{"symbol_class":"function","requested_name":"other","conflicting_declaration":{"uri":"file:///workspace/main.veln","range":{"start":{"line":1,"column":4},"end":{"line":1,"column":8}}},"affected_scope":{"kind":"module","name":"main"}}}),
        serde_json::json!({"code":"rename.conflict","message":"failed","details":{"symbol_class":"value_binding","requested_name":"other","conflicting_declaration":{"uri":"file:///workspace/main.veln","range":{"start":{"line":1,"column":4},"end":{"line":1,"column":8}}},"affected_scope":{"kind":"lexical","file":"main.veln","start_offset":0,"end_offset":10}}}),
        serde_json::json!({"code":"invalid_path","message":"failed","details":{}}),
        serde_json::json!({"code":"snapshot_changed","message":"failed","details":{}}),
        serde_json::json!({"code":"invalid_position","message":"failed","details":{"source":"main.veln","line":9,"column":1}}),
    ] {
        assert!(tool.accepts_result(&result), "{result:#}");
    }
    for result in [
        serde_json::json!({"edits":[],"code":"snapshot_changed","message":"failed","details":{}}),
        serde_json::json!({"code":"rename.invalid_name","message":"failed","details":{"requested_name":"bad!","extra":true}}),
        serde_json::json!({"code":"resource_capacity","message":"failed","details":{}}),
        serde_json::json!({"edits":[{"uri":"file:///main.veln","range":{"start":{"line":1,"column":1},"end":{"line":1,"column":2}},"new_text":"x","extra":true}]}),
    ] {
        assert!(!tool.accepts_result(&result), "{result:#}");
    }
}

#[test]
fn references_input_branches_are_closed_under_draft_2020_12_composition() {
    let schema = tool("references").unwrap().input_schema();
    assert!(matches_schema(
        &schema,
        &serde_json::json!({"source":"main.veln","line":1,"column":1,"page_size":1000})
    ));
    assert!(matches_schema(
        &schema,
        &serde_json::json!({"cursor":"opaque"})
    ));
    assert!(!matches_schema(
        &schema,
        &serde_json::json!({"source":"main.veln","line":1,"column":1,"unknown":true})
    ));
    assert!(!matches_schema(
        &schema,
        &serde_json::json!({"cursor":"opaque","page_size":1})
    ));
    assert!(!matches_schema(
        &schema,
        &serde_json::json!({"cursor":"opaque","include_declaration":true})
    ));
}

fn assert_position_input_schema(tool: ToolSchema) {
    assert!(tool.accepts_input(&serde_json::json!({
        "source": "main.veln",
        "line": 1,
        "column": 1
    })));
    assert!(tool.accepts_input(&serde_json::json!({
        "source": "main.veln",
        "line": u64::MAX,
        "column": 1
    })));
    assert!(tool.accepts_input(&serde_json::json!({
        "source": "main.veln",
        "line": 1.0,
        "column": 1e0
    })));
    let above_u64 =
        serde_json::from_str(r#"{"source":"main.veln","line":18446744073709551616,"column":1}"#)
            .unwrap();
    assert!(tool.accepts_input(&above_u64));
    let huge_positive_exponent =
        serde_json::from_str(r#"{"source":"main.veln","line":1e9223372036854775807,"column":1}"#)
            .unwrap();
    assert!(tool.accepts_input(&huge_positive_exponent));
    let huge_negative_exponent =
        serde_json::from_str(r#"{"source":"main.veln","line":1e-9223372036854775808,"column":1}"#)
            .unwrap();
    assert!(!tool.accepts_input(&huge_negative_exponent));
    let non_integer =
        serde_json::from_str(r#"{"source":"main.veln","line":6.0000000000000001,"column":1}"#)
            .unwrap();
    assert!(!tool.accepts_input(&non_integer));
    for value in [
        serde_json::json!({}),
        serde_json::json!({"source":"main.veln","line":1}),
        serde_json::json!({"source":"main.veln","line":0,"column":1}),
        serde_json::json!({"source":"main.veln","line":1,"column":-1}),
        serde_json::json!({"source":"main.veln","line":1.5,"column":1}),
        serde_json::json!({"source":null,"line":1,"column":1}),
        serde_json::json!({"source":"main.veln","line":1,"column":1,"extra":true}),
        serde_json::json!([]),
    ] {
        assert!(!tool.accepts_input(&value), "{value}");
    }
}

#[test]
fn definition_result_accepts_empty_location_and_domain_failures() {
    let tool = tool("definition").unwrap();
    assert!(tool.accepts_result(&serde_json::json!({"definition": null})));
    assert!(tool.accepts_result(&serde_json::json!({
        "definition": {
            "uri": "veln-pkg:///example%2Fdep/snapshot/d/source/main.veln",
            "packageDocumentationUri": "veln-doc:///package/example%2Fdep/snapshot/d/documentation/e/declaration/value",
            "range": {
                "start": {"line": 2, "column": 3},
                "end": {"line": 2, "column": 8}
            }
        }
    })));
    for code in [
        "invalid_path",
        "invalid_position",
        "snapshot_changed",
        "resource_capacity",
    ] {
        let result = serde_json::json!({
            "code": code,
            "message": "failed",
            "details": {"source": "main.veln"}
        });
        assert!(tool.accepts_result(&result), "{result}");
    }
}

#[test]
fn references_result_accepts_locations_scope_and_domain_failures() {
    let tool = tool("references").unwrap();
    let location = serde_json::json!({
        "uri": "file:///workspace/main.veln",
        "range": {
            "start": {"line": 2, "column": 3},
            "end": {"line": 2, "column": 9}
        }
    });
    assert!(tool.accepts_result(&serde_json::json!({
        "references": [location.clone()],
        "scope": {
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        }
    })));
    assert!(tool.accepts_result(&serde_json::json!({
        "references": [],
        "scope": {
            "mode": "single_file",
            "generation": 1,
            "project": ".",
            "source": "main.veln",
            "project_wide": false
        }
    })));
    assert!(tool.accepts_result(&serde_json::json!({
        "references": [location],
        "scope": {
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        },
        "next_cursor": "opaque"
    })));
    for code in [
        "invalid_path",
        "invalid_position",
        "snapshot_changed",
        "resource_capacity",
        "invalid_cursor",
        "stale_snapshot",
    ] {
        let result = serde_json::json!({
            "code": code,
            "message": "failed",
            "details": {}
        });
        assert!(tool.accepts_result(&result), "{result}");
        if matches!(code, "invalid_cursor" | "stale_snapshot") {
            assert!(!tool.accepts_result(&serde_json::json!({
                "code": code,
                "message": "failed",
                "details": {"source": "main.veln"}
            })));
        }
    }
    assert!(!tool.accepts_result(&serde_json::json!({
        "code": "snapshot_changed",
        "message": "failed",
        "details": {},
        "references": []
    })));
}

#[test]
fn search_docs_input_enforces_query_scope_and_limit_bounds() {
    let tool = tool("search_docs").unwrap();
    for value in [
        serde_json::json!({"query": "schema"}),
        serde_json::json!({"query": "schema", "scope": "language"}),
        serde_json::json!({"query": "schema", "scope": "stdlib"}),
        serde_json::json!({"query": "schema", "scope": "package"}),
        serde_json::json!({"query": "schema", "scope": "all"}),
        serde_json::json!({"query": "schema", "limit": 1}),
        serde_json::json!({"query": "schema", "limit": 50}),
        serde_json::json!({"query": "schema", "limit": 5.0}),
    ] {
        assert!(tool.accepts_input(&value), "{value}");
    }
    let exponent_limit = serde_json::from_str(r#"{"query":"schema","limit":1e0}"#).unwrap();
    assert!(tool.accepts_input(&exponent_limit), "{exponent_limit}");
    for value in [
        serde_json::json!({}),
        serde_json::json!({"query": ""}),
        serde_json::json!({"query": " \t\n"}),
        serde_json::json!({"query": "x".repeat(257)}),
        serde_json::json!({"query": null}),
        serde_json::json!({"query": "schema", "scope": "other"}),
        serde_json::json!({"query": "schema", "limit": 0}),
        serde_json::json!({"query": "schema", "limit": 51}),
        serde_json::json!({"query": "schema", "limit": 1.5}),
        serde_json::json!({"query": "schema", "unknown": true}),
        serde_json::json!(null),
        serde_json::json!([]),
    ] {
        assert!(!tool.accepts_input(&value), "{value}");
    }
}

#[test]
fn language_doc_result_schemas_accept_success_and_domain_failure() {
    let search = tool("search_docs").unwrap();
    assert!(search.accepts_result(&serde_json::json!({
        "scope": "language",
        "results": [{
            "uri": "veln-doc:///language/snapshot/d/topic/schemas",
            "title": "Schemas",
            "summary": "Schemas describe format-neutral and binary fields.",
            "excerpt": "Schemas",
            "prefix_truncated": false,
            "suffix_truncated": false
        }]
    })));
    assert!(search.accepts_result(&serde_json::json!({
        "scope": "all",
        "results": []
    })));
    assert!(!search.accepts_result(&serde_json::json!({
        "scope": "other",
        "results": []
    })));
    assert!(!search.accepts_result(&serde_json::json!({
        "scope": "language",
        "results": [{
            "uri": "uri",
            "title": "title",
            "summary": "summary",
            "excerpt": "x".repeat(161),
            "prefix_truncated": false,
            "suffix_truncated": false
        }]
    })));

    let read = tool("read_doc").unwrap();
    assert!(
        read.accepts_input(&serde_json::json!({"uri": "veln-doc:///language/snapshot/d/index"}))
    );
    assert!(!read.accepts_input(&serde_json::json!({"uri": null})));
    assert!(!read.accepts_input(&serde_json::json!({"uri": "uri", "unknown": true})));
    assert!(read.accepts_result(&serde_json::json!({
        "uri": "veln-doc:///language/snapshot/d/index",
        "name": "language-index",
        "title": "Veln Language Reference",
        "mimeType": "text/markdown; charset=utf-8",
        "text": "# Veln Language Reference\n"
    })));
    assert!(read.accepts_result(&serde_json::json!({
        "code": "resource_not_found",
        "message": "language reference resource not found",
        "details": {"uri": "missing"}
    })));
    assert!(!read.accepts_result(&serde_json::json!({
        "code": "generation_failed",
        "message": "failed",
        "details": {"uri": "missing"}
    })));
}

#[test]
fn check_project_result_schema_closes_diagnostics_summary_and_analysis_modes() {
    let tool = tool("check_project").unwrap();
    let diagnostic = serde_json::json!({
        "id": "type.mismatch",
        "kind": "type",
        "severity": "error",
        "message": "type mismatch",
        "span": span(),
        "related": [
            {"message": "expected here", "span": span()},
            {"message": "Accepted integer form: 0 or 1."},
            {"kind": "repair_hint", "message": "Use a selected declaration.", "span": null},
            {
                "kind": "effect_declaration",
                "message": "Candidate effect is declared here.",
                "effect": "net",
                "operations": ["request"],
                "span": span()
            },
            {
                "message": "Field path: Packet.kind.",
                "field_path": [
                    {"kind": "schema", "name": "Packet"},
                    {"kind": "field", "name": "kind"}
                ]
            }
        ],
        "details": {"expected": "Int"}
    });
    let project_success = serde_json::json!({
        "schema_version": 1,
        "analysis": {
            "mode": "project",
            "generation": 0,
            "project": ".",
            "project_wide": true
        },
        "diagnostics": [diagnostic],
        "summary": {
            "diagnostic_count": 1,
            "by_severity": {"error": 1},
            "by_kind": {"type": 1}
        }
    });
    assert!(tool.accepts_result(&project_success));
    assert!(tool.accepts_result(&serde_json::json!({
        "schema_version": 1,
        "analysis": {
            "mode": "single_file",
            "generation": 0,
            "project": ".",
            "source": "main.veln",
            "project_wide": false
        },
        "diagnostics": [],
        "summary": {
            "diagnostic_count": 0,
            "by_severity": {},
            "by_kind": {}
        }
    })));
    assert!(tool.accepts_result(&serde_json::json!({
        "code": "snapshot_changed",
        "message": "workspace files changed during capture",
        "details": {}
    })));
    assert!(tool.accepts_result(&serde_json::json!({
        "code": "resource_capacity",
        "message": "dependency source resource capacity exceeded",
        "details": {}
    })));

    let mut invalid_mode = project_success.clone();
    invalid_mode["analysis"]["mode"] = serde_json::json!("single_file");
    assert!(!tool.accepts_result(&invalid_mode));

    let mut invalid_extra = project_success.clone();
    invalid_extra["diagnostics"][0]["extra"] = serde_json::json!(true);
    assert!(!tool.accepts_result(&invalid_extra));

    let mut invalid_related_extra = project_success.clone();
    invalid_related_extra["diagnostics"][0]["related"][0]["unexpected"] = serde_json::json!(true);
    assert!(!tool.accepts_result(&invalid_related_extra));

    let mut invalid_related_path_extra = project_success.clone();
    invalid_related_path_extra["diagnostics"][0]["related"][4]["field_path"][0]["unexpected"] =
        serde_json::json!(true);
    assert!(!tool.accepts_result(&invalid_related_path_extra));

    let mut invalid_summary = project_success;
    invalid_summary["summary"]["by_kind"]["type"] = serde_json::json!("one");
    assert!(!tool.accepts_result(&invalid_summary));
}

fn matches_schema(schema: &Value, value: &Value) -> bool {
    super::matches_schema(schema, value)
}

fn span() -> Value {
    serde_json::json!({
        "file": "main.veln",
        "start": {"line": 1, "column": 1, "offset": 0},
        "end": {"line": 1, "column": 2, "offset": 1}
    })
}
