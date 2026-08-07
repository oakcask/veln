use super::*;
use crate::manifest::read_manifest;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn keeps_explicit_files_sorted_and_unique() {
    let root = PathBuf::from(".");
    let paths = discover_source_paths(
        &root,
        &[
            PathBuf::from("b.veln"),
            PathBuf::from("a.veln"),
            PathBuf::from("a.veln"),
        ],
    )
    .unwrap();

    assert_eq!(
        paths,
        vec![PathBuf::from("a.veln"), PathBuf::from("b.veln")]
    );
}

#[test]
fn discovers_veln_files_recursively_and_only_skips_git_directories() {
    let temp = TempProject::new("recursive-discovery");
    temp.write("src/main.veln", "main");
    temp.write("src/nested/lib.veln", "lib");
    temp.write("src/readme.txt", "not source");
    temp.write("target/generated.veln", "owned");
    temp.write(".git/hooks/hook.veln", "ignored");

    let paths = discover_source_paths(temp.root(), &[]).unwrap();

    assert_eq!(
        paths,
        vec![
            temp.path("src/main.veln"),
            temp.path("src/nested/lib.veln"),
            temp.path("target/generated.veln"),
        ]
    );
}

#[test]
fn nested_manifest_files_bound_recursive_discovery_without_being_parsed() {
    let temp = TempProject::new("nested-manifest-boundaries");
    temp.write("app.veln", "owned");
    temp.write("vendor/deep/package/veln.toml", "not valid manifest syntax");
    temp.write("vendor/deep/package/nested.veln", "not owned");
    temp.write("target/owned.veln", "owned");
    temp.write("target/package/veln.toml", "[package");
    temp.write("target/package/nested.veln", "not owned");

    let paths = discover_source_paths(temp.root(), &[]).unwrap();

    assert_eq!(
        paths,
        vec![temp.path("app.veln"), temp.path("target/owned.veln")]
    );
}

#[test]
fn directory_named_veln_toml_does_not_establish_a_boundary() {
    let temp = TempProject::new("manifest-marker-directory");
    temp.write("branch/veln.toml/contents.txt", "not a manifest file");
    temp.write("branch/owned.veln", "owned");

    let paths = discover_source_paths(temp.root(), &[]).unwrap();

    assert_eq!(paths, vec![temp.path("branch/owned.veln")]);
}

#[test]
fn discovers_veln_files_from_explicit_directories() {
    let temp = TempProject::new("directory-input");
    temp.write("src/main.veln", "main");
    temp.write("tests/case.veln", "case");
    temp.write("tests/case.txt", "ignored");

    let paths = discover_source_paths(temp.root(), &[PathBuf::from("tests")]).unwrap();

    assert_eq!(paths, vec![temp.path("tests/case.veln")]);
}

#[test]
fn explicit_directories_skip_git_but_include_target_subdirectories() {
    let temp = TempProject::new("directory-input-ignored-subdirs");
    temp.write("tests/case.veln", "case");
    temp.write("tests/target/generated.veln", "owned");
    temp.write("tests/.git/hooks/hook.veln", "ignored");

    let paths = discover_source_paths(temp.root(), &[PathBuf::from("tests")]).unwrap();

    assert_eq!(
        paths,
        vec![
            temp.path("tests/case.veln"),
            temp.path("tests/target/generated.veln"),
        ]
    );
}

#[test]
fn explicit_inputs_reject_nested_package_ownership() {
    let temp = TempProject::new("explicit-nested-package");
    temp.write("nested/veln.toml", "");
    temp.write("nested/source.veln", "nested");

    let error = discover_source_paths(
        temp.root(),
        &[
            PathBuf::from("app.veln"),
            PathBuf::from("nested/source.veln"),
        ],
    )
    .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("nested/source.veln"), "{message}");
    assert!(
        message.contains(&temp.path("nested").display().to_string()),
        "{message}"
    );

    let directory_error =
        discover_source_paths(temp.root(), &[PathBuf::from("nested")]).unwrap_err();
    let directory_message = directory_error.to_string();
    assert!(directory_message.contains("nested`"), "{directory_message}");
    assert!(
        directory_message.contains(&temp.path("nested").display().to_string()),
        "{directory_message}"
    );
}

#[test]
fn explicit_inputs_reject_paths_outside_the_package_root() {
    let temp = TempProject::new("explicit-outside-package");
    let outside = temp.root().parent().unwrap().join("outside.veln");

    let absolute_error = discover_source_paths(temp.root(), &[outside.clone()]).unwrap_err();
    assert!(
        absolute_error
            .to_string()
            .contains(&outside.display().to_string())
    );

    let parent_error =
        discover_source_paths(temp.root(), &[PathBuf::from("../outside.veln")]).unwrap_err();
    assert!(parent_error.to_string().contains("../outside.veln"));
}

#[test]
fn explicit_inputs_reject_parent_component_escaping_the_package_root() {
    let temp = TempProject::new("explicit-parent-escape");
    temp.write("source.veln", "source");

    let error =
        discover_source_paths(temp.root(), &[PathBuf::from("src/../../source.veln")]).unwrap_err();

    let message = error.to_string();
    assert!(message.contains("src/../../source.veln"), "{message}");
    assert!(
        message.contains("outside the supplied package root"),
        "{message}"
    );
}

#[cfg(unix)]
#[test]
fn recursive_discovery_does_not_follow_source_directory_or_manifest_symlinks() {
    use std::os::unix::fs::symlink;

    let temp = TempProject::new("recursive-symlinks");
    temp.write("owned/source.veln", "owned");
    temp.write("linked-target/source.veln", "not reached through link");
    symlink(temp.path("linked-target"), temp.path("directory-link")).unwrap();
    symlink(
        temp.path("owned/source.veln"),
        temp.path("source-link.veln"),
    )
    .unwrap();
    fs::create_dir_all(temp.path("marker-link")).unwrap();
    symlink(
        temp.path("missing-manifest"),
        temp.path("marker-link/veln.toml"),
    )
    .unwrap();
    temp.write("marker-link/owned.veln", "owned");

    let paths = discover_source_paths(temp.root(), &[]).unwrap();

    assert_eq!(
        paths,
        vec![
            temp.path("linked-target/source.veln"),
            temp.path("marker-link/owned.veln"),
            temp.path("owned/source.veln"),
        ]
    );
}

#[cfg(unix)]
#[test]
fn explicit_inputs_reject_symlinks_below_the_package_root() {
    use std::os::unix::fs::symlink;

    let temp = TempProject::new("explicit-symlink");
    temp.write("real/source.veln", "source");
    symlink(temp.path("real"), temp.path("linked")).unwrap();

    let error =
        discover_source_paths(temp.root(), &[PathBuf::from("linked/source.veln")]).unwrap_err();

    let message = error.to_string();
    assert!(message.contains("linked/source.veln"), "{message}");
    assert!(message.contains("symbolic link"), "{message}");
}

#[cfg(unix)]
#[test]
fn explicit_inputs_reject_symlinks_before_parent_components() {
    use std::os::unix::fs::symlink;

    let temp = TempProject::new("explicit-symlink-parent");
    temp.write("real/source.veln", "source");
    fs::create_dir_all(temp.path("through")).unwrap();
    symlink(temp.path("through"), temp.path("linked")).unwrap();

    let error = discover_source_paths(temp.root(), &[PathBuf::from("linked/../real/source.veln")])
        .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("linked/../real/source.veln"), "{message}");
    assert!(message.contains("symbolic link"), "{message}");
}

#[cfg(unix)]
#[test]
fn unreadable_regular_manifest_still_establishes_a_boundary() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempProject::new("unreadable-manifest-boundary");
    temp.write("nested/veln.toml", "");
    temp.write("nested/source.veln", "not owned");
    fs::set_permissions(
        temp.path("nested/veln.toml"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();

    let paths = discover_source_paths(temp.root(), &[]).unwrap();

    assert!(paths.is_empty());
}

#[cfg(unix)]
#[test]
fn boundary_candidate_classification_error_fails_discovery() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempProject::new("manifest-boundary-classification-error");
    temp.write("owned.veln", "owned");
    temp.write("restricted/veln.toml", "");
    fs::set_permissions(temp.path("restricted"), fs::Permissions::from_mode(0o000)).unwrap();

    let error = discover_source_paths(temp.root(), &[]).unwrap_err();

    fs::set_permissions(temp.path("restricted"), fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
}

#[test]
fn deduplicates_overlapping_explicit_directory_inputs() {
    let temp = TempProject::new("overlapping-directory-inputs");
    temp.write("tests/unit/a.veln", "a");
    temp.write("tests/unit/b.veln", "b");
    temp.write("tests/integration/c.veln", "c");

    let paths = discover_source_paths(
        temp.root(),
        &[PathBuf::from("tests"), PathBuf::from("tests/unit")],
    )
    .unwrap();

    assert_eq!(
        paths,
        vec![
            temp.path("tests/integration/c.veln"),
            temp.path("tests/unit/a.veln"),
            temp.path("tests/unit/b.veln"),
        ]
    );
}

#[test]
fn discovers_veln_files_from_absolute_directory_inputs() {
    let temp = TempProject::new("absolute-directory-input");
    temp.write("src/main.veln", "main");
    temp.write("tests/case.veln", "case");
    temp.write("tests/case.txt", "ignored");

    let paths = discover_source_paths(temp.root(), &[temp.path("tests")]).unwrap();

    assert_eq!(paths, vec![temp.path("tests/case.veln")]);
}

#[test]
fn keeps_explicit_non_veln_files() {
    let temp = TempProject::new("explicit-non-veln");
    temp.write("notes.txt", "notes");

    let paths = discover_source_paths(temp.root(), &[PathBuf::from("notes.txt")]).unwrap();

    assert_eq!(paths, vec![temp.path("notes.txt")]);
}

#[test]
fn keeps_absolute_explicit_files_sorted_and_unique() {
    let temp = TempProject::new("absolute-file-input");
    temp.write("src/a.veln", "a");
    temp.write("src/b.veln", "b");

    let paths = discover_source_paths(
        temp.root(),
        &[
            temp.path("src/b.veln"),
            temp.path("src/a.veln"),
            temp.path("src/a.veln"),
        ],
    )
    .unwrap();

    assert_eq!(
        paths,
        vec![temp.path("src/a.veln"), temp.path("src/b.veln")]
    );
}

#[test]
fn deduplicates_mixed_relative_and_absolute_file_inputs() {
    let temp = TempProject::new("mixed-file-input");
    temp.write("src/a.veln", "a");
    temp.write("src/b.veln", "b");

    let paths = discover_source_paths(
        temp.root(),
        &[
            PathBuf::from("src/a.veln"),
            temp.path("src/a.veln"),
            PathBuf::from("src/b.veln"),
        ],
    )
    .unwrap();

    assert_eq!(
        paths,
        vec![temp.path("src/a.veln"), temp.path("src/b.veln")]
    );
}

#[test]
fn project_discover_reads_sources_with_project_relative_paths() {
    let temp = TempProject::new("project-discover");
    temp.write("src/b.veln", "second");
    temp.write("src/a.veln", "first");

    let project = Project::discover(temp.root().to_path_buf(), &[]).unwrap();

    assert_eq!(project.root, temp.root().to_path_buf());
    let files = project
        .files
        .iter()
        .map(|file| (file.path().as_str().to_string(), file.text().to_string()))
        .collect::<Vec<_>>();
    assert_eq!(
        files,
        vec![
            ("src/a.veln".to_string(), "first".to_string()),
            ("src/b.veln".to_string(), "second".to_string()),
        ]
    );
}

#[test]
fn project_discover_reads_explicit_files_with_project_relative_paths() {
    let temp = TempProject::new("project-discover-explicit-files");
    temp.write("examples/b.veln", "second");
    temp.write("examples/a.veln", "first");

    let project = Project::discover(
        temp.root().to_path_buf(),
        &[
            temp.path("examples/b.veln"),
            PathBuf::from("examples/a.veln"),
        ],
    )
    .unwrap();

    let files = project
        .files
        .iter()
        .map(|file| (file.path().as_str().to_string(), file.text().to_string()))
        .collect::<Vec<_>>();
    assert_eq!(
        files,
        vec![
            ("examples/a.veln".to_string(), "first".to_string()),
            ("examples/b.veln".to_string(), "second".to_string()),
        ]
    );
}

#[cfg(unix)]
#[test]
fn project_discover_does_not_add_a_symlinked_companion_target() {
    use std::os::unix::fs::symlink;

    let temp = TempProject::new("symlinked-companion-target");
    temp.write("real.veln", "target");
    temp.write("linked.test.veln", "companion");
    symlink(temp.path("real.veln"), temp.path("linked.veln")).unwrap();

    let project = Project::discover(
        temp.root().to_path_buf(),
        &[PathBuf::from("linked.test.veln")],
    )
    .unwrap();

    assert_eq!(project.files.len(), 1);
    assert_eq!(project.files[0].path().as_str(), "linked.test.veln");
}

#[test]
fn project_discover_reads_manifest_with_explicit_inputs() {
    let temp = TempProject::new("project-discover-explicit-manifest");
    temp.write("src/main.veln", "mod app.main\n");
    temp.write("src/extra.veln", "mod app.extra\n");
    temp.write("veln.toml", "[lib]\nexports = [\"src/main.veln\"]\n");

    let project = Project::discover(
        temp.root().to_path_buf(),
        &[PathBuf::from("src/extra.veln")],
    )
    .unwrap();

    let files = project
        .files
        .iter()
        .map(|file| file.path().as_str().to_string())
        .collect::<Vec<_>>();
    let manifest = project.manifest.expect("manifest should be loaded");
    assert_eq!(files, vec!["src/extra.veln".to_string()]);
    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
}

#[test]
fn project_discover_reports_missing_explicit_file() {
    let temp = TempProject::new("project-discover-missing-explicit-file");

    let error = Project::discover(temp.root().to_path_buf(), &[PathBuf::from("missing.veln")])
        .expect_err("missing explicit file should fail");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn project_discover_reports_missing_absolute_explicit_file() {
    let temp = TempProject::new("project-discover-missing-absolute-explicit-file");

    let error = Project::discover(temp.root().to_path_buf(), &[temp.path("missing.veln")])
        .expect_err("missing explicit file should fail");

    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

#[test]
fn project_discover_reads_manifest_lib_exports() {
    let temp = TempProject::new("manifest-lib-exports");
    temp.write("src/main.veln", "mod app.main\n");
    temp.write("veln.toml", "[lib]\nexports = [\"src/main.veln\"]\n");

    let project = Project::discover(temp.root().to_path_buf(), &[]).unwrap();
    let manifest = project.manifest.expect("manifest should be loaded");

    assert_eq!(manifest.path.as_str(), "veln.toml");
    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 2);
    assert_eq!(manifest.lib.exports[0].path_span.start.column, 13);
}

#[test]
fn read_manifest_returns_none_when_manifest_is_absent() {
    let temp = TempProject::new("manifest-absent");

    let manifest = read_manifest(temp.root()).unwrap();

    assert!(manifest.is_none());
}

#[test]
fn read_manifest_tracks_lib_exports_and_ignores_other_sections() {
    let temp = TempProject::new("manifest-lib-sections");
    temp.write(
        "veln.toml",
        concat!(
            "[package]\n",
            "\"ignored.veln\" = \"ignored.module\"\n",
            "[lib]\n",
            "# comment\n",
            "not-an-entry\n",
            "exports = [\"src/main.veln\"]\n",
            "[other]\n",
            "\"ignored-again.veln\" = \"ignored.again\"\n",
            "[lib]\n",
            "exports = [\n",
            "  \"src/lib.veln\",\n",
            "]\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.path.as_str(), "veln.toml");
    assert_eq!(manifest.lib.exports.len(), 2);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 6);
    assert_eq!(manifest.lib.exports[0].path_span.start.column, 13);
    assert_eq!(manifest.lib.exports[1].path, "src/lib.veln");
    assert_eq!(manifest.lib.exports[1].path_span.start.line, 11);
    assert_eq!(manifest.lib.exports[1].path_span.start.column, 4);
}

#[test]
fn read_manifest_tracks_package_and_tool_string_fields() {
    let temp = TempProject::new("manifest-package-tool-fields");
    temp.write(
        "veln.toml",
        concat!(
            "[package]\n",
            "name = \"demo\"\n",
            "version = \"0.1.0\"\n",
            "ignored = 1\n",
            "[tool.docs]\n",
            "template = \"reference\"\n",
            "[tool.docs]\n",
            "output = \"docs/api.md\"\n",
            "[lib]\n",
            "exports = [\"src/main.veln\"]\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.package.fields.len(), 2);
    assert_eq!(manifest.package.fields[0].key, "name");
    assert_eq!(manifest.package.fields[0].value, "demo");
    assert_eq!(manifest.package.fields[0].key_span.start.line, 2);
    assert_eq!(manifest.package.fields[0].value_span.start.column, 9);
    assert_eq!(manifest.package.fields[1].key, "version");
    assert_eq!(manifest.tools.len(), 1);
    assert_eq!(manifest.tools[0].name, "docs");
    assert_eq!(manifest.tools[0].fields.len(), 2);
    assert_eq!(manifest.tools[0].fields[0].key, "template");
    assert_eq!(manifest.tools[0].fields[0].value, "reference");
    assert_eq!(manifest.tools[0].fields[1].key, "output");
    assert_eq!(manifest.tools[0].fields[1].value, "docs/api.md");
    assert_eq!(manifest.lib.exports.len(), 1);
}

#[test]
fn read_manifest_tracks_path_dependencies() {
    let temp = TempProject::new("manifest-path-dependencies");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/foo\"]\n",
            "path = \"vendor/foo\"\n",
            "[dependencies.\"github.com/oakcask/bar\"]\n",
            "git = \"https://example.invalid/bar.git\"\n",
            "path = \"vendor/bar\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.dependencies.len(), 2);
    assert_eq!(manifest.dependencies[0].package, "github.com/oakcask/foo");
    assert_eq!(manifest.dependencies[0].package_span.start.line, 1);
    assert_eq!(manifest.dependencies[0].package_span.start.column, 16);
    let foo_path = manifest.dependencies[0]
        .path
        .as_ref()
        .expect("foo dependency should have a path");
    assert_eq!(foo_path.key, "path");
    assert_eq!(foo_path.value, "vendor/foo");
    assert_eq!(foo_path.value_span.start.line, 2);
    assert_eq!(foo_path.value_span.start.column, 9);
    assert_eq!(manifest.dependencies[1].package, "github.com/oakcask/bar");
    assert_eq!(
        manifest.dependencies[1]
            .path
            .as_ref()
            .expect("bar dependency should have a path")
            .value,
        "vendor/bar"
    );
}

#[test]
fn read_manifest_tracks_git_dependency_metadata() {
    let temp = TempProject::new("manifest-git-dependencies");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/foo\"]\n",
            "git = \"https://example.invalid/foo.git\"\n",
            "tag = \"v1.2.0\"\n",
            "[dependencies.\"github.com/oakcask/bar\"]\n",
            "git = \"https://example.invalid/mono.git\"\n",
            "branch = \"main\"\n",
            "subdir = \"packages/bar\"\n",
            "[dependencies.\"github.com/oakcask/baz\"]\n",
            "git = \"https://example.invalid/baz.git\"\n",
            "rev = \"0123456789abcdef\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    let foo = &manifest.dependencies[0];
    assert_eq!(foo.package, "github.com/oakcask/foo");
    assert_eq!(
        foo.git
            .as_ref()
            .expect("foo should have a git source")
            .value,
        "https://example.invalid/foo.git"
    );
    assert_eq!(foo.selectors.len(), 1);
    assert_eq!(foo.selectors[0].kind, ManifestDependencySelectorKind::Tag);
    assert_eq!(foo.selectors[0].field.value, "v1.2.0");
    assert_eq!(foo.selectors[0].field.key_span.start.line, 3);
    assert!(foo.subdir.is_none());

    let bar = &manifest.dependencies[1];
    assert_eq!(
        bar.selectors[0].kind,
        ManifestDependencySelectorKind::Branch
    );
    assert_eq!(bar.selectors[0].field.value, "main");
    assert_eq!(
        bar.subdir.as_ref().expect("bar should have a subdir").value,
        "packages/bar"
    );

    let baz = &manifest.dependencies[2];
    assert_eq!(baz.selectors[0].kind, ManifestDependencySelectorKind::Rev);
    assert_eq!(baz.selectors[0].field.value, "0123456789abcdef");
}

#[test]
fn read_manifest_tracks_vendor_dependency_metadata() {
    let temp = TempProject::new("manifest-vendor-dependencies");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/vendor-lib\"]\n",
            "vendor = \"vendor/vendor-lib\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    let dependency = &manifest.dependencies[0];
    assert_eq!(dependency.package, "github.com/oakcask/vendor-lib");
    let vendor = dependency
        .vendor
        .as_ref()
        .expect("dependency should have a vendor source");
    assert_eq!(vendor.key, "vendor");
    assert_eq!(vendor.value, "vendor/vendor-lib");
    assert_eq!(vendor.value_span.start.line, 2);
    assert_eq!(vendor.value_span.start.column, 11);
}

#[test]
fn read_manifest_tracks_mirror_dependency_metadata() {
    let temp = TempProject::new("manifest-mirror-dependencies");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/mirror-lib\"]\n",
            "mirror = \"mirror/github.com/oakcask/mirror-lib\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    let dependency = &manifest.dependencies[0];
    assert_eq!(dependency.package, "github.com/oakcask/mirror-lib");
    let mirror = dependency
        .mirror
        .as_ref()
        .expect("dependency should have a mirror source");
    assert_eq!(mirror.key, "mirror");
    assert_eq!(mirror.value, "mirror/github.com/oakcask/mirror-lib");
    assert_eq!(mirror.key_span.start.line, 2);
    assert_eq!(mirror.value_span.start.column, 11);
}

#[test]
fn lockfile_package_records_identity_separately_from_git_source() {
    let lockfile = ProjectLockfile {
        packages: vec![LockfilePackage {
            name: "github.com/oakcask/bar".to_string(),
            source: LockfileSource::Git {
                url: "https://example.invalid/mono.git".to_string(),
                selector: LockfileGitSelector::Branch("main".to_string()),
                rev: "0123456789abcdef".to_string(),
                subdir: Some("packages/bar".to_string()),
            },
            checksum: "sha256:source-tree".to_string(),
        }],
    };

    let package = &lockfile.packages[0];
    assert_eq!(package.name, "github.com/oakcask/bar");
    assert_eq!(package.checksum, "sha256:source-tree");
    assert_eq!(
        package.source,
        LockfileSource::Git {
            url: "https://example.invalid/mono.git".to_string(),
            selector: LockfileGitSelector::Branch("main".to_string()),
            rev: "0123456789abcdef".to_string(),
            subdir: Some("packages/bar".to_string()),
        }
    );
}

#[test]
fn lockfile_package_records_identity_separately_from_vendor_source() {
    let lockfile = ProjectLockfile {
        packages: vec![LockfilePackage {
            name: "github.com/oakcask/vendor-lib".to_string(),
            source: LockfileSource::Vendor {
                path: "vendor/vendor-lib".to_string(),
            },
            checksum: "sha256:source-tree".to_string(),
        }],
    };

    assert_eq!(
        lockfile.render(),
        concat!(
            "[[package]]\n",
            "name = \"github.com/oakcask/vendor-lib\"\n",
            "source = { kind = \"vendor\", path = \"vendor/vendor-lib\" }\n",
            "checksum = \"sha256:source-tree\"\n",
        )
    );
}

#[test]
fn lockfile_package_records_identity_separately_from_mirror_source() {
    let lockfile = ProjectLockfile {
        packages: vec![LockfilePackage {
            name: "github.com/oakcask/mirror-lib".to_string(),
            source: LockfileSource::Mirror {
                path: "mirror/github.com/oakcask/mirror-lib".to_string(),
            },
            checksum: "sha256:source-tree".to_string(),
        }],
    };

    assert_eq!(
        lockfile.render(),
        concat!(
            "[[package]]\n",
            "name = \"github.com/oakcask/mirror-lib\"\n",
            "source = { kind = \"mirror\", path = \"mirror/github.com/oakcask/mirror-lib\" }\n",
            "checksum = \"sha256:source-tree\"\n",
        )
    );
}

#[test]
fn lockfile_render_sorts_packages_and_normalizes_path_source_records() {
    let lockfile = ProjectLockfile {
        packages: vec![
            LockfilePackage {
                name: "github.com/oakcask/zeta".to_string(),
                source: LockfileSource::Path {
                    path: "vendor/zeta".to_string(),
                },
                checksum: "sha256:zeta".to_string(),
            },
            LockfilePackage {
                name: "github.com/oakcask/alpha".to_string(),
                source: LockfileSource::Path {
                    path: normalize_lockfile_path("vendor\\alpha"),
                },
                checksum: "sha256:alpha".to_string(),
            },
            LockfilePackage {
                name: "github.com/oakcask/vendor-lib".to_string(),
                source: LockfileSource::Vendor {
                    path: normalize_lockfile_path("vendor\\vendor-lib"),
                },
                checksum: "sha256:vendor".to_string(),
            },
        ],
    };

    assert_eq!(
        lockfile.render(),
        concat!(
            "[[package]]\n",
            "name = \"github.com/oakcask/alpha\"\n",
            "source = { kind = \"path\", path = \"vendor/alpha\" }\n",
            "checksum = \"sha256:alpha\"\n",
            "\n",
            "[[package]]\n",
            "name = \"github.com/oakcask/vendor-lib\"\n",
            "source = { kind = \"vendor\", path = \"vendor/vendor-lib\" }\n",
            "checksum = \"sha256:vendor\"\n",
            "\n",
            "[[package]]\n",
            "name = \"github.com/oakcask/zeta\"\n",
            "source = { kind = \"path\", path = \"vendor/zeta\" }\n",
            "checksum = \"sha256:zeta\"\n",
        )
    );
}

#[test]
fn source_tree_checksum_tracks_owned_source_paths_including_target() {
    let base = TempProject::new("lockfile-checksum-base");
    base.write("alpha.veln", "fn alpha() -> Int\n\t1\nend\n");
    base.write("nested/beta.veln", "fn beta() -> Int\n\t2\nend\n");
    base.write("target/generated.veln", "owned");

    let same_without_build_output = TempProject::new("lockfile-checksum-same");
    same_without_build_output.write("alpha.veln", "fn alpha() -> Int\n\t1\nend\n");
    same_without_build_output.write("nested/beta.veln", "fn beta() -> Int\n\t2\nend\n");

    let changed_contents = TempProject::new("lockfile-checksum-contents");
    changed_contents.write("alpha.veln", "fn alpha() -> Int\n\t9\nend\n");
    changed_contents.write("nested/beta.veln", "fn beta() -> Int\n\t2\nend\n");

    let changed_path = TempProject::new("lockfile-checksum-path");
    changed_path.write("alpha.veln", "fn alpha() -> Int\n\t1\nend\n");
    changed_path.write("renamed/beta.veln", "fn beta() -> Int\n\t2\nend\n");

    let base_checksum = source_tree_checksum(base.root()).expect("checksum should be computed");
    assert_ne!(
        base_checksum,
        source_tree_checksum(same_without_build_output.root())
            .expect("an owned target source should affect the checksum")
    );
    assert_ne!(
        base_checksum,
        source_tree_checksum(changed_contents.root()).expect("checksum should change")
    );
    assert_ne!(
        base_checksum,
        source_tree_checksum(changed_path.root()).expect("checksum should change")
    );
    assert!(base_checksum.starts_with("sha256:"));
    assert_eq!(base_checksum.len(), "sha256:".len() + 64);
}

#[test]
fn source_tree_checksum_ignores_changes_below_nested_manifest_roots() {
    let base = TempProject::new("lockfile-checksum-boundary-base");
    base.write("alpha.veln", "fn alpha() -> Int\n\t1\nend\n");
    base.write("nested/veln.toml", "[package]\nname = \"nested\"\n");
    base.write("nested/source.veln", "fn nested() -> Int\n\t1\nend\n");

    let changed_nested = TempProject::new("lockfile-checksum-boundary-changed");
    changed_nested.write("alpha.veln", "fn alpha() -> Int\n\t1\nend\n");
    changed_nested.write("nested/veln.toml", "malformed but still a boundary");
    changed_nested.write("nested/source.veln", "fn nested() -> Int\n\t999\nend\n");
    changed_nested.write("nested/added.veln", "fn added() -> Int\n\t2\nend\n");

    assert_eq!(
        source_tree_checksum(base.root()).expect("checksum should be computed"),
        source_tree_checksum(changed_nested.root())
            .expect("nested package changes should not affect the outer checksum")
    );
}

#[test]
fn read_manifest_accepts_crlf_export_arrays_and_trailing_text() {
    let temp = TempProject::new("manifest-export-crlf");
    temp.write(
        "veln.toml",
        "[lib]\r\n  exports = [\"src/main.veln\"] # owner note\r\n",
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 2);
    assert_eq!(manifest.lib.exports[0].path_span.start.column, 15);
}

#[test]
fn read_manifest_accepts_final_export_without_newline() {
    let temp = TempProject::new("manifest-final-export");
    temp.write("veln.toml", "[lib]\nexports = [\"src/main.veln\"]");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 2);
    assert_eq!(manifest.lib.exports[0].path_span.start.column, 13);
}

#[test]
fn read_manifest_tracks_export_path_span_ends() {
    let temp = TempProject::new("manifest-export-span-ends");
    temp.write("veln.toml", "[lib]\n  exports = [\"src/main.veln\"]\n");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    let export = &manifest.lib.exports[0];
    assert_eq!(export.path_span.start.line, 2);
    assert_eq!(export.path_span.start.column, 15);
    assert_eq!(export.path_span.end.line, 2);
    assert_eq!(export.path_span.end.column, 28);
}

#[test]
fn read_manifest_tracks_empty_export_path_span() {
    let temp = TempProject::new("manifest-empty-export-path");
    temp.write("veln.toml", "[lib]\nexports = [\"\"]\n");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    let export = &manifest.lib.exports[0];
    assert_eq!(export.path, "");
    assert_eq!(export.path_span.start.line, 2);
    assert_eq!(export.path_span.start.column, 13);
    assert_eq!(export.path_span.end.line, 2);
    assert_eq!(export.path_span.end.column, 13);
}

#[test]
fn read_manifest_accepts_empty_exports_array() {
    let temp = TempProject::new("manifest-empty-exports");
    temp.write("veln.toml", "[lib]\nexports = []\n");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert!(manifest.lib.exports.is_empty());
}

#[test]
fn read_manifest_tracks_modules_as_unsupported_section() {
    let temp = TempProject::new("manifest-unsupported-modules");
    temp.write("veln.toml", "[modules]\n\"main.veln\" = \"app.main\"\n");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert!(manifest.lib.exports.is_empty());
    assert_eq!(manifest.unsupported_sections.len(), 1);
    assert_eq!(manifest.unsupported_sections[0].name, "modules");
    assert_eq!(manifest.unsupported_sections[0].span.start.line, 1);
    assert_eq!(manifest.unsupported_sections[0].span.start.column, 2);
}

#[test]
fn read_manifest_accepts_modules_header_without_entries_as_unsupported() {
    let temp = TempProject::new("manifest-empty-unsupported-modules");
    temp.write("veln.toml", "[modules]");

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert!(manifest.lib.exports.is_empty());
    assert_eq!(manifest.unsupported_sections.len(), 1);
}

#[test]
fn read_manifest_accepts_trailing_text_after_section_headers() {
    let temp = TempProject::new("manifest-section-header-trailing-text");
    temp.write(
        "veln.toml",
        concat!(
            "[package] # ignored section\n",
            "\"ignored.veln\" = \"ignored.module\"\n",
            "[lib] # source exports\n",
            "exports = [\"src/main.veln\"]\n",
            "[other] # ignored again\n",
            "\"ignored-again.veln\" = \"ignored.again\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/main.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 4);
}

#[test]
fn read_manifest_ignores_malformed_export_entries() {
    let temp = TempProject::new("manifest-malformed-exports");
    temp.write(
        "veln.toml",
        concat!(
            "[lib]\n",
            "exports = [\n",
            "src/main.veln,\n",
            "\"src/unclosed.veln,\n",
            "\"src/lib.veln\",\n",
            "]\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(manifest.lib.exports.len(), 1);
    assert_eq!(manifest.lib.exports[0].path, "src/lib.veln");
    assert_eq!(manifest.lib.exports[0].path_span.start.line, 5);
}

struct TempProject {
    root: PathBuf,
}

impl TempProject {
    fn new(name: &str) -> Self {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "veln-project-test-{name}-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir_all(&root).unwrap();
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    fn write(&self, path: &str, contents: &str) {
        let path = self.path(path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, contents).unwrap();
    }
}

impl Drop for TempProject {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
