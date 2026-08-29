use super::*;

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

    let absolute_error =
        discover_source_paths(temp.root(), std::slice::from_ref(&outside)).unwrap_err();
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
