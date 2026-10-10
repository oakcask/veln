use super::*;
use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

use veln_language_service::{
    EffectiveProjectSnapshot, NavigationSource, RenameFailureKind, SourcePosition,
    navigate_for_rename, validate_rename_in_snapshot,
};
use veln_project::PackageSnapshotSource;
use veln_source::{SourceFile, SourcePath};

use super::dependency_resources::fill_dependency_resource_capacity_completely;
use super::references_support::{
    assert_reference_ranges, dependency_resource_is_listed,
    write_workspace_with_dependency_and_sources,
};

fn rename_result(
    workspace: &TempWorkspace,
    source: &str,
    line: usize,
    column: usize,
    new_name: &str,
) -> Value {
    initialized_server(workspace).rename_tool(&json!({
        "source": source,
        "line": line,
        "column": column,
        "new_name": new_name
    }))
}

#[test]
fn variant_refinement_navigation_uses_shared_constructor_identity() {
    let workspace = TempWorkspace::new("variant-refinement-navigation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready(Int)\n",
            "  pub Closed\n",
            "end\n\n",
            "pub type Alias = State\n\n",
            "fn observe(value: State::Ready, other: Alias::Ready | Alias::Closed) -> State\n",
            "  let made = State::Ready(1)\n",
            "  match value\n",
            "    State::Ready(payload) => made\n",
            "    State::Closed => made\n",
            "  end\n",
            "end\n\n",
            "pub type Other\n",
            "  pub Ready\n",
            "end\n\n",
            "fn observe_other(value: Other::Ready) -> Other\n",
            "  Other::Ready\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);

    let direct_base = server.definition_tool(&json!({
        "source":"main.veln", "line":8, "column":20
    }));
    assert_eq!(
        direct_base["structuredContent"]["definition"]["range"],
        json!({"start":{"line":1,"column":10},"end":{"line":1,"column":15}}),
        "{direct_base:#}"
    );
    let alias_base = server.definition_tool(&json!({
        "source":"main.veln", "line":8, "column":41
    }));
    assert_eq!(
        alias_base["structuredContent"]["definition"]["range"],
        json!({"start":{"line":6,"column":10},"end":{"line":6,"column":15}}),
        "{alias_base:#}"
    );
    let definition = server.definition_tool(&json!({"source":"main.veln","line":8,"column":27}));
    assert_eq!(
        definition["structuredContent"]["definition"]["range"],
        json!({"start":{"line":2,"column":7},"end":{"line":2,"column":12}}),
        "{definition:#}"
    );
    let references = server.references_tool(&json!({
        "source":"main.veln", "line":8, "column":48,
        "include_declaration":true
    }));
    assert_eq!(
        references["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        5,
        "{references:#}"
    );
    let direct_references = server.references_tool(&json!({
        "source":"main.veln", "line":8, "column":27,
        "include_declaration":true
    }));
    assert_eq!(
        direct_references["structuredContent"]["references"],
        references["structuredContent"]["references"],
        "direct={direct_references:#}\nalias={references:#}"
    );
    let renamed = server.rename_tool(&json!({
        "source":"main.veln", "line":8, "column":48, "new_name":"Prepared"
    }));
    assert_eq!(edits(&renamed).len(), 5, "{renamed:#}");

    let alias_renamed = server.rename_tool(&json!({
        "source":"main.veln", "line":8, "column":41, "new_name":"Phase"
    }));
    assert_eq!(edits(&alias_renamed).len(), 3, "{alias_renamed:#}");
    assert_eq!(
        edits(&alias_renamed)
            .iter()
            .map(|edit| edit["range"]["start"]["line"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![6, 8, 8],
        "{alias_renamed:#}"
    );
}

#[test]
fn invalid_variant_refinement_alias_bases_do_not_select_or_rename() {
    let workspace = TempWorkspace::new("invalid-variant-refinement-alias-base");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "end\n\n",
            "type Other\n",
            "  Shared\n",
            "end\n\n",
            "type Helper\n",
            "end\n\n",
            "pub type Alias = State\n\n",
            "fn valid_single(value: Alias::Ready) -> Int\n  0\nend\n\n",
            "fn valid_union(value: Alias::Ready | Alias::Ready) -> Int\n  0\nend\n\n",
            "fn missing(value: Alias::Missing) -> Int\n  0\nend\n\n",
            "fn wrong_owner(value: Alias::Shared) -> Int\n  0\nend\n\n",
            "fn non_constructor(value: Alias::Helper) -> Int\n  0\nend\n",
        ),
    );
    let mut server = initialized_server(&workspace);

    for (line, column) in [(22, 19), (26, 23), (30, 27)] {
        let definition = server.definition_tool(&json!({
            "source": "main.veln", "line": line, "column": column
        }));
        assert_eq!(
            definition["structuredContent"]["definition"],
            Value::Null,
            "{definition:#}"
        );

        let renamed = server.rename_tool(&json!({
            "source": "main.veln", "line": line, "column": column,
            "new_name": "RenamedAlias"
        }));
        assert!(edits(&renamed).is_empty(), "{renamed:#}");
    }

    for (line, column) in [(14, 24), (18, 23), (18, 38)] {
        let renamed = server.rename_tool(&json!({
            "source": "main.veln", "line": line, "column": column,
            "new_name": "RenamedAlias"
        }));
        assert!(!edits(&renamed).is_empty(), "{renamed:#}");
    }
}

#[test]
fn ambiguous_package_refinement_bases_do_not_select_or_rename() {
    let source = concat!(
        "use shared from \"first/pkg\"\n",
        "use shared from \"second/pkg\"\n\n",
        "pub type Alias = shared::State\n",
        "pub type Chain = Alias\n\n",
        "fn direct_single(value: shared::State::Ready) -> Int\n  0\nend\n\n",
        "fn direct_union(value: shared::State::Ready | shared::State::Closed) -> Int\n  0\nend\n\n",
        "fn alias_single(value: Chain::Ready) -> Int\n  0\nend\n\n",
        "fn alias_union(value: Chain::Ready | Chain::Closed) -> Int\n  0\nend\n",
    );
    let dependency_source = "pub type State\n  pub Ready\n  pub Closed\nend\n";

    for (name, dependencies) in [
        (
            "ambiguous-package-refinement-forward",
            ["first/pkg", "second/pkg"],
        ),
        (
            "ambiguous-package-refinement-reverse",
            ["second/pkg", "first/pkg"],
        ),
    ] {
        let workspace = TempWorkspace::new(name);
        workspace.write(
            "veln.toml",
            &format!(
                "[dependencies.\"{}\"]\npath = \"vendor/{}\"\n\n[dependencies.\"{}\"]\npath = \"vendor/{}\"\n",
                dependencies[0], dependencies[0], dependencies[1], dependencies[1],
            ),
        );
        workspace.write("main.veln", source);
        for identity in dependencies {
            workspace.write(
                &format!("vendor/{identity}/veln.toml"),
                &format!(
                    "[package]\nname = \"{identity}\"\n\n[lib]\nexports = [\"shared.veln\"]\n"
                ),
            );
            workspace.write(&format!("vendor/{identity}/shared.veln"), dependency_source);
        }
        let mut server = initialized_server(&workspace);

        for line_text in [
            "direct_single",
            "direct_union",
            "alias_single",
            "alias_union",
        ] {
            let (line, source_line) = source
                .lines()
                .enumerate()
                .find(|(_, candidate)| candidate.contains(line_text))
                .unwrap();
            let base = if line_text.starts_with("direct") {
                "State"
            } else {
                "Chain"
            };
            for column in source_line
                .match_indices(base)
                .map(|(offset, _)| offset + 1)
                .chain(
                    source_line
                        .match_indices("Ready")
                        .map(|(offset, _)| offset + 1),
                )
                .chain(
                    source_line
                        .match_indices("Closed")
                        .map(|(offset, _)| offset + 1),
                )
            {
                let line = line + 1;
                let definition = server
                    .definition_tool(&json!({"source":"main.veln", "line":line, "column":column}));
                assert_eq!(
                    definition["structuredContent"]["definition"],
                    Value::Null,
                    "{definition:#}"
                );
                let references = server.references_tool(&json!({
                    "source":"main.veln", "line":line, "column":column,
                    "include_declaration":true
                }));
                assert_eq!(
                    references["structuredContent"]["references"],
                    json!([]),
                    "{references:#}"
                );
                let renamed = server.rename_tool(&json!({
                    "source":"main.veln", "line":line, "column":column,
                    "new_name":"Packed"
                }));
                assert!(edits(&renamed).is_empty(), "{renamed:#}");
            }
        }
    }
}

#[test]
fn variant_refinement_navigation_rejects_wrong_generic_arity() {
    let workspace = TempWorkspace::new("variant-refinement-generic-arity");
    workspace.write("veln.toml", "");
    let source = concat!(
        "pub type Box<A>\n",
        "  pub Boxed(A)\n",
        "end\n\n",
        "pub type GenericAlias = Box\n\n",
        "fn valid(value: GenericAlias<Int>::Boxed) -> Int\n  0\nend\n\n",
        "fn valid_union(value: GenericAlias<Int>::Boxed | GenericAlias<Int>::Boxed) -> Int\n  0\nend\n\n",
        "fn missing_direct(value: Box::Boxed) -> Int\n  0\nend\n\n",
        "fn excess_direct(value: Box<Int, Int>::Boxed) -> Int\n  0\nend\n\n",
        "fn missing_alias(value: GenericAlias::Boxed) -> Int\n  0\nend\n\n",
        "fn excess_alias(value: GenericAlias<Int, Int>::Boxed) -> Int\n  0\nend\n",
    );
    workspace.write("main.veln", source);
    let mut server = initialized_server(&workspace);
    let position = |line_text: &str, needle: &str| {
        let (line, source_line) = source
            .lines()
            .enumerate()
            .find(|(_, candidate)| candidate.contains(line_text))
            .unwrap();
        (line + 1, source_line.find(needle).unwrap() + 1)
    };

    for (line_text, base) in [
        ("missing_direct", "Box::"),
        ("excess_direct", "Box<Int"),
        ("missing_alias", "GenericAlias::"),
        ("excess_alias", "GenericAlias<Int"),
    ] {
        for needle in [base, "Boxed"] {
            let (line, column) = position(line_text, needle);
            let definition = server.definition_tool(&json!({
                "source": "main.veln", "line": line, "column": column
            }));
            assert_eq!(
                definition["structuredContent"]["definition"],
                Value::Null,
                "{definition:#}"
            );
            let references = server.references_tool(&json!({
                "source": "main.veln", "line": line, "column": column,
                "include_declaration": true
            }));
            assert_eq!(
                references["structuredContent"]["references"],
                json!([]),
                "{references:#}"
            );
            let renamed = server.rename_tool(&json!({
                "source": "main.veln", "line": line, "column": column,
                "new_name": "Packed"
            }));
            assert!(edits(&renamed).is_empty(), "{renamed:#}");
        }
    }

    let (line, column) = position("fn valid(", "Boxed");
    let definition = server.definition_tool(&json!({
        "source": "main.veln", "line": line, "column": column
    }));
    assert_ne!(
        definition["structuredContent"]["definition"],
        Value::Null,
        "{definition:#}"
    );
    let references = server.references_tool(&json!({
        "source": "main.veln", "line": line, "column": column,
        "include_declaration": true
    }));
    assert_eq!(
        references["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        4,
        "{references:#}"
    );
    let renamed = server.rename_tool(&json!({
        "source": "main.veln", "line": line, "column": column,
        "new_name": "Packed"
    }));
    assert_eq!(edits(&renamed).len(), 4, "{renamed:#}");
}

#[test]
fn variant_refinement_navigation_rejects_invalid_union_identity() {
    let workspace = TempWorkspace::new("variant-refinement-invalid-union-identity");
    workspace.write("veln.toml", "");
    let source = concat!(
        "pub type Left\n  pub LeftReady\nend\n\n",
        "pub type Right\n  pub RightReady\nend\n\n",
        "pub type Box<A>\n  pub Boxed(A)\n  pub Empty\nend\n\n",
        "fn cross_base(value: Left::LeftReady | Right::RightReady) -> Int\n  0\nend\n\n",
        "fn different_args(value: Box<Int>::Boxed | Box<String>::Empty) -> Int\n  0\nend\n",
    );
    workspace.write("main.veln", source);
    let mut server = initialized_server(&workspace);
    let position = |line_text: &str, needle: &str| {
        let (line, source_line) = source
            .lines()
            .enumerate()
            .find(|(_, candidate)| candidate.contains(line_text))
            .unwrap();
        (line + 1, source_line.find(needle).unwrap() + 1)
    };

    for (line_text, needle) in [
        ("cross_base", "Left::"),
        ("cross_base", "LeftReady"),
        ("cross_base", "Right::"),
        ("cross_base", "RightReady"),
        ("different_args", "Box<Int>"),
        ("different_args", "Boxed"),
        ("different_args", "Box<String>"),
        ("different_args", "Empty"),
    ] {
        let (line, column) = position(line_text, needle);
        let definition = server.definition_tool(&json!({
            "source": "main.veln", "line": line, "column": column
        }));
        assert_eq!(
            definition["structuredContent"]["definition"],
            Value::Null,
            "{definition:#}"
        );
        let references = server.references_tool(&json!({
            "source": "main.veln", "line": line, "column": column,
            "include_declaration": true
        }));
        assert_eq!(
            references["structuredContent"]["references"],
            json!([]),
            "{references:#}"
        );
        let renamed = server.rename_tool(&json!({
            "source": "main.veln", "line": line, "column": column,
            "new_name": "Renamed"
        }));
        assert!(edits(&renamed).is_empty(), "{renamed:#}");
    }
}

#[test]
fn variant_refinement_navigation_rejects_unresolved_generic_arguments() {
    let workspace = TempWorkspace::new("variant-refinement-unresolved-generic-arguments");
    workspace.write("veln.toml", "");
    let source = concat!(
        "pub type Box<A>\n  pub Boxed(A)\n  pub Empty\nend\n\n",
        "pub type GenericAlias = Box\n\n",
        "fn valid(value: Box<Int>::Boxed) -> Int\n  0\nend\n\n",
        "fn direct_single(value: Box<Missing>::Boxed) -> Int\n  0\nend\n\n",
        "fn direct_union(value: Box<Missing>::Boxed | Box<Missing>::Empty) -> Int\n  0\nend\n\n",
        "fn alias_single(value: GenericAlias<Missing>::Boxed) -> Int\n  0\nend\n\n",
        "fn alias_union(value: GenericAlias<Missing>::Boxed | GenericAlias<Missing>::Empty) -> Int\n  0\nend\n",
    );
    workspace.write("main.veln", source);
    let mut server = initialized_server(&workspace);
    let position = |line_text: &str, needle: &str| {
        let (line, source_line) = source
            .lines()
            .enumerate()
            .find(|(_, candidate)| candidate.contains(line_text))
            .unwrap();
        (line + 1, source_line.find(needle).unwrap() + 1)
    };

    for (line_text, base) in [
        ("direct_single", "Box<Missing>"),
        ("direct_union", "Box<Missing>"),
        ("alias_single", "GenericAlias<Missing>"),
        ("alias_union", "GenericAlias<Missing>"),
    ] {
        for needle in [base, "Boxed"] {
            let (line, column) = position(line_text, needle);
            let definition = server.definition_tool(&json!({
                "source": "main.veln", "line": line, "column": column
            }));
            assert_eq!(
                definition["structuredContent"]["definition"],
                Value::Null,
                "{definition:#}"
            );
            let references = server.references_tool(&json!({
                "source": "main.veln", "line": line, "column": column,
                "include_declaration": true
            }));
            assert_eq!(
                references["structuredContent"]["references"],
                json!([]),
                "{references:#}"
            );
            let renamed = server.rename_tool(&json!({
                "source": "main.veln", "line": line, "column": column,
                "new_name": "Packed"
            }));
            assert!(edits(&renamed).is_empty(), "{renamed:#}");
        }
    }

    let (line, column) = position("fn valid(", "Boxed");
    let references = server.references_tool(&json!({
        "source": "main.veln", "line": line, "column": column,
        "include_declaration": true
    }));
    assert_eq!(
        references["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        2,
        "{references:#}"
    );
    let renamed = server.rename_tool(&json!({
        "source": "main.veln", "line": line, "column": column,
        "new_name": "Packed"
    }));
    assert_eq!(edits(&renamed).len(), 2, "{renamed:#}");
}

#[test]
fn variant_refinement_navigation_projects_unicode_scalar_ranges() {
    let workspace = TempWorkspace::new("variant-refinement-unicode-scalar-ranges");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "pub type State\n",
            "  pub Ready(Int)\n",
            "end\n\n",
            "fn observe(value: State::Ready) -> {label: String, state: State}\n",
            "  {label: \"😀\", state: keep<State::Ready>(\"x\", value)}\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);

    let definition = server.definition_tool(&json!({
        "source":"main.veln", "line":6, "column":36
    }));
    assert_eq!(
        definition["structuredContent"]["definition"]["range"],
        json!({"start":{"line":2,"column":7},"end":{"line":2,"column":12}}),
        "{definition:#}"
    );
    let references = server.references_tool(&json!({
        "source":"main.veln", "line":6, "column":36,
        "include_declaration":true
    }));
    assert_reference_ranges(
        &references,
        &[
            ("main.veln", 2, 7, 2, 12),
            ("main.veln", 5, 26, 5, 31),
            ("main.veln", 6, 35, 6, 40),
        ],
        "variant refinement Unicode-scalar references",
    );
    let renamed = server.rename_tool(&json!({
        "source":"main.veln", "line":6, "column":36, "new_name":"Prepared"
    }));
    assert_eq!(
        edits(&renamed)
            .iter()
            .map(|edit| edit["range"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!({"start":{"line":2,"column":7},"end":{"line":2,"column":12}}),
            json!({"start":{"line":5,"column":26},"end":{"line":5,"column":31}}),
            json!({"start":{"line":6,"column":35},"end":{"line":6,"column":40}}),
        ],
        "{renamed:#}"
    );
}

#[test]
fn variant_refinement_navigation_projects_generic_transitive_and_imported_aliases() {
    let workspace = TempWorkspace::new("variant-refinement-alias-chain-navigation");
    workspace.write("veln.toml", "");
    workspace.write(
        "model.veln",
        concat!(
            "pub type State\n",
            "  pub Ready(Int)\n",
            "  pub Closed\n",
            "end\n\n",
            "pub type A = State\n",
            "pub type B = A\n",
            "pub type Alias = State\n",
        ),
    );
    workspace.write(
        "main.veln",
        concat!(
            "use model\n\n",
            "pub type Box<A>\n",
            "  pub Boxed(A)\n",
            "end\n",
            "pub type GenericAlias = Box\n\n",
            "fn generic(value: GenericAlias<Int>::Boxed) -> Box<Int>\n  value\nend\n\n",
            "fn transitive(value: model::B::Ready) -> model::State\n  value\nend\n\n",
            "fn imported(value: model::Alias::Ready | model::Alias::Closed) -> model::State\n",
            "  let made = model::State::Ready(1)\n",
            "  match value\n",
            "    model::State::Ready(payload) => made\n",
            "    model::State::Closed => made\n",
            "  end\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);

    let generic_base = server.definition_tool(&json!({
        "source":"main.veln", "line":8, "column":19
    }));
    assert_eq!(
        generic_base["structuredContent"]["definition"]["range"],
        json!({"start":{"line":6,"column":10},"end":{"line":6,"column":22}}),
        "{generic_base:#}"
    );
    let generic_variant = server.definition_tool(&json!({
        "source":"main.veln", "line":8, "column":38
    }));
    assert_eq!(
        generic_variant["structuredContent"]["definition"]["range"],
        json!({"start":{"line":4,"column":7},"end":{"line":4,"column":12}}),
        "{generic_variant:#}"
    );
    let transitive = server.definition_tool(&json!({
        "source":"main.veln", "line":12, "column":32
    }));
    assert_eq!(
        transitive["structuredContent"]["definition"]["range"],
        json!({"start":{"line":2,"column":7},"end":{"line":2,"column":12}}),
        "{transitive:#}"
    );
    let imported_base = server.definition_tool(&json!({
        "source":"main.veln", "line":16, "column":27
    }));
    assert_eq!(
        imported_base["structuredContent"]["definition"]["range"],
        json!({"start":{"line":8,"column":10},"end":{"line":8,"column":15}}),
        "{imported_base:#}"
    );
    let references = server.references_tool(&json!({
        "source":"main.veln", "line":16, "column":34,
        "include_declaration":true
    }));
    assert_eq!(
        references["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .len(),
        5,
        "{references:#}"
    );
    let renamed = server.rename_tool(&json!({
        "source":"main.veln", "line":12, "column":32, "new_name":"Prepared"
    }));
    assert_eq!(edits(&renamed).len(), 5, "{renamed:#}");
    let alias_renamed = server.rename_tool(&json!({
        "source":"main.veln", "line":8, "column":19, "new_name":"GenericBox"
    }));
    assert_eq!(edits(&alias_renamed).len(), 2, "{alias_renamed:#}");
}

#[test]
fn standard_library_alias_to_implicit_prelude_type_projects_definitions() {
    let workspace = TempWorkspace::new("standard-library-implicit-prelude-alias-navigation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "use bridge from \"std\"\n\n",
            "pub fn observe(value: bridge::Alias::Ready) -> Int\n",
            "  0\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        concat!(
            "[package]\nname = \"std\"\n\n",
            "[lib]\nexports = [\"prelude.veln\", \"bridge.veln\"]\n",
        ),
        [
            PackageSnapshotSource::new("prelude.veln", b"pub type State\n  pub Ready\nend\n"),
            PackageSnapshotSource::new("bridge.veln", b"pub type Alias = State\n"),
        ],
    );

    let alias = server.definition_tool(&json!({
        "source":"main.veln", "line":3, "column":31
    }));
    assert!(
        alias["structuredContent"]["definition"]["uri"]
            .as_str()
            .is_some_and(|uri| uri.ends_with("/bridge.veln")),
        "{alias:#}"
    );
    assert_eq!(
        alias["structuredContent"]["definition"]["range"],
        json!({"start":{"line":1,"column":10},"end":{"line":1,"column":15}}),
        "{alias:#}"
    );

    let variant = server.definition_tool(&json!({
        "source":"main.veln", "line":3, "column":38
    }));
    assert!(
        variant["structuredContent"]["definition"]["uri"]
            .as_str()
            .is_some_and(|uri| uri.ends_with("/prelude.veln")),
        "{variant:#}"
    );
    assert_eq!(
        variant["structuredContent"]["definition"]["range"],
        json!({"start":{"line":2,"column":7},"end":{"line":2,"column":12}}),
        "{variant:#}"
    );

    for column in [31, 38] {
        let renamed = server.rename_tool(&json!({
            "source":"main.veln", "line":3, "column":column, "new_name":"Renamed"
        }));
        assert!(edits(&renamed).is_empty(), "{renamed:#}");
    }
}

fn refinement_variant_positions(source: &str, line_text: &str) -> Vec<(usize, usize)> {
    let (line, text) = source
        .lines()
        .enumerate()
        .find(|(_, candidate)| candidate.contains(line_text))
        .unwrap();
    text.match_indices("Boxed")
        .chain(text.match_indices("Empty"))
        .map(|(column, _)| (line + 1, column + 1))
        .collect()
}

fn assert_retained_refinement_position_is_navigable(
    server: &mut Server,
    line_text: &str,
    line: usize,
    column: usize,
) {
    let definition =
        server.definition_tool(&json!({"source":"main.veln", "line":line, "column":column}));
    assert!(
        definition["structuredContent"]["definition"]["uri"]
            .as_str()
            .is_some_and(|uri| uri.ends_with("/prelude.veln")),
        "{line_text}: {definition:#}"
    );
    let references = server.references_tool(&json!({
        "source":"main.veln", "line":line, "column":column,
        "include_declaration":true
    }));
    assert!(
        !references["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{line_text}: {references:#}"
    );
    let renamed = server.rename_tool(&json!({
        "source":"main.veln", "line":line, "column":column, "new_name":"Renamed"
    }));
    assert!(edits(&renamed).is_empty(), "{line_text}: {renamed:#}");
}

fn assert_retained_refinement_position_is_ineligible(
    server: &mut Server,
    line_text: &str,
    line: usize,
    column: usize,
) {
    let definition =
        server.definition_tool(&json!({"source":"main.veln", "line":line, "column":column}));
    assert!(
        definition["structuredContent"]["definition"].is_null(),
        "{line_text}: {definition:#}"
    );
    let references = server.references_tool(&json!({
        "source":"main.veln", "line":line, "column":column,
        "include_declaration":true
    }));
    assert!(
        references["structuredContent"]["references"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{line_text}: {references:#}"
    );
    let renamed = server.rename_tool(&json!({
        "source":"main.veln", "line":line, "column":column, "new_name":"Renamed"
    }));
    assert!(edits(&renamed).is_empty(), "{line_text}: {renamed:#}");
}

#[test]
fn retained_package_refinement_unions_project_canonical_generic_arguments() {
    let workspace = TempWorkspace::new("retained-package-refinement-canonical-arguments");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/bridge\"]\npath = \"vendor/bridge\"\n",
    );
    workspace.write(
        "vendor/bridge/veln.toml",
        "[package]\nname = \"example/bridge\"\n\n[lib]\nexports = [\"bridge.veln\"]\n",
    );
    workspace.write("vendor/bridge/bridge.veln", "pub type Alias = Box\n");
    let source = concat!(
        "use bridge from \"example/bridge\"\n\n",
        "pub type Phase\n  pub Started\nend\n",
        "pub type OtherPhase\n  pub Started\nend\n",
        "pub type PhaseAlias = Phase\n\n",
        "pub type Envelope<A, B>\n",
        "  pub Same(bridge::Alias<A>::Boxed | bridge::Alias<A>::Empty)\n",
        "  pub Different(bridge::Alias<A>::Boxed | bridge::Alias<B>::Empty)\n",
        "end\n\n",
        "fn direct_same(value: Box<Phase>::Boxed | Box<Phase>::Empty) -> Int\n  0\nend\n\n",
        "fn direct_alias(value: Box<PhaseAlias>::Boxed | Box<Phase>::Empty) -> Int\n  0\nend\n\n",
        "fn qualified_same(value: bridge::Alias<Phase>::Boxed | bridge::Alias<Phase>::Empty) -> Int\n  0\nend\n\n",
        "fn qualified_alias(value: bridge::Alias<PhaseAlias>::Boxed | bridge::Alias<Phase>::Empty) -> Int\n  0\nend\n\n",
        "fn mismatch(value: bridge::Alias<Phase>::Boxed | bridge::Alias<OtherPhase>::Empty) -> Int\n  0\nend\n\n",
        "fn unresolved(value: bridge::Alias<Missing>::Boxed | bridge::Alias<Missing>::Empty) -> Int\n  0\nend\n\n",
        "fn wrong_arity(value: bridge::Alias<Phase, Phase>::Boxed | bridge::Alias<Phase, Phase>::Empty) -> Int\n  0\nend\n\n",
        "fn mixed(value: bridge::Alias<Phase>::Boxed | Other<Phase>::OtherReady) -> Int\n  0\nend\n",
    );
    workspace.write("main.veln", source);
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            concat!(
                "pub type Box<A>\n",
                "  pub Boxed(A)\n",
                "  pub Empty\n",
                "end\n",
                "pub type Other<A>\n",
                "  pub OtherReady(A)\n",
                "end\n",
            )
            .as_bytes(),
        )],
    );
    for line_text in [
        "Same(",
        "direct_same",
        "direct_alias",
        "qualified_same",
        "qualified_alias",
    ] {
        for (line, column) in refinement_variant_positions(source, line_text) {
            assert_retained_refinement_position_is_navigable(&mut server, line_text, line, column);
        }
    }
    for line_text in [
        "Different(",
        "mismatch",
        "unresolved",
        "wrong_arity",
        "mixed",
    ] {
        for (line, column) in refinement_variant_positions(source, line_text) {
            assert_retained_refinement_position_is_ineligible(&mut server, line_text, line, column);
        }
    }
}

#[test]
fn private_variant_refinement_navigation_uses_exact_companion_visibility() {
    let workspace = TempWorkspace::new("private-variant-refinement-exact-companion");
    workspace.write("veln.toml", "");
    workspace.write("model.veln", "type State\n  Ready(Int)\nend\n");
    let companion = "use model\n\ntest companion(value: model::State::Ready) -> Int\n  0\nend\n";
    let wrong = "use model\n\ntest wrong(value: model::State::Ready) -> Int\n  0\nend\n";
    workspace.write("model.test.veln", companion);
    workspace.write("other.test.veln", wrong);
    let mut server = initialized_server(&workspace);
    let line = 3;

    for (needle, expected_line, expected_column, new_name) in
        [("State", 1, 6, "Modeled"), ("Ready", 2, 3, "Prepared")]
    {
        let column = companion
            .lines()
            .nth(line - 1)
            .unwrap()
            .find(needle)
            .unwrap()
            + 1;
        let definition = server
            .definition_tool(&json!({"source":"model.test.veln", "line":line, "column":column}));
        assert_eq!(
            definition["structuredContent"]["definition"]["range"]["start"],
            json!({"line":expected_line,"column":expected_column}),
            "{definition:#}"
        );
        let references = server.references_tool(&json!({
            "source":"model.test.veln", "line":line, "column":column,
            "include_declaration":true
        }));
        assert_eq!(
            references["structuredContent"]["references"]
                .as_array()
                .unwrap()
                .len(),
            2,
            "{references:#}"
        );
        let renamed = server.rename_tool(&json!({
            "source":"model.test.veln", "line":line, "column":column, "new_name":new_name
        }));
        assert_eq!(edits(&renamed).len(), 2, "{renamed:#}");

        let wrong_column = wrong.lines().nth(line - 1).unwrap().find(needle).unwrap() + 1;
        let wrong_definition = server.definition_tool(
            &json!({"source":"other.test.veln", "line":line, "column":wrong_column}),
        );
        assert!(
            wrong_definition["structuredContent"]["definition"].is_null(),
            "{wrong_definition:#}"
        );
        let wrong_references = server.references_tool(&json!({
            "source":"other.test.veln", "line":line, "column":wrong_column,
            "include_declaration":true
        }));
        assert!(
            wrong_references["structuredContent"]["references"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{wrong_references:#}"
        );
        let wrong_rename = server.rename_tool(&json!({
            "source":"other.test.veln", "line":line, "column":wrong_column,
            "new_name":new_name
        }));
        assert!(edits(&wrong_rename).is_empty(), "{wrong_rename:#}");
    }
}

fn edits(result: &Value) -> &Vec<Value> {
    result["structuredContent"]["edits"]
        .as_array()
        .unwrap_or_else(|| panic!("rename edits missing: {result:#}"))
}

fn live_main_reference_cursor(server: &mut Server) -> String {
    server.references_tool(&json!({
        "source":"main.veln", "line":2, "column":4, "page_size":1,
        "include_declaration":true
    }))["structuredContent"]["next_cursor"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn assert_main_reference_continuation(server: &mut Server, cursor: String, expected_uri: &str) {
    let continuation = server.references_tool(&json!({"cursor": cursor}));
    assert_eq!(continuation["isError"], false, "{continuation:#}");
    let references = continuation["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert_eq!(references.len(), 1, "{continuation:#}");
    assert_eq!(references[0]["uri"], expected_uri, "{continuation:#}");
    assert_eq!(
        references[0]["range"],
        json!({"start":{"line":2,"column":3},"end":{"line":2,"column":9}}),
        "{continuation:#}"
    );
    assert!(
        continuation["structuredContent"]
            .get("next_cursor")
            .is_none(),
        "{continuation:#}"
    );
}

#[derive(Clone, Copy)]
enum RenameCursorOutcome {
    Edits(usize),
    Error(&'static str),
}

#[derive(Clone, Copy)]
struct RenameCursorCase {
    name: &'static str,
    source: &'static str,
    line: usize,
    column: usize,
    new_name: &'static str,
    outcome: RenameCursorOutcome,
}

fn assert_rename_cursor_outcome(case: RenameCursorCase, result: &Value) {
    match case.outcome {
        RenameCursorOutcome::Error(code) => {
            assert_eq!(
                result["structuredContent"]["code"], code,
                "{}: {result:#}",
                case.name
            );
            assert!(result["structuredContent"].get("edits").is_none());
        }
        RenameCursorOutcome::Edits(count) => {
            assert_eq!(edits(result).len(), count, "{}: {result:#}", case.name);
        }
    }
}

fn shared_conflicting_uri(
    server: &mut Server,
    source: &str,
    line: usize,
    column: usize,
    requested_name: &str,
) -> String {
    let Ok((captured, captured_source, _)) = crate::check_project::capture_navigation_source(
        &server.base,
        &server.selection,
        source,
        &mut server.capture_cache,
    ) else {
        panic!("saved navigation capture failed")
    };
    let snapshot = server.language_resources.read_only_navigation_snapshot(
        captured.project.files,
        &captured.dependencies,
        captured.key,
    );
    let result = navigate_for_rename(
        snapshot.as_ref(),
        SourcePosition {
            source: SourcePath::new(captured_source),
            line,
            column,
        },
    )
    .unwrap();
    let failure =
        validate_rename_in_snapshot(snapshot.as_ref(), &result, requested_name).unwrap_err();
    let RenameFailureKind::Conflict {
        conflicting_declaration,
        ..
    } = failure.kind
    else {
        panic!("expected rename conflict")
    };
    match conflicting_declaration.source {
        NavigationSource::Workspace => crate::definition::path_to_uri(
            &server
                .base
                .path()
                .join(conflicting_declaration.span.file.as_str()),
        ),
        NavigationSource::Package { uri } => uri,
    }
}

fn assert_rename_matches_shared(
    workspace: &TempWorkspace,
    sources: &[(&str, &str)],
    source: &str,
    line: usize,
    column: usize,
    new_name: &str,
) {
    let snapshot = EffectiveProjectSnapshot::new(
        sources
            .iter()
            .map(|(path, text)| SourceFile::new(*path, *text))
            .collect(),
    );
    let shared = navigate_for_rename(
        &snapshot,
        SourcePosition {
            source: SourcePath::new(source),
            line,
            column,
        },
    )
    .unwrap();
    let expected = std::iter::once(&shared.definition.span)
        .chain(&shared.references)
        .map(|span| {
            (
                crate::definition::path_to_uri(&workspace.path(span.file.as_str())),
                span.start.line,
                span.start.column,
                span.end.line,
                span.end.column,
            )
        })
        .collect::<BTreeSet<_>>();
    let result = rename_result(workspace, source, line, column, new_name);
    let actual = edits(&result)
        .iter()
        .map(|edit| {
            (
                edit["uri"].as_str().unwrap().to_owned(),
                edit["range"]["start"]["line"].as_u64().unwrap() as usize,
                edit["range"]["start"]["column"].as_u64().unwrap() as usize,
                edit["range"]["end"]["line"].as_u64().unwrap() as usize,
                edit["range"]["end"]["column"].as_u64().unwrap() as usize,
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected, "{source}:{line}:{column} {result:#}");
    assert_eq!(edits(&result).len(), actual.len(), "{result:#}");
}

#[test]
fn rename_edit_set_matches_shared_language_service_locations() {
    let workspace = TempWorkspace::new("rename-shared-comparison");
    workspace.write("veln.toml", "");
    let main =
        "pub type Alias = Int\n\npub fn target(input: Alias) -> Alias\n  target(input)\nend\n";
    let other = "use main\n\nfn other(input: Alias) -> Alias\n  target(input)\nend\n";
    workspace.write("main.veln", main);
    workspace.write("other.veln", other);

    for (line, column, new_name) in [(1, 10, "Renamed"), (4, 4, "renamed")] {
        assert_rename_matches_shared(
            &workspace,
            &[("main.veln", main), ("other.veln", other)],
            "main.veln",
            line,
            column,
            new_name,
        );
    }
}

#[test]
fn rename_begin_local_edits_declaration_and_deferred_cleanup_references() {
    let workspace = TempWorkspace::new("rename-cleanup-region-local");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn cleanup(value: Int) -> ()\n",
            "  ()\n",
            "end\n\n",
            "fn read(input: Int) -> Int\n",
            "  begin\n",
            "    let captured = input\n",
            "    defer\n",
            "      cleanup(captured)\n",
            "    end\n",
            "    captured\n",
            "  end\n",
            "end\n",
        ),
    );

    let result = rename_result(&workspace, "main.veln", 9, 15, "saved");

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(edits(&result).len(), 3, "{result:#}");
    let ranges = edits(&result)
        .iter()
        .map(|edit| {
            let range = &edit["range"];
            (
                range["start"]["line"].as_u64().unwrap(),
                range["start"]["column"].as_u64().unwrap(),
                range["end"]["line"].as_u64().unwrap(),
                range["end"]["column"].as_u64().unwrap(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        ranges,
        BTreeSet::from([(7, 9, 7, 17), (9, 15, 9, 23), (11, 5, 11, 13),])
    );
}

#[test]
fn rename_handler_clause_begin_local_edits_declaration_and_cleanup_uses() {
    let workspace = TempWorkspace::new("rename-handler-clause-cleanup-region-local");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n\n",
            "handler ask() for Ask\n",
            "  value() => begin\n",
            "    let captured = 1\n",
            "    defer\n",
            "      captured\n",
            "    end\n",
            "    captured\n",
            "  end\n",
            "end\n",
        ),
    );

    let result = rename_result(&workspace, "main.veln", 9, 8, "saved");

    assert_eq!(result["isError"], false, "{result:#}");
    let ranges = edits(&result)
        .iter()
        .map(|edit| {
            let range = &edit["range"];
            (
                range["start"]["line"].as_u64().unwrap(),
                range["start"]["column"].as_u64().unwrap(),
                range["end"]["line"].as_u64().unwrap(),
                range["end"]["column"].as_u64().unwrap(),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        ranges,
        BTreeSet::from([(7, 9, 7, 17), (9, 7, 9, 15), (11, 5, 11, 13),])
    );
}

#[test]
fn rename_uses_innermost_binding_across_nested_cleanup_scopes() {
    let workspace = TempWorkspace::new("rename-cleanup-region-shadowing");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn read(value: Int) -> Int\n",
            "  let value = value\n",
            "  begin\n",
            "    let value = value\n",
            "    defer\n",
            "      let value = value\n",
            "      value\n",
            "    end\n",
            "    value\n",
            "  end\n",
            "  value\n",
            "end\n",
        ),
    );

    let cases = [
        (2, 15, BTreeSet::from([(1, 9, 1, 14), (2, 15, 2, 20)])),
        (
            11,
            4,
            BTreeSet::from([(2, 7, 2, 12), (4, 17, 4, 22), (11, 3, 11, 8)]),
        ),
        (
            9,
            6,
            BTreeSet::from([(4, 9, 4, 14), (6, 19, 6, 24), (9, 5, 9, 10)]),
        ),
        (7, 8, BTreeSet::from([(6, 11, 6, 16), (7, 7, 7, 12)])),
    ];
    for (line, column, expected) in cases {
        let result = rename_result(&workspace, "main.veln", line, column, "renamed");
        assert_eq!(result["isError"], false, "{line}:{column}: {result:#}");
        let actual = edits(&result)
            .iter()
            .map(|edit| {
                let range = &edit["range"];
                (
                    range["start"]["line"].as_u64().unwrap(),
                    range["start"]["column"].as_u64().unwrap(),
                    range["end"]["line"].as_u64().unwrap(),
                    range["end"]["column"].as_u64().unwrap(),
                )
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(actual, expected, "{line}:{column}: {result:#}");
    }
}

#[test]
fn rename_type_edits_function_and_handler_cleanup_annotations() {
    let workspace = TempWorkspace::new("rename-cleanup-region-type");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type Resource\n",
            "  Ready\n",
            "end\n\n",
            "effect Ask\n",
            "  value() -> Resource\n",
            "end\n\n",
            "fn work(input: Resource) -> Resource\n",
            "  defer\n",
            "    let deferred: Resource = input\n",
            "    ()\n",
            "  end\n",
            "  let begun: Resource = begin\n",
            "    let nested: Resource = input\n",
            "    nested\n",
            "  end\n",
            "  begun\n",
            "end\n\n",
            "handler ask(seed: Resource) for Ask\n",
            "  value() => begin\n",
            "    defer\n",
            "      let deferred: Resource = seed\n",
            "      ()\n",
            "    end\n",
            "    let clause: Resource = seed\n",
            "    clause\n",
            "  end\n",
            "end\n",
        ),
    );

    let result = rename_result(&workspace, "main.veln", 27, 19, "Handle");

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(edits(&result).len(), 10, "{result:#}");
}

#[test]
fn rename_type_alias_constructor_qualifiers_share_validated_identity() {
    let workspace = TempWorkspace::new("rename-type-alias-constructor-qualifier");
    workspace.write("veln.toml", "");
    let model = concat!(
        "pub type Item\n",
        "  pub Ready(Int)\n",
        "end\n\n",
        "pub type Alias = Item\n",
        "pub fn helper(value: Int) -> Int\n  value\nend\n",
    );
    let main = concat!(
        "use model\n\n",
        "fn make(input: Alias) -> Alias\n",
        "  Alias::Ready(input)\n",
        "  model::Alias::Ready(input)\n",
        "end\n\n",
        "fn missing() -> Int\n",
        "  Alias::Missing\n",
        "end\n",
        "\nfn non_constructor() -> Int\n",
        "  Alias::helper(1)\n",
        "end\n",
    );
    workspace.write("model.veln", model);
    workspace.write("main.veln", main);

    assert_rename_matches_shared(
        &workspace,
        &[("model.veln", model), ("main.veln", main)],
        "main.veln",
        4,
        3,
        "RenamedAlias",
    );
    let valid = rename_result(&workspace, "main.veln", 4, 3, "RenamedAlias");
    assert_eq!(edits(&valid).len(), 5, "{valid:#}");

    for (line, name) in [(9, "unresolved"), (13, "non-constructor")] {
        let invalid = rename_result(&workspace, "main.veln", line, 3, "RenamedAlias");
        assert_eq!(invalid["isError"], false, "{name}: {invalid:#}");
        assert!(edits(&invalid).is_empty(), "{name}: {invalid:#}");
    }
}

#[test]
fn rename_type_alias_rejects_exact_dependency_import_collision() {
    let workspace = TempWorkspace::new("rename-type-alias-dependency-collision");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/pkg\"]\npath = \"vendor/pkg\"\n",
    );
    workspace.write("model.veln", "pub type Alias = Int\n");
    workspace.write(
        "main.veln",
        concat!(
            "use model\n",
            "use model from \"example/pkg\"\n\n",
            "fn read(input: model::Alias) -> model::Alias\n",
            "  input\n",
            "end\n",
        ),
    );
    workspace.write(
        "vendor/pkg/veln.toml",
        "[package]\nname = \"example/pkg\"\n\n[lib]\nexports = [\"model.veln\"]\n",
    );
    workspace.write("vendor/pkg/model.veln", "pub type Alias = Int\n");

    let occurrence = rename_result(&workspace, "main.veln", 4, 23, "Renamed");
    assert_eq!(occurrence["isError"], false, "{occurrence:#}");
    assert!(edits(&occurrence).is_empty(), "{occurrence:#}");

    let declaration = rename_result(&workspace, "model.veln", 1, 10, "Renamed");
    assert_eq!(declaration["isError"], false, "{declaration:#}");
    assert_eq!(edits(&declaration).len(), 1, "{declaration:#}");
}

#[test]
fn rename_type_alias_rejects_bare_standard_prelude_alias_collision() {
    let workspace = TempWorkspace::new("rename-type-alias-prelude-collision");
    workspace.write("veln.toml", "");
    workspace.write("left.veln", "pub type Alias = Int\n");
    workspace.write(
        "main.veln",
        "use left\n\nfn read(input: Alias) -> Alias\n  input\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub type Alias = Int\n",
        )],
    );

    let occurrence = server.rename_tool(&json!({
        "source":"main.veln", "line":3, "column":16, "new_name":"Renamed"
    }));
    assert_eq!(occurrence["isError"], false, "{occurrence:#}");
    assert!(edits(&occurrence).is_empty(), "{occurrence:#}");

    let declaration = server.rename_tool(&json!({
        "source":"left.veln", "line":1, "column":10, "new_name":"Renamed"
    }));
    assert_eq!(declaration["isError"], false, "{declaration:#}");
    assert_eq!(edits(&declaration).len(), 1, "{declaration:#}");
}

#[test]
fn rename_type_alias_constructor_qualifier_requires_target_import() {
    let workspace = TempWorkspace::new("rename-type-alias-target-import");
    workspace.write("veln.toml", "");
    workspace.write("model.veln", "pub type Item\n  pub Ready(Int)\nend\n");
    workspace.write("bridge.veln", "pub type Alias = model::Item\n");
    workspace.write(
        "main.veln",
        concat!(
            "use bridge\n",
            "use model\n\n",
            "fn make(input: Int) -> model::Item\n",
            "  Alias::Ready(input)\n",
            "end\n",
        ),
    );

    let occurrence = rename_result(&workspace, "main.veln", 5, 3, "Renamed");
    assert_eq!(occurrence["isError"], false, "{occurrence:#}");
    assert!(edits(&occurrence).is_empty(), "{occurrence:#}");

    let declaration = rename_result(&workspace, "bridge.veln", 1, 10, "Renamed");
    assert_eq!(declaration["isError"], false, "{declaration:#}");
    assert_eq!(edits(&declaration).len(), 1, "{declaration:#}");
}

#[test]
fn rename_supported_class_locations_match_shared_language_service() {
    let workspace = TempWorkspace::new("rename-all-class-shared-comparison");
    workspace.write("veln.toml", "");
    let main = concat!(
        "type Item\n",
        "  Value(value: Int)\n",
        "end\n\n",
        "fn convert(input: Item) -> Item\n",
        "  Value(input)\n",
        "end\n\n",
        "test converts() -> Int\n",
        "  convert(1)\n",
        "end\n\n",
        "fn qualified() -> Item\n  Item::Value(1)\nend\n",
    );
    let math = concat!(
        "pub type Number\nend\n",
        "pub type Alias = Number\n\n",
        "fn use_alias(input: Alias) -> Alias\n  input\nend\n\n",
        "fn increment(value: Int) -> Int\n  value + 1\nend\n",
        "pub fn advance = increment\n",
        "fn use_advance() -> Int\n  advance(1)\nend\n",
    );
    let math_test = "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n";
    let handler = concat!(
        "effect Choose\n  pick(value: Bool) -> Int\nend\n\n",
        "handler choose(callback: fn(Int) -> Int) for Choose\n",
        "  pick(value) => callback(value)\nend\n",
    );
    for (path, text) in [
        ("main.veln", main),
        ("math.veln", math),
        ("math.test.veln", math_test),
        ("handler.veln", handler),
    ] {
        workspace.write(path, text);
    }
    let sources = [
        ("main.veln", main),
        ("math.veln", math),
        ("math.test.veln", math_test),
        ("handler.veln", handler),
    ];
    for (source, line, column, new_name) in [
        ("main.veln", 1, 6, "Entry"),
        ("main.veln", 2, 3, "Created"),
        ("main.veln", 5, 4, "adapt"),
        ("main.veln", 6, 9, "value"),
        ("main.veln", 9, 6, "checks"),
        ("math.veln", 5, 22, "RenamedAlias"),
        ("math.veln", 14, 4, "move"),
        ("math.test.veln", 4, 10, "step"),
        ("handler.veln", 6, 19, "apply"),
        ("handler.veln", 6, 28, "input"),
    ] {
        assert_rename_matches_shared(&workspace, &sources, source, line, column, new_name);
    }
}

#[test]
fn rename_returns_sorted_complete_workspace_edits_for_supported_classes() {
    let workspace = TempWorkspace::new("rename-supported-classes");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type Item\n",
            "  Value(value: Int)\n",
            "end\n\n",
            "fn convert(input: Item) -> Item\n",
            "  Value(input)\n",
            "end\n\n",
            "test converts() -> Int\n",
            "  convert(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        "fn other(input: Item) -> Item\n  convert(input)\nend\n",
    );

    let cases = [
        ("type", 1, 6, "Entry", 3usize),
        ("constructor", 2, 3, "Created", 2),
        ("function", 5, 4, "adapt", 2),
        ("value binding", 6, 9, "value", 2),
        ("test declaration", 9, 6, "checks", 1),
    ];
    for (name, line, column, new_name, count) in cases {
        let result = rename_result(&workspace, "main.veln", line, column, new_name);
        assert_eq!(result["isError"], false, "{name}: {result:#}");
        assert_eq!(edits(&result).len(), count, "{name}: {result:#}");
        assert!(
            edits(&result)
                .iter()
                .all(|edit| edit["new_text"] == new_name),
            "{result:#}"
        );
        let keys = edits(&result)
            .iter()
            .map(|edit| {
                (
                    edit["uri"].as_str().unwrap(),
                    edit["range"]["start"]["line"].as_u64().unwrap(),
                    edit["range"]["start"]["column"].as_u64().unwrap(),
                    edit["range"]["end"]["line"].as_u64().unwrap(),
                    edit["range"]["end"]["column"].as_u64().unwrap(),
                )
            })
            .collect::<Vec<_>>();
        assert!(keys.windows(2).all(|pair| pair[0] < pair[1]), "{result:#}");
    }
}

#[test]
fn rename_supports_aliases_companion_private_functions_and_handler_bindings() {
    let workspace = TempWorkspace::new("rename-alias-handler-classes");
    workspace.write("veln.toml", "");
    workspace.write(
        "math.veln",
        concat!(
            "pub type Number\nend\n",
            "pub type Alias = Number\n\n",
            "fn use_alias(input: Alias) -> Alias\n  input\nend\n\n",
            "fn increment(value: Int) -> Int\n  value + 1\nend\n",
            "pub fn advance = increment\n",
            "fn use_advance() -> Int\n  advance(1)\nend\n",
        ),
    );
    workspace.write(
        "math.test.veln",
        "use math\n\ntest companion() -> Int\n  math::increment(1)\nend\n",
    );
    workspace.write(
        "handler.veln",
        concat!(
            "effect Choose\n  pick(value: Bool) -> Int\nend\n\n",
            "handler choose(callback: fn(Int) -> Int) for Choose\n",
            "  pick(value) => callback(value)\nend\n",
        ),
    );

    for (name, source, line, column, new_name, edit_count) in [
        ("type alias", "math.veln", 5, 22, "RenamedAlias", 3),
        ("function alias", "math.veln", 14, 4, "move", 2),
        ("companion private", "math.test.veln", 4, 10, "step", 3),
        ("handler context", "handler.veln", 6, 19, "apply", 2),
        ("handler clause", "handler.veln", 6, 28, "input", 2),
    ] {
        let result = rename_result(&workspace, source, line, column, new_name);
        assert_eq!(result["isError"], false, "{name}: {result:#}");
        assert_eq!(edits(&result).len(), edit_count, "{name}: {result:#}");
    }
}

#[test]
fn rename_preserves_recovery_identity_and_same_name_edit_sets() {
    let workspace = TempWorkspace::new("rename-recovery-and-idempotence");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type item\n  value(input: Int)\nend\n\n",
            "fn Bad() -> Int\n  Bad()\nend\n\n",
            "fn read(value: item) -> item\n  value\nend\n\n",
            "fn local(input: Int) -> Int\n  let Local = input\n  Local\nend\n",
            "\nfn clean() -> Int\n  clean()\nend\n",
        ),
    );

    for (line, column, new_name, count) in [
        (9, 19, "Entry", 3usize),
        (6, 4, "good", 2),
        (14, 8, "binding", 2),
    ] {
        let result = rename_result(&workspace, "main.veln", line, column, new_name);
        assert_eq!(result["isError"], false, "{result:#}");
        assert_eq!(edits(&result).len(), count, "{result:#}");
    }

    let same = rename_result(&workspace, "main.veln", 19, 4, "clean");
    let changed = rename_result(&workspace, "main.veln", 19, 4, "better");
    let same_ranges = edits(&same)
        .iter()
        .map(|edit| (&edit["uri"], &edit["range"]))
        .collect::<Vec<_>>();
    let changed_ranges = edits(&changed)
        .iter()
        .map(|edit| (&edit["uri"], &edit["range"]))
        .collect::<Vec<_>>();
    assert_eq!(same_ranges, changed_ranges);

    let lexical_same = rename_result(&workspace, "main.veln", 13, 10, "input");
    let lexical_changed = rename_result(&workspace, "main.veln", 13, 10, "binding");
    assert_eq!(
        edits(&lexical_same)
            .iter()
            .map(|edit| (&edit["uri"], &edit["range"]))
            .collect::<Vec<_>>(),
        edits(&lexical_changed)
            .iter()
            .map(|edit| (&edit["uri"], &edit["range"]))
            .collect::<Vec<_>>()
    );
}

#[test]
fn rename_preserves_callable_constructor_and_handler_recovery_identities() {
    let workspace = TempWorkspace::new("rename-recovery-remaining-classes");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type item\n  value(input: Int)\nend\n\n",
            "fn read_constructor() -> item\n  value(1)\nend\n\n",
            "fn read_callback(Callback: fn() -> Int) -> Int\n  Callback\n  Callback()\nend\n\n",
            "effect Adjust\n  amount(value: Int) -> Int\nend\n\n",
            "handler adjust(Callback: fn(Int) -> Int) for Adjust\n",
            "  amount(Value) => Callback(Value)\nend\n",
        ),
    );

    for (name, line, column, new_name, count) in [
        ("constructor", 6, 4, "Value", 2usize),
        ("callable", 10, 4, "callback", 3),
        ("handler context", 19, 20, "callback", 2),
        ("operation clause", 19, 29, "value", 2),
    ] {
        let result = rename_result(&workspace, "main.veln", line, column, new_name);
        assert_eq!(result["isError"], false, "{name}: {result:#}");
        assert_eq!(edits(&result).len(), count, "{name}: {result:#}");
    }
}

#[test]
fn rename_returns_exact_identifier_case_and_conflict_failures() {
    let workspace = TempWorkspace::new("rename-domain-failures");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "type Item\n  Value(value: Int)\nend\n\n",
            "type Status\n  Ready\nend\n\n",
            "fn convert(input: Item) -> Item\n  Value(input)\nend\n",
        ),
    );

    for bad in ["two words", "punct!", "café", "2value"] {
        let result = rename_result(&workspace, "main.veln", 1, 6, bad);
        assert_eq!(result["isError"], true, "{result:#}");
        assert_eq!(
            result["structuredContent"],
            json!({
                "code": "rename.invalid_name",
                "message": "replacement is not a valid identifier",
                "details": {"requested_name": bad}
            })
        );
    }

    for (line, column, new_name, class, required) in [
        (1, 6, "entry", "type", "ascii_uppercase"),
        (2, 3, "created", "constructor", "ascii_uppercase"),
        (9, 4, "Adapt", "function", "ascii_lowercase"),
        (10, 10, "Input", "value_binding", "ascii_lowercase"),
    ] {
        let result = rename_result(&workspace, "main.veln", line, column, new_name);
        assert_eq!(
            result["structuredContent"]["code"], "rename.invalid_case",
            "{line}:{column} {result:#}"
        );
        assert_eq!(
            result["structuredContent"]["details"],
            json!({
                "symbol_class": class,
                "requested_name": new_name,
                "required_initial": required
            })
        );
        assert!(result["structuredContent"].get("edits").is_none());
    }

    let mut server = initialized_server(&workspace);
    let shared_uri = shared_conflicting_uri(&mut server, "main.veln", 5, 6, "Item");
    let conflict = server.rename_tool(&json!({
        "source":"main.veln", "line":5, "column":6, "new_name":"Item"
    }));
    assert_eq!(conflict["structuredContent"]["code"], "rename.conflict");
    assert_eq!(
        conflict["structuredContent"]["details"]["affected_scope"],
        json!({"kind": "module", "name": "main"})
    );
    assert_eq!(
        conflict["structuredContent"]["details"]["conflicting_declaration"]["uri"],
        shared_uri
    );
    assert!(shared_uri.starts_with("file:"));
    assert!(conflict["structuredContent"].get("edits").is_none());
}

#[test]
fn rename_returns_exact_lexical_conflict_failure() {
    let workspace = TempWorkspace::new("rename-lexical-conflict");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn target(value: Int) -> Int\n",
            "  value\n",
            "end\n\n",
            "fn caller(value: Int) -> Int\n",
            "  let conflict = value\n",
            "  let observed = conflict\n",
            "  target(observed)\n",
            "end\n",
        ),
    );

    let lexical = rename_result(&workspace, "main.veln", 8, 4, "conflict");
    assert_eq!(lexical["structuredContent"]["code"], "rename.conflict");
    assert_eq!(
        lexical["structuredContent"]["details"]["symbol_class"],
        "function"
    );
    assert_eq!(
        lexical["structuredContent"]["details"]["requested_name"],
        "conflict"
    );
    assert_eq!(
        lexical["structuredContent"]["details"]["conflicting_declaration"]["uri"],
        crate::definition::path_to_uri(&workspace.path("main.veln"))
    );
    assert_eq!(
        lexical["structuredContent"]["details"]["conflicting_declaration"]["range"],
        json!({
            "start": {"line": 6, "column": 7},
            "end": {"line": 6, "column": 15}
        })
    );
    assert_eq!(
        lexical["structuredContent"]["details"]["affected_scope"],
        json!({
            "kind": "lexical",
            "file": "main.veln",
            "start_offset": 71,
            "end_offset": 139
        })
    );
    assert!(lexical["structuredContent"].get("edits").is_none());
}

#[test]
fn rename_reports_handler_binding_and_recovery_conflicts() {
    let workspace = TempWorkspace::new("rename-handler-recovery-conflicts");
    workspace.write("veln.toml", "");
    workspace.write(
        "handler.veln",
        concat!(
            "effect Choose\n",
            "  choose(target: Int) -> Int\n",
            "end\n\n",
            "fn source() -> Int\n",
            "  1\n",
            "end\n\n",
            "handler choose() for Choose\n",
            "  choose(target) => source()\n",
            "end\n",
        ),
    );
    let handler = rename_result(&workspace, "handler.veln", 5, 4, "target");
    assert_eq!(handler["structuredContent"]["code"], "rename.conflict");
    assert_eq!(
        handler["structuredContent"]["details"]["conflicting_declaration"]["range"],
        json!({
            "start":{"line":10,"column":10},
            "end":{"line":10,"column":16}
        })
    );
    assert_eq!(
        handler["structuredContent"]["details"]["affected_scope"]["kind"],
        "lexical"
    );

    workspace.write(
        "handler_context.veln",
        concat!(
            "effect Adjust\n",
            "  amount(value: Int) -> Int\n",
            "end\n\n",
            "fn origin() -> Int\n",
            "  1\n",
            "end\n\n",
            "handler adjust(target: Int) for Adjust\n",
            "  amount(value) => origin()\n",
            "end\n",
        ),
    );
    let context = rename_result(&workspace, "handler_context.veln", 5, 4, "target");
    assert_eq!(context["structuredContent"]["code"], "rename.conflict");
    assert_eq!(
        context["structuredContent"]["details"]["conflicting_declaration"]["range"],
        json!({
            "start":{"line":9,"column":16},
            "end":{"line":9,"column":22}
        })
    );
    assert_eq!(
        context["structuredContent"]["details"]["affected_scope"]["kind"],
        "lexical"
    );

    workspace.write(
        "recovery.veln",
        concat!(
            "type item\n",
            "  value(input: Int)\n",
            "  Ready\n",
            "end\n\n",
            "type Entry\n",
            "  Existing\n",
            "end\n\n",
            "fn Bad() -> Int\n",
            "  Bad()\n",
            "end\n\n",
            "fn good() -> Int\n",
            "  1\n",
            "end\n\n",
            "fn read(Input: Int, other: Int) -> Int\n",
            "  Input\n",
            "end\n",
        ),
    );
    for (line, column, requested, conflict_line, conflict_column) in [
        (1, 6, "Entry", 6, 6),
        (2, 3, "Ready", 3, 3),
        (10, 4, "good", 14, 4),
        (18, 9, "other", 18, 21),
    ] {
        let result = rename_result(&workspace, "recovery.veln", line, column, requested);
        assert_eq!(
            result["structuredContent"]["code"], "rename.conflict",
            "{result:#}"
        );
        assert_eq!(
            result["structuredContent"]["details"]["conflicting_declaration"]["range"]["start"],
            json!({"line":conflict_line,"column":conflict_column}),
            "{result:#}"
        );
        assert!(result["structuredContent"].get("edits").is_none());
    }
}

#[test]
fn rename_preserves_direct_dependency_conflict_provenance() {
    let workspace = TempWorkspace::new("rename-dependency-conflict-provenance");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        "use dep from \"example/dep\"\n\ntype Local\n  Value\nend\n\nfn read(input: Local) -> Local\n  input\nend\n",
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write("vendor/dep/dep.veln", "pub type Occupied\n  Taken\nend\n");
    let mut server = initialized_server(&workspace);
    let before = server.language_resources.list_result();
    let shared_uri = shared_conflicting_uri(&mut server, "main.veln", 3, 6, "Occupied");
    let result = server.rename_tool(&json!({
        "source":"main.veln", "line":3, "column":6, "new_name":"Occupied"
    }));
    assert_eq!(
        result["structuredContent"]["code"], "rename.conflict",
        "{result:#}"
    );
    assert_eq!(
        result["structuredContent"]["details"]["conflicting_declaration"]["uri"],
        shared_uri
    );
    assert!(shared_uri.starts_with("veln-pkg:///example%2Fdep/snapshot/"));
    assert_eq!(server.language_resources.list_result(), before);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn rename_preserves_standard_library_conflict_provenance() {
    let workspace = TempWorkspace::new("rename-standard-conflict-provenance");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "type Local\n  Value\nend\n\nfn read(input: Local) -> Local\n  input\nend\n",
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            b"pub type Occupied\n  Taken\nend\n",
        )],
    );
    let before = server.language_resources.list_result();
    let shared_uri = shared_conflicting_uri(&mut server, "main.veln", 1, 6, "Occupied");
    let result = server.rename_tool(&json!({
        "source":"main.veln", "line":1, "column":6, "new_name":"Occupied"
    }));
    assert_eq!(
        result["structuredContent"]["code"], "rename.conflict",
        "{result:#}"
    );
    assert_eq!(
        result["structuredContent"]["details"]["conflicting_declaration"]["uri"],
        shared_uri
    );
    assert!(shared_uri.starts_with("veln-pkg:///std/snapshot/"));
    assert_eq!(server.language_resources.list_result(), before);
}

#[test]
fn rename_returns_empty_for_unsupported_and_package_backed_selections() {
    let workspace = TempWorkspace::new("rename-unsupported-boundaries");
    workspace.write(
        "veln.toml",
        "[dependencies.\"example/dep\"]\npath = \"vendor/dep\"\n",
    );
    workspace.write(
        "main.veln",
        concat!(
            "use dep from \"example/dep\"\n\n",
            "schema Packet\n  value: Int\nend\n\n",
            "effect Choose\n  pick() -> Int\nend\n\n",
            "fn main() -> Int\n  dep::target()\nend\n",
        ),
    );
    workspace.write(
        "vendor/dep/veln.toml",
        "[package]\nname = \"example/dep\"\n\n[lib]\nexports = [\"dep.veln\"]\n",
    );
    workspace.write("vendor/dep/dep.veln", "pub fn target() -> Int\n  1\nend\n");

    let mut server = initialized_server(&workspace);
    let before_resources = server.language_resources.list_result();
    for (line, column) in [
        (1, 5),  // module segment
        (1, 15), // package import
        (3, 8),  // schema
        (7, 8),  // effect
        (8, 3),  // effect operation
        (12, 9), // package-backed function
    ] {
        let result = server.rename_tool(&json!({
            "source":"main.veln", "line":line, "column":column, "new_name":"renamed"
        }));
        assert_eq!(result["isError"], false, "{result:#}");
        assert_eq!(edits(&result), &Vec::<Value>::new(), "{result:#}");
    }
    assert_eq!(server.language_resources.list_result(), before_resources);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn rename_returns_empty_for_an_ambiguous_recovery_occurrence() {
    let workspace = TempWorkspace::new("rename-ambiguous-recovery");
    workspace.write("veln.toml", "");
    let source = concat!(
        "fn Bad() -> Int\n  1\nend\n\n",
        "fn Bad() -> Int\n  2\nend\n\n",
        "fn caller() -> Int\n  Bad()\nend\n",
    );
    workspace.write("main.veln", source);

    let snapshot = EffectiveProjectSnapshot::new(vec![SourceFile::new("main.veln", source)]);
    assert!(
        navigate_for_rename(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 10,
                column: 4,
            },
        )
        .is_none()
    );
    let result = rename_result(&workspace, "main.veln", 10, 4, "better");
    assert_eq!(result["isError"], false, "{result:#}");
    assert!(edits(&result).is_empty(), "{result:#}");
}

#[test]
fn rename_is_anonymous_single_file_and_non_mutating() {
    let workspace = TempWorkspace::new("rename-anonymous-non-mutating");
    let original = "fn target(value: Int) -> Int\n  target(value - 1)\nend\n";
    workspace.write("main.veln", original);
    workspace.write("other.veln", "fn other() -> Int\n  target(1)\nend\n");
    let mut server = initialized_server(&workspace);

    let before_resources = server.language_resources.list_result();
    let result = server.rename_tool(&json!({
        "source": "main.veln", "line": 2, "column": 4, "new_name": "next"
    }));
    assert_eq!(edits(&result).len(), 2, "{result:#}");
    assert!(
        edits(&result)
            .iter()
            .all(|edit| edit["uri"].as_str().unwrap().ends_with("main.veln"))
    );
    assert_eq!(server.language_resources.list_result(), before_resources);
    assert_eq!(
        fs::read_to_string(workspace.path("main.veln")).unwrap(),
        original
    );

    let definition = server.definition_tool(&json!({
        "source": "main.veln", "line": 2, "column": 4
    }));
    assert_eq!(definition["isError"], false, "{definition:#}");
    assert_eq!(
        definition["structuredContent"]["definition"]["range"],
        json!({
            "start": {"line": 1, "column": 4},
            "end": {"line": 1, "column": 10}
        })
    );
}

#[test]
fn rename_is_limited_to_the_selected_manifest_project() {
    let workspace = TempWorkspace::new("rename-manifest-project-isolation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "pub fn target(input: Int) -> Int\n  target(input)\nend\n",
    );
    workspace.write(
        "helper.veln",
        "use main\n\nfn helper(input: Int) -> Int\n  main::target(input)\nend\n",
    );
    workspace.write("nested/veln.toml", "");
    workspace.write("nested/main.veln", "fn target() -> Int\n  target()\nend\n");

    let result = rename_result(&workspace, "main.veln", 2, 4, "renamed");
    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(edits(&result).len(), 3, "{result:#}");
    let actual_uris = edits(&result)
        .iter()
        .map(|edit| edit["uri"].as_str().unwrap().to_owned())
        .collect::<BTreeSet<_>>();
    let expected_uris = BTreeSet::from([
        crate::definition::path_to_uri(&workspace.path("helper.veln")),
        crate::definition::path_to_uri(&workspace.path("main.veln")),
    ]);
    assert_eq!(actual_uris, expected_uris, "{result:#}");
    assert!(
        edits(&result)
            .iter()
            .all(|edit| !edit["uri"].as_str().unwrap().contains("/nested/")),
        "{result:#}"
    );
}

#[test]
fn rename_reuses_saved_navigation_path_and_position_failures() {
    let workspace = TempWorkspace::new("rename-path-position");
    workspace.write(
        "main.veln",
        "# 😀\r\nfn target() -> Int\r\n  target()\r\nend\r\n",
    );

    let valid_non_bmp_and_crlf = rename_result(&workspace, "main.veln", 3, 4, "next");
    assert_eq!(edits(&valid_non_bmp_and_crlf).len(), 2);

    let invalid_path = rename_result(&workspace, "missing.veln", 1, 1, "next");
    assert_eq!(invalid_path["structuredContent"]["code"], "invalid_path");
    assert_eq!(invalid_path["structuredContent"]["details"], json!({}));

    let invalid_position = rename_result(&workspace, "main.veln", 1, 99, "next");
    assert_eq!(
        invalid_position["structuredContent"]["code"],
        "invalid_position"
    );
    assert_eq!(
        invalid_position["structuredContent"]["details"],
        json!({"source":"main.veln","line":1,"column":99})
    );
    assert!(invalid_position["structuredContent"].get("edits").is_none());

    let after_non_bmp = rename_result(&workspace, "main.veln", 1, 6, "next");
    assert_eq!(
        after_non_bmp["structuredContent"]["code"],
        "invalid_position"
    );

    let after_crlf = rename_result(&workspace, "main.veln", 6, 1, "next");
    assert_eq!(after_crlf["structuredContent"]["code"], "invalid_position");
}

#[test]
fn invalid_protocol_input_does_not_prevent_a_follow_up_rename() {
    let workspace = TempWorkspace::new("rename-invalid-protocol-input");
    workspace.write("main.veln", "fn target() -> Int\n  target()\nend\n");
    let mut server = initialized_server(&workspace);
    for arguments in [
        json!({"source":"main.veln","line":1,"column":4,"new_name":""}),
        json!({"source":"main.veln","line":1,"column":4}),
        json!({"source":"main.veln","line":1,"column":4,"new_name":1}),
        json!({"source":"main.veln","line":1,"column":4,"new_name":"next","extra":true}),
    ] {
        let response = server
            .handle_request(json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"rename","arguments":arguments}}))
            .unwrap();
        assert_eq!(response["error"]["code"], -32602, "{response:#}");
    }
    let valid = server.rename_tool(&json!({
        "source":"main.veln","line":1,"column":4,"new_name":"next"
    }));
    assert_eq!(edits(&valid).len(), 2, "{valid:#}");
}

#[test]
fn rename_rejects_oversized_identifiers_before_edit_construction() {
    let workspace = TempWorkspace::new("rename-long-identifier");
    workspace.write("main.veln", "fn target() -> Int\n  target()\nend\n");
    let mut server = initialized_server(&workspace);
    let oversized_name = "a".repeat(257);
    let rejected = server
        .handle_request(json!({
            "jsonrpc":"2.0",
            "id":1,
            "method":"tools/call",
            "params":{
                "name":"rename",
                "arguments":{
                    "source":"main.veln",
                    "line":1,
                    "column":4,
                    "new_name":oversized_name
                }
            }
        }))
        .unwrap();
    assert_eq!(rejected["error"]["code"], -32602, "{rejected:#}");
    assert!(rejected.get("result").is_none(), "{rejected:#}");

    let follow_up = server.rename_tool(&json!({
        "source":"main.veln", "line":1, "column":4, "new_name":"next"
    }));
    assert_eq!(edits(&follow_up).len(), 2, "{follow_up:#}");
}

#[test]
fn rename_non_capture_results_preserve_live_reference_cursors() {
    let workspace = TempWorkspace::new("rename-preserve-cursors");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn target() -> Int\n  target()\nend\n\n",
            "fn occupied() -> Int\n  1\nend\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    let expected_uri = crate::definition::path_to_uri(&workspace.path("main.veln"));
    let cases = [
        RenameCursorCase {
            name: "success",
            source: "main.veln",
            line: 1,
            column: 4,
            new_name: "next",
            outcome: RenameCursorOutcome::Edits(2),
        },
        RenameCursorCase {
            name: "empty selection",
            source: "main.veln",
            line: 4,
            column: 1,
            new_name: "next",
            outcome: RenameCursorOutcome::Edits(0),
        },
        RenameCursorCase {
            name: "invalid name",
            source: "main.veln",
            line: 1,
            column: 4,
            new_name: "two words",
            outcome: RenameCursorOutcome::Error("rename.invalid_name"),
        },
        RenameCursorCase {
            name: "invalid case",
            source: "main.veln",
            line: 1,
            column: 4,
            new_name: "Next",
            outcome: RenameCursorOutcome::Error("rename.invalid_case"),
        },
        RenameCursorCase {
            name: "conflict",
            source: "main.veln",
            line: 1,
            column: 4,
            new_name: "occupied",
            outcome: RenameCursorOutcome::Error("rename.conflict"),
        },
        RenameCursorCase {
            name: "invalid path",
            source: "missing.veln",
            line: 1,
            column: 1,
            new_name: "next",
            outcome: RenameCursorOutcome::Error("invalid_path"),
        },
        RenameCursorCase {
            name: "invalid position",
            source: "main.veln",
            line: 1,
            column: 99,
            new_name: "next",
            outcome: RenameCursorOutcome::Error("invalid_position"),
        },
    ];

    for case in cases {
        let cursor = live_main_reference_cursor(&mut server);
        let result = server.rename_tool(&json!({
            "source": case.source,
            "line": case.line,
            "column": case.column,
            "new_name": case.new_name
        }));
        assert_rename_cursor_outcome(case, &result);
        assert_main_reference_continuation(&mut server, cursor, &expected_uri);
    }
}

#[test]
fn rename_capture_exhaustion_preserves_state_and_allows_a_later_call() {
    let workspace = TempWorkspace::new("rename-capture-exhaustion");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "fn target() -> Int\n  target()\nend\n");
    let mut server = initialized_server(&workspace);
    let before_resources = server.language_resources.list_result();
    let before_selection = server.selection_result();
    let cursor = live_main_reference_cursor(&mut server);
    let attempts = Rc::new(Cell::new(0usize));
    let attempts_for_hook = Rc::clone(&attempts);
    let path = workspace.path("main.veln");
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        fs::write(
            &path,
            if attempt.is_multiple_of(2) {
                "fn target() -> Int\n  target() + 1\nend\n"
            } else {
                "fn target() -> Int\n  target()\nend\n"
            },
        )
        .unwrap();
    });

    let failed = server.rename_tool(&json!({
        "source":"main.veln", "line":2, "column":4, "new_name":"next"
    }));
    assert_eq!(failed["structuredContent"]["code"], "snapshot_changed");
    assert!(failed["structuredContent"].get("edits").is_none());
    assert_eq!(attempts.get(), 3);
    assert_eq!(server.language_resources.list_result(), before_resources);
    assert_eq!(server.selection_result(), before_selection);

    let expected_uri = crate::definition::path_to_uri(&workspace.path("main.veln"));
    assert_main_reference_continuation(&mut server, cursor, &expected_uri);

    drop(_hook);
    workspace.write("main.veln", "fn target() -> Int\n  target()\nend\n");
    let valid = server.rename_tool(&json!({
        "source":"main.veln", "line":2, "column":4, "new_name":"next"
    }));
    assert_eq!(edits(&valid).len(), 2, "{valid:#}");
}

#[test]
fn repeated_renames_reuse_read_only_navigation() {
    let workspace = TempWorkspace::new("rename-reused-read-only-navigation");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "fn target() -> Int\n  target()\nend\n\nfn other() -> Int\n  target()\nend\n",
    );
    let mut server = initialized_server(&workspace);
    crate::language_resources::reset_workspace_navigation_builds();

    let first = server.rename_tool(&json!({
        "source":"main.veln", "line":1, "column":4, "new_name":"first"
    }));
    let second = server.rename_tool(&json!({
        "source":"main.veln", "line":6, "column":4, "new_name":"second"
    }));

    assert_eq!(edits(&first).len(), 3, "{first:#}");
    assert_eq!(edits(&second).len(), 3, "{second:#}");
    assert_eq!(crate::language_resources::workspace_navigation_builds(), 1);

    workspace.write("main.veln", "fn target() -> Int\n  target()\nend\n");
    server
        .handle_request(json!({
            "jsonrpc":"2.0", "id":"refresh-rename-workspace", "method":"tools/call",
            "params":{"name":"refresh_workspace", "arguments":{}}
        }))
        .unwrap();
    let changed = server.rename_tool(&json!({
        "source":"main.veln", "line":1, "column":4, "new_name":"changed"
    }));

    assert_eq!(edits(&changed).len(), 2, "{changed:#}");
    assert_eq!(crate::language_resources::workspace_navigation_builds(), 2);
}

#[test]
fn rename_result_is_independent_of_full_retained_resource_capacity() {
    let workspace = TempWorkspace::new("rename-full-resource-capacity");
    write_workspace_with_dependency_and_sources(
        &workspace,
        "fn target() -> Int\n  target()\nend\n\nfn occupied() -> Int\n  1\nend\n",
        None,
    );
    let mut server = initialized_server_with_embedded_resources(&workspace);
    let arguments = [
        json!({"source":"main.veln", "line":2, "column":4, "new_name":"next"}),
        json!({"source":"main.veln", "line":2, "column":2, "new_name":"next"}),
        json!({"source":"main.veln", "line":2, "column":4, "new_name":"two words"}),
        json!({"source":"main.veln", "line":2, "column":4, "new_name":"Next"}),
        json!({"source":"main.veln", "line":2, "column":4, "new_name":"occupied"}),
    ];
    let before = arguments
        .iter()
        .map(|arguments| server.rename_tool(arguments))
        .collect::<Vec<_>>();
    fill_dependency_resource_capacity_completely(&mut server);
    let full_resources = server.language_resources.list_result();

    let after = arguments
        .iter()
        .map(|arguments| server.rename_tool(arguments))
        .collect::<Vec<_>>();

    assert_eq!(after, before);
    assert_eq!(server.language_resources.list_result(), full_resources);
}
