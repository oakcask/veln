use super::*;

#[test]
fn unqualified_workspace_navigation_does_not_build_a_classification_context() {
    let dependencies = IndexedDependencies::new_direct(vec![crate::tests::dependency_snapshot(
        "example/pkg",
        &[(
            "helper.veln",
            "use helper\npub fn answer() -> Int\n  42\nend\npub fn call() -> Int\n  helper::answer()\nend\n",
        )],
        ["helper.veln"],
    )]);
    assert!(!dependencies.files[0].classified_path_segments.is_empty());
    let standard_library = IndexedDependencies::new_standard_library(None);
    for name in ["good", "Bad"] {
        let source = SourceFile::new(
            "main.veln",
            format!("fn {name}() -> Int\n  1\nend\nfn caller() -> Int\n  {name}()\nend\n"),
        );
        let (file, _, parsed) = index_workspace_source(source.clone());
        let mut module = empty_surface_module();
        append_parsed_surface_module(&mut module, &file, &parsed);
        let mut project = module.clone();
        append_surface_module(&mut project, dependencies.module.clone());
        assert!(
            veln_sema::classified_project_qualified_path_segments_with_context(&module, &project)
                .is_empty(),
        );
        PATH_CLASSIFICATION_CONTEXTS.set(0);
        let index = Arc::new(SymbolIndex::new(
            vec![source],
            &dependencies,
            &standard_library,
        ));
        assert!(
            index
                .symbol_at_position(
                    "main.veln",
                    &SourcePosition {
                        source: SourcePath::new("main.veln"),
                        line: 5,
                        column: 3,
                    },
                )
                .is_some(),
            "unqualified function selection must preserve recovery for {name}",
        );
        assert_eq!(PATH_CLASSIFICATION_CONTEXTS.get(), 0);
    }
}

#[test]
fn workspace_classification_matches_sema_for_qualified_and_invalid_paths() {
    let dependencies = IndexedDependencies::new_direct(Vec::new());
    let standard_library = IndexedDependencies::new_standard_library(None);
    for text in [
        "use main\npub fn target() -> Int\n  1\nend\nfn caller() -> Int\n  main::target()\nend\n",
        "use main\npub fn target() -> Int\n  1\nend\nfn caller() -> Int\n  Main::target()\nend\n",
        "use Bad\nfn caller() -> Int\n  1\nend\n",
        "use Bad.foo\nfn caller() -> Int\n  1\nend\n",
        "pub type Item\n  pub Ready\nend\nfn caller(value: main::Item) -> main::Item\n  main::Item::Ready\nend\n",
    ] {
        let source = SourceFile::new("main.veln", text);
        let (file, _, parsed) = index_workspace_source(source.clone());
        assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
        let mut module = empty_surface_module();
        append_parsed_surface_module(&mut module, &file, &parsed);
        let expected = veln_sema::classified_project_qualified_path_segments(&module);
        assert!(!expected.is_empty(), "fixture must classify a path: {text}");
        let index = SymbolIndex::new(vec![source], &dependencies, &standard_library);
        assert_eq!(index.files[0].classified_path_segments, expected, "{text}");
    }
}
