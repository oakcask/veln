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
fn workspace_effect_references_include_predicate_perform_qualifiers() {
    let project = TempProject::new("workspace-effect-reference-predicates");
    project.write("veln.toml", "");
    project.write(
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
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));

    let result = server.handle_message(&references_request_with_declaration(
        &main_uri, 5, 18, false,
    ));
    assert_eq!(
        result,
        [response(
            "2",
            &format!(
                concat!(
                    "[{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":5,\"character\":18}},\"end\":{{\"line\":5,\"character\":24}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":6,\"character\":60}},\"end\":{{\"line\":6,\"character\":66}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":12,\"character\":29}},\"end\":{{\"line\":12,\"character\":35}}}}}},",
                    "{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":13,\"character\":19}},\"end\":{{\"line\":13,\"character\":25}}}}}}]"
                ),
                main_uri, main_uri, main_uri, main_uri,
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
fn workspace_effect_references_reject_generic_occurrences() {
    assert_empty_effect_references(
        "workspace-effect-reference-generic",
        concat!(
            "effect E\n",
            "  run() -> Int\n",
            "end\n\n",
            "fn generic<effect E>() -> Int effects [...E]\n",
            "  1\n",
            "end\n",
        ),
        &[(4, 42, true)],
    );
}

#[test]
fn workspace_effect_references_reject_ambiguous_declarations() {
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
}

#[test]
fn workspace_effect_references_reject_recovered_occurrences() {
    assert_empty_effect_references(
        "workspace-effect-reference-recovery",
        concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn broken() -> Int effects [Choose\n",
            "  1\n",
            "end\n",
        ),
        &[(4, 28, true)],
    );
}

#[test]
fn workspace_effect_references_keep_complete_qualifiers_with_recovered_arguments() {
    let project = TempProject::new("workspace-effect-reference-recovered-argument");
    project.write("veln.toml", "");
    project.write(
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
    let root_uri = path_to_uri(&project.root);
    let main_uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root_uri));
    let expected = format!(
        "[{{\"uri\":\"{}\",\"range\":{{\"start\":{{\"line\":5,\"character\":10}},\"end\":{{\"line\":5,\"character\":16}}}}}}]",
        main_uri
    );

    for request in [
        references_request_with_declaration(&main_uri, 0, 7, false),
        references_request_with_declaration(&main_uri, 5, 10, false),
    ] {
        assert_eq!(server.handle_message(&request), [response("2", &expected)]);
    }
}

#[test]
fn workspace_effect_references_reject_balanced_recovery_shapes_and_recovered_declarations() {
    assert_empty_effect_references(
        "workspace-effect-reference-balanced-recovery",
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
        &[(9, 11, true), (10, 10, true), (11, 16, true)],
    );
    assert_empty_effect_references(
        "workspace-effect-reference-recovered-row-token",
        concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn broken() -> Int effects [Choose @]\n",
            "  1\n",
            "end\n",
        ),
        &[(4, 28, true), (0, 7, false)],
    );
    assert_empty_effect_references(
        "workspace-effect-reference-recovered-handler-token",
        concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "handler broken() handles Choose @\n",
            "  pick() => 1\n",
            "end\n",
        ),
        &[(4, 25, true), (0, 7, false)],
    );
    assert_empty_effect_references(
        "workspace-effect-reference-recovered-declaration",
        "effect Choose\nend\n\nfn use() -> Int effects [Choose]\n  1\nend\n",
        &[(0, 7, true)],
    );
    assert_empty_effect_references(
        "workspace-effect-reference-recovered-perform",
        concat!(
            "effect Choose\n",
            "  pick() -> Int\n",
            "end\n\n",
            "fn broken() -> Int\n",
            "  perform Choose::pick(\n",
            "end\n",
        ),
        &[(0, 7, false), (5, 10, true)],
    );
    assert_empty_effect_references(
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
        &[
            (3, 24, true),
            (4, 10, true),
            (7, 32, true),
            (8, 20, true),
        ],
    );
}

fn assert_empty_effect_references(
    name: &str,
    text: &str,
    positions: &[(usize, usize, bool)],
) {
    let project = TempProject::new(name);
    project.write("veln.toml", "");
    project.write("main.veln", text);
    let root = path_to_uri(&project.root);
    let uri = path_to_uri(&project.root.join("main.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root));

    for &(line, character, include_declaration) in positions {
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

#[test]
fn recovered_effect_declaration_makes_a_clean_same_module_declaration_ambiguous() {
    let project = TempProject::new("workspace-effect-reference-recovered-ambiguity");
    project.write("veln.toml", "");
    project.write(
        "clean.veln",
        "mod shared\n\neffect Choose\n  pick() -> Int\nend\n",
    );
    project.write("recovered.veln", "mod shared\n\neffect Choose\nend\n");
    project.write(
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
    let root = path_to_uri(&project.root);
    let clean_uri = path_to_uri(&project.root.join("clean.veln"));
    let recovered_uri = path_to_uri(&project.root.join("recovered.veln"));
    let use_uri = path_to_uri(&project.root.join("use.veln"));
    let mut server = Server::default();
    server.handle_message(&initialize_request(&root));

    for request in [
        references_request_with_declaration(&clean_uri, 2, 7, true),
        references_request_with_declaration(&recovered_uri, 2, 7, true),
        references_request_with_declaration(&use_uri, 2, 24, true),
        references_request_with_declaration(&use_uri, 3, 10, true),
        references_request_with_declaration(&use_uri, 6, 29, true),
        references_request_with_declaration(&use_uri, 7, 19, true),
    ] {
        assert_eq!(server.handle_message(&request), [response("2", "[]")]);
    }
}
