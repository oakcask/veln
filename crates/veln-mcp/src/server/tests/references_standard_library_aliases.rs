use super::*;
use std::cell::Cell;
use std::rc::Rc;
use veln_project::PackageSnapshotSource;

fn assert_reference_ranges(
    result: &Value,
    expected: &[(&str, usize, usize, usize, usize)],
    case: &str,
) {
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert_eq!(references.len(), expected.len(), "{case}: {result:#}");
    for (reference, (path, start_line, start_column, end_line, end_column)) in
        references.iter().zip(expected)
    {
        assert!(
            reference["uri"].as_str().unwrap().ends_with(path),
            "{case}: {reference:#}"
        );
        assert_eq!(reference["range"]["start"]["line"], *start_line, "{case}");
        assert_eq!(
            reference["range"]["start"]["column"], *start_column,
            "{case}"
        );
        assert_eq!(reference["range"]["end"]["line"], *end_line, "{case}");
        assert_eq!(reference["range"]["end"]["column"], *end_column, "{case}");
    }
}

fn install_alias_standard_library(server: &mut Server, source: &str) {
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"math.veln\"]\n",
        [PackageSnapshotSource::new("math.veln", source.as_bytes())],
    );
}

fn install_type_alias_standard_library(server: &mut Server, sources: &[(&str, &str)]) {
    install_type_alias_standard_library_with_exports(
        server,
        sources,
        sources.iter().map(|(path, _)| *path).collect(),
    );
}

fn install_type_alias_standard_library_with_exports(
    server: &mut Server,
    sources: &[(&str, &str)],
    exports: Vec<&str>,
) {
    let exports = sources
        .iter()
        .map(|(path, _)| *path)
        .filter(|path| exports.contains(path))
        .map(|path| format!("\"{path}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let manifest = format!("[package]\nname = \"std\"\n\n[lib]\nexports = [{exports}]\n");
    server.language_resources.replace_test_standard_library(
        &manifest,
        sources
            .iter()
            .map(|(path, source)| PackageSnapshotSource::new(*path, source.as_bytes())),
    );
}

fn assert_snapshot_changed_without_references_or_scope(result: &Value) {
    assert_eq!(result["isError"], true);
    assert_eq!(result["structuredContent"]["code"], "snapshot_changed");
    assert!(
        result["structuredContent"]
            .as_object()
            .unwrap()
            .get("references")
            .is_none()
    );
}

fn all_resource_state(server: &mut Server) -> Value {
    let resources = server
        .handle_request(json!({"jsonrpc":"2.0","id":"references-state","method":"resources/list"}))
        .unwrap()["result"]["resources"]
        .as_array()
        .unwrap()
        .clone();
    json!(resources)
}

fn dependency_resource_is_listed(server: &mut Server, identity: &str) -> bool {
    let prefix = format!("veln-pkg:///{}/snapshot/", identity.replace('/', "%2F"));
    all_resource_state(server)
        .as_array()
        .unwrap()
        .iter()
        .any(|resource| resource["uri"].as_str().unwrap().starts_with(&prefix))
}

#[test]
fn references_return_standard_library_function_alias_locations() {
    let workspace = TempWorkspace::new("references-standard-library-alias");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "use math from \"std\"\n\n",
            "fn first(value: Int) -> Int\n",
            "  math::renamed(value)\n",
            "end\n\n",
            "fn second(value: Int) -> Int\n",
            "  let callback: fn(Int) -> Int = math::renamed\n",
            "  callback(math::renamed(value)) + math::target(value)\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    install_alias_standard_library(
        &mut server,
        "pub fn target(value: Int) -> Int\n  value\nend\n\npub fn renamed = target\n",
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"]["project_wide"], true,
        "{result:#}"
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 4, 9, 4, 16),
            ("main.veln", 8, 40, 8, 47),
            ("main.veln", 9, 18, 9, 25),
        ],
        "standard library function alias",
    );
}

#[test]
fn references_keep_standard_library_function_alias_chain_boundary_empty() {
    let workspace = TempWorkspace::new("references-standard-library-alias-chain");
    workspace.write("veln.toml", "");
    workspace.write("main.veln", "fn main() -> Int\n  prelude::chained()\nend\n");
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"prelude.veln\"]\n",
        [PackageSnapshotSource::new(
            "prelude.veln",
            concat!(
                "pub fn target() -> Int\n",
                "  1\n",
                "end\n\n",
                "pub fn renamed = target\n",
                "pub fn chained = renamed\n",
            )
            .as_bytes(),
        )],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":2,"column":12}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["references"],
        json!([]),
        "{result:#}"
    );
}

#[test]
fn references_keep_standard_library_alias_chain_empty_for_non_exported_target_source() {
    let workspace = TempWorkspace::new("references-standard-library-alias-private-source-chain");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "use api from \"std\"\n\n",
            "fn main() -> Int\n",
            "  api::chained()\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"api.veln\"]\n",
        [
            PackageSnapshotSource::new(
                "api.veln",
                "pub fn chained = implementation::renamed\n".as_bytes(),
            ),
            PackageSnapshotSource::new(
                "implementation.veln",
                concat!(
                    "pub fn target() -> Int\n",
                    "  1\n",
                    "end\n\n",
                    "pub fn renamed = target\n",
                )
                .as_bytes(),
            ),
        ],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["references"],
        json!([]),
        "{result:#}"
    );
}

#[test]
fn references_keep_invalid_standard_library_function_alias_targets_empty() {
    let workspace = TempWorkspace::new("references-standard-library-invalid-alias-targets");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "use api from \"std\"\n\n",
            "fn main() -> Int\n",
            "  api::missing()\n",
            "  api::wrong_kind()\n",
            "  api::invalid_case()\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    server.language_resources.replace_test_standard_library(
        "[package]\nname = \"std\"\n\n[lib]\nexports = [\"api.veln\"]\n",
        [PackageSnapshotSource::new(
            "api.veln",
            concat!(
                "pub type Document\n",
                "  pub Text(String)\n",
                "end\n\n",
                "pub fn missing = missing\n",
                "pub fn wrong_kind = Document\n",
                "pub fn invalid_case = Missing\n",
            )
            .as_bytes(),
        )],
    );

    for (case, line) in [
        ("unresolved target", 4),
        ("wrong-kind target", 5),
        ("invalid-casing target", 6),
    ] {
        let result = server.references_tool(&json!({"source":"main.veln","line":line,"column":9}));
        assert_eq!(result["isError"], false, "{case}: {result:#}");
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{case}: {result:#}"
        );
    }
}

#[test]
fn references_project_capture_exhausts_retries_for_standard_library_alias_selection() {
    let workspace = TempWorkspace::new("references-standard-library-alias-capture-retry");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "use math from \"std\"\n\nfn main() -> Int\n  math::renamed()\nend\n",
    );
    let mut server = initialized_server(&workspace);
    install_alias_standard_library(
        &mut server,
        "pub fn value() -> Int\n  1\nend\n\npub fn renamed = value\n",
    );
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let value = if attempt % 2 == 0 { 1 } else { 2 };
        fs::write(
            root.join("main.veln"),
            format!(
                "use math from \"std\"\n\nfn main() -> Int\n  math::renamed() + {value}\nend\n"
            ),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":4,"column":9}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}

#[test]
fn references_return_standard_library_type_alias_locations() {
    let workspace = TempWorkspace::new("references-standard-library-type-alias");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "use facade from \"std\"\n\n",
            "pub type Local = facade::Count\n\n",
            "fn read(input: facade::Count) -> Vec<facade::Count>\n",
            "  facade::Count::Count(1)\n",
            "end\n",
        ),
    );
    workspace.write(
        "other.veln",
        concat!(
            "use facade from \"std\"\n\n",
            "fn other(input: facade::Count) -> facade::Count\n",
            "  input\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    install_type_alias_standard_library(
        &mut server,
        &[
            ("facade.veln", "use core\n\npub type Count = core::Count\n"),
            ("core.veln", "pub type Count\n  pub Count(Int)\nend\n"),
        ],
    );

    let result = server.references_tool(&json!({"source":"main.veln","line":5,"column":25}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"]["project_wide"], true,
        "{result:#}"
    );
    assert_reference_ranges(
        &result,
        &[
            ("main.veln", 3, 26, 3, 31),
            ("main.veln", 5, 24, 5, 29),
            ("main.veln", 5, 46, 5, 51),
            ("main.veln", 6, 11, 6, 16),
            ("other.veln", 3, 25, 3, 30),
            ("other.veln", 3, 43, 3, 48),
        ],
        "standard library type alias",
    );
    let references = result["structuredContent"]["references"]
        .as_array()
        .unwrap();
    assert!(references.iter().all(|reference| {
        reference["uri"].as_str().unwrap().starts_with("file://")
            && !reference["uri"].as_str().unwrap().contains("veln-pkg:")
    }));
}

#[test]
fn references_keep_standard_library_type_aliases_inside_selected_project() {
    let workspace = TempWorkspace::new("references-standard-library-type-alias-project-isolation");
    workspace.write("app/veln.toml", "");
    workspace.write(
        "app/main.veln",
        concat!(
            "fn app(input: Count) -> Count\n",
            "  Count::Count(1)\n",
            "end\n",
        ),
    );
    workspace.write("other/veln.toml", "");
    workspace.write(
        "other/main.veln",
        concat!(
            "fn other(input: Count) -> Count\n",
            "  Count::Count(2)\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    install_type_alias_standard_library(
        &mut server,
        &[(
            "prelude.veln",
            "pub type Target\n  pub Count(Int)\nend\n\npub type Count = Target\n",
        )],
    );

    let result = server.references_tool(&json!({"source":"app/main.veln","line":1,"column":15}));

    assert_eq!(result["isError"], false, "{result:#}");
    assert_eq!(
        result["structuredContent"]["scope"],
        json!({
            "mode": "project",
            "generation": 0,
            "project": "app",
            "project_wide": true
        }),
        "{result:#}"
    );
    assert_reference_ranges(
        &result,
        &[
            ("app/main.veln", 1, 15, 1, 20),
            ("app/main.veln", 1, 25, 1, 30),
            ("app/main.veln", 2, 3, 2, 8),
        ],
        "standard library type alias project isolation",
    );
}

#[test]
fn references_keep_unsupported_standard_library_type_alias_selections_empty() {
    struct Case {
        name: &'static str,
        source: &'static str,
        standard_sources: Vec<(&'static str, &'static str)>,
        exports: Vec<&'static str>,
        column: usize,
    }

    for case in [
        Case {
            name: "private alias",
            source: "use api from \"std\"\n\nfn read(input: api::PrivateAlias) -> Int\n  0\nend\n",
            standard_sources: vec![(
                "api.veln",
                "pub type Target\nend\n\ntype PrivateAlias = Target\n",
            )],
            exports: vec!["api.veln"],
            column: 21,
        },
        Case {
            name: "non-exported module alias",
            source: "use hidden from \"std\"\n\nfn read(input: hidden::HiddenAlias) -> Int\n  0\nend\n",
            standard_sources: vec![
                ("api.veln", "pub type Target\nend\n"),
                (
                    "hidden.veln",
                    "pub type Target\nend\n\npub type HiddenAlias = Target\n",
                ),
            ],
            exports: vec!["api.veln"],
            column: 24,
        },
        Case {
            name: "invalid-casing alias record",
            source: "use api from \"std\"\n\nfn read(input: api::alias) -> Int\n  0\nend\n",
            standard_sources: vec![(
                "api.veln",
                "pub type Target\nend\n\npub type alias = Target\n",
            )],
            exports: vec!["api.veln"],
            column: 21,
        },
    ] {
        let workspace = TempWorkspace::new(&format!(
            "references-standard-library-type-alias-unsupported-{}",
            case.name.replace(' ', "-")
        ));
        workspace.write("veln.toml", "");
        workspace.write("main.veln", case.source);
        let mut server = initialized_server(&workspace);
        install_type_alias_standard_library_with_exports(
            &mut server,
            &case.standard_sources,
            case.exports,
        );
        let result =
            server.references_tool(&json!({"source":"main.veln","line":3,"column":case.column}));
        assert_eq!(result["isError"], false, "{}: {result:#}", case.name);
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{}: {result:#}",
            case.name
        );
    }
}

#[test]
fn references_keep_invalid_standard_library_type_alias_targets_empty() {
    let workspace = TempWorkspace::new("references-standard-library-invalid-type-alias-targets");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        concat!(
            "fn read(missing: MissingAlias, wrong: WrongKind, chain: Chain) -> Int\n",
            "  0\n",
            "end\n",
        ),
    );
    let mut server = initialized_server(&workspace);
    install_type_alias_standard_library(
        &mut server,
        &[(
            "prelude.veln",
            concat!(
                "pub type Target\n",
                "end\n\n",
                "pub fn value() -> Int\n",
                "  1\n",
                "end\n\n",
                "pub type Good = Target\n",
                "pub type MissingAlias = Missing\n",
                "pub type WrongKind = value\n",
                "pub type Chain = Good\n",
            ),
        )],
    );

    for (case, column) in [
        ("unresolved target", 18),
        ("wrong-kind target", 39),
        ("alias-chain target", 57),
    ] {
        let result =
            server.references_tool(&json!({"source":"main.veln","line":1,"column":column}));
        assert_eq!(result["isError"], false, "{case}: {result:#}");
        assert_eq!(
            result["structuredContent"]["references"],
            json!([]),
            "{case}: {result:#}"
        );
    }
}

#[test]
fn references_project_capture_exhausts_retries_for_standard_library_type_alias_selection() {
    let workspace = TempWorkspace::new("references-standard-library-type-alias-capture-retry");
    workspace.write("veln.toml", "");
    workspace.write(
        "main.veln",
        "fn main(input: Count) -> Count\n  input\nend\n",
    );
    let mut server = initialized_server(&workspace);
    install_type_alias_standard_library(
        &mut server,
        &[(
            "prelude.veln",
            "pub type Target\nend\n\npub type Count = Target\n",
        )],
    );
    let before_resources = all_resource_state(&mut server);
    let before_selection = server.selection_result();
    let attempts = Rc::new(Cell::new(0));
    let attempts_for_hook = attempts.clone();
    let root = workspace.root.clone();
    let _hook = crate::check_project::set_after_first_stable_capture_hook(move || {
        let attempt = attempts_for_hook.get();
        attempts_for_hook.set(attempt + 1);
        let suffix = if attempt % 2 == 0 {
            ""
        } else {
            "\n# changed\n"
        };
        fs::write(
            root.join("main.veln"),
            format!("fn main(input: Count) -> Count\n  input\nend\n{suffix}"),
        )
        .unwrap();
    });

    let result = server.references_tool(&json!({"source":"main.veln","line":1,"column":17}));

    assert_snapshot_changed_without_references_or_scope(&result);
    assert_eq!(attempts.get(), 3);
    assert_eq!(all_resource_state(&mut server), before_resources);
    assert_eq!(server.selection_result(), before_selection);
    assert!(!dependency_resource_is_listed(&mut server, "example/dep"));
}
