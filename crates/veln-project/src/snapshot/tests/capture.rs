use super::*;

#[test]
fn package_snapshot_capture_owns_the_distribution_set_in_path_order() {
    let package = SnapshotFixture::new("distribution-set");
    package.write_bytes("veln.toml", b"manifest\r\n\xff");
    let cases = [
        ("z.veln", b"public".as_slice(), true),
        ("src/private.veln", b"private", true),
        ("generated/api.veln", b"generated", true),
        ("target/code.veln", b"target", true),
        ("src/unit.test.veln", b"companion", false),
        ("tests/api_test.veln", b"integration", false),
        ("notes.txt", b"other", false),
        ("nested/.git/hidden.veln", b"git", false),
        ("dependency/owned.veln", b"descendant", false),
    ];
    for (path, bytes, _) in cases {
        package.write_bytes(path, bytes);
    }
    package.write_bytes("dependency/veln.toml", b"nested manifest");

    let snapshot = capture_package_snapshot(package.root()).unwrap();
    let actual = snapshot
        .sources()
        .iter()
        .map(|source| (source.path(), source.bytes()))
        .collect::<Vec<_>>();

    assert_eq!(snapshot.manifest_bytes(), b"manifest\r\n\xff");
    assert_eq!(
        actual,
        vec![
            ("generated/api.veln", b"generated".as_slice()),
            ("src/private.veln", b"private".as_slice()),
            ("target/code.veln", b"target".as_slice()),
            ("z.veln", b"public".as_slice()),
        ]
    );
    assert!(snapshot.sources().iter().all(|source| {
        cases.iter().any(|(path, bytes, included)| {
            *included && *path == source.path() && *bytes == source.bytes()
        })
    }));
}

#[test]
fn embedded_snapshot_matches_filesystem_snapshot_without_materialization() {
    let package = SnapshotFixture::new("embedded-equivalence");
    package.write_bytes("veln.toml", b"[package]\nname = \"std\"\n");
    package.write_bytes("nested/b.veln", b"pub fn b() -> Int\n  2\nend\n");
    package.write_bytes("a.veln", b"pub fn a() -> Int\r\n  1\r\nend\r\n");
    let filesystem = capture_package_snapshot(package.root()).unwrap();
    let embedded = capture_embedded_package_snapshot(
        filesystem.manifest_bytes(),
        filesystem
            .sources()
            .iter()
            .rev()
            .map(|source| PackageSnapshotSource::new(source.path(), source.bytes())),
    )
    .unwrap();

    assert_eq!(embedded, filesystem);
}

#[test]
fn embedded_snapshot_applies_distribution_test_source_exclusions() {
    let snapshot = capture_embedded_package_snapshot(
        b"[package]\nname = \"demo\"\n",
        [
            PackageSnapshotSource::new("main.veln", b"pub fn main() -> Int\n\t1\nend\n"),
            PackageSnapshotSource::new("main.test.veln", b"not parsed"),
            PackageSnapshotSource::new("main_test.veln", b"not parsed"),
            PackageSnapshotSource::new("notes.txt", b"not captured"),
        ],
    )
    .unwrap();

    assert_eq!(snapshot.sources().len(), 1);
    assert_eq!(snapshot.sources()[0].path(), "main.veln");
}

#[test]
fn embedded_snapshot_reuses_portable_source_validation() {
    let error = capture_embedded_package_snapshot(
        b"manifest",
        [PackageSnapshotSource::new("dir/../main.veln", b"main\n")],
    )
    .unwrap_err();

    assert!(matches!(
        error,
        PackageSnapshotCaptureError::InvalidSourcePath { path, .. }
            if path == "dir/../main.veln"
    ));
}

#[test]
fn package_snapshot_capture_digest_uses_the_retained_exact_bytes() {
    let package = SnapshotFixture::new("digest-integration");
    package.write_bytes("veln.toml", b"manifest\0bytes");
    package.write_bytes("src/b.veln", b"b\r\n");
    package.write_bytes("src/a.veln", "a\0λ".as_bytes());

    let snapshot = capture_package_snapshot(package.root()).unwrap();
    let digest_sources = snapshot
        .sources()
        .iter()
        .map(|source| PackageSnapshotSource::new(source.path(), source.bytes()))
        .collect::<Vec<_>>();

    assert_eq!(
        snapshot.digest(),
        package_snapshot_digest(snapshot.manifest_bytes(), &digest_sources).unwrap()
    );

    let original = snapshot.digest().to_string();
    package.write_bytes("veln.toml", b"manifest\0byteS");
    assert_ne!(
        capture_package_snapshot(package.root()).unwrap().digest(),
        original
    );
    package.write_bytes("veln.toml", b"manifest\0bytes");
    package.write_bytes("src/a.veln", "A\0λ".as_bytes());
    assert_ne!(
        capture_package_snapshot(package.root()).unwrap().digest(),
        original
    );
    package.write_bytes("src/a.veln", "a\0λ".as_bytes());
    package.write_bytes("src/new.veln", b"new");
    assert_ne!(
        capture_package_snapshot(package.root()).unwrap().digest(),
        original
    );
}

#[test]
fn package_snapshot_capture_digest_is_independent_of_physical_parent() {
    let first = SnapshotFixture::new("relocation-first");
    let second = SnapshotFixture::new("relocation-second");
    for package in [&first, &second] {
        package.write_bytes("veln.toml", b"same manifest");
        package.write_bytes("src/main.veln", b"same source");
    }

    assert_ne!(first.root(), second.root());
    assert_eq!(
        capture_package_snapshot(first.root()).unwrap(),
        capture_package_snapshot(second.root()).unwrap()
    );
}

#[cfg(unix)]
#[test]
fn package_snapshot_capture_excludes_all_symbolic_links() {
    use std::os::unix::fs::symlink;

    let package = SnapshotFixture::new("symbolic-links");
    package.write_bytes("veln.toml", b"manifest");
    package.write_bytes("real.veln", b"real");
    package.write_bytes("ignored.txt", b"\xff");
    package.write_bytes("outside/linked.veln", b"outside");
    fs::create_dir_all(package.path("links")).unwrap();
    symlink(package.path("real.veln"), package.path("linked.veln")).unwrap();
    symlink(package.path("ignored.txt"), package.path("invalid.veln")).unwrap();
    symlink(package.path("outside"), package.path("links/directory")).unwrap();

    let snapshot = capture_package_snapshot(package.root()).unwrap();
    let paths = snapshot
        .sources()
        .iter()
        .map(CapturedPackageSource::path)
        .collect::<Vec<_>>();
    assert_eq!(paths, vec!["outside/linked.veln", "real.veln"]);
}
