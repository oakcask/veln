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
fn imported_workspace_effect_navigation_pages_shared_scalar_locations() {
    let (workspace, mut server) = imported_workspace_effect_navigation_server();
    let before = assert_imported_effect_definition_and_references(&workspace, &mut server);
    assert_imported_effect_navigation_failures(&mut server, &before);
    assert_imported_effect_reference_pages(&workspace, &mut server);
}

fn imported_workspace_effect_navigation_server() -> (TempWorkspace, Server) {
    let workspace = TempWorkspace::new("references-workspace-imported-effect");
    workspace.write("veln.toml", "");
    workspace.write(
        "foreign.veln",
        concat!(
            "pub effect E\r\n",
            "  run() -> Int\r\n",
            "end\r\n\r\n",
            "fn local() -> Int effects [E]\r\n",
            "  \"😀\" + perform E::run()\r\n",
            "end\r\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use foreign\r\n\r\n",
            "fn imported(callback: fn() -> Int effects [foreign::E]) -> Int effects [foreign::E]\r\n",
            "  \"😀\" + perform foreign::E::missing()\r\n",
            "end\r\n",
        ),
    );
    let server = initialized_server(&workspace);
    (workspace, server)
}

fn assert_imported_effect_definition_and_references(
    workspace: &TempWorkspace,
    server: &mut Server,
) -> Value {
    let foreign_uri = crate::definition::path_to_uri(&workspace.path("foreign.veln"));
    let main_uri = crate::definition::path_to_uri(&workspace.path("main.veln"));

    let definition = server.definition_tool(&json!({
        "source":"main.veln", "line":3, "column":53
    }));
    assert_eq!(
        definition["structuredContent"]["definition"],
        json!({
            "uri": foreign_uri,
            "range": {"start":{"line":1,"column":12},"end":{"line":1,"column":13}}
        })
    );

    let without_declaration = server.references_tool(&json!({
        "source":"main.veln", "line":4, "column":26, "include_declaration":false
    }));
    assert_eq!(
        without_declaration["structuredContent"]["references"],
        json!([
            {"uri": foreign_uri, "range":{"start":{"line":5,"column":28},"end":{"line":5,"column":29}}},
            {"uri": foreign_uri, "range":{"start":{"line":6,"column":17},"end":{"line":6,"column":18}}},
            {"uri": main_uri, "range":{"start":{"line":3,"column":53},"end":{"line":3,"column":54}}},
            {"uri": main_uri, "range":{"start":{"line":3,"column":82},"end":{"line":3,"column":83}}},
            {"uri": main_uri, "range":{"start":{"line":4,"column":26},"end":{"line":4,"column":27}}}
        ])
    );
    without_declaration
}

fn assert_imported_effect_navigation_failures(server: &mut Server, expected: &Value) {
    let invalid_position = server.references_tool(&json!({
        "source":"main.veln", "line":99, "column":1
    }));
    assert_eq!(
        invalid_position["structuredContent"]["code"],
        "invalid_position"
    );
    let invalid_path = server.references_tool(&json!({
        "source":"missing.veln", "line":1, "column":1
    }));
    assert_eq!(invalid_path["structuredContent"]["code"], "invalid_path");
    let missing_resource = server
        .handle_request(json!({
            "jsonrpc":"2.0",
            "id":"missing-imported-effect-resource",
            "method":"resources/read",
            "params":{"uri":"veln-pkg:///missing/snapshot/main.veln"}
        }))
        .unwrap();
    assert_eq!(
        missing_resource["error"]["data"]["code"],
        "resource_not_found"
    );
    assert_eq!(
        server.references_tool(&json!({
            "source":"main.veln", "line":4, "column":26,
            "include_declaration":false
        })),
        *expected
    );
}

fn assert_imported_effect_reference_pages(workspace: &TempWorkspace, server: &mut Server) {
    let foreign_uri = crate::definition::path_to_uri(&workspace.path("foreign.veln"));
    let main_uri = crate::definition::path_to_uri(&workspace.path("main.veln"));
    let first = server.references_tool(&json!({
        "source":"main.veln", "line":3, "column":82,
        "include_declaration":true, "page_size":2
    }));
    assert_eq!(
        first["structuredContent"]["references"],
        json!([
            {"uri": foreign_uri, "range":{"start":{"line":1,"column":12},"end":{"line":1,"column":13}}},
            {"uri": foreign_uri, "range":{"start":{"line":5,"column":28},"end":{"line":5,"column":29}}}
        ])
    );
    let cursor = first["structuredContent"]["next_cursor"].as_str().unwrap();
    let invalid_cursor = server.references_tool(&json!({"cursor":format!("{cursor}x")}));
    assert_eq!(
        invalid_cursor["structuredContent"]["code"],
        "invalid_cursor"
    );
    let second = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        second["structuredContent"]["references"],
        json!([
            {"uri": foreign_uri, "range":{"start":{"line":6,"column":17},"end":{"line":6,"column":18}}},
            {"uri": main_uri, "range":{"start":{"line":3,"column":53},"end":{"line":3,"column":54}}}
        ])
    );
    let cursor = second["structuredContent"]["next_cursor"].as_str().unwrap();
    let final_page = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        final_page["structuredContent"]["references"],
        json!([
            {"uri": main_uri, "range":{"start":{"line":3,"column":82},"end":{"line":3,"column":83}}},
            {"uri": main_uri, "range":{"start":{"line":4,"column":26},"end":{"line":4,"column":27}}}
        ])
    );
    assert!(final_page["structuredContent"].get("next_cursor").is_none());
}

#[test]
fn imported_workspace_effect_navigation_rejects_visibility_and_import_recovery_boundaries() {
    fn position_of(source: &str, needle: &str, leaf_offset: usize) -> (usize, usize) {
        let offset = source.find(needle).unwrap() + leaf_offset;
        let prefix = &source[..offset];
        (
            prefix.bytes().filter(|byte| *byte == b'\n').count() + 1,
            prefix.rsplit('\n').next().unwrap().chars().count() + 1,
        )
    }

    for (name, imports) in [
        ("private", "use fx\n"),
        ("ambiguous", "use first::fx\nuse second::fx\n"),
        ("duplicate", "use first::fx\nuse first::fx\n"),
        ("recovered", "use first::fx unexpected\n"),
    ] {
        let workspace = TempWorkspace::new(&format!("references-imported-effect-{name}"));
        workspace.write("veln.toml", "");
        workspace.write("fx.veln", "effect Remote\n  run() -> Int\nend\n");
        workspace.write("first/fx.veln", "pub effect Remote\n  run() -> Int\nend\n");
        workspace.write("second/fx.veln", "pub effect Remote\n  run() -> Int\nend\n");
        workspace.write("stable.veln", "pub effect Stable\n  run() -> Int\nend\n");
        let source = format!(
            "use stable\n{imports}\nfn invalid() -> Int effects [fx::Remote]\n  1\nend\n\nfn valid() -> Int effects [stable::Stable]\n  1\nend\n"
        );
        workspace.write("main.veln", &source);
        let mut server = initialized_server(&workspace);

        let (valid_line, valid_column) = position_of(&source, "stable::Stable", 8);
        let valid_input = json!({
            "source":"main.veln", "line":valid_line, "column":valid_column,
            "include_declaration":true
        });
        let before = server.references_tool(&valid_input);
        assert_ne!(
            before["structuredContent"]["references"],
            json!([]),
            "{name}"
        );

        let (invalid_line, invalid_column) = position_of(&source, "fx::Remote", 4);
        let invalid = server.references_tool(&json!({
            "source":"main.veln", "line":invalid_line, "column":invalid_column,
            "include_declaration":true
        }));
        assert_eq!(invalid["structuredContent"]["references"], json!([]));
        assert_eq!(server.references_tool(&valid_input), before, "{name}");
    }
}

#[test]
fn references_page_workspace_effect_operation_locations_with_unicode_scalar_coordinates() {
    let workspace = TempWorkspace::new("references-workspace-effect-operation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "effect Choose\r\n  pick() -> Int\r\nend\r\n\r\nfn choose() -> Int effects [Choose]\r\n  \"😀😀\" + perform Choose::pick()\r\nend\r\n\r\nhandler chooser() handles Choose\r\n  pick() => 1\r\nend\r\n",
    );
    let mut server = initialized_server(&workspace);
    let uri = crate::definition::path_to_uri(&workspace.path("main.veln"));

    let without_declaration = server.references_tool(&json!({
        "source":"main.veln", "line":10, "column":3, "include_declaration":false
    }));
    assert_eq!(
        without_declaration["structuredContent"]["references"],
        json!([
            {
                "uri": uri,
                "range": {"start":{"line":6,"column":26},"end":{"line":6,"column":30}}
            },
            {
                "uri": uri,
                "range": {"start":{"line":10,"column":3},"end":{"line":10,"column":7}}
            }
        ])
    );

    let first = server.references_tool(&json!({
        "source":"main.veln", "line":2, "column":3,
        "include_declaration":true, "page_size":1
    }));
    assert_eq!(
        first["structuredContent"]["references"],
        json!([{
            "uri": uri,
            "range": {"start":{"line":2,"column":3},"end":{"line":2,"column":7}}
        }])
    );
    let cursor = first["structuredContent"]["next_cursor"].as_str().unwrap();
    let perform_page = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        perform_page["structuredContent"]["references"],
        json!([{
            "uri": crate::definition::path_to_uri(&workspace.path("main.veln")),
            "range": {"start":{"line":6,"column":26},"end":{"line":6,"column":30}}
        }])
    );
    let cursor = perform_page["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap();
    let final_page = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        final_page["structuredContent"]["references"],
        json!([{
            "uri": crate::definition::path_to_uri(&workspace.path("main.veln")),
            "range": {"start":{"line":10,"column":3},"end":{"line":10,"column":7}}
        }])
    );
    assert!(final_page["structuredContent"].get("next_cursor").is_none());
    assert_eq!(
        server.references_tool(&json!({"cursor":cursor}))["structuredContent"]["code"],
        "invalid_cursor"
    );
}

#[test]
fn references_page_imported_workspace_effect_operation_locations() {
    let workspace = TempWorkspace::new("references-imported-workspace-effect-operation");
    workspace.write("veln.toml", "");
    workspace.write(
        "foreign.veln",
        concat!(
            "pub effect Remote\r\n",
            "  run() -> Int\r\n",
            "end\r\n\r\n",
            "fn local() -> Int\r\n",
            "  \"😀\" + perform Remote::run()\r\n",
            "end\r\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use foreign\r\n\r\n",
            "fn imported() -> Int\r\n",
            "  \"😀😀\" + perform foreign::Remote::run()\r\n",
            "end\r\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    let foreign_uri = crate::definition::path_to_uri(&workspace.path("foreign.veln"));
    let main_uri = crate::definition::path_to_uri(&workspace.path("main.veln"));

    let without_declaration = server.references_tool(&json!({
        "source":"main.veln", "line":4, "column":35, "include_declaration":false
    }));
    assert_eq!(
        without_declaration["structuredContent"]["references"],
        json!([
            {
                "uri": foreign_uri,
                "range": {"start":{"line":6,"column":25},"end":{"line":6,"column":28}}
            },
            {
                "uri": main_uri,
                "range": {"start":{"line":4,"column":35},"end":{"line":4,"column":38}}
            }
        ])
    );

    let first = server.references_tool(&json!({
        "source":"foreign.veln", "line":2, "column":3,
        "include_declaration":true, "page_size":1
    }));
    assert_eq!(
        first["structuredContent"]["references"],
        json!([{
            "uri": crate::definition::path_to_uri(&workspace.path("foreign.veln")),
            "range": {"start":{"line":2,"column":3},"end":{"line":2,"column":6}}
        }])
    );
    let cursor = first["structuredContent"]["next_cursor"].as_str().unwrap();
    let second = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        second["structuredContent"]["references"],
        json!([{
            "uri": crate::definition::path_to_uri(&workspace.path("foreign.veln")),
            "range": {"start":{"line":6,"column":25},"end":{"line":6,"column":28}}
        }])
    );
    let cursor = second["structuredContent"]["next_cursor"].as_str().unwrap();
    let final_page = server.references_tool(&json!({"cursor":cursor}));
    assert_eq!(
        final_page["structuredContent"]["references"],
        json!([{
            "uri": crate::definition::path_to_uri(&workspace.path("main.veln")),
            "range": {"start":{"line":4,"column":35},"end":{"line":4,"column":38}}
        }])
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

struct ImportedOperationReferenceState {
    _workspace: TempWorkspace,
    server: Server,
    before: Value,
    continuation: Value,
    resources: Value,
    selection: Value,
    cursor: String,
}

fn imported_operation_reference_state() -> ImportedOperationReferenceState {
    let workspace = TempWorkspace::new("references-imported-effect-failure-state");
    workspace.write("veln.toml", "");
    workspace.write(
        "foreign.veln",
        concat!(
            "pub effect Remote\n",
            "  run() -> Int\n",
            "end\n\n",
            "fn local() -> Int\n",
            "  perform Remote::run()\n",
            "end\n",
        ),
    );
    workspace.write(
        "main.veln",
        "use foreign\n\nfn use() -> Int\n  perform foreign::Remote::run()\nend\n",
    );
    let mut server = initialized_server(&workspace);
    let before = server.references_tool(&json!({
        "source":"main.veln", "line":4, "column":28
    }));
    let seeded = server.references_tool(&json!({
        "source":"main.veln", "line":4, "column":28,
        "include_declaration":false, "page_size":1
    }));
    let cursor = seeded["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned();
    let uri = crate::definition::path_to_uri(&workspace.path("main.veln"));
    ImportedOperationReferenceState {
        continuation: json!([{
            "uri": uri,
            "range": {"start":{"line":4,"column":28},"end":{"line":4,"column":31}}
        }]),
        resources: all_resource_state(&mut server),
        selection: server.selection_result(),
        _workspace: workspace,
        server,
        before,
        cursor,
    }
}

fn assert_imported_operation_reference_state_is_live(state: &mut ImportedOperationReferenceState) {
    assert_eq!(all_resource_state(&mut state.server), state.resources);
    assert_eq!(state.server.selection_result(), state.selection);
    let continuation = state
        .server
        .references_tool(&json!({"cursor":state.cursor}));
    assert_eq!(continuation["isError"], false, "{continuation:#}");
    assert_eq!(
        continuation["structuredContent"]["scope"],
        state.before["structuredContent"]["scope"]
    );
    assert_eq!(
        continuation["structuredContent"]["references"],
        state.continuation
    );
    assert!(
        continuation["structuredContent"]
            .get("next_cursor")
            .is_none()
    );
    let after = state.server.references_tool(&json!({
        "source":"main.veln", "line":4, "column":28
    }));
    assert_eq!(after, state.before);
}

#[test]
fn invalid_imported_workspace_effect_operation_position_preserves_live_state() {
    let mut state = imported_operation_reference_state();
    let invalid_position = state.server.references_tool(&json!({
        "source":"main.veln", "line":99, "column":1
    }));
    assert_eq!(
        invalid_position["structuredContent"]["code"],
        "invalid_position"
    );
    assert_imported_operation_reference_state_is_live(&mut state);
}

#[test]
fn invalid_imported_workspace_effect_operation_path_preserves_live_state() {
    let mut state = imported_operation_reference_state();
    let invalid_path = state.server.references_tool(&json!({
        "source":"missing.veln", "line":1, "column":1
    }));
    assert_eq!(invalid_path["structuredContent"]["code"], "invalid_path");
    assert_imported_operation_reference_state_is_live(&mut state);
}

#[test]
fn invalid_imported_workspace_effect_operation_cursor_preserves_live_state() {
    let mut state = imported_operation_reference_state();
    let invalid_continuation = state
        .server
        .references_tool(&json!({"cursor":format!("{}x", state.cursor)}));
    assert_eq!(
        invalid_continuation["structuredContent"]["code"],
        "invalid_cursor"
    );
    assert_imported_operation_reference_state_is_live(&mut state);
}

#[test]
fn missing_imported_workspace_effect_operation_resource_preserves_live_state() {
    let mut state = imported_operation_reference_state();
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
    assert_imported_operation_reference_state_is_live(&mut state);
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
        "effect Task\n  run() -> Int\nend\n\nuse dep from \"example/dep\"\n\nfn imported() -> Int effects [dep::Task]\n  perform dep::Task::run()\nend\n\nhandler imported_handler() handles dep::Task\n  run() => 1\nend\n",
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
        ("main.veln", 7, 36),
        ("main.veln", 8, 22),
        ("main.veln", 12, 3),
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
            "end\n\n",
            "handler missing_handler() handles Missing\n",
            "  pick() => 1\n",
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
    let result = server.references_tool(&json!({
        "source":"main.veln", "line":10, "column":3,
        "include_declaration":true
    }));
    assert_eq!(result["structuredContent"]["references"], json!([]));
}

#[test]
fn references_reject_qualified_workspace_handler_clause_headings() {
    let workspace = TempWorkspace::new("references-workspace-handler-qualified-effect");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "mod local\n\neffect Task\n  run() -> Int\nend\n\nhandler qualified() handles foreign::Task\n  run() => 1\nend\n",
    );
    workspace.write(
        "foreign.veln",
        "mod foreign\n\neffect Task\n  run() -> Int\nend\n",
    );
    let mut server = initialized_server(&workspace);

    let result = server.references_tool(&json!({
        "source":"main.veln", "line":8, "column":3,
        "include_declaration":true
    }));
    assert_eq!(result["structuredContent"]["references"], json!([]));
}

#[test]
fn references_reject_standard_library_effect_and_handler_clause_headings() {
    let workspace = TempWorkspace::new("references-workspace-handler-standard-library-effect");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "use transport from \"std\"\n\nhandler standard() handles transport::DuplexStream\n  read_chunk() => 1\nend\n",
    );
    let mut server = initialized_server_with_embedded_resources(&workspace);

    for (line, column) in [(3, 39), (4, 3)] {
        let result = server.references_tool(&json!({
            "source":"main.veln", "line":line, "column":column,
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

    for (line, column, include_declaration) in [(2, 3, false), (6, 19, true)] {
        let result = server.references_tool(&json!({
            "source":"main.veln", "line":line, "column":column,
            "include_declaration":include_declaration
        }));
        assert_eq!(result["structuredContent"]["references"], json!([]));
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
