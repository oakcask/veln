use super::super::*;
use super::TempProject;

#[test]
fn locks_vendor_dependency_package_root() {
    let project = TempProject::new("lock-vendor");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/vendor-lib\"]\n",
            "vendor = \"vendor/vendor-lib\"\n",
        ),
    );
    project.write(
        "vendor/vendor-lib/veln.toml",
        "[package]\nname = \"github.com/oakcask/vendor-lib\"\n",
    );
    project.write(
        "vendor/vendor-lib/vendor.veln",
        "pub fn vendor() -> Int\n\t4\nend\n",
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let package = lock_vendor_dependency(project.root(), &manifest.dependencies[0])
        .expect("vendor dependency should lock");

    assert_eq!(package.name, "github.com/oakcask/vendor-lib");
    assert_eq!(
        package.source,
        LockfileSource::Vendor {
            path: "vendor/vendor-lib".to_string(),
        }
    );
    assert_eq!(
        package.checksum,
        source_tree_checksum(&project.path("vendor/vendor-lib"))
            .expect("checksum should be computed")
    );
}

#[test]
fn package_lock_rejects_mixed_vendor_and_git_sources() {
    let project = TempProject::new("lock-mixed-vendor-git");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/mixed\"]\n",
            "vendor = \"vendor/mixed\"\n",
            "git = \"vendor/mixed.git\"\n",
            "rev = \"abc123\"\n",
        ),
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let diagnostic = lock_dependency(project.root(), &manifest.dependencies[0])
        .expect_err("mixed sources should fail");

    assert_eq!(diagnostic.id, "package.unsupported_dependency_source");
    assert_eq!(
        diagnostic.message,
        "package lock supports only one of path, git, vendor, or mirror dependencies for `github.com/oakcask/mixed`"
    );
}

#[test]
fn package_lock_rejects_dependency_identity_dot_segments() {
    let project = TempProject::new("lock-dot-segment-identity");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/../shared\"]\n",
            "path = \"vendor/shared\"\n",
        ),
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let mut resolver = PackageLockResolver::new(project.root());
    resolver.lock_manifest_dependencies(project.root(), "", &manifest);

    assert_eq!(resolver.diagnostics.len(), 1);
    assert_eq!(
        resolver.diagnostics[0].id,
        "package.invalid_dependency_identity"
    );
    assert_eq!(
        resolver.diagnostics[0].message,
        "dependency package identity `github.com/oakcask/../shared` is invalid"
    );
}

#[test]
fn package_lock_rejects_incompatible_transitive_sources() {
    let project = TempProject::new("lock-incompatible-transitive-source");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/alpha\"]\n",
            "path = \"vendor/alpha\"\n",
            "[dependencies.\"github.com/oakcask/zeta\"]\n",
            "path = \"vendor/zeta\"\n",
        ),
    );
    project.write(
        "vendor/alpha/veln.toml",
        concat!(
            "[package]\n",
            "name = \"github.com/oakcask/alpha\"\n",
            "[dependencies.\"github.com/oakcask/shared\"]\n",
            "path = \"vendor/shared-one\"\n",
        ),
    );
    project.write(
        "vendor/zeta/veln.toml",
        concat!(
            "[package]\n",
            "name = \"github.com/oakcask/zeta\"\n",
            "[dependencies.\"github.com/oakcask/shared\"]\n",
            "path = \"vendor/shared-two\"\n",
        ),
    );
    project.write(
        "vendor/alpha/vendor/shared-one/veln.toml",
        "[package]\nname = \"github.com/oakcask/shared\"\n",
    );
    project.write(
        "vendor/zeta/vendor/shared-two/veln.toml",
        "[package]\nname = \"github.com/oakcask/shared\"\n",
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let mut resolver = PackageLockResolver::new(project.root());
    resolver.lock_manifest_dependencies(project.root(), "", &manifest);

    assert_eq!(resolver.diagnostics.len(), 1);
    let diagnostic = &resolver.diagnostics[0];
    assert_eq!(diagnostic.id, "package.incompatible_dependency_source");
    assert_eq!(
        diagnostic.message,
        "dependency `github.com/oakcask/shared` selects path `vendor/zeta/vendor/shared-two`, but that package identity is already selected as path `vendor/alpha/vendor/shared-one`"
    );
    assert_eq!(
        diagnostic
            .span
            .as_ref()
            .expect("diagnostic should have a span")
            .file
            .as_str(),
        "vendor/zeta/veln.toml"
    );
    assert_eq!(resolver.packages().len(), 3);
}

#[test]
fn package_lock_rejects_incompatible_transitive_source_kinds() {
    let project = TempProject::new("lock-incompatible-transitive-source-kind");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/alpha\"]\n",
            "path = \"vendor/alpha\"\n",
            "[dependencies.\"github.com/oakcask/zeta\"]\n",
            "path = \"vendor/zeta\"\n",
        ),
    );
    project.write(
        "vendor/alpha/veln.toml",
        concat!(
            "[package]\n",
            "name = \"github.com/oakcask/alpha\"\n",
            "[dependencies.\"github.com/oakcask/shared\"]\n",
            "path = \"../shared\"\n",
        ),
    );
    project.write(
        "vendor/zeta/veln.toml",
        concat!(
            "[package]\n",
            "name = \"github.com/oakcask/zeta\"\n",
            "[dependencies.\"github.com/oakcask/shared\"]\n",
            "vendor = \"../shared\"\n",
        ),
    );
    project.write(
        "vendor/shared/veln.toml",
        "[package]\nname = \"github.com/oakcask/shared\"\n",
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let mut resolver = PackageLockResolver::new(project.root());
    resolver.lock_manifest_dependencies(project.root(), "", &manifest);

    assert_eq!(resolver.diagnostics.len(), 1);
    let diagnostic = &resolver.diagnostics[0];
    assert_eq!(diagnostic.id, "package.incompatible_dependency_source");
    assert_eq!(
        diagnostic.message,
        "dependency `github.com/oakcask/shared` selects vendor `vendor/shared`, but that package identity is already selected as path `vendor/shared`"
    );
    assert_eq!(
        diagnostic
            .span
            .as_ref()
            .expect("diagnostic should have a span")
            .file
            .as_str(),
        "vendor/zeta/veln.toml"
    );
}

#[test]
fn package_lock_reuses_compatible_transitive_sources() {
    let project = TempProject::new("lock-compatible-transitive-source");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/alpha\"]\n",
            "path = \"vendor/alpha\"\n",
            "[dependencies.\"github.com/oakcask/zeta\"]\n",
            "path = \"vendor/zeta\"\n",
        ),
    );
    project.write(
        "vendor/alpha/veln.toml",
        concat!(
            "[package]\n",
            "name = \"github.com/oakcask/alpha\"\n",
            "[dependencies.\"github.com/oakcask/shared\"]\n",
            "path = \"../shared\"\n",
        ),
    );
    project.write(
        "vendor/zeta/veln.toml",
        concat!(
            "[package]\n",
            "name = \"github.com/oakcask/zeta\"\n",
            "[dependencies.\"github.com/oakcask/shared\"]\n",
            "path = \"../shared\"\n",
        ),
    );
    project.write(
        "vendor/shared/veln.toml",
        "[package]\nname = \"github.com/oakcask/shared\"\n",
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let mut resolver = PackageLockResolver::new(project.root());
    resolver.lock_manifest_dependencies(project.root(), "", &manifest);

    assert!(resolver.diagnostics.is_empty());
    assert_eq!(
        resolver
            .packages()
            .into_iter()
            .map(|package| package.name)
            .collect::<Vec<_>>(),
        vec![
            "github.com/oakcask/alpha".to_string(),
            "github.com/oakcask/shared".to_string(),
            "github.com/oakcask/zeta".to_string(),
        ]
    );
}

#[test]
fn locks_mirror_dependency() {
    let project = TempProject::new("lock-mirror");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/mirror-lib\"]\n",
            "mirror = \"mirror/github.com/oakcask/mirror-lib\"\n",
        ),
    );
    project.write(
        "mirror/github.com/oakcask/mirror-lib/veln.toml",
        "[package]\nname = \"github.com/oakcask/mirror-lib\"\n",
    );
    project.write(
        "mirror/github.com/oakcask/mirror-lib/lib.veln",
        "pub fn value() -> Int\n\t5\nend\n",
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let package = lock_mirror_dependency(project.root(), &manifest.dependencies[0])
        .expect("mirror dependency should lock");

    assert_eq!(package.name, "github.com/oakcask/mirror-lib");
    assert_eq!(
        package.source,
        LockfileSource::Mirror {
            path: "mirror/github.com/oakcask/mirror-lib".to_string(),
        }
    );
    assert_eq!(
        package.checksum,
        source_tree_checksum(&project.path("mirror/github.com/oakcask/mirror-lib"))
            .expect("checksum should be computed")
    );
}
