use super::*;

#[test]
fn package_snapshot_capture_requires_a_regular_manifest() {
    let missing = SnapshotFixture::new("missing-manifest");
    assert!(matches!(
        capture_package_snapshot(missing.root()),
        Err(PackageSnapshotCaptureError::ManifestNotRegular(_))
    ));

    let directory = SnapshotFixture::new("directory-manifest");
    fs::create_dir_all(directory.path("veln.toml")).unwrap();
    assert!(matches!(
        capture_package_snapshot(directory.root()),
        Err(PackageSnapshotCaptureError::ManifestNotRegular(_))
    ));
}

#[cfg(unix)]
#[test]
fn package_snapshot_capture_rejects_a_symbolic_manifest() {
    use std::os::unix::fs::symlink;

    let package = SnapshotFixture::new("symbolic-manifest");
    package.write_bytes("manifest-target", b"manifest");
    symlink(package.path("manifest-target"), package.path("veln.toml")).unwrap();

    assert!(matches!(
        capture_package_snapshot(package.root()),
        Err(PackageSnapshotCaptureError::ManifestNotRegular(_))
    ));
}

#[cfg(unix)]
#[test]
fn package_snapshot_capture_rejects_unrepresentable_entry_paths() {
    use std::os::unix::ffi::OsStringExt;

    let package = SnapshotFixture::new("non-utf8-path");
    package.write_bytes("veln.toml", b"manifest");
    let invalid_name = std::ffi::OsString::from_vec(b"invalid-\xff.veln".to_vec());
    fs::write(package.root().join(invalid_name), b"source").unwrap();

    assert!(matches!(
        capture_package_snapshot(package.root()),
        Err(PackageSnapshotCaptureError::UnrepresentablePath(_))
    ));
}

#[cfg(unix)]
#[test]
fn package_snapshot_capture_excludes_unrepresentable_symbolic_links() {
    use std::os::unix::ffi::OsStringExt;
    use std::os::unix::fs::symlink;

    let package = SnapshotFixture::new("non-utf8-symlink");
    package.write_bytes("veln.toml", b"manifest");
    package.write_bytes("real.veln", b"real");
    let invalid_link = std::ffi::OsString::from_vec(b"invalid-\xff.veln".to_vec());
    symlink(package.path("real.veln"), package.root().join(invalid_link)).unwrap();

    let snapshot = capture_package_snapshot(package.root()).unwrap();
    let paths = snapshot
        .sources()
        .iter()
        .map(CapturedPackageSource::path)
        .collect::<Vec<_>>();
    assert_eq!(paths, vec!["real.veln"]);
}

#[cfg(unix)]
#[test]
fn package_snapshot_capture_excludes_unrepresentable_descendant_packages() {
    use std::os::unix::ffi::OsStringExt;

    let package = SnapshotFixture::new("non-utf8-descendant");
    package.write_bytes("veln.toml", b"manifest");
    package.write_bytes("outer.veln", b"outer");
    let invalid_dir = std::ffi::OsString::from_vec(b"dependency-\xff".to_vec());
    let nested_root = package.root().join(invalid_dir);
    fs::create_dir_all(&nested_root).unwrap();
    fs::write(nested_root.join("veln.toml"), b"nested manifest").unwrap();
    fs::write(nested_root.join("inner.veln"), b"inner").unwrap();

    let snapshot = capture_package_snapshot(package.root()).unwrap();
    let paths = snapshot
        .sources()
        .iter()
        .map(CapturedPackageSource::path)
        .collect::<Vec<_>>();
    assert_eq!(paths, vec!["outer.veln"]);
}

#[cfg(unix)]
#[test]
fn package_snapshot_capture_rejects_non_regular_distribution_sources() {
    let package = SnapshotFixture::new("non-regular-source");
    package.write_bytes("veln.toml", b"manifest");
    let fifo_path = package.path("fifo.veln");
    package.mkfifo("fifo.veln");

    assert!(matches!(
        capture_package_snapshot(package.root()),
        Err(PackageSnapshotCaptureError::SourceNotRegular(path)) if path == fifo_path
    ));
}

#[cfg(unix)]
#[test]
fn package_snapshot_capture_ignores_non_regular_non_sources() {
    let package = SnapshotFixture::new("non-regular-non-source");
    package.write_bytes("veln.toml", b"manifest");
    package.write_bytes("source.veln", b"source");
    package.mkfifo("fifo.txt");

    let snapshot = capture_package_snapshot(package.root()).unwrap();
    let paths = snapshot
        .sources()
        .iter()
        .map(CapturedPackageSource::path)
        .collect::<Vec<_>>();
    assert_eq!(paths, vec!["source.veln"]);
}

#[cfg(target_os = "linux")]
#[test]
fn package_snapshot_capture_rejects_nonportable_source_paths() {
    let cases = [
        (
            "non-nfc",
            "cafe\u{301}.veln",
            PortableSourcePathError::NotNfc,
        ),
        (
            "control",
            "line\nfeed.veln",
            PortableSourcePathError::Control {
                segment_index: 0,
                character: '\n',
            },
        ),
        (
            "backslash",
            "dir\\source.veln",
            PortableSourcePathError::ForbiddenCharacter {
                segment_index: 0,
                character: '\\',
            },
        ),
        (
            "colon",
            "device:source.veln",
            PortableSourcePathError::ForbiddenCharacter {
                segment_index: 0,
                character: ':',
            },
        ),
        (
            "trailing-space",
            "dir /source.veln",
            PortableSourcePathError::TrailingSpace { segment_index: 0 },
        ),
        (
            "trailing-dot",
            "dir./source.veln",
            PortableSourcePathError::TrailingDot { segment_index: 0 },
        ),
        (
            "reserved-device",
            "NUL.veln",
            PortableSourcePathError::ReservedDevice { segment_index: 0 },
        ),
        (
            "reserved-device-stem-space",
            "NUL .veln",
            PortableSourcePathError::ReservedDevice { segment_index: 0 },
        ),
    ];

    for (label, path, reason) in cases {
        let package = SnapshotFixture::new(label);
        package.write_bytes("veln.toml", b"manifest");
        package.write_bytes(path, b"source");
        assert!(matches!(
            capture_package_snapshot(package.root()),
            Err(PackageSnapshotCaptureError::InvalidSourcePath {
                path: actual_path,
                reason: actual_reason,
            }) if actual_path == path && actual_reason == reason
        ));
    }
}

#[cfg(target_os = "linux")]
#[test]
fn package_snapshot_capture_reports_portable_failures_in_path_order() {
    let package = SnapshotFixture::new("portable-error-order");
    package.write_bytes("veln.toml", b"manifest");
    package.write_bytes("z:bad.veln", b"source");
    package.write_bytes("a:bad.veln", b"source");

    assert!(matches!(
        capture_package_snapshot(package.root()),
        Err(PackageSnapshotCaptureError::InvalidSourcePath { path, .. })
            if path == "a:bad.veln"
    ));
}

#[test]
fn package_snapshot_capture_rejects_non_utf8_source_text() {
    let package = SnapshotFixture::new("non-utf8-source-text");
    package.write_bytes("veln.toml", b"manifest\xff");
    package.write_bytes("src/main.veln", b"valid\xff");

    assert!(matches!(
        capture_package_snapshot(package.root()),
        Err(PackageSnapshotCaptureError::InvalidSourceText {
            path,
            valid_up_to: 5,
        }) if path == "src/main.veln"
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn package_snapshot_capture_rejects_default_case_fold_collisions() {
    let package = SnapshotFixture::new("case-fold-collision");
    package.write_bytes("veln.toml", b"manifest");
    package.write_bytes("Straße.veln", b"first");
    package.write_bytes("STRASSE.veln", b"second");

    assert_eq!(
        capture_package_snapshot(package.root())
            .unwrap_err()
            .to_string(),
        "package snapshot source paths `STRASSE.veln` and `Straße.veln` collide after Unicode default case folding"
    );
    assert!(matches!(
        capture_package_snapshot(package.root()),
        Err(PackageSnapshotCaptureError::SourcePathCollision { first, second })
            if first == "STRASSE.veln" && second == "Straße.veln"
    ));
}

#[cfg(target_os = "linux")]
#[test]
fn package_snapshot_capture_rejects_three_character_case_fold_collisions() {
    let package = SnapshotFixture::new("case-fold-three-collision");
    package.write_bytes("veln.toml", b"manifest");
    package.write_bytes("ffi.veln", b"first");
    package.write_bytes("ﬃ.veln", b"second");

    assert_eq!(
        capture_package_snapshot(package.root())
            .unwrap_err()
            .to_string(),
        "package snapshot source paths `ffi.veln` and `ﬃ.veln` collide after Unicode default case folding"
    );
    assert!(matches!(
        capture_package_snapshot(package.root()),
        Err(PackageSnapshotCaptureError::SourcePathCollision { first, second })
            if first == "ffi.veln" && second == "ﬃ.veln"
    ));
}

#[test]
fn package_snapshot_capture_preserves_valid_unicode_and_source_bytes() {
    let package = SnapshotFixture::new("portable-exact-input");
    package.write_bytes("veln.toml", b"manifest\r\n\xff");
    package.write_bytes("src/café.veln", "λ\r\n".as_bytes());

    let snapshot = capture_package_snapshot(package.root()).unwrap();
    assert_eq!(snapshot.manifest_bytes(), b"manifest\r\n\xff");
    assert_eq!(snapshot.sources()[0].path(), "src/café.veln");
    assert_eq!(snapshot.sources()[0].bytes(), "λ\r\n".as_bytes());
}

#[cfg(target_os = "linux")]
#[test]
fn package_snapshot_capture_excludes_entries_before_portable_validation() {
    let package = SnapshotFixture::new("excluded-portability");
    package.write_bytes("veln.toml", b"manifest");
    package.write_bytes("kept.veln", b"kept");
    package.write_bytes("bad:/ignored.test.veln", b"\xff");
    package.write_bytes("bad./ignored_test.veln", b"\xff");
    package.write_bytes(".git/NUL.veln", b"\xff");
    package.write_bytes("dependency:/veln.toml", b"nested manifest");
    package.write_bytes("dependency:/NUL.veln", b"\xff");
    package.write_bytes_raw_relative(b"ignored-\xff.test.veln", b"\xff");
    package.write_bytes_raw_relative(b"ignored-\xff_test.veln", b"\xff");

    let snapshot = capture_package_snapshot(package.root()).unwrap();
    let paths = snapshot
        .sources()
        .iter()
        .map(CapturedPackageSource::path)
        .collect::<Vec<_>>();
    assert_eq!(paths, vec!["kept.veln"]);
}

#[cfg(unix)]
#[test]
fn unix_test_source_exclusion_uses_raw_path_bytes() {
    assert!(is_excluded_test_source_bytes(b"bad-\xff.test.veln"));
    assert!(is_excluded_test_source_bytes(b"bad-\xff_test.veln"));
    assert!(!is_excluded_test_source_bytes(b"bad-\xff.veln"));
}

#[cfg(windows)]
#[test]
fn windows_test_source_exclusion_uses_raw_wide_code_units() {
    let mut companion = "bad-".encode_utf16().collect::<Vec<_>>();
    companion.push(0xD800);
    companion.extend(".test.veln".encode_utf16());
    assert!(is_excluded_test_source_wide(&companion));

    let mut integration = "bad-".encode_utf16().collect::<Vec<_>>();
    integration.push(0xD800);
    integration.extend("_test.veln".encode_utf16());
    assert!(is_excluded_test_source_wide(&integration));

    let mut retained = "bad-".encode_utf16().collect::<Vec<_>>();
    retained.push(0xD800);
    retained.extend(".veln".encode_utf16());
    assert!(!is_excluded_test_source_wide(&retained));
}

#[cfg(windows)]
#[test]
fn package_snapshot_capture_excludes_ill_formed_utf16_test_sources() {
    let package = SnapshotFixture::new("ill-formed-utf16-excluded");
    package.write_bytes("veln.toml", b"manifest");
    package.write_bytes("kept.veln", b"kept");
    package.write_bytes_raw_wide_relative(&ill_formed_wide_name(".test.veln"), b"\xff");
    package.write_bytes_raw_wide_relative(&ill_formed_wide_name("_test.veln"), b"\xff");

    let snapshot = capture_package_snapshot(package.root()).unwrap();
    let paths = snapshot
        .sources()
        .iter()
        .map(CapturedPackageSource::path)
        .collect::<Vec<_>>();
    assert_eq!(paths, vec!["kept.veln"]);
}
