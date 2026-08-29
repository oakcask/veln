use super::super::*;
use super::TempProject;

#[test]
fn locks_git_rev_dependency_with_subdir_package_root() {
    let project = TempProject::new("lock-git-rev-subdir");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/bar\"]\n",
            "git = \"vendor/mono\"\n",
            "rev = \"abc123\"\n",
            "subdir = \"packages/bar\"\n",
        ),
    );
    project.write(
        "vendor/mono/packages/bar/veln.toml",
        "[package]\nname = \"github.com/oakcask/bar\"\n",
    );
    project.write(
        "vendor/mono/packages/bar/bar.veln",
        "pub fn bar() -> Int\n\t7\nend\n",
    );
    project.write(
        "vendor/mono/ignored.veln",
        "fn ignored() -> Int\n\t9\nend\n",
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let package = lock_git_dependency_with(
        project.root(),
        &manifest.dependencies[0],
        |repo, selector| {
            assert_eq!(repo, project.path("vendor/mono").as_path());
            assert_eq!(selector.kind, ManifestDependencySelectorKind::Rev);
            assert_eq!(selector.field.value, "abc123");
            Ok("0123456789abcdef0123456789abcdef01234567".to_string())
        },
    )
    .expect("git dependency should lock");

    assert_eq!(package.name, "github.com/oakcask/bar");
    assert_eq!(
        package.source,
        LockfileSource::Git {
            url: "vendor/mono".to_string(),
            selector: LockfileGitSelector::Rev("abc123".to_string()),
            rev: "0123456789abcdef0123456789abcdef01234567".to_string(),
            subdir: Some("packages/bar".to_string()),
        }
    );
    assert_eq!(
        package.checksum,
        source_tree_checksum(&project.path("vendor/mono/packages/bar"))
            .expect("checksum should be computed")
    );
}

#[test]
fn locks_git_rev_dependency_from_local_file_url() {
    let project = TempProject::new("lock-git-rev-file-url");
    let repository = project.path("vendor/file repo");
    let url = format!("file://{}", repository.display()).replace(' ', "%20");
    project.write(
        "veln.toml",
        &format!(
            "[dependencies.\"github.com/oakcask/file\"]\n\
             git = \"{url}\"\n\
             rev = \"abc123\"\n"
        ),
    );
    project.write(
        "vendor/file repo/veln.toml",
        "[package]\nname = \"github.com/oakcask/file\"\n",
    );
    project.write(
        "vendor/file repo/file.veln",
        "pub fn file() -> Int\n\t1\nend\n",
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let package = lock_git_dependency_with(project.root(), &manifest.dependencies[0], |repo, _| {
        assert_eq!(repo, repository.as_path());
        Ok("fedcba9876543210fedcba9876543210fedcba98".to_string())
    })
    .expect("file URL git dependency should lock");

    assert_eq!(
        package.source,
        LockfileSource::Git {
            url,
            selector: LockfileGitSelector::Rev("abc123".to_string()),
            rev: "fedcba9876543210fedcba9876543210fedcba98".to_string(),
            subdir: None,
        }
    );
}

#[test]
fn locks_git_tag_dependency() {
    let project = TempProject::new("lock-git-tag");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/tagged\"]\n",
            "git = \"vendor/tagged\"\n",
            "tag = \"v1\"\n",
        ),
    );
    project.write(
        "vendor/tagged/veln.toml",
        "[package]\nname = \"github.com/oakcask/tagged\"\n",
    );
    project.write(
        "vendor/tagged/tagged.veln",
        "pub fn tagged() -> Int\n\t2\nend\n",
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let package = lock_git_dependency_with(
        project.root(),
        &manifest.dependencies[0],
        |repo, selector| {
            assert_eq!(repo, project.path("vendor/tagged").as_path());
            assert_eq!(selector.kind, ManifestDependencySelectorKind::Tag);
            assert_eq!(selector.field.value, "v1");
            Ok("1111111111111111111111111111111111111111".to_string())
        },
    )
    .expect("tag selector should lock");

    assert_eq!(
        package.source,
        LockfileSource::Git {
            url: "vendor/tagged".to_string(),
            selector: LockfileGitSelector::Tag("v1".to_string()),
            rev: "1111111111111111111111111111111111111111".to_string(),
            subdir: None,
        }
    );
}

#[test]
fn locks_git_branch_dependency() {
    let project = TempProject::new("lock-git-branch");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/branchy\"]\n",
            "git = \"vendor/branchy\"\n",
            "branch = \"main\"\n",
        ),
    );
    project.write(
        "vendor/branchy/veln.toml",
        "[package]\nname = \"github.com/oakcask/branchy\"\n",
    );
    project.write(
        "vendor/branchy/branchy.veln",
        "pub fn branchy() -> Int\n\t3\nend\n",
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let package = lock_git_dependency_with(
        project.root(),
        &manifest.dependencies[0],
        |repo, selector| {
            assert_eq!(repo, project.path("vendor/branchy").as_path());
            assert_eq!(selector.kind, ManifestDependencySelectorKind::Branch);
            assert_eq!(selector.field.value, "main");
            Ok("2222222222222222222222222222222222222222".to_string())
        },
    )
    .expect("branch selector should lock");

    assert_eq!(
        package.source,
        LockfileSource::Git {
            url: "vendor/branchy".to_string(),
            selector: LockfileGitSelector::Branch("main".to_string()),
            rev: "2222222222222222222222222222222222222222".to_string(),
            subdir: None,
        }
    );
}

#[test]
fn package_lock_rejects_multiple_git_selectors_before_resolving() {
    let project = TempProject::new("lock-git-multiple-selectors");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/multiple\"]\n",
            "git = \"vendor/multiple\"\n",
            "tag = \"v1\"\n",
            "branch = \"main\"\n",
        ),
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let diagnostic = lock_git_dependency_with(project.root(), &manifest.dependencies[0], |_, _| {
        panic!("multiple selectors should not resolve git")
    })
    .expect_err("multiple selectors should fail");

    assert_eq!(diagnostic.id, "package.unsupported_git_selector");
    assert_eq!(
        diagnostic.message,
        "package lock requires git dependency `github.com/oakcask/multiple` to specify exactly one selector: `rev`, `tag`, or `branch`"
    );
}

#[test]
fn git_selector_resolution_uses_selector_ref_namespace() {
    let project = TempProject::new("git-selector-revspec");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/rev\"]\n",
            "git = \"vendor/rev\"\n",
            "rev = \"abc123\"\n",
            "\n",
            "[dependencies.\"github.com/oakcask/tagged\"]\n",
            "git = \"vendor/tagged\"\n",
            "tag = \"v1\"\n",
            "\n",
            "[dependencies.\"github.com/oakcask/branchy\"]\n",
            "git = \"vendor/branchy\"\n",
            "branch = \"main\"\n",
        ),
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    assert_eq!(
        git_selector_revspec(&manifest.dependencies[0].selectors[0]),
        "abc123^{commit}"
    );
    assert_eq!(
        git_selector_revspec(&manifest.dependencies[1].selectors[0]),
        "refs/tags/v1^{commit}"
    );
    assert_eq!(
        git_selector_revspec(&manifest.dependencies[2].selectors[0]),
        "refs/heads/main^{commit}"
    );
}

#[test]
fn locks_non_local_git_url_by_materializing_source() {
    let project = TempProject::new("lock-git-non-local-url");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/bar\"]\n",
            "git = \"https://example.invalid/mono.git\"\n",
            "rev = \"abc123\"\n",
            "subdir = \"packages/bar\"\n",
        ),
    );

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let materialized_root = project.path(".veln/package/git/materialized");
    let package = lock_git_dependency_with_materializer(
        project.root(),
        &manifest.dependencies[0],
        |repo, selector| {
            assert_eq!(repo, materialized_root.as_path());
            assert_eq!(selector.kind, ManifestDependencySelectorKind::Rev);
            Ok("3333333333333333333333333333333333333333".to_string())
        },
        |root, git_field, selector| {
            assert_eq!(root, project.root());
            assert_eq!(git_field.value, "https://example.invalid/mono.git");
            assert_eq!(selector.field.value, "abc123");
            project.write(
                ".veln/package/git/materialized/packages/bar/veln.toml",
                "[package]\nname = \"github.com/oakcask/bar\"\n",
            );
            project.write(
                ".veln/package/git/materialized/packages/bar/bar.veln",
                "pub fn bar() -> Int\n\t7\nend\n",
            );
            Ok(materialized_root.clone())
        },
    )
    .expect("non-local git dependency should lock");

    assert_eq!(package.name, "github.com/oakcask/bar");
    assert_eq!(
        package.source,
        LockfileSource::Git {
            url: "https://example.invalid/mono.git".to_string(),
            selector: LockfileGitSelector::Rev("abc123".to_string()),
            rev: "3333333333333333333333333333333333333333".to_string(),
            subdir: Some("packages/bar".to_string()),
        }
    );
    assert_eq!(
        package.checksum,
        source_tree_checksum(&project.path(".veln/package/git/materialized/packages/bar"))
            .expect("checksum should be computed")
    );
}

#[test]
fn rejects_materialized_git_source_that_is_not_a_directory_before_resolving() {
    let project = TempProject::new("lock-git-materialized-file");
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/bar\"]\n",
            "git = \"https://example.invalid/mono.git\"\n",
            "rev = \"abc123\"\n",
        ),
    );
    project.write("materialized-source", "not a repository");

    let manifest = read_manifest(project.root())
        .expect("manifest read should succeed")
        .expect("manifest should exist");
    let diagnostic = lock_git_dependency_with_materializer(
        project.root(),
        &manifest.dependencies[0],
        |_, _| panic!("an invalid materialized source must not resolve a selector"),
        |_, _, _| Ok(project.path("materialized-source")),
    )
    .expect_err("a materialized file should not lock as a git dependency");

    assert_eq!(diagnostic.id, "package.git_unavailable");
    assert_eq!(
        diagnostic.message,
        "git dependency `github.com/oakcask/bar` is not available at `https://example.invalid/mono.git`"
    );
}
