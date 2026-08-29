use super::*;

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
fn source_tree_checksum_uses_normalized_root_for_relative_paths() {
    let temp = TempProject::new("lockfile-checksum-normalized-root");
    temp.write("dep/alpha.veln", "fn alpha() -> Int\n\t1\nend\n");
    temp.write("dep/target/generated.veln", "owned");
    fs::create_dir_all(temp.path("through")).unwrap();

    assert_eq!(
        source_tree_checksum(&temp.path("dep")).expect("checksum should be computed"),
        source_tree_checksum(&temp.path("through/../dep"))
            .expect("lexically equivalent root should compute the same checksum")
    );
}
