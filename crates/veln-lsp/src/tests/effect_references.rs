#[test]
fn workspace_effect_references_preserve_utf16_crlf_and_declaration_policy() {
    let project = TempProject::new("workspace-effect-references");
    project.write("veln.toml", "");
    project.write(
        "main.veln",
        "effect Choose\r\n  pick() -> Int\r\nend\r\n\r\nfn choose() -> Int effects [Choose]\r\n  \"😀\" + perform Choose::pick()\r\nend\r\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    let without_declaration = server.handle_message(&references_request_with_declaration(
        &main_uri, 5, 17, false,
    ));
    assert_eq!(
        without_declaration,
        [response(
            "2",
            &format!(
                concat!(
                    "[{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":4,\"character\":28}},\"end\":{{\"line\":4,\"character\":34}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":5,\"character\":17}},\"end\":{{\"line\":5,\"character\":23}}}}}}]"
                ),
                main_uri,
                main_uri,
            ),
        )]
    );

    let with_declaration = server.handle_message(&references_request_with_declaration(
        &main_uri, 0, 7, true,
    ));
    assert_eq!(with_declaration[0].matches(&main_uri).count(), 3);
    assert!(with_declaration[0].contains(
        r#""range":{"start":{"line":0,"character":7},"end":{"line":0,"character":13}}"#
    ));
}

#[test]
fn workspace_effect_references_cover_same_module_sources_and_exclude_collisions() {
    let project = TempProject::new("workspace-effect-reference-collisions");
    project.write("veln.toml", "");
    project.write(
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
    project.write(
        "uses.veln",
        concat!(
            "mod shared\n\n",
            "handler choose(callback: fn() -> Int effects [Choose], value: Choose) handles Choose effects [Choose]\n",
            "  pick() => perform Choose::pick()\n",
            "end\n",
        ),
    );
    project.write(
        "other.veln",
        "effect Choose\n  other() -> Int\nend\n\nfn other() -> Int effects [Choose]\n  1\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let declaration_uri = path_to_uri(&project.root.join("declaration.veln"));
    let uses_uri = path_to_uri(&project.root.join("uses.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    let result = server.handle_message(&references_request_with_declaration(
        &declaration_uri,
        2,
        7,
        false,
    ));
    assert_eq!(
        result,
        [response(
            "2",
            &format!(
                concat!(
                    "[{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":6,\"character\":28}},\"end\":{{\"line\":6,\"character\":34}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":7,\"character\":10}},\"end\":{{\"line\":7,\"character\":16}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":2,\"character\":46}},\"end\":{{\"line\":2,\"character\":52}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":2,\"character\":78}},\"end\":{{\"line\":2,\"character\":84}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":2,\"character\":94}},\"end\":{{\"line\":2,\"character\":100}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":3,\"character\":20}},\"end\":{{\"line\":3,\"character\":26}}}}}}]"
                ),
                declaration_uri,
                declaration_uri,
                uses_uri,
                uses_uri,
                uses_uri,
                uses_uri,
            ),
        )]
    );
}

#[test]
fn workspace_effect_references_reject_imported_and_invalid_cased_effects() {
    let project = TempProject::new("workspace-effect-reference-negative-origins");
    project.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    project.write(
        "main.veln",
        "use dep from \"example/dep\"\n\nfn imported() -> Int effects [dep::Task]\n  1\nend\n",
    );
    project.write(
        "invalid.veln",
        "effect choose\n  pick() -> Int\nend\n\nfn invalid() -> Int effects [choose]\n  1\nend\n",
    );
    project.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    project.write(
        "vendor/dep/dep.veln",
        "pub effect Task\n  run() -> Int\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let invalid_uri = path_to_uri(&project.root.join("invalid.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    for request in [
        references_request_with_declaration(&main_uri, 2, 35, true),
        references_request_with_declaration(&invalid_uri, 0, 7, true),
        references_request_with_declaration(&invalid_uri, 4, 28, true),
    ] {
        let response = server.handle_message(&request);
        assert_empty_result_array(&response[0]);
    }
}
