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
    assert_eq!(
        with_declaration,
        [response(
            "2",
            &format!(
                concat!(
                    "[{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":0,\"character\":7}},\"end\":{{\"line\":0,\"character\":13}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":4,\"character\":28}},\"end\":{{\"line\":4,\"character\":34}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":5,\"character\":17}},\"end\":{{\"line\":5,\"character\":23}}}}}}]"
                ),
                main_uri, main_uri, main_uri,
            ),
        )]
    );

    let operation_leaf = server.handle_message(&references_request_with_declaration(
        &main_uri, 5, 25, true,
    ));
    assert_eq!(
        operation_leaf,
        [response(
            "2",
            &format!(
                "[{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":1,\"character\":2}},\"end\":{{\"line\":1,\"character\":6}}}}}}]",
                main_uri
            ),
        )]
    );
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
        concat!(
            "effect Task\n",
            "  local() -> Int\n",
            "end\n\n",
            "use dep from \"example/dep\"\n\n",
            "fn imported() -> Int effects [dep::Task]\n",
            "  1\n",
            "end\n",
        ),
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
        references_request_with_declaration(&main_uri, 6, 35, true),
        references_request_with_declaration(&invalid_uri, 0, 7, true),
        references_request_with_declaration(&invalid_uri, 4, 28, true),
    ] {
        let response = server.handle_message(&request);
        assert_empty_result_array(&response[0]);
    }
}

#[test]
fn workspace_effect_references_reject_qualified_workspace_effects() {
    let project = TempProject::new("workspace-effect-reference-qualified-workspace");
    project.write("veln.toml", "");
    project.write(
        "main.veln",
        concat!(
            "mod local\n\n",
            "effect E\n",
            "  local() -> Int\n",
            "end\n\n",
            "fn qualified() -> Int effects [foreign::E]\n",
            "  perform foreign::E::run()\n",
            "end\n",
        ),
    );
    project.write(
        "foreign.veln",
        "mod foreign\n\neffect E\n  run() -> Int\nend\n",
    );
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    for (line, character) in [(6, 39), (7, 18)] {
        assert_eq!(
            server.handle_message(&references_request_with_declaration(
                &main_uri, line, character, true,
            )),
            [response("2", "[]")]
        );
    }
}

#[test]
fn workspace_effect_references_reject_unresolved_and_mismatched_occurrences() {
    let project = TempProject::new("workspace-effect-reference-negative-occurrences");
    project.write("veln.toml", "");
    project.write(
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
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    for character in [40, 49] {
        assert_eq!(
            server.handle_message(&references_request_with_declaration(
                &main_uri, 4, character, true,
            )),
            [response("2", "[]")]
        );
    }
}

#[test]
fn workspace_effect_references_reject_generic_ambiguous_and_recovered_occurrences() {
    let generic = TempProject::new("workspace-effect-reference-generic");
    generic.write("veln.toml", "");
    generic.write(
        "main.veln",
        concat!(
            "effect E\n",
            "  run() -> Int\n",
            "end\n\n",
            "fn generic<effect E>() -> Int effects [...E]\n",
            "  1\n",
            "end\n",
        ),
    );
    let generic_root = path_to_uri(&generic.root);
    let generic_uri = path_to_uri(&generic.root.join("main.veln"));
    let mut generic_server = Server::default();
    generic_server.handle_message(&initialize_request(&generic_root));
    assert_eq!(
        generic_server.handle_message(&references_request_with_declaration(
            &generic_uri,
            4,
            42,
            true,
        )),
        [response("2", "[]")]
    );

    let ambiguous = TempProject::new("workspace-effect-reference-ambiguous");
    ambiguous.write("veln.toml", "");
    ambiguous.write(
        "first.veln",
        "mod shared\n\neffect Choose\n  first() -> Int\nend\n",
    );
    ambiguous.write(
        "second.veln",
        concat!(
            "mod shared\n\n",
            "effect Choose\n",
            "  second() -> Int\n",
            "end\n\n",
            "fn choose() -> Int effects [Choose]\n",
            "  1\n",
            "end\n",
        ),
    );
    let ambiguous_root = path_to_uri(&ambiguous.root);
    let first_uri = path_to_uri(&ambiguous.root.join("first.veln"));
    let second_uri = path_to_uri(&ambiguous.root.join("second.veln"));
    let mut ambiguous_server = Server::default();
    ambiguous_server.handle_message(&initialize_request(&ambiguous_root));
    for request in [
        references_request_with_declaration(&first_uri, 2, 7, true),
        references_request_with_declaration(&second_uri, 6, 28, true),
    ] {
        assert_eq!(
            ambiguous_server.handle_message(&request),
            [response("2", "[]")]
        );
    }

    let recovered = TempProject::new("workspace-effect-reference-recovery");
    recovered.write("veln.toml", "");
    recovered.write(
        "main.veln",
        concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn broken() -> Int effects [Choose\n",
            "  1\n",
            "end\n",
        ),
    );
    let recovered_root = path_to_uri(&recovered.root);
    let recovered_uri = path_to_uri(&recovered.root.join("main.veln"));
    let mut recovered_server = Server::default();
    recovered_server.handle_message(&initialize_request(&recovered_root));
    assert_eq!(
        recovered_server.handle_message(&references_request_with_declaration(
            &recovered_uri,
            4,
            28,
            true,
        )),
        [response("2", "[]")]
    );
}

#[test]
fn workspace_effect_references_reject_balanced_recovery_shapes_and_recovered_declarations() {
    let recovered = TempProject::new("workspace-effect-reference-balanced-recovery");
    recovered.write("veln.toml", "");
    recovered.write(
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
    let recovered_root = path_to_uri(&recovered.root);
    let recovered_uri = path_to_uri(&recovered.root.join("main.veln"));
    let mut recovered_server = Server::default();
    recovered_server.handle_message(&initialize_request(&recovered_root));
    for (line, character) in [(9, 11), (10, 10), (11, 16)] {
        assert_eq!(
            recovered_server.handle_message(&references_request_with_declaration(
                &recovered_uri,
                line,
                character,
                true,
            )),
            [response("2", "[]")]
        );
    }

    for (name, declaration, line, character) in [
        (
            "workspace-effect-reference-recovered-row-token",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "end\n\n",
                "fn broken() -> Int effects [Choose @]\n",
                "  1\n",
                "end\n",
            ),
            4,
            28,
        ),
        (
            "workspace-effect-reference-recovered-handler-token",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "end\n\n",
                "handler broken() handles Choose @\n",
                "  pick() => 1\n",
                "end\n",
            ),
            4,
            25,
        ),
    ] {
        let project = TempProject::new(name);
        project.write("veln.toml", "");
        project.write("main.veln", declaration);
        let root = path_to_uri(&project.root);
        let uri = path_to_uri(&project.root.join("main.veln"));
        let mut server = Server::default();
        server.handle_message(&initialize_request(&root));

        for (query_line, query_character, include_declaration) in
            [(line, character, true), (0, 7, false)]
        {
            assert_eq!(
                server.handle_message(&references_request_with_declaration(
                    &uri,
                    query_line,
                    query_character,
                    include_declaration,
                )),
                [response("2", "[]")]
            );
        }
    }

    let declaration = TempProject::new("workspace-effect-reference-recovered-declaration");
    declaration.write("veln.toml", "");
    declaration.write(
        "main.veln",
        "effect Choose\nend\n\nfn use() -> Int effects [Choose]\n  1\nend\n",
    );
    let declaration_root = path_to_uri(&declaration.root);
    let declaration_uri = path_to_uri(&declaration.root.join("main.veln"));
    let mut declaration_server = Server::default();
    declaration_server.handle_message(&initialize_request(&declaration_root));
    assert_eq!(
        declaration_server.handle_message(&references_request_with_declaration(
            &declaration_uri,
            0,
            7,
            true,
        )),
        [response("2", "[]")]
    );

    for (name, text, positions) in [
        (
            "workspace-effect-reference-recovered-perform",
            concat!(
                "effect Choose\n",
                "  pick() -> Int\n",
                "end\n\n",
                "fn broken() -> Int\n",
                "  perform Choose::pick(\n",
                "end\n",
            ),
            vec![(0, 7, false), (5, 10, true)],
        ),
        (
            "workspace-effect-reference-clean-uses-recovered-declaration",
            concat!(
                "effect Choose\n",
                "end\n\n",
                "fn use() -> Int effects [Choose]\n",
                "  perform Choose::pick()\n",
                "end\n\n",
                "handler choose_handler() handles Choose\n",
                "  pick() => perform Choose::pick()\n",
                "end\n",
            ),
            vec![
                (3, 24, true),
                (4, 10, true),
                (7, 32, true),
                (8, 20, true),
            ],
        ),
    ] {
        let project = TempProject::new(name);
        project.write("veln.toml", "");
        project.write("main.veln", text);
        let root = path_to_uri(&project.root);
        let uri = path_to_uri(&project.root.join("main.veln"));
        let mut server = Server::default();
        server.handle_message(&initialize_request(&root));

        for (line, character, include_declaration) in positions {
            assert_eq!(
                server.handle_message(&references_request_with_declaration(
                    &uri,
                    line,
                    character,
                    include_declaration,
                )),
                [response("2", "[]")]
            );
        }
    }
}
