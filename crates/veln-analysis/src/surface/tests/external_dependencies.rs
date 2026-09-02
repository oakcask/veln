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
