use super::*;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NormalizedPosition {
    line: usize,
    column: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NormalizedRange {
    start: NormalizedPosition,
    end: NormalizedPosition,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct NormalizedLocation {
    source: String,
    range: NormalizedRange,
}

struct SavedWorkspace {
    project: TestProject,
    main_uri: String,
    other_uri: String,
    source_text: BTreeMap<String, String>,
}

#[test]
fn saved_navigation_matches_across_lsp_and_mcp_adapters() {
    let workspace = saved_workspace();
    let lsp_output = run_lsp(&workspace);
    let mcp_output = run_mcp(&workspace);
    assert_success("saved navigation LSP", &lsp_output);
    assert_success("saved navigation MCP", &mcp_output);

    let lsp = decode_lsp_stdout(
        std::str::from_utf8(&lsp_output.stdout).expect("LSP stdout should be UTF-8"),
    )
    .expect("LSP output should decode");
    let mcp = decode_mcp_stdout(
        std::str::from_utf8(&mcp_output.stdout).expect("MCP stdout should be UTF-8"),
    )
    .expect("MCP output should decode");

    assert_definition_conformance(&workspace, &lsp, &mcp);
    assert_reference_conformance(&workspace, &lsp, &mcp);
    assert_failure_conformance(&workspace, &lsp, &mcp);
    assert_recovery_conformance(&workspace, &lsp, &mcp);
}

fn assert_definition_conformance(workspace: &SavedWorkspace, lsp: &[JsonValue], mcp: &[JsonValue]) {
    let expected_workspace_definition = NormalizedLocation {
        source: "main.veln".to_string(),
        range: NormalizedRange {
            start: NormalizedPosition { line: 3, column: 8 },
            end: NormalizedPosition {
                line: 3,
                column: 14,
            },
        },
    };
    let workspace_definition = normalize_lsp_definition(workspace, response(lsp, 2));
    assert_eq!(
        workspace_definition,
        Some(expected_workspace_definition.clone()),
        "LSP workspace definition should retain the expected declaration"
    );
    assert_eq!(
        normalize_mcp_definition(workspace, response(mcp, 2)),
        Some(expected_workspace_definition.clone()),
        "MCP workspace definition should retain the expected declaration"
    );

    let package_definition = normalize_lsp_definition(workspace, response(lsp, 3));
    assert_eq!(
        package_definition,
        normalize_mcp_definition(workspace, response(mcp, 3))
    );
    assert!(
        package_definition
            .as_ref()
            .is_some_and(|location| location.source.starts_with("veln-pkg:"))
    );

    let coordinate_pairs = [(4, 4), (6, 6), (7, 7)];
    for (lsp_id, mcp_id) in coordinate_pairs {
        assert_eq!(
            normalize_lsp_definition(workspace, response(lsp, lsp_id)),
            Some(expected_workspace_definition.clone()),
            "LSP coordinate case {lsp_id} changed the definition"
        );
        assert_eq!(
            normalize_mcp_definition(workspace, response(mcp, mcp_id)),
            Some(expected_workspace_definition.clone()),
            "MCP coordinate case {mcp_id} changed the definition"
        );
    }
    assert_eq!(
        normalize_lsp_definition(workspace, response(lsp, 5)),
        None,
        "a half-open token-end LSP position should be empty"
    );
    assert_eq!(
        normalize_mcp_definition(workspace, response(mcp, 5)),
        None,
        "a half-open token-end MCP position should be empty"
    );
}

fn assert_reference_conformance(workspace: &SavedWorkspace, lsp: &[JsonValue], mcp: &[JsonValue]) {
    let lsp_without_declaration = normalize_lsp_references(workspace, response(lsp, 8));
    let mcp_without_declaration = normalize_mcp_reference_pages(workspace, &[response(mcp, 8)]);
    assert_eq!(lsp_without_declaration, mcp_without_declaration);
    assert_eq!(lsp_without_declaration.len(), 3);

    let lsp_with_declaration = normalize_lsp_references(workspace, response(lsp, 9));
    let mcp_with_declaration =
        normalize_mcp_reference_pages(workspace, &[response(mcp, 9), response(mcp, 11)]);
    assert_eq!(lsp_with_declaration, mcp_with_declaration);
    assert_eq!(lsp_with_declaration.len(), 4);
    let without_declaration = lsp_without_declaration.iter().collect::<BTreeSet<_>>();
    let with_declaration = lsp_with_declaration.iter().collect::<BTreeSet<_>>();
    assert_eq!(
        with_declaration
            .difference(&without_declaration)
            .copied()
            .collect::<Vec<_>>(),
        vec![&NormalizedLocation {
            source: "main.veln".to_string(),
            range: NormalizedRange {
                start: NormalizedPosition { line: 3, column: 8 },
                end: NormalizedPosition {
                    line: 3,
                    column: 14,
                },
            },
        }],
        "declaration inclusion should add exactly the workspace declaration"
    );
    assert!(
        without_declaration
            .difference(&with_declaration)
            .next()
            .is_none(),
        "declaration inclusion should retain every non-declaration reference"
    );
    assert!(
        structured_content(response(mcp, 9))
            .object_field("next_cursor")
            .is_some(),
        "MCP reference evidence must require continuation"
    );
    assert!(
        structured_content(response(mcp, 11))
            .object_field("next_cursor")
            .is_none(),
        "the collected MCP reference page should be terminal"
    );
}

fn assert_failure_conformance(workspace: &SavedWorkspace, lsp: &[JsonValue], mcp: &[JsonValue]) {
    assert_eq!(normalize_lsp_definition(workspace, response(lsp, 10)), None);
    assert!(normalize_lsp_references(workspace, response(lsp, 11)).is_empty());
    assert_eq!(normalize_mcp_definition(workspace, response(mcp, 12)), None);
    assert!(normalize_mcp_reference_pages(workspace, &[response(mcp, 13)]).is_empty());

    assert_lsp_invalid_position(response(lsp, 12));
    assert_lsp_invalid_position(response(lsp, 13));
    for id in 16..=30 {
        assert_lsp_invalid_position(response(lsp, id));
    }
    assert_mcp_invalid_position(response(mcp, 10));
    assert!(
        structured_content(response(mcp, 10))
            .object_field("next_cursor")
            .is_none(),
        "an invalid MCP selection must not create a continuation cursor"
    );

    let workspace_definition = normalize_lsp_definition(workspace, response(lsp, 2));
    assert_eq!(
        workspace_definition,
        normalize_lsp_definition(workspace, response(lsp, 14))
    );
    assert_eq!(
        workspace_definition,
        normalize_mcp_definition(workspace, response(mcp, 14))
    );

    assert!(normalize_lsp_references(workspace, response(lsp, 15)).is_empty());
    assert!(normalize_mcp_reference_pages(workspace, &[response(mcp, 15)]).is_empty());
}

fn assert_recovery_conformance(workspace: &SavedWorkspace, lsp: &[JsonValue], mcp: &[JsonValue]) {
    let recovery_definition = normalize_lsp_definition(workspace, response(lsp, 32));
    assert_eq!(
        recovery_definition,
        normalize_mcp_definition(workspace, response(mcp, 16)),
        "LSP and MCP should retain the same recovery declaration"
    );
    assert_eq!(
        recovery_definition,
        Some(NormalizedLocation {
            source: "main.veln".to_string(),
            range: NormalizedRange {
                start: NormalizedPosition {
                    line: 23,
                    column: 4,
                },
                end: NormalizedPosition {
                    line: 23,
                    column: 7,
                },
            },
        })
    );
    let lsp_recovery_without = normalize_lsp_references(workspace, response(lsp, 33));
    let mcp_recovery_without = normalize_mcp_reference_pages(workspace, &[response(mcp, 17)]);
    assert_eq!(lsp_recovery_without, mcp_recovery_without);
    assert_eq!(lsp_recovery_without.len(), 2);
    let lsp_recovery_with = normalize_lsp_references(workspace, response(lsp, 34));
    let mcp_recovery_with =
        normalize_mcp_reference_pages(workspace, &[response(mcp, 18), response(mcp, 19)]);
    assert_eq!(lsp_recovery_with, mcp_recovery_with);
    assert_eq!(lsp_recovery_with.len(), 3);
}

fn saved_workspace() -> SavedWorkspace {
    let project = TestProject::new(
        "saved-navigation-cross-adapter".to_string(),
        &ToolSetup::default(),
    );
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/specification/lsp/saved-navigation-cross-adapter");
    project.copy_fixtures(&fixture);

    let main_path = project.root.join("main.veln");
    let main_lf = fs::read_to_string(&main_path).expect("main fixture should be readable");
    let main_crlf = main_lf.replace('\n', "\r\n");
    fs::write(&main_path, &main_crlf).expect("CRLF fixture should be writable");

    let main_uri = workspace_file_uri(&project.root, "main.veln").unwrap();
    let other_uri = workspace_file_uri(&project.root, "other.veln").unwrap();
    let source_text = saved_source_text(&project, &main_uri, &other_uri, main_crlf);
    SavedWorkspace {
        project,
        main_uri,
        other_uri,
        source_text,
    }
}

fn saved_source_text(
    project: &TestProject,
    main_uri: &str,
    other_uri: &str,
    main_crlf: String,
) -> BTreeMap<String, String> {
    let mut source_text = BTreeMap::new();
    source_text.insert(main_uri.to_string(), main_crlf);
    source_text.insert(
        other_uri.to_string(),
        fs::read_to_string(project.root.join("other.veln")).unwrap(),
    );
    source_text.insert(
        "model.veln".to_string(),
        fs::read_to_string(project.root.join("vendor/model/model.veln")).unwrap(),
    );
    source_text
}

fn run_lsp(workspace: &SavedWorkspace) -> Output {
    let mut requests = lsp_success_requests(workspace);
    requests.extend(lsp_invalid_shape_requests(workspace));
    requests.extend(lsp_recovery_requests(workspace));
    let stdin = requests
        .iter()
        .map(|request| lsp_frame(request))
        .collect::<String>();
    workspace
        .project
        .veln_with_artifact(&["lsp".to_string()], None, &[], Some(&stdin), None)
}

fn lsp_success_requests(workspace: &SavedWorkspace) -> Vec<String> {
    let root_uri = workspace
        .main_uri
        .strip_suffix("/main.veln")
        .expect("main URI should end with its source path");
    vec![
        format!(
            r#"{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{"rootUri":"{root_uri}"}}}}"#
        ),
        lsp_definition(2, &workspace.main_uri, 7, 2),
        lsp_definition(3, &workspace.main_uri, 15, 16),
        lsp_definition(4, &workspace.main_uri, 7, 2),
        lsp_definition(5, &workspace.main_uri, 7, 8),
        lsp_definition(6, &workspace.main_uri, 11, 15),
        lsp_definition(7, &workspace.other_uri, 3, 8),
        lsp_references(8, &workspace.main_uri, 7, 2, false),
        lsp_references(9, &workspace.main_uri, 7, 2, true),
        lsp_definition(10, &workspace.main_uri, 1, 0),
        lsp_references(11, &workspace.main_uri, 1, 0, true),
        lsp_definition(12, &workspace.main_uri, 99, 0),
        lsp_references(13, &workspace.main_uri, 7, 99, true),
        lsp_definition(14, &workspace.main_uri, 7, 2),
        lsp_references(15, &workspace.main_uri, 15, 16, true),
    ]
}

fn lsp_invalid_shape_requests(workspace: &SavedWorkspace) -> Vec<String> {
    vec![
        lsp_navigation_with_position(
            16,
            "textDocument/definition",
            &workspace.main_uri,
            "-1",
            "0",
        ),
        lsp_navigation_with_position(
            17,
            "textDocument/references",
            &workspace.main_uri,
            "7",
            "-1",
        ),
        lsp_navigation_with_position(
            18,
            "textDocument/definition",
            &workspace.main_uri,
            "7.5",
            "0",
        ),
        lsp_navigation_with_position(
            19,
            "textDocument/references",
            &workspace.main_uri,
            "7",
            "2.5",
        ),
        lsp_navigation_with_position(
            20,
            "textDocument/definition",
            &workspace.main_uri,
            "184467440737095516160",
            "0",
        ),
        lsp_navigation_with_position(
            21,
            "textDocument/references",
            &workspace.main_uri,
            "7",
            "184467440737095516160",
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":22,"method":"textDocument/rename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":99,"character":0}}}}}}"#,
            workspace.main_uri
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":23,"method":"textDocument/rename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":99,"character":0}},"newName":"not valid"}}}}"#,
            workspace.main_uri
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":24,"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{}"}},"position":{{"character":2}},"extension":{{"line":7}}}}}}"#,
            workspace.main_uri
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":25,"method":"textDocument/references","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":7}},"extension":{{"character":2}},"context":{{"includeDeclaration":true}}}}}}"#,
            workspace.main_uri
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":26,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"extension":{{"line":7}},"character":2}}}}}}"#,
            workspace.main_uri
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":27,"method":"textDocument/rename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":7,"extension":{{"character":2}}}},"newName":"assist"}}}}"#,
            workspace.main_uri
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":28,"method":"textDocument/definition","extension":{{"position":{{"line":7,"character":2}}}},"params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":99,"character":0}}}}}}"#,
            workspace.main_uri
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":29,"method":"textDocument/references","params":{{"textDocument":{{"uri":"{}"}},"context":{{"includeDeclaration":true}}}}}}"#,
            workspace.main_uri
        ),
        format!(
            r#"{{"jsonrpc":"2.0","id":30,"method":"textDocument/prepareRename","params":{{"textDocument":{{"uri":"{}"}},"position":{{"line":7,"character":2}},"position":{{"line":7,"character":3}}}}}}"#,
            workspace.main_uri
        ),
    ]
}

fn lsp_recovery_requests(workspace: &SavedWorkspace) -> Vec<String> {
    vec![
        lsp_definition(32, &workspace.main_uri, 23, 14),
        lsp_references(33, &workspace.main_uri, 23, 14, false),
        lsp_references(34, &workspace.main_uri, 23, 14, true),
        r#"{"jsonrpc":"2.0","id":31,"method":"shutdown","params":null}"#.to_string(),
        r#"{"jsonrpc":"2.0","method":"exit","params":null}"#.to_string(),
    ]
}

fn run_mcp(workspace: &SavedWorkspace) -> Output {
    let requests = [
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"conformance","version":"1"}}}"#.to_string(),
        mcp_call(2, "definition", r#"{"source":"main.veln","line":8,"column":3}"#),
        mcp_call(3, "definition", r#"{"source":"main.veln","line":16,"column":17}"#),
        mcp_call(4, "definition", r#"{"source":"main.veln","line":8,"column":3}"#),
        mcp_call(5, "definition", r#"{"source":"main.veln","line":8,"column":9}"#),
        mcp_call(6, "definition", r#"{"source":"main.veln","line":12,"column":15}"#),
        mcp_call(7, "definition", r#"{"source":"other.veln","line":4,"column":9}"#),
        mcp_call(8, "references", r#"{"source":"main.veln","line":8,"column":3,"include_declaration":false}"#),
        mcp_call(9, "references", r#"{"source":"main.veln","line":8,"column":3,"include_declaration":true,"page_size":2}"#),
        mcp_call(10, "references", r#"{"source":"main.veln","line":100,"column":1,"include_declaration":true,"page_size":2}"#),
        mcp_call(11, "references", r#"{"cursor":"$mcp_cursor:9"}"#),
        mcp_call(12, "definition", r#"{"source":"main.veln","line":2,"column":1}"#),
        mcp_call(13, "references", r#"{"source":"main.veln","line":2,"column":1,"include_declaration":true}"#),
        mcp_call(14, "definition", r#"{"source":"main.veln","line":8,"column":3}"#),
        mcp_call(15, "references", r#"{"source":"main.veln","line":16,"column":17,"include_declaration":true}"#),
        mcp_call(16, "definition", r#"{"source":"main.veln","line":24,"column":15}"#),
        mcp_call(17, "references", r#"{"source":"main.veln","line":24,"column":15,"include_declaration":false}"#),
        mcp_call(18, "references", r#"{"source":"main.veln","line":24,"column":15,"include_declaration":true,"page_size":2}"#),
        mcp_call(19, "references", r#"{"cursor":"$mcp_cursor:18"}"#),
    ];
    workspace.project.veln_with_interactive_mcp(
        &["mcp".to_string()],
        None,
        &[],
        &requests.join("\n"),
        None,
    )
}

fn lsp_definition(id: i64, uri: &str, line: usize, character: usize) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"textDocument/definition","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}}}}}}"#
    )
}

fn lsp_references(
    id: i64,
    uri: &str,
    line: usize,
    character: usize,
    include_declaration: bool,
) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"textDocument/references","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}},"context":{{"includeDeclaration":{include_declaration}}}}}}}"#
    )
}

fn lsp_navigation_with_position(
    id: i64,
    method: &str,
    uri: &str,
    line: &str,
    character: &str,
) -> String {
    let context = if method == "textDocument/references" {
        r#","context":{"includeDeclaration":true}"#
    } else {
        ""
    };
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"{method}","params":{{"textDocument":{{"uri":"{uri}"}},"position":{{"line":{line},"character":{character}}}{context}}}}}"#
    )
}

fn mcp_call(id: i64, name: &str, arguments: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"{name}","arguments":{arguments}}}}}"#
    )
}

fn response(messages: &[JsonValue], id: i64) -> &JsonValue {
    messages
        .iter()
        .find(|message| message.object_field("id").and_then(JsonValue::as_i64) == Some(id))
        .unwrap_or_else(|| panic!("response id {id} should be present"))
}

fn structured_content(response: &JsonValue) -> &JsonValue {
    response
        .object_field("result")
        .and_then(|result| result.object_field("structuredContent"))
        .expect("MCP response should contain structuredContent")
}

fn normalize_lsp_definition(
    workspace: &SavedWorkspace,
    response: &JsonValue,
) -> Option<NormalizedLocation> {
    let result = response
        .object_field("result")
        .expect("LSP result should exist");
    (result != &JsonValue::Null).then(|| normalize_lsp_location(workspace, result))
}

fn normalize_mcp_definition(
    workspace: &SavedWorkspace,
    response: &JsonValue,
) -> Option<NormalizedLocation> {
    let definition = structured_content(response)
        .object_field("definition")
        .expect("MCP definition should exist");
    (definition != &JsonValue::Null).then(|| normalize_mcp_location(workspace, definition))
}

fn normalize_lsp_references(
    workspace: &SavedWorkspace,
    response: &JsonValue,
) -> Vec<NormalizedLocation> {
    let mut locations = response
        .object_field("result")
        .and_then(JsonValue::as_array)
        .expect("LSP references should be an array")
        .iter()
        .map(|location| normalize_lsp_location(workspace, location))
        .collect::<Vec<_>>();
    locations.sort();
    locations
}

fn normalize_mcp_reference_pages(
    workspace: &SavedWorkspace,
    responses: &[&JsonValue],
) -> Vec<NormalizedLocation> {
    let mut locations = responses
        .iter()
        .flat_map(|response| {
            structured_content(response)
                .object_field("references")
                .and_then(JsonValue::as_array)
                .expect("MCP references should be an array")
        })
        .map(|location| normalize_mcp_location(workspace, location))
        .collect::<Vec<_>>();
    locations.sort();
    locations
}

fn normalize_lsp_location(workspace: &SavedWorkspace, location: &JsonValue) -> NormalizedLocation {
    let uri = string_field(location, "uri");
    let text = source_text(workspace, uri);
    NormalizedLocation {
        source: source_identity(workspace, uri),
        range: normalize_lsp_range(
            text,
            location
                .object_field("range")
                .expect("LSP range should exist"),
        ),
    }
}

fn normalize_mcp_location(workspace: &SavedWorkspace, location: &JsonValue) -> NormalizedLocation {
    let uri = string_field(location, "uri");
    NormalizedLocation {
        source: source_identity(workspace, uri),
        range: normalize_mcp_range(
            location
                .object_field("range")
                .expect("MCP range should exist"),
        ),
    }
}

fn source_identity(workspace: &SavedWorkspace, uri: &str) -> String {
    if uri == workspace.main_uri {
        "main.veln".to_string()
    } else if uri == workspace.other_uri {
        "other.veln".to_string()
    } else if uri.starts_with("veln-pkg:") {
        uri.to_string()
    } else {
        panic!("unexpected retained source URI: {uri}")
    }
}

fn source_text<'a>(workspace: &'a SavedWorkspace, uri: &str) -> &'a str {
    if uri.starts_with("veln-pkg:") {
        workspace.source_text["model.veln"].as_str()
    } else {
        workspace
            .source_text
            .get(uri)
            .unwrap_or_else(|| panic!("source text should exist for {uri}"))
    }
}

fn normalize_lsp_range(text: &str, range: &JsonValue) -> NormalizedRange {
    NormalizedRange {
        start: normalize_lsp_position(text, object_field(range, "start")),
        end: normalize_lsp_position(text, object_field(range, "end")),
    }
}

fn normalize_lsp_position(text: &str, position: &JsonValue) -> NormalizedPosition {
    let line = usize_field(position, "line");
    let utf16_character = usize_field(position, "character");
    let line_text = source_line(text, line).expect("LSP result line should exist");
    let mut utf16_offset = 0usize;
    let mut scalar_column = 1usize;
    for ch in line_text.chars() {
        if utf16_offset == utf16_character {
            break;
        }
        utf16_offset += ch.len_utf16();
        assert!(utf16_offset <= utf16_character, "LSP range split a scalar");
        scalar_column += 1;
    }
    assert_eq!(utf16_offset, utf16_character, "LSP range exceeded its line");
    NormalizedPosition {
        line: line + 1,
        column: scalar_column,
    }
}

fn normalize_mcp_range(range: &JsonValue) -> NormalizedRange {
    NormalizedRange {
        start: normalize_mcp_position(object_field(range, "start")),
        end: normalize_mcp_position(object_field(range, "end")),
    }
}

fn normalize_mcp_position(position: &JsonValue) -> NormalizedPosition {
    NormalizedPosition {
        line: usize_field(position, "line"),
        column: usize_field(position, "column"),
    }
}

fn source_line(text: &str, line: usize) -> Option<&str> {
    let mut start = 0usize;
    for _ in 0..line {
        start += text.get(start..)?.find('\n')? + 1;
    }
    let rest = text.get(start..)?;
    match rest.find('\n') {
        Some(end) => rest[..end].strip_suffix('\r').or(Some(&rest[..end])),
        None => Some(rest),
    }
}

fn object_field<'a>(value: &'a JsonValue, field: &str) -> &'a JsonValue {
    value.object_field(field).unwrap_or_else(|| {
        panic!(
            "field {field} should exist in {}",
            value.to_compact_string()
        )
    })
}

fn string_field<'a>(value: &'a JsonValue, field: &str) -> &'a str {
    object_field(value, field)
        .as_str()
        .unwrap_or_else(|| panic!("field {field} should be a string"))
}

fn usize_field(value: &JsonValue, field: &str) -> usize {
    object_field(value, field)
        .as_i64()
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or_else(|| panic!("field {field} should be a nonnegative integer"))
}

fn assert_lsp_invalid_position(response: &JsonValue) {
    let error = response
        .object_field("error")
        .expect("invalid LSP position should be a protocol failure");
    assert_eq!(object_field(error, "code").as_i64(), Some(-32602));
    assert!(response.object_field("result").is_none());
}

fn assert_mcp_invalid_position(response: &JsonValue) {
    assert_eq!(
        structured_content(response).object_field("code"),
        Some(&JsonValue::String("invalid_position".to_string()))
    );
    assert_eq!(
        response
            .object_field("result")
            .and_then(|result| result.object_field("isError")),
        Some(&JsonValue::Bool(true))
    );
    assert!(
        structured_content(response)
            .object_field("references")
            .is_none()
    );
}
