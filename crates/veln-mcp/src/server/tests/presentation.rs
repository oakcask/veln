use super::*;

#[test]
fn tools_present_callsite_completion_and_signature_help() {
    let workspace = TempWorkspace::new("callsite-presentation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn located(message: String) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
            "fn ordinary() -> Int\n",
            "  1\n",
            "end\n",
            "fn caller() -> SourceLocation\n",
            "  located(\"hello\")\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    let resources_before = server.language_resources.list_result();

    let builtin = server
        .handle_request(json!({
            "jsonrpc": "2.0", "id": 1, "method": "tools/call",
            "params": {"name": "completion", "arguments": {"source": "main.veln", "line": 2, "column": 3}}
        }))
        .unwrap();
    assert_eq!(
        builtin["result"]["structuredContent"]["items"],
        json!([{"label":"callsite","kind":"builtin_local","detail":"built-in SourceLocation local"}])
    );

    let ordinary = server
        .handle_request(json!({
            "jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": "completion", "arguments": {"source": "main.veln", "line": 5, "column": 3}}
        }))
        .unwrap();
    assert_eq!(ordinary["result"]["structuredContent"]["items"], json!([]));

    let signature = server
        .handle_request(json!({
            "jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "signature_help", "arguments": {"source": "main.veln", "line": 8, "column": 18}}
        }))
        .unwrap();
    assert_eq!(
        signature["result"]["structuredContent"]["signature"],
        json!({
            "label": "fn located(message: String) -> SourceLocation callsite",
            "parameters": ["message: String"],
            "activeParameter": 0
        })
    );
    assert_eq!(server.language_resources.list_result(), resources_before);
}
