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
        "include_declaration":true, "page_size":1
    }));
    assert_eq!(
        first["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        first["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":1,"column":8},"end":{"line":1,"column":14}})
    );
    let first_cursor = first["structuredContent"]["next_cursor"].as_str().unwrap();
    let second = server.references_tool(&json!({"cursor":first_cursor}));
    assert_eq!(
        second["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        second["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":5,"column":29},"end":{"line":5,"column":35}})
    );
    let second_cursor = second["structuredContent"]["next_cursor"].as_str().unwrap();
    let final_page = server.references_tool(&json!({"cursor":second_cursor}));
    assert_eq!(
        final_page["structuredContent"]["references"][0]["range"],
        json!({"start":{"line":6,"column":17},"end":{"line":6,"column":23}})
    );
    assert!(final_page["structuredContent"].get("next_cursor").is_none());
}

#[test]
fn references_include_workspace_effect_predicate_perform_qualifiers() {
    let workspace = TempWorkspace::new("references-workspace-effect-predicates");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Int) -> Int\n",
            "end\n\n",
            "fn guarded(value: Int) -> Int\n",
            "  require perform Choose::pick(value) > 0\n",
            "  let constrained = _candidate satisfy candidate => perform Choose::pick(candidate) > 0\n",
            "  constrained\n",
            "end\n\n",
            "schema Packet\n",
            "  format binary\n",
            "  value: UInt8 where perform Choose::pick(value) > 0\n",
            "  validate perform Choose::pick(value) > 0\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);

    let result = server.references_tool(&json!({
        "source":"main.veln", "line":14, "column":20, "include_declaration":false
    }));
    let uri = crate::definition::path_to_uri(&workspace.path("main.veln"));
    assert_eq!(
        result["structuredContent"]["references"],
        json!([
            {
                "uri": uri,
                "range": {"start":{"line":6,"column":19},"end":{"line":6,"column":25}}
            },
            {
                "uri": uri,
                "range": {"start":{"line":7,"column":61},"end":{"line":7,"column":67}}
            },
            {
                "uri": uri,
                "range": {"start":{"line":13,"column":30},"end":{"line":13,"column":36}}
            },
            {
                "uri": uri,
                "range": {"start":{"line":14,"column":20},"end":{"line":14,"column":26}}
            }
        ])
    );
}

struct WorkspaceEffectReferenceState {
    _workspace: TempWorkspace,
    server: Server,
    before: Value,
    resources: Value,
    selection: Value,
    cursor: String,
}

fn workspace_effect_reference_state() -> WorkspaceEffectReferenceState {
    let workspace = TempWorkspace::new("references-workspace-effect-failure-state");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn choose() -> Int effects [Choose]\n",
            "  perform Choose::pick()\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    let before = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":8
    }));
    let seeded = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":8, "page_size":1
    }));
    let cursor = seeded["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    WorkspaceEffectReferenceState {
        resources: all_resource_state(&mut server),
        selection: server.selection_result(),
        _workspace: workspace,
        server,
        before,
        cursor,
    }
}

fn assert_workspace_effect_reference_state_is_live(state: &mut WorkspaceEffectReferenceState) {
    assert_eq!(all_resource_state(&mut state.server), state.resources);
    assert_eq!(state.server.selection_result(), state.selection);
    let continuation = state
        .server
        .references_tool(&json!({"cursor":state.cursor}));
    assert_eq!(continuation["isError"], false);
    assert_eq!(
        continuation["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let after = state.server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":8
    }));
    assert_eq!(after, state.before);
}

#[test]
fn invalid_workspace_effect_reference_position_preserves_live_state() {
    let mut state = workspace_effect_reference_state();
    let invalid_position = state.server.references_tool(&json!({
        "source":"main.veln", "line":99, "column":1
    }));
    assert_eq!(
        invalid_position["structuredContent"]["code"],
        "invalid_position"
    );
    assert_workspace_effect_reference_state_is_live(&mut state);
}

#[test]
fn invalid_workspace_effect_reference_path_preserves_live_state() {
    let mut state = workspace_effect_reference_state();
    let invalid_path = state.server.references_tool(&json!({
        "source":"missing.veln", "line":1, "column":1
    }));
    assert_eq!(invalid_path["structuredContent"]["code"], "invalid_path");
    assert_workspace_effect_reference_state_is_live(&mut state);
}

#[test]
fn invalid_workspace_effect_reference_cursor_preserves_live_state() {
    let mut state = workspace_effect_reference_state();
    let invalid_continuation = state
        .server
        .references_tool(&json!({"cursor":format!("{}x", state.cursor)}));
    assert_eq!(
        invalid_continuation["structuredContent"]["code"],
        "invalid_cursor"
    );
    assert_workspace_effect_reference_state_is_live(&mut state);
}

#[test]
fn missing_workspace_effect_resource_preserves_live_state() {
    let mut state = workspace_effect_reference_state();
    let missing_resource = state
        .server
        .handle_request(json!({
            "jsonrpc":"2.0",
            "id":"missing-retained-effect-resource",
            "method":"resources/read",
            "params":{"uri":"veln-pkg:///missing/snapshot/main.veln"}
        }))
        .unwrap();
    assert_eq!(
        missing_resource["error"]["data"]["code"],
        "resource_not_found"
    );
    assert_workspace_effect_reference_state_is_live(&mut state);
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

#[test]
fn references_reject_unresolved_and_mismatched_effect_occurrences() {
    let workspace = TempWorkspace::new("references-workspace-effect-negative-occurrences");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn boundaries() -> Int effects [Choose, Missing, choose]\n",
            "  perform Choose::pick()\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);

    for column in [41, 50] {
        let result = server.references_tool(&json!({
            "source":"main.veln", "line":5, "column":column,
            "include_declaration":true
        }));
        assert_eq!(result["structuredContent"]["references"], json!([]));
    }
}

#[test]
fn references_reject_balanced_recovery_shapes() {
    let workspace = TempWorkspace::new("references-workspace-effect-balanced-recovery");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn valid() -> Int effects [Choose]\n",
            "  perform Choose::pick()\n",
            "end\n\n",
            "fn broken() -> Int\n",
            "  effects [Choose]\n",
            "  handles Choose\n",
            "  value perform Choose::pick()\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    for (line, column) in [(10, 12), (11, 11), (12, 17)] {
        let result = server.references_tool(&json!({
            "source":"main.veln", "line":line, "column":column,
            "include_declaration":true
        }));
        assert_eq!(result["structuredContent"]["references"], json!([]));
    }
}

#[test]
fn references_reject_recovered_effect_row_and_handler_tokens() {
    for (name, source, line, column) in [
        (
            "references-workspace-effect-recovered-row-token",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "end\n\n",
                "fn broken() -> Int effects [Choose @]\n",
                "  1\n",
                "end\n",
            ),
            5,
            29,
        ),
        (
            "references-workspace-effect-recovered-handler-token",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "end\n\n",
                "handler broken() handles Choose @\n",
                "  pick() => 1\n",
                "end\n",
            ),
            5,
            26,
        ),
    ] {
        let workspace = TempWorkspace::new(name);
        workspace.write("veln.toml", "");
        workspace.write("main.veln", source);
        let mut server = initialized_server(&workspace);

        for (query_line, query_column, include_declaration) in [(line, column, true), (1, 8, false)]
        {
            let result = server.references_tool(&json!({
                "source":"main.veln", "line":query_line, "column":query_column,
                "include_declaration":include_declaration
            }));
            assert_eq!(result["structuredContent"]["references"], json!([]));
        }
    }
}

#[test]
fn references_reject_recovered_effect_declarations() {
    let declaration = TempWorkspace::new("references-workspace-effect-recovered-declaration");
    declaration.write("veln.toml", "");
    declaration.write(
        "main.veln",
        "effect Choose\nend\n\nfn use() -> Int effects [Choose]\n  1\nend\n",
    );
    let mut server = initialized_server(&declaration);
    let result = server.references_tool(&json!({
        "source":"main.veln", "line":1, "column":8,
        "include_declaration":true
    }));
    assert_eq!(result["structuredContent"]["references"], json!([]));
}

#[test]
fn references_reject_recovered_perform_qualifiers() {
    let workspace = TempWorkspace::new("references-workspace-effect-recovered-perform");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "effect Choose\n  pick() -> Int\nend\n\nfn broken() -> Int\n  perform Choose::pick(\nend\n",
    );
    let mut server = initialized_server(&workspace);
    for (line, column, include_declaration) in [(1, 8, false), (6, 11, true)] {
        let result = server.references_tool(&json!({
            "source":"main.veln", "line":line, "column":column,
            "include_declaration":include_declaration
        }));
        assert_eq!(result["structuredContent"]["references"], json!([]));
    }
}

#[test]
fn references_do_not_resolve_clean_uses_to_recovered_declarations() {
    let workspace =
        TempWorkspace::new("references-workspace-effect-clean-uses-recovered-declaration");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "effect Choose\nend\n\nfn use() -> Int effects [Choose]\n  perform Choose::pick()\nend\n\nhandler choose_handler() handles Choose\n  pick() => perform Choose::pick()\nend\n",
    );
    let mut server = initialized_server(&workspace);
    for (line, column) in [(4, 25), (5, 11), (8, 33), (9, 21)] {
        let result = server.references_tool(&json!({
            "source":"main.veln", "line":line, "column":column,
            "include_declaration":true
        }));
        assert_eq!(result["structuredContent"]["references"], json!([]));
    }
}

#[test]
fn references_keep_complete_effect_qualifiers_with_recovered_arguments() {
    let workspace = TempWorkspace::new("references-workspace-effect-recovered-argument");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick(value: Int) -> Int\n",
            "end\n\n",
            "fn broken() -> Int\n",
            "  perform Choose::pick(1 2)\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    let expected = json!([{
        "uri": crate::definition::path_to_uri(&workspace.path("main.veln")),
        "range": {"start":{"line":6,"column":11},"end":{"line":6,"column":17}}
    }]);

    for (line, column) in [(1, 8), (6, 11)] {
        let result = server.references_tool(&json!({
            "source":"main.veln", "line":line, "column":column,
            "include_declaration":false
        }));
        assert_eq!(result["structuredContent"]["references"], expected);
    }
}

#[test]
fn recovered_effect_declaration_makes_a_clean_same_module_declaration_ambiguous() {
    let workspace = TempWorkspace::new("references-workspace-effect-recovered-ambiguity");
    workspace.write("veln.toml", "");
    workspace.write(
        "clean.veln",
        "mod shared\n\neffect Choose\n  pick() -> Int\nend\n",
    );
    workspace.write("recovered.veln", "mod shared\n\neffect Choose\nend\n");
    workspace.write(
        "use.veln",
        concat!(
            "mod shared\n\n",
            "fn use() -> Int effects [Choose]\n",
            "  perform Choose::pick()\n",
            "end\n\n",
            "handler use_handler() handles Choose\n",
            "  pick() => perform Choose::pick()\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);

    for (source, line, column) in [
        ("clean.veln", 3, 8),
        ("recovered.veln", 3, 8),
        ("use.veln", 3, 25),
        ("use.veln", 4, 11),
        ("use.veln", 7, 30),
        ("use.veln", 8, 20),
    ] {
        let result = server.references_tool(&json!({
            "source":source, "line":line, "column":column,
            "include_declaration":true
        }));
        assert_eq!(result["structuredContent"]["references"], json!([]));
    }
}
