use super::*;

#[cfg(test)]
pub(super) fn lock_git_dependency_with<F>(
    root: &Path,
    dependency: &ManifestDependency,
    resolve_rev: F,
) -> Result<LockfilePackage, Box<Diagnostic>>
where
    F: Fn(&Path, &ManifestDependencySelector) -> Result<String, String>,
{
    Ok(lock_git_dependency_with_materializer_and_manifest(
        root,
        root,
        dependency,
        resolve_rev,
        materialize_git_source,
    )?
    .package)
}

#[cfg(test)]
pub(super) fn lock_git_dependency_with_materializer<F, M>(
    root: &Path,
    dependency: &ManifestDependency,
    resolve_rev: F,
    materialize_source: M,
) -> Result<LockfilePackage, Box<Diagnostic>>
where
    F: Fn(&Path, &ManifestDependencySelector) -> Result<String, String>,
    M: Fn(&Path, &ManifestField, &ManifestDependencySelector) -> Result<PathBuf, String>,
{
    Ok(lock_git_dependency_with_materializer_and_manifest(
        root,
        root,
        dependency,
        resolve_rev,
        materialize_source,
    )?
    .package)
}

pub(super) fn lock_git_dependency_with_manifest(
    lockfile_root: &Path,
    owner_root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockedDependency, Box<Diagnostic>> {
    lock_git_dependency_with_materializer_and_manifest(
        lockfile_root,
        owner_root,
        dependency,
        resolve_git_selector,
        materialize_git_source,
    )
}

struct PreparedGitDependency {
    repository_root: PathBuf,
    package_root: PathBuf,
    manifest: ProjectManifest,
    lockfile_url: String,
    display_path: String,
}

fn git_dependency_repository_root<M>(
    lockfile_root: &Path,
    owner_root: &Path,
    dependency: &ManifestDependency,
    git_field: &ManifestField,
    selector: &ManifestDependencySelector,
    materialize_source: &M,
) -> Result<PathBuf, Box<Diagnostic>>
where
    M: Fn(&Path, &ManifestField, &ManifestDependencySelector) -> Result<PathBuf, String>,
{
    let repository_root = match git_source_root(owner_root, &git_field.value).map_err(|reason| {
        Box::new(unavailable_git_dependency_diagnostic(
            dependency,
            git_field,
            direct_analysis_source_error_reason(reason),
        ))
    })? {
        Some(repository_root) => repository_root,
        None => materialize_source(lockfile_root, git_field, selector).map_err(|reason| {
            Box::new(git_materialization_diagnostic(
                dependency, git_field, &reason,
            ))
        })?,
    };
    if !repository_root.is_dir() {
        return Err(Box::new(unavailable_git_dependency_diagnostic(
            dependency,
            git_field,
            "source_not_directory",
        )));
    }
    Ok(repository_root)
}

fn prepare_git_dependency(
    lockfile_root: &Path,
    owner_root: &Path,
    dependency: &ManifestDependency,
    git_field: &ManifestField,
    repository_root: PathBuf,
) -> Result<PreparedGitDependency, Box<Diagnostic>> {
    let package_root =
        git_package_root(&repository_root, dependency).map_err(|reason| match reason {
            DirectAnalysisSourceError::InvalidGitSubdir => dependency
                .subdir
                .as_ref()
                .map(|subdir| Box::new(invalid_git_subdir_diagnostic(dependency, subdir)))
                .unwrap_or_else(|| {
                    Box::new(unavailable_git_dependency_diagnostic(
                        dependency,
                        git_field,
                        direct_analysis_source_error_reason(reason),
                    ))
                }),
            _ => Box::new(unavailable_git_dependency_diagnostic(
                dependency,
                git_field,
                direct_analysis_source_error_reason(reason),
            )),
        })?;
    let lockfile_url = git_lockfile_url(lockfile_root, owner_root, git_field);
    let display_path = git_dependency_display_path(&lockfile_url, dependency.subdir.as_ref());
    let manifest = read_manifest(&package_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, git_field, error)))?
        .ok_or_else(|| {
            Box::new(dependency_missing_manifest_diagnostic(
                dependency, git_field,
            ))
        })?;
    validate_package_name(dependency, &manifest, &display_path)?;
    Ok(PreparedGitDependency {
        repository_root,
        package_root,
        manifest,
        lockfile_url,
        display_path,
    })
}

fn lock_git_dependency_with_materializer_and_manifest<F, M>(
    lockfile_root: &Path,
    owner_root: &Path,
    dependency: &ManifestDependency,
    resolve_rev: F,
    materialize_source: M,
) -> Result<LockedDependency, Box<Diagnostic>>
where
    F: Fn(&Path, &ManifestDependencySelector) -> Result<String, String>,
    M: Fn(&Path, &ManifestField, &ManifestDependencySelector) -> Result<PathBuf, String>,
{
    let git_field = dependency.git.as_ref().expect("git source should exist");
    let selector = git_selector(dependency)?;
    let repository_root = git_dependency_repository_root(
        lockfile_root,
        owner_root,
        dependency,
        git_field,
        selector,
        &materialize_source,
    )?;
    let prepared = prepare_git_dependency(
        lockfile_root,
        owner_root,
        dependency,
        git_field,
        repository_root,
    )?;
    let resolved_rev = resolve_rev(&prepared.repository_root, selector).map_err(|reason| {
        Box::new(git_selector_resolution_diagnostic(
            dependency, selector, &reason,
        ))
    })?;
    let checksum = source_tree_checksum(&prepared.package_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, git_field, error)))?;
    let subdir = dependency
        .subdir
        .as_ref()
        .map(|subdir| normalize_lockfile_path(&subdir.value));

    let package = LockfilePackage {
        name: dependency.package.clone(),
        source: LockfileSource::Git {
            url: prepared.lockfile_url.clone(),
            selector: lockfile_git_selector(selector),
            rev: resolved_rev,
            subdir: subdir.clone(),
        },
        checksum,
    };
    Ok(LockedDependency {
        selection: DependencySelection::Git {
            url: prepared.lockfile_url,
            selector: dependency_git_selector(selector),
            subdir,
        },
        package,
        manifest: prepared.manifest,
        package_root: prepared.package_root,
        manifest_display_path: prepared.display_path,
        package_span: dependency.package_span.clone(),
    })
}

pub(super) fn dependency_selection(
    lockfile_root: &Path,
    owner_root: &Path,
    dependency: &ManifestDependency,
) -> Option<DependencySelection> {
    let source_count = usize::from(dependency.path.is_some())
        + usize::from(dependency.git.is_some())
        + usize::from(dependency.vendor.is_some())
        + usize::from(dependency.mirror.is_some());
    if source_count != 1 {
        return None;
    }
    if let Some(path) = &dependency.path {
        return Some(DependencySelection::Path {
            path: source_path_for_lockfile(lockfile_root, owner_root, &path.value),
        });
    }
    if let Some(vendor) = &dependency.vendor {
        return Some(DependencySelection::Vendor {
            path: source_path_for_lockfile(lockfile_root, owner_root, &vendor.value),
        });
    }
    if let Some(mirror) = &dependency.mirror {
        return Some(DependencySelection::Mirror {
            path: source_path_for_lockfile(lockfile_root, owner_root, &mirror.value),
        });
    }
    let selector = dependency.selectors.first()?;
    if dependency.selectors.len() != 1 {
        return None;
    }
    let git = dependency.git.as_ref()?;
    Some(DependencySelection::Git {
        url: git_lockfile_url(lockfile_root, owner_root, git),
        selector: dependency_git_selector(selector),
        subdir: dependency
            .subdir
            .as_ref()
            .map(|subdir| normalize_lockfile_path(&subdir.value)),
    })
}

fn dependency_git_selector(selector: &ManifestDependencySelector) -> DependencyGitSelector {
    match selector.kind {
        ManifestDependencySelectorKind::Rev => {
            DependencyGitSelector::Rev(selector.field.value.clone())
        }
        ManifestDependencySelectorKind::Tag => {
            DependencyGitSelector::Tag(selector.field.value.clone())
        }
        ManifestDependencySelectorKind::Branch => {
            DependencyGitSelector::Branch(selector.field.value.clone())
        }
    }
}

pub(super) fn source_path_for_lockfile(
    lockfile_root: &Path,
    owner_root: &Path,
    path: &str,
) -> String {
    let resolved = dependency_root(owner_root, path);
    let lockfile_path = resolved.strip_prefix(lockfile_root).unwrap_or(&resolved);
    path_to_lockfile_string(lockfile_path)
}

fn path_to_lockfile_string(path: &Path) -> String {
    let mut parts = Vec::<String>::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                if parts
                    .last()
                    .is_some_and(|part| part != ".." && part != "/" && !part.ends_with(':'))
                {
                    parts.pop();
                } else {
                    parts.push("..".to_string());
                }
            }
            component => parts.push(component.as_os_str().to_string_lossy().into_owned()),
        }
    }
    parts.join("/")
}

fn git_lockfile_url(lockfile_root: &Path, owner_root: &Path, git_field: &ManifestField) -> String {
    if local_file_url_path(&git_field.value).is_ok_and(|path| path.is_some())
        || is_non_local_git_source(&git_field.value)
    {
        normalize_lockfile_path(&git_field.value)
    } else {
        source_path_for_lockfile(lockfile_root, owner_root, &git_field.value)
    }
}

fn materialize_git_source(
    root: &Path,
    git_field: &ManifestField,
    selector: &ManifestDependencySelector,
) -> Result<PathBuf, String> {
    let repository_root = materialized_git_repository_root(root, &git_field.value);
    if repository_root.exists() {
        if !repository_root.is_dir() {
            return Err("materialized source path is not a directory".to_string());
        }
        run_git_in(
            &repository_root,
            &["fetch", "--tags", "--force", "--prune", "origin"],
        )?;
    } else {
        if let Some(parent) = repository_root.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        run_git_clone(&git_field.value, &repository_root)?;
    }

    checkout_materialized_git_source(&repository_root, selector)?;
    run_git_in(&repository_root, &["clean", "-fdx"])?;
    Ok(repository_root)
}

fn checkout_materialized_git_source(
    repository_root: &Path,
    selector: &ManifestDependencySelector,
) -> Result<(), String> {
    match selector.kind {
        ManifestDependencySelectorKind::Rev => run_git_in(
            repository_root,
            &["checkout", "--force", &selector.field.value],
        ),
        ManifestDependencySelectorKind::Tag => {
            let tag_ref = format!("refs/tags/{}", selector.field.value);
            run_git_in(repository_root, &["checkout", "--force", &tag_ref])
        }
        ManifestDependencySelectorKind::Branch => {
            let remote_ref = format!("refs/remotes/origin/{}", selector.field.value);
            run_git_in(
                repository_root,
                &[
                    "checkout",
                    "--force",
                    "-B",
                    &selector.field.value,
                    &remote_ref,
                ],
            )
        }
    }
}

fn run_git_clone(source: &str, destination: &Path) -> Result<(), String> {
    let output = Command::new("git")
        .arg("clone")
        .arg("--no-checkout")
        .arg(source)
        .arg(destination)
        .output()
        .map_err(|error| error.to_string())?;
    git_command_result(output)
}

fn run_git_in(repository_root: &Path, args: &[&str]) -> Result<(), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .args(args)
        .output()
        .map_err(|error| error.to_string())?;
    git_command_result(output)
}

fn git_command_result(output: std::process::Output) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if !stderr.is_empty() {
        return Err(stderr);
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        Err("git command failed".to_string())
    } else {
        Err(stdout)
    }
}

fn git_selector(
    dependency: &ManifestDependency,
) -> Result<&ManifestDependencySelector, Box<Diagnostic>> {
    validate_unique_git_selector(dependency).map_err(|reason| {
        Box::new(unsupported_git_selector_diagnostic(
            dependency,
            dependency.selectors.first(),
            direct_analysis_source_error_reason(reason),
        ))
    })
}

fn direct_analysis_source_error_reason(reason: DirectAnalysisSourceError) -> &'static str {
    match reason {
        DirectAnalysisSourceError::InvalidFileUrl => "invalid_file_url",
        DirectAnalysisSourceError::NonLocalFileUrl => "non_local_file_url",
        DirectAnalysisSourceError::MissingGitSelector => "missing_selector",
        DirectAnalysisSourceError::MultipleGitSelectors => "multiple_selectors",
        DirectAnalysisSourceError::InvalidGitSubdir => "invalid_package_subdir",
    }
}

fn lockfile_git_selector(selector: &ManifestDependencySelector) -> LockfileGitSelector {
    match selector.kind {
        ManifestDependencySelectorKind::Rev => {
            LockfileGitSelector::Rev(selector.field.value.clone())
        }
        ManifestDependencySelectorKind::Tag => {
            LockfileGitSelector::Tag(selector.field.value.clone())
        }
        ManifestDependencySelectorKind::Branch => {
            LockfileGitSelector::Branch(selector.field.value.clone())
        }
    }
}

fn resolve_git_selector(
    repository_root: &Path,
    selector: &ManifestDependencySelector,
) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repository_root)
        .arg("rev-parse")
        .arg("--verify")
        .arg(git_selector_revspec(selector))
        .output()
        .map_err(|error| error.to_string())?;
    if output.status.success() {
        let rev = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !rev.is_empty() {
            return Ok(rev);
        }
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err("git rev-parse did not resolve a commit".to_string())
    } else {
        Err(stderr)
    }
}

pub(super) fn git_selector_revspec(selector: &ManifestDependencySelector) -> String {
    match selector.kind {
        ManifestDependencySelectorKind::Rev => {
            format!("{}^{{commit}}", selector.field.value)
        }
        ManifestDependencySelectorKind::Tag => {
            format!("refs/tags/{}^{{commit}}", selector.field.value)
        }
        ManifestDependencySelectorKind::Branch => {
            format!("refs/heads/{}^{{commit}}", selector.field.value)
        }
    }
}

pub(super) fn validate_package_name(
    dependency: &ManifestDependency,
    manifest: &ProjectManifest,
    dependency_path: &str,
) -> Result<(), Box<Diagnostic>> {
    let Some(name_field) = manifest_package_name(manifest) else {
        return Err(Box::new(package_name_mismatch_diagnostic(
            dependency, None, None,
        )));
    };
    if name_field.value != dependency.package {
        let actual_span = dependency_manifest_span(dependency_path, &name_field.value_span);
        return Err(Box::new(package_name_mismatch_diagnostic(
            dependency,
            Some(&name_field.value),
            Some(&actual_span),
        )));
    }
    Ok(())
}

fn dependency_manifest_span(dependency_path: &str, span: &SourceSpan) -> SourceSpan {
    let file = if dependency_path.is_empty() {
        span.file.as_str().to_string()
    } else {
        format!("{dependency_path}/{}", span.file.as_str())
    };
    let mut span = span.clone();
    span.file = SourcePath::new(file);
    span
}

fn git_dependency_display_path(lockfile_url: &str, subdir: Option<&ManifestField>) -> String {
    let mut path = lockfile_url.to_string();
    if let Some(subdir) = subdir {
        path.push('/');
        path.push_str(&normalize_lockfile_path(&subdir.value));
    }
    path
}

fn manifest_package_name(manifest: &ProjectManifest) -> Option<&ManifestField> {
    manifest
        .package
        .fields
        .iter()
        .find(|field| field.key == "name")
}
