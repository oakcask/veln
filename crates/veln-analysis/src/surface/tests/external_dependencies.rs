use super::*;

#[test]
fn external_path_dependency_loads_direct_manifest_package_root() {
    let temp = TempProject::new("external-path-dependency-root");
    temp.write(
        "veln.toml",
        "[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n",
    );
    temp.write(
        "main.veln",
        "use foo from \"github.com/oakcask/foo\"\n\npub fn main() -> Int\n  add_one(1)\nend\n",
    );
    temp.write(
        "vendor/foo/veln.toml",
        "[package]\nname = \"github.com/oakcask/foo\"\n\n[lib]\nexports = [\"foo.veln\"]\n",
    );
    temp.write(
        "vendor/foo/foo.veln",
        "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
    );

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
    let (_, diagnostics) = load_surface_module(&project);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn external_adt_keeps_one_identity_through_alias_and_written_path() {
    let temp = TempProject::new("external-adt-written-path");
    temp.write(
        "veln.toml",
        "[dependencies.\"example/state\"]\npath = \"vendor/state\"\n",
    );
    temp.write(
        "main.veln",
        concat!(
            "use api::model as selected from \"example/state\"\n",
            "use rival\n",
            "\n",
            "fn ordinary(value: selected::State) -> api::model::State\n",
            "  value\n",
            "end\n",
            "\n",
            "fn refined(value: api::model::State::Ready) -> selected::State::Ready\n",
            "  value\n",
            "end\n",
            "\n",
            "fn construct_alias() -> selected::State::Ready\n",
            "  selected::State::Ready(1)\n",
            "end\n",
            "\n",
            "fn construct_written() -> api::model::State::Ready\n",
            "  api::model::State::Ready(2)\n",
            "end\n",
            "\n",
            "fn match_alias(value: selected::State) -> Int\n",
            "  match value\n",
            "    selected::State::Ready(payload) => payload\n",
            "    selected::State::Closed => 0\n",
            "  end\n",
            "end\n",
            "\n",
            "fn match_written(value: api::model::State) -> Int\n",
            "  match value\n",
            "    api::model::State::Ready(payload) => payload\n",
            "    api::model::State::Closed => 0\n",
            "  end\n",
            "end\n",
        ),
    );
    temp.write(
        "rival.veln",
        concat!(
            "pub type State\n",
            "  pub Ready(String)\n",
            "  pub Closed\n",
            "end\n",
        ),
    );
    temp.write(
        "vendor/state/veln.toml",
        concat!(
            "[package]\n",
            "name = \"example/state\"\n",
            "\n",
            "[lib]\n",
            "exports = [\"api/model.veln\"]\n",
        ),
    );
    temp.write(
        "vendor/state/api/model.veln",
        concat!(
            "pub type State\n",
            "  pub Ready(Int)\n",
            "  pub Closed\n",
            "end\n",
        ),
    );

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
    let analysis = analyze_project(project, DoctestMode::Exclude);
    let diagnostics = analysis.checked_diagnostics();

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn distinct_package_origins_keep_shared_written_path_ambiguous_in_reachability() {
    for (case_name, imports) in [
        (
            "package-alias-order-a",
            "use util as first from \"pkg_a\"\nuse util as second from \"pkg_b\"\n",
        ),
        (
            "package-alias-order-b",
            "use util as second from \"pkg_b\"\nuse util as first from \"pkg_a\"\n",
        ),
    ] {
        let temp = write_shared_package_origin_project(case_name, imports);

        let project =
            Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
        let (module, source_diagnostics) = load_surface_module(&project);
        assert!(source_diagnostics.is_empty(), "{source_diagnostics:#?}");
        assert_shared_package_origin_imports(&module);
        assert_shared_package_origin_reachability(&module);

        let analysis = analyze_project(project, DoctestMode::Exclude);
        let diagnostics = analysis.checked_diagnostics();
        for unavailable in ["util::value", "pkg_a::util::value", "pkg_b::util::value"] {
            assert!(
                diagnostics.iter().any(|diagnostic| {
                    diagnostic.id == "name.unresolved" && diagnostic.message.contains(unavailable)
                }),
                "unwritten route {unavailable:?} unexpectedly resolved: {diagnostics:#?}"
            );
        }
        assert!(
            diagnostics.iter().all(|diagnostic| {
                !diagnostic.message.contains("first::") && !diagnostic.message.contains("second::")
            }),
            "an explicit package alias failed to select its identity: {diagnostics:#?}"
        );
    }
}

fn write_shared_package_origin_project(case_name: &str, imports: &str) -> TempProject {
    let temp = TempProject::new(case_name);
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"pkg_a\"]\n",
            "path = \"vendor/a\"\n",
            "\n",
            "[dependencies.\"pkg_b\"]\n",
            "path = \"vendor/b\"\n",
        ),
    );
    temp.write(
        "main.veln",
        &format!(
            concat!(
                "{imports}",
                "pub fn aliases() -> Int\n",
                "  first::value() + second::value()\n",
                "end\n",
                "pub fn shared() -> Int\n",
                "  util::value()\n",
                "end\n",
                "pub fn internal_a() -> Int\n",
                "  pkg_a::util::value()\n",
                "end\n",
                "pub fn internal_b() -> Int\n",
                "  pkg_b::util::value()\n",
                "end\n",
                "fn alias_adts(left: first::State, right: second::State) -> Int\n",
                "  0\n",
                "end\n",
            ),
            imports = imports,
        ),
    );
    for (directory, package, value) in [("a", "pkg_a", 1), ("b", "pkg_b", 2)] {
        temp.write(
            &format!("vendor/{directory}/veln.toml"),
            &format!("[package]\nname = \"{package}\"\n\n[lib]\nexports = [\"util.veln\"]\n"),
        );
        temp.write(
            &format!("vendor/{directory}/util.veln"),
            &format!(
                concat!(
                    "pub fn value() -> Int\n",
                    "  {value}\n",
                    "end\n",
                    "pub type State\n",
                    "  pub Ready(Int)\n",
                    "end\n",
                ),
                value = value,
            ),
        );
    }
    temp
}

fn assert_shared_package_origin_imports(module: &SurfaceModule) {
    let app_uses = module
        .uses
        .iter()
        .filter(|use_decl| {
            use_decl.module_name.as_deref() == Some("main") && use_decl.origin == UseOrigin::Source
        })
        .collect::<Vec<_>>();
    assert_eq!(app_uses.len(), 2);
    for (alias, identity) in [("first", "pkg_a::util"), ("second", "pkg_b::util")] {
        let selected = app_uses
            .iter()
            .find(|use_decl| use_decl.alias == alias)
            .expect("package alias");
        assert_eq!(selected.written_name, "util");
        assert_eq!(selected.name, identity);
    }
}

fn assert_shared_package_origin_reachability(module: &SurfaceModule) {
    let aliases = reachable_entry_module(module, "aliases", FunctionKind::Function);
    assert_eq!(
        reachable_function_names(&aliases),
        [
            ("main", "aliases"),
            ("pkg_a::util", "value"),
            ("pkg_b::util", "value"),
        ]
    );
    for entry in ["shared", "internal_a", "internal_b"] {
        let unreachable_route = reachable_entry_module(module, entry, FunctionKind::Function);
        assert_eq!(
            reachable_function_names(&unreachable_route),
            [("main", entry)],
            "unwritten route retained a package target for {entry}"
        );
    }
}

#[test]
fn invalid_cased_dependency_export_isolated_from_valid_sibling() {
    let temp = TempProject::new("mixed-dependency-export-casing");
    temp.write(
        "veln.toml",
        "[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n",
    );
    temp.write(
        "main.veln",
        concat!(
            "use App from \"github.com/oakcask/foo\"\n",
            "use ok from \"github.com/oakcask/foo\"\n",
            "\n",
            "pub fn main() -> Int\n",
            "  ok::value() + App::value()\n",
            "end\n",
        ),
    );
    temp.write(
        "vendor/foo/veln.toml",
        concat!(
            "[package]\n",
            "name = \"github.com/oakcask/foo\"\n",
            "\n",
            "[lib]\n",
            "exports = [\"App/value.veln\", \"ok.veln\"]\n",
        ),
    );
    temp.write(
        "vendor/foo/App/value.veln",
        "pub fn value() -> Int\n  1\nend\n",
    );
    temp.write("vendor/foo/ok.veln", "pub fn value() -> Int\n  2\nend\n");

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
    let analysis = analyze_project(project, DoctestMode::Exclude);
    let diagnostics = analysis.checked_diagnostics();

    assert_eq!(
        diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.as_str())
            .collect::<Vec<_>>(),
        [
            "name.invalid_case",
            "module.unexported_import",
            "name.invalid_case",
            "name.unresolved",
        ],
        "{diagnostics:#?}"
    );
    assert_eq!(
        detail_string(&diagnostics[0], "origin"),
        Some("source_path")
    );
    assert_eq!(
        detail_string(&diagnostics[0], "source_kind"),
        Some("export")
    );
    assert_eq!(
        detail_string(&diagnostics[0], "source_path"),
        Some("App/value.veln")
    );
    assert_eq!(
        diagnostics[1].message,
        "external module `App` is not exported by package `github.com/oakcask/foo`"
    );
    assert_eq!(
        diagnostics[3].message,
        "unresolved call_target `App::value`"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| !diagnostic.message.contains("ok::value")),
        "{diagnostics:#?}"
    );
    assert!(
        analysis.module.functions.iter().any(|function| {
            function.module_name.as_deref() == Some("github.com/oakcask/foo::ok")
                && function.name.as_deref() == Some("value")
        }),
        "{:#?}",
        analysis.module
    );
    assert!(
        analysis.module.functions.iter().all(|function| {
            function.module_name.as_deref() != Some("github.com/oakcask/foo::App::value")
        }),
        "{:#?}",
        analysis.module
    );
}

#[test]
fn unexported_dependency_import_blocks_competing_qualifier_fallback() {
    let temp = TempProject::new("unexported-dependency-import-tombstone");
    temp.write(
        "veln.toml",
        "[dependencies.\"example/dependency\"]\npath = \"vendor/dependency\"\n",
    );
    temp.write(
        "app/main.veln",
        concat!(
            "use secret from \"example/dependency\"\n",
            "use app::fallback as secret\n",
            "pub fn main() -> Int\n",
            "  secret::value()\n",
            "end\n",
        ),
    );
    temp.write("app/fallback.veln", "pub fn value() -> Int\n  42\nend\n");
    temp.write(
        "vendor/dependency/veln.toml",
        concat!(
            "[package]\n",
            "name = \"example/dependency\"\n",
            "\n",
            "[lib]\n",
            "exports = [\"public.veln\"]\n",
        ),
    );
    temp.write(
        "vendor/dependency/public.veln",
        "pub fn value() -> Int\n  1\nend\n",
    );
    temp.write(
        "vendor/dependency/secret.veln",
        "pub fn value() -> Int\n  2\nend\n",
    );

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("app/main.veln")]).unwrap();
    let (module, diagnostics) = load_surface_module(&project);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "module.unexported_import"
            && diagnostic.message.contains("external module `secret`")
    }));
    let rejected = module
        .uses
        .iter()
        .find(|use_decl| use_decl.package.as_deref() == Some("example/dependency"))
        .expect("rejected import tombstone");
    assert!(rejected.resolution_quarantined, "{rejected:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    assert!(reachable.functions.iter().all(|function| {
        function.module_name.as_deref() != Some("app::fallback")
            || function.name.as_deref() != Some("value")
    }));

    let analysis = analyze_project(project, DoctestMode::Exclude);
    assert!(analysis.checked_diagnostics().iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved" && diagnostic.message.contains("secret::value")
    }));
}

#[test]
fn external_git_dependency_loads_materialized_subdir_package_root() {
    let temp = TempProject::new("external-git-dependency-subdir-root");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/foo\"]\n",
            "git = \"materialized/mono\"\n",
            "rev = \"abc123\"\n",
            "subdir = \"packages/foo\"\n",
        ),
    );
    temp.write(
        "main.veln",
        "use foo from \"github.com/oakcask/foo\"\n\npub fn main() -> Int\n  add_one(1)\nend\n",
    );
    temp.write(
        "materialized/mono/packages/foo/veln.toml",
        "[package]\nname = \"github.com/oakcask/foo\"\n\n[lib]\nexports = [\"foo.veln\"]\n",
    );
    temp.write(
        "materialized/mono/packages/foo/foo.veln",
        "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
    );

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
    let (_, diagnostics) = load_surface_module(&project);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn captured_dependencies_do_not_fall_back_to_direct_sources() {
    let temp = TempProject::new("captured-dependencies-no-direct-fallback");
    temp.write(
        "veln.toml",
        "[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n",
    );
    temp.write(
        "main.veln",
        "use foo from \"github.com/oakcask/foo\"\n\npub fn main() -> Int\n  add_one(1)\nend\n",
    );
    temp.write(
        "vendor/foo/veln.toml",
        "[package]\nname = \"github.com/oakcask/foo\"\n\n[lib]\nexports = [\"foo.veln\"]\n",
    );
    temp.write(
        "vendor/foo/foo.veln",
        "pub fn add_one(value: Int) -> Int\n  value + 1\nend\n",
    );

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
    let (modules, diagnostics) = load_surface_modules_with_captured_dependencies(&project, &[]);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "module.unavailable_package");
    assert!(
        modules
            .application
            .functions
            .iter()
            .all(|function| function.name.as_deref() != Some("add_one")),
        "{:#?}",
        modules.application
    );
}

#[cfg(unix)]
#[test]
fn external_path_dependency_without_direct_manifest_does_not_read_sources() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempProject::new("external-path-dependency-missing-manifest");
    temp.write(
        "veln.toml",
        "[dependencies.\"github.com/oakcask/foo\"]\npath = \"vendor/foo\"\n",
    );
    temp.write(
        "main.veln",
        "use foo from \"github.com/oakcask/foo\"\n\npub fn main() -> Int\n  0\nend\n",
    );
    temp.write("vendor/foo/foo.veln", "unreadable source");
    let source = temp.path("vendor/foo/foo.veln");
    let original = fs::metadata(&source).unwrap().permissions();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o000)).unwrap();

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
    let (_, diagnostics) = load_surface_module(&project);

    fs::set_permissions(&source, original).unwrap();
    if !nix_like_effective_root() {
        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
        assert_eq!(diagnostics[0].id, "manifest.package_name_mismatch");
        assert!(
            diagnostics[0]
                .message
                .contains("dependency package name `<missing>`"),
            "{diagnostics:#?}"
        );
    }
}

#[cfg(unix)]
fn nix_like_effective_root() -> bool {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("Uid:"))
                .and_then(|line| line.split_whitespace().nth(2))
                .and_then(|uid| uid.parse::<u32>().ok())
        })
        == Some(0)
}
