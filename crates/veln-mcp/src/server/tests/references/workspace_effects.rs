use super::*;

#[test]
fn references_page_workspace_effect_locations_with_unicode_scalar_coordinates() {
    let workspace = TempWorkspace::new("references-workspace-effect");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "effect Choose\r\n  pick() -> Int\r\nend\r\n\r\nfn choose() -> Int effects [Choose]\r\n  \"😀\" + perform Choose::pick()\r\nend\r\n",
    );
    let mut server = initialized_server(&workspace);

    let without_declaration = server.references_tool(&json!({
        "source":"main.veln", "line":6, "column":17, "include_declaration":false
    }));
    assert_eq!(
        without_declaration["structuredContent"]["references"],
        json!([
            {
                "uri": crate::definition::path_to_uri(&workspace.path("main.veln")),
                "range": {"start":{"line":5,"column":29},"end":{"line":5,"column":35}}
            },
            {
                "uri": crate::definition::path_to_uri(&workspace.path("main.veln")),
                "range": {"start":{"line":6,"column":17},"end":{"line":6,"column":23}}
            }
        ])
    );

    let first = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":8,
        "include_declaration":true, "page_size":2
    }));
    assert_eq!(
        first["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        first["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":1,"column":8},"end":{"line":1,"column":14}})
    );
    assert_eq!(
        first["structuredContent"]["references"][1]["range"],
        json!({"start":{"line":5,"column":29},"end":{"line":5,"column":35}})
    );
    let cursor = first["structuredContent"]["next_cursor"].as_str().unwrap();
    let final_page = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        final_page["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":6,"column":17},"end":{"line":6,"column":23}})
    );
    assert!(final_page["structuredContent"].get("next_cursor").is_none());
}
