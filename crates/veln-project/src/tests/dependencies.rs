use super::*;

#[test]
fn direct_local_source_tracks_path_vendor_and_mirror_dependencies() {
    let temp = TempProject::new("manifest-direct-local-source");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/path-lib\"]\n",
            "path = \"vendor/path-lib\"\n",
            "[dependencies.\"github.com/oakcask/vendor-lib\"]\n",
            "vendor = \"vendor/vendor-lib\"\n",
            "[dependencies.\"github.com/oakcask/mirror-lib\"]\n",
            "mirror = \"mirror/github.com/oakcask/mirror-lib\"\n",
            "[dependencies.\"github.com/oakcask/git-lib\"]\n",
            "git = \"https://example.invalid/git-lib.git\"\n",
            "rev = \"abc123\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(
        manifest.dependencies[0]
            .direct_local_source()
            .unwrap()
            .value,
        "vendor/path-lib"
    );
    assert_eq!(
        manifest.dependencies[1]
            .direct_local_source()
            .unwrap()
            .value,
        "vendor/vendor-lib"
    );
    assert_eq!(
        manifest.dependencies[2]
            .direct_local_source()
            .unwrap()
            .value,
        "mirror/github.com/oakcask/mirror-lib"
    );
    assert!(manifest.dependencies[3].direct_local_source().is_none());
}

#[test]
fn direct_analysis_source_root_includes_git_subdir() {
    let temp = TempProject::new("manifest-direct-analysis-source");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/path-lib\"]\n",
            "path = \"vendor/path-lib\"\n",
            "[dependencies.\"github.com/oakcask/git-lib\"]\n",
            "git = \"materialized/mono\"\n",
            "rev = \"abc123\"\n",
            "subdir = \"packages/git-lib\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(
        manifest.dependencies[0].direct_analysis_source_root(temp.root()),
        Ok(Some(temp.path("vendor/path-lib")))
    );
    assert_eq!(
        manifest.dependencies[1].direct_analysis_source_root(temp.root()),
        Ok(Some(temp.path("materialized/mono/packages/git-lib")))
    );
}

#[test]
fn direct_analysis_source_root_resolves_local_file_git_url() {
    let temp = TempProject::new("manifest-direct-analysis-file-url");
    let repository = temp.path("materialized/file repo");
    let url = format!("file://{}", repository.display()).replace(' ', "%20");
    temp.write(
        "veln.toml",
        &format!(
            "[dependencies.\"github.com/oakcask/git-lib\"]\n\
             git = \"{url}\"\n\
             rev = \"abc123\"\n\
             subdir = \"packages/git-lib\"\n"
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(
        manifest.dependencies[0].direct_analysis_source_root(temp.root()),
        Ok(Some(repository.join("packages/git-lib")))
    );
}

#[test]
fn direct_analysis_source_root_resolves_materialized_remote_git_url() {
    let temp = TempProject::new("manifest-direct-analysis-remote-url");
    let url = "https://example.invalid/mono.git";
    let repository = materialized_git_repository_root(temp.root(), url);
    fs::create_dir_all(&repository).unwrap();
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/git-lib\"]\n",
            "git = \"https://example.invalid/mono.git\"\n",
            "rev = \"abc123\"\n",
            "subdir = \"packages/git-lib\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(
        manifest.dependencies[0].direct_analysis_source_root(temp.root()),
        Ok(Some(repository.join("packages/git-lib")))
    );
}

#[test]
fn direct_analysis_source_root_rejects_unselected_git_dependency() {
    let temp = TempProject::new("manifest-direct-analysis-git-selector");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/missing\"]\n",
            "git = \"materialized/missing\"\n",
            "[dependencies.\"github.com/oakcask/multiple\"]\n",
            "git = \"materialized/multiple\"\n",
            "rev = \"abc123\"\n",
            "branch = \"main\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(
        manifest.dependencies[0].direct_analysis_source_root(temp.root()),
        Err(DirectAnalysisSourceError::MissingGitSelector)
    );
    assert_eq!(
        manifest.dependencies[1].direct_analysis_source_root(temp.root()),
        Err(DirectAnalysisSourceError::MultipleGitSelectors)
    );
}

#[test]
fn direct_analysis_source_root_rejects_escaping_git_subdir() {
    let temp = TempProject::new("manifest-direct-analysis-git-subdir");
    temp.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/escape\"]\n",
            "git = \"materialized/mono\"\n",
            "rev = \"abc123\"\n",
            "subdir = \"../escape\"\n",
        ),
    );

    let manifest = read_manifest(temp.root())
        .unwrap()
        .expect("manifest should be loaded");

    assert_eq!(
        manifest.dependencies[0].direct_analysis_source_root(temp.root()),
        Err(DirectAnalysisSourceError::InvalidGitSubdir)
    );
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
