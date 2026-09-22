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

#[test]
fn references_collect_same_module_effects_and_exclude_lexical_and_module_collisions() {
    let workspace = TempWorkspace::new("references-workspace-effect-collisions");
    workspace.write("veln.toml", "");
    workspace.write(
        "declaration.veln",
        concat!(
            "mod shared\n\n",
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn choose() -> Int effects [Choose]\n",
            "  perform Choose::pick()\n",
            "end\n",
        ),
    );
    workspace.write(
        "uses.veln",
        concat!(
            "mod shared\n\n",
            "handler choose(callback: fn() -> Int effects [Choose], value: Choose) handles Choose effects [Choose]\n",
            "  pick() => perform Choose::pick()\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        "effect Choose\n  other() -> Int\nend\n\nfn other() -> Int effects [Choose]\n  1\nend\n",
    );
    let mut server = initialized_server(&workspace);

    let result = server.references_tool(&json!({
        "source":"declaration.veln", "line":3, "column":8,
        "include_declaration":false
    }));
    let declaration_uri = crate::definition::path_to_uri(&workspace.path("declaration.veln"));
    let uses_uri = crate::definition::path_to_uri(&workspace.path("uses.veln"));
    assert_eq!(
        result["structuredContent"]["references"],
        json!([
            {
                "uri": declaration_uri,
                "range": {"start":{"line":7,"column":29},"end":{"line":7,"column":35}}
            },
            {
                "uri": crate::definition::path_to_uri(&workspace.path("declaration.veln")),
                "range": {"start":{"line":8,"column":11},"end":{"line":8,"column":17}}
            },
            {
                "uri": uses_uri,
                "range": {"start":{"line":3,"column":47},"end":{"line":3,"column":53}}
            },
            {
                "uri": crate::definition::path_to_uri(&workspace.path("uses.veln")),
                "range": {"start":{"line":3,"column":79},"end":{"line":3,"column":85}}
            },
            {
                "uri": crate::definition::path_to_uri(&workspace.path("uses.veln")),
                "range": {"start":{"line":3,"column":95},"end":{"line":3,"column":101}}
            },
            {
                "uri": crate::definition::path_to_uri(&workspace.path("uses.veln")),
                "range": {"start":{"line":4,"column":21},"end":{"line":4,"column":27}}
            }
        ])
    );
}

#[test]
fn references_reject_imported_and_invalid_cased_effects() {
    let workspace = TempWorkspace::new("references-workspace-effect-negative-origins");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        "use dep from \"example/dep\"\n\nfn imported() -> Int effects [dep::Task]\n  1\nend\n",
    );
    workspace.write(
        "invalid.veln",
        "effect choose\n  pick() -> Int\nend\n\nfn invalid() -> Int effects [choose]\n  1\nend\n",
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write(
        "vendor/dep/dep.veln",
        "pub effect Task\n  run() -> Int\nend\n",
    );
    let mut server = initialized_server(&workspace);

    for (source, line, column) in [
        ("main.veln", 3, 36),
        ("invalid.veln", 1, 8),
        ("invalid.veln", 5, 29),
    ] {
        let result = server.references_tool(&json!({
            "source":source, "line":line, "column":column,
            "include_declaration":true
        }));
        assert_eq!(result["structuredContent"]["references"], json!([]));
    }
}
