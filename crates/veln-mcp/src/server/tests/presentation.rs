use super::dependency_resources::fill_dependency_resource_capacity_completely;
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

#[test]
fn presentation_uses_dependencies_without_admitting_resources_at_full_capacity() {
    let workspace = TempWorkspace::new("callsite-presentation-capacity");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n",
            "fn caller() -> SourceLocation\n",
            "  dep::located(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/dep.veln",
        concat!(
            "pub fn located(value: Int) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    fill_dependency_resource_capacity_completely(&mut server);
    let resources_before = server.language_resources.list_result();

    let completion = server.completion_tool(&json!({
        "source": "main.veln", "line": 2, "column": 30
    }));
    assert_eq!(completion["isError"], false);

    let signature = server.signature_help_tool(&json!({
        "source": "main.veln", "line": 3, "column": 17
    }));
    assert_eq!(signature["isError"], false);
    assert_eq!(
        signature["structuredContent"]["signature"],
        json!({
            "label": "fn located(value: Int) -> SourceLocation callsite",
            "parameters": ["value: Int"],
            "activeParameter": 0
        })
    );
    assert_eq!(server.language_resources.list_result(), resources_before);
}
