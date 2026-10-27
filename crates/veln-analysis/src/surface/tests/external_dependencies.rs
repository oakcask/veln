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
fn dependency_loader_preserves_written_paths_for_quarantined_public_alias_recovery() {
    let temp = TempProject::new("dependency-written-path-alias-recovery");
    temp.write(
        "veln.toml",
        "[dependencies.\"example.com/pkg\"]\npath = \"vendor/pkg\"\n",
    );
    temp.write(
        "main.veln",
        concat!(
            "use Http as wire from \"example.com/pkg\"\n",
            "\n",
            "pub fn written_entry = Http::entry\n",
            "pub fn qualified_entry = wire::entry\n",
            "pub type WrittenState = Http::State\n",
            "pub type QualifiedState = wire::State\n",
            "pub schema WrittenPacket = Http::Packet\n",
            "pub schema QualifiedPacket = wire::Packet\n",
            "\n",
            "pub fn main() -> Int\n",
            "  0\n",
            "end\n",
        ),
    );
    temp.write(
        "vendor/pkg/veln.toml",
        concat!(
            "[package]\n",
            "name = \"example.com/pkg\"\n",
            "\n",
            "[lib]\n",
            "exports = [\"http.veln\"]\n",
        ),
    );
    temp.write(
        "vendor/pkg/http.veln",
        concat!(
            "pub fn entry() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "pub type State\n",
            "  Ready\n",
            "end\n",
            "\n",
            "pub schema Packet\n",
            "  value: Int\n",
            "end\n",
        ),
    );

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
    let analysis = analyze_project(project, DoctestMode::Exclude);
    let diagnostics = analysis.checked_diagnostics();

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "module.unexported_import")
            .count(),
        1,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "written and alias-qualified targets must recover equally: {diagnostics:#?}"
    );
    assert!(
        analysis
            .module
            .uses
            .iter()
            .all(|use_decl| use_decl.name != "Http"),
        "loader must rewrite internal package identities: {:#?}",
        analysis.module.uses
    );
    let reachable = analysis.lower_reachable_entry("main", FunctionKind::Function);
    assert!(reachable.lowered.core.is_none());
    assert!(
        reachable
            .module
            .functions
            .iter()
            .all(|function| function.module_name.as_deref() != Some("example.com/pkg::http")),
        "recovery must not restore target reachability: {:#?}",
        reachable.module.functions
    );
}

#[test]
fn dependency_loader_keeps_failed_quarantined_alias_selection_unresolved() {
    let temp = TempProject::new("dependency-alias-recovery-boundaries");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"example.com/pkg\"]\n",
            "path = \"vendor/pkg\"\n",
            "[dependencies.\"example.com/first\"]\n",
            "path = \"vendor/first\"\n",
            "[dependencies.\"example.com/second\"]\n",
            "path = \"vendor/second\"\n",
        ),
    );
    temp.write(
        "main.veln",
        concat!(
            "use Http as wire from \"example.com/pkg\"\n",
            "use Collide as clash from \"example.com/pkg\"\n",
            "use Other as clash from \"example.com/pkg\"\n",
            "use Shared as first from \"example.com/first\"\n",
            "use Shared as second from \"example.com/second\"\n",
            "\n",
            "pub fn missing_written = Http::missing\n",
            "pub fn missing_qualified = wire::missing\n",
            "pub type PrivateWritten = Http::Secret\n",
            "pub type PrivateQualified = wire::Secret\n",
            "pub schema WrongWritten = Http::State\n",
            "pub schema WrongQualified = wire::State\n",
            "pub fn ambiguous_written = Http::duplicate\n",
            "pub fn ambiguous_qualified = wire::duplicate\n",
            "pub type CollisionWritten = Collide::State\n",
            "pub type CollisionQualified = clash::State\n",
            "pub schema ImportAmbiguous = Shared::Packet\n",
            "pub schema FirstControl = first::Packet\n",
            "pub schema SecondControl = second::Packet\n",
            "\n",
            "pub fn main() -> Int\n",
            "  0\n",
            "end\n",
        ),
    );
    temp.write(
        "vendor/pkg/veln.toml",
        concat!(
            "[package]\n",
            "name = \"example.com/pkg\"\n",
            "[lib]\n",
            "exports = [\"http.veln\", \"collide.veln\", \"other.veln\"]\n",
        ),
    );
    temp.write(
        "vendor/pkg/http.veln",
        concat!(
            "pub type State\n  Ready\nend\n",
            "type Secret\n  Hidden\nend\n",
            "pub fn duplicate() -> Int\n  1\nend\n",
            "pub fn duplicate() -> Int\n  2\nend\n",
        ),
    );
    temp.write("vendor/pkg/collide.veln", "pub type State\n  Ready\nend\n");
    temp.write("vendor/pkg/other.veln", "pub type State\n  Ready\nend\n");
    for package in ["first", "second"] {
        temp.write(
            &format!("vendor/{package}/veln.toml"),
            &format!(
                "[package]\nname = \"example.com/{package}\"\n[lib]\nexports = [\"shared.veln\"]\n"
            ),
        );
        temp.write(
            &format!("vendor/{package}/shared.veln"),
            "pub schema Packet\n  value: Int\nend\n",
        );
    }

    let project =
        Project::discover(temp.root().to_path_buf(), &[PathBuf::from("main.veln")]).unwrap();
    let analysis = analyze_project(project, DoctestMode::Exclude);
    let diagnostics = analysis.checked_diagnostics();
    let unresolved_targets = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .filter_map(|diagnostic| detail_string(diagnostic, "target"))
        .collect::<Vec<_>>();

    for target in [
        "Http::missing",
        "wire::missing",
        "Http::Secret",
        "wire::Secret",
        "Http::State",
        "wire::State",
        "Http::duplicate",
        "wire::duplicate",
        "Collide::State",
        "clash::State",
        "Shared::Packet",
    ] {
        assert!(
            unresolved_targets.contains(&target),
            "missing unresolved target {target}: {diagnostics:#?}"
        );
    }
    assert_eq!(unresolved_targets.len(), 11, "{diagnostics:#?}");
    assert!(
        unresolved_targets
            .iter()
            .all(|target| !matches!(*target, "first::Packet" | "second::Packet")),
        "unique qualifier controls must still recover: {diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.duplicate"),
        "collision and duplicate declarations must retain independent diagnostics"
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
