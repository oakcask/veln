use super::*;

#[test]
fn package_root_selection_uses_nearest_manifest_or_resolved_start() {
    let temp = TempProject::new("package-root-selection");
    temp.write("veln.toml", "[package]\nname = \"outer\"\n");
    temp.write("nested/veln.toml", "[package]\nname = \"nested\"\n");
    fs::create_dir_all(temp.path("nested/deep")).unwrap();
    fs::create_dir_all(temp.path("anonymous/deep")).unwrap();

    assert_eq!(
        select_package_root(&temp.path("nested/deep")).unwrap(),
        temp.path("nested").canonicalize().unwrap()
    );

    let anonymous = TempProject::new("anonymous-package-root-selection");
    fs::create_dir_all(anonymous.path("deep")).unwrap();
    assert_eq!(
        select_package_root(&anonymous.path("deep")).unwrap(),
        anonymous.path("deep").canonicalize().unwrap()
    );
}

#[cfg(unix)]
#[test]
fn package_root_selection_resolves_symbolic_analysis_starts() {
    use std::os::unix::fs::symlink;

    let temp = TempProject::new("package-root-symlink-start");
    temp.write("package/veln.toml", "");
    fs::create_dir_all(temp.path("package/src")).unwrap();
    symlink(temp.path("package/src"), temp.path("linked-start")).unwrap();

    assert_eq!(
        select_package_root(&temp.path("linked-start")).unwrap(),
        select_package_root(&temp.path("package/src")).unwrap()
    );
}

#[cfg(unix)]
#[test]
fn package_root_selection_ignores_non_regular_markers() {
    use std::os::unix::fs::symlink;

    let temp = TempProject::new("package-root-marker-kind");
    temp.write("veln.toml", "");
    fs::create_dir_all(temp.path("directory-marker/veln.toml")).unwrap();
    fs::create_dir_all(temp.path("symlink-marker")).unwrap();
    symlink(
        temp.path("veln.toml"),
        temp.path("symlink-marker/veln.toml"),
    )
    .unwrap();

    assert_eq!(
        select_package_root(&temp.path("directory-marker")).unwrap(),
        temp.root().canonicalize().unwrap()
    );
    assert_eq!(
        select_package_root(&temp.path("symlink-marker")).unwrap(),
        temp.root().canonicalize().unwrap()
    );
    assert!(
        read_manifest(&temp.path("symlink-marker"))
            .unwrap()
            .is_none()
    );
}

#[test]
fn package_root_selection_stops_at_marker_classification_failure() {
    let temp = TempProject::new("package-root-classification-failure");
    temp.write("veln.toml", "");
    fs::create_dir_all(temp.path("nested")).unwrap();
    let start = temp.path("nested").canonicalize().unwrap();
    let failed_marker = start.join("veln.toml");

    let error = select_package_root_with(start, |marker| {
        if marker == failed_marker {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "injected marker classification failure",
            ))
        } else {
            fs::symlink_metadata(marker)
        }
    })
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
    assert!(error.to_string().contains("injected marker"));
}

#[cfg(unix)]
#[test]
fn selected_unreadable_manifest_fails_when_project_loads() {
    use std::os::unix::fs::PermissionsExt;

    let temp = TempProject::new("selected-unreadable-manifest");
    temp.write("veln.toml", "");
    fs::create_dir_all(temp.path("src")).unwrap();
    let marker = temp.path("veln.toml");
    let original = fs::metadata(&marker).unwrap().permissions();
    fs::set_permissions(&marker, fs::Permissions::from_mode(0o000)).unwrap();

    let root = select_package_root(&temp.path("src")).unwrap();
    let result = Project::discover(root, &[]);

    fs::set_permissions(&marker, original).unwrap();
    if !nix_like_effective_root() {
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
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
