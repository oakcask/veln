use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use veln_diagnostics::{Diagnostic, DiagnosticEnvelope, DiagnosticKind, JsonValue, Severity};
use veln_project::{
    DirectAnalysisSourceError, LockfileGitSelector, LockfilePackage, LockfileSource,
    ManifestDependency, ManifestDependencySelector, ManifestDependencySelectorKind, ManifestField,
    PackageIdentity, ProjectLockfile, ProjectManifest, dependency_root, git_package_root,
    git_source_root, is_non_local_git_source, local_file_url_path,
    materialized_git_repository_root, normalize_lockfile_path, read_manifest, source_tree_checksum,
    validate_unique_git_selector, write_lockfile,
};
use veln_source::{SourcePath, SourceSpan};

use crate::diagnostics::{has_error, print_human_stderr, tool_info};

pub(crate) fn lock(start: super::CommandAnalysisStart) -> Result<ExitCode, String> {
    let root = start.package_root;
    let Some(manifest) = read_manifest(&root).map_err(|error| error.to_string())? else {
        return report_package_diagnostics(vec![missing_manifest_diagnostic()]);
    };

    let mut resolver = PackageLockResolver::new(&root);
    resolver.lock_manifest_dependencies(&root, "", &manifest);

    if has_error(&resolver.diagnostics) {
        return report_package_diagnostics(resolver.diagnostics);
    }

    let packages = resolver.packages();
    let lockfile = ProjectLockfile { packages };
    write_lockfile(&root, &lockfile).map_err(|error| error.to_string())?;
    println!("wrote veln.lock");
    Ok(ExitCode::SUCCESS)
}

struct PackageLockResolver<'a> {
    lockfile_root: &'a Path,
    locked: BTreeMap<String, LockedDependency>,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Clone)]
struct LockedDependency {
    selection: DependencySelection,
    package: LockfilePackage,
    manifest: ProjectManifest,
    package_root: PathBuf,
    manifest_display_path: String,
    package_span: SourceSpan,
}

impl<'a> PackageLockResolver<'a> {
    fn new(lockfile_root: &'a Path) -> Self {
        Self {
            lockfile_root,
            locked: BTreeMap::new(),
            diagnostics: Vec::new(),
        }
    }

    fn lock_manifest_dependencies(
        &mut self,
        owner_root: &Path,
        owner_display_path: &str,
        manifest: &ProjectManifest,
    ) {
        for dependency in &manifest.dependencies {
            self.lock_manifest_dependency(owner_root, owner_display_path, dependency);
        }
    }

    fn lock_manifest_dependency(
        &mut self,
        owner_root: &Path,
        owner_display_path: &str,
        dependency: &ManifestDependency,
    ) {
        let dependency = dependency_in_context(dependency, owner_display_path);
        if dependency.package == veln_stdlib::PACKAGE_NAME {
            self.diagnostics
                .push(reserved_standard_package_diagnostic(&dependency));
            return;
        }
        if let Err(reason) = PackageIdentity::new(dependency.package.clone()) {
            self.diagnostics
                .push(invalid_dependency_identity_diagnostic(&dependency, &reason));
            return;
        }
        let selection = dependency_selection(self.lockfile_root, owner_root, &dependency);
        if let Some(selection) = &selection
            && let Some(existing) = self.locked.get(&dependency.package)
        {
            if existing.selection != *selection {
                self.diagnostics
                    .push(incompatible_dependency_source_diagnostic(
                        &dependency,
                        selection,
                        existing,
                    ));
            }
            return;
        }

        let locked =
            match lock_dependency_with_manifest(self.lockfile_root, owner_root, &dependency) {
                Ok(locked) => locked,
                Err(diagnostic) => {
                    self.diagnostics.push(*diagnostic);
                    return;
                }
            };

        self.locked
            .insert(dependency.package.clone(), locked.clone());
        self.lock_manifest_dependencies(
            &locked.package_root,
            &locked.manifest_display_path,
            &locked.manifest,
        );
    }

    fn packages(&self) -> Vec<LockfilePackage> {
        self.locked
            .values()
            .map(|locked| locked.package.clone())
            .collect()
    }
}

fn reserved_standard_package_diagnostic(dependency: &ManifestDependency) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "manifest.reserved_standard_package",
        Severity::Error,
        DiagnosticKind::Module,
        "dependency package `std` is reserved by the Veln toolchain",
        Some(dependency.package_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies")),
            ("package", JsonValue::string(veln_stdlib::PACKAGE_NAME)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string(
                "Remove this dependency; the standard package is supplied by the toolchain and is not written to veln.lock.",
            ),
        ),
    ]));
    diagnostic
}

fn invalid_dependency_identity_diagnostic(
    dependency: &ManifestDependency,
    reason: &dyn std::error::Error,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "package.invalid_dependency_identity",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "dependency package identity `{}` is invalid",
            dependency.package
        ),
        Some(dependency.package_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("reason", JsonValue::string(reason.to_string())),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        (
            "message",
            JsonValue::string(format!("Use a portable package identity: {reason}.")),
        ),
    ]));
    diagnostic
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DependencySelection {
    Path {
        path: String,
    },
    Vendor {
        path: String,
    },
    Mirror {
        path: String,
    },
    Git {
        url: String,
        selector: DependencyGitSelector,
        subdir: Option<String>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DependencyGitSelector {
    Rev(String),
    Tag(String),
    Branch(String),
}

#[cfg(test)]
fn lock_dependency(
    root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockfilePackage, Box<Diagnostic>> {
    Ok(lock_dependency_with_manifest(root, root, dependency)?.package)
}

fn lock_dependency_with_manifest(
    lockfile_root: &Path,
    owner_root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockedDependency, Box<Diagnostic>> {
    let source_count = usize::from(dependency.path.is_some())
        + usize::from(dependency.git.is_some())
        + usize::from(dependency.vendor.is_some())
        + usize::from(dependency.mirror.is_some());
    if source_count > 1 {
        return Err(Box::new(unsupported_dependency_source_diagnostic(
            dependency,
            "mixed_sources",
        )));
    }
    if dependency.git.is_some() {
        return lock_git_dependency_with_manifest(lockfile_root, owner_root, dependency);
    }
    if dependency.vendor.is_some() {
        return lock_vendor_dependency_with_manifest(lockfile_root, owner_root, dependency);
    }
    if dependency.mirror.is_some() {
        return lock_mirror_dependency_with_manifest(lockfile_root, owner_root, dependency);
    }
    lock_path_dependency_with_manifest(lockfile_root, owner_root, dependency)
}

fn report_package_diagnostics(diagnostics: Vec<Diagnostic>) -> Result<ExitCode, String> {
    let envelope = DiagnosticEnvelope::new(tool_info(), diagnostics);
    print_human_stderr(&envelope)?;
    Ok(ExitCode::from(1))
}

fn lock_path_dependency_with_manifest(
    lockfile_root: &Path,
    owner_root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockedDependency, Box<Diagnostic>> {
    let Some(path_field) = &dependency.path else {
        return Err(Box::new(unsupported_dependency_source_diagnostic(
            dependency,
            "unsupported_source",
        )));
    };

    let dependency_root = dependency_root(owner_root, &path_field.value);
    let display_path = source_path_for_lockfile(lockfile_root, owner_root, &path_field.value);
    if !dependency_root.is_dir() {
        return Err(Box::new(unavailable_path_dependency_diagnostic(
            dependency, path_field,
        )));
    }

    let manifest = read_manifest(&dependency_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, path_field, error)))?
        .ok_or_else(|| {
            Box::new(dependency_missing_manifest_diagnostic(
                dependency, path_field,
            ))
        })?;
    validate_package_name(dependency, &manifest, &display_path)?;

    let checksum = source_tree_checksum(&dependency_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, path_field, error)))?;
    let package = LockfilePackage {
        name: dependency.package.clone(),
        source: LockfileSource::Path {
            path: display_path.clone(),
        },
        checksum,
    };
    Ok(LockedDependency {
        selection: DependencySelection::Path {
            path: display_path.clone(),
        },
        package,
        manifest,
        package_root: dependency_root,
        manifest_display_path: display_path,
        package_span: dependency.package_span.clone(),
    })
}

#[cfg(test)]
fn lock_vendor_dependency(
    root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockfilePackage, Box<Diagnostic>> {
    Ok(lock_vendor_dependency_with_manifest(root, root, dependency)?.package)
}

fn lock_vendor_dependency_with_manifest(
    lockfile_root: &Path,
    owner_root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockedDependency, Box<Diagnostic>> {
    let vendor_field = dependency
        .vendor
        .as_ref()
        .expect("vendor source should exist");

    let dependency_root = dependency_root(owner_root, &vendor_field.value);
    let display_path = source_path_for_lockfile(lockfile_root, owner_root, &vendor_field.value);
    if !dependency_root.is_dir() {
        return Err(Box::new(unavailable_vendor_dependency_diagnostic(
            dependency,
            vendor_field,
        )));
    }

    let manifest = read_manifest(&dependency_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, vendor_field, error)))?
        .ok_or_else(|| {
            Box::new(dependency_missing_manifest_diagnostic(
                dependency,
                vendor_field,
            ))
        })?;
    validate_package_name(dependency, &manifest, &display_path)?;

    let checksum = source_tree_checksum(&dependency_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, vendor_field, error)))?;
    let package = LockfilePackage {
        name: dependency.package.clone(),
        source: LockfileSource::Vendor {
            path: display_path.clone(),
        },
        checksum,
    };
    Ok(LockedDependency {
        selection: DependencySelection::Vendor {
            path: display_path.clone(),
        },
        package,
        manifest,
        package_root: dependency_root,
        manifest_display_path: display_path,
        package_span: dependency.package_span.clone(),
    })
}

#[cfg(test)]
fn lock_mirror_dependency(
    root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockfilePackage, Box<Diagnostic>> {
    Ok(lock_mirror_dependency_with_manifest(root, root, dependency)?.package)
}

fn lock_mirror_dependency_with_manifest(
    lockfile_root: &Path,
    owner_root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockedDependency, Box<Diagnostic>> {
    let mirror_field = dependency
        .mirror
        .as_ref()
        .expect("mirror source should exist");

    let dependency_root = dependency_root(owner_root, &mirror_field.value);
    let display_path = source_path_for_lockfile(lockfile_root, owner_root, &mirror_field.value);
    if !dependency_root.is_dir() {
        return Err(Box::new(unavailable_mirror_dependency_diagnostic(
            dependency,
            mirror_field,
        )));
    }

    let manifest = read_manifest(&dependency_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, mirror_field, error)))?
        .ok_or_else(|| {
            Box::new(dependency_missing_manifest_diagnostic(
                dependency,
                mirror_field,
            ))
        })?;
    validate_package_name(dependency, &manifest, &display_path)?;

    let checksum = source_tree_checksum(&dependency_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, mirror_field, error)))?;
    let package = LockfilePackage {
        name: dependency.package.clone(),
        source: LockfileSource::Mirror {
            path: display_path.clone(),
        },
        checksum,
    };
    Ok(LockedDependency {
        selection: DependencySelection::Mirror {
            path: display_path.clone(),
        },
        package,
        manifest,
        package_root: dependency_root,
        manifest_display_path: display_path,
        package_span: dependency.package_span.clone(),
    })
}

#[cfg(test)]
fn lock_git_dependency_with<F>(
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
fn lock_git_dependency_with_materializer<F, M>(
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

fn lock_git_dependency_with_manifest(
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

fn dependency_selection(
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

fn source_path_for_lockfile(lockfile_root: &Path, owner_root: &Path, path: &str) -> String {
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

fn git_selector_revspec(selector: &ManifestDependencySelector) -> String {
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

fn validate_package_name(
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

fn missing_manifest_diagnostic() -> Diagnostic {
    Diagnostic::new(
        "package.missing_manifest",
        Severity::Error,
        DiagnosticKind::Module,
        "package lock requires veln.toml in the current project",
        None,
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("manifest")),
        ]),
    )
}

fn dependency_in_context(
    dependency: &ManifestDependency,
    manifest_display_path: &str,
) -> ManifestDependency {
    let mut dependency = dependency.clone();
    dependency.package_span = span_in_context(&dependency.package_span, manifest_display_path);
    dependency.path = dependency
        .path
        .map(|field| field_in_context(field, manifest_display_path));
    dependency.git = dependency
        .git
        .map(|field| field_in_context(field, manifest_display_path));
    dependency.vendor = dependency
        .vendor
        .map(|field| field_in_context(field, manifest_display_path));
    dependency.mirror = dependency
        .mirror
        .map(|field| field_in_context(field, manifest_display_path));
    dependency.subdir = dependency
        .subdir
        .map(|field| field_in_context(field, manifest_display_path));
    dependency.selectors = dependency
        .selectors
        .into_iter()
        .map(|selector| ManifestDependencySelector {
            kind: selector.kind,
            field: field_in_context(selector.field, manifest_display_path),
        })
        .collect();
    dependency
}

fn field_in_context(field: ManifestField, manifest_display_path: &str) -> ManifestField {
    ManifestField {
        key_span: span_in_context(&field.key_span, manifest_display_path),
        value_span: span_in_context(&field.value_span, manifest_display_path),
        ..field
    }
}

fn span_in_context(span: &SourceSpan, manifest_display_path: &str) -> SourceSpan {
    if manifest_display_path.is_empty() {
        return span.clone();
    }
    let mut span = span.clone();
    span.file = SourcePath::new(format!("{manifest_display_path}/{}", span.file.as_str()));
    span
}

fn incompatible_dependency_source_diagnostic(
    dependency: &ManifestDependency,
    selection: &DependencySelection,
    existing: &LockedDependency,
) -> Diagnostic {
    let requested = selection_summary(selection);
    let existing_source = selection_summary(&existing.selection);
    let mut diagnostic = Diagnostic::new(
        "package.incompatible_dependency_source",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "dependency `{}` selects {requested}, but that package identity is already selected as {existing_source}",
            dependency.package
        ),
        Some(dependency.package_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("requested_source", JsonValue::string(requested)),
            ("existing_source", JsonValue::string(existing_source)),
            ("reason", JsonValue::string("incompatible_source")),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("previous_dependency_source")),
        (
            "message",
            JsonValue::string("This dependency selected the package identity first."),
        ),
        ("span", span_json(&existing.package_span)),
    ]));
    diagnostic
}

fn selection_summary(selection: &DependencySelection) -> String {
    match selection {
        DependencySelection::Path { path } => format!("path `{path}`"),
        DependencySelection::Vendor { path } => format!("vendor `{path}`"),
        DependencySelection::Mirror { path } => format!("mirror `{path}`"),
        DependencySelection::Git {
            url,
            selector,
            subdir,
        } => {
            let selector = git_selector_summary(selector);
            let mut summary = format!("git `{url}` with {selector}");
            if let Some(subdir) = subdir {
                summary.push_str(&format!(" in subdir `{subdir}`"));
            }
            summary
        }
    }
}

fn git_selector_summary(selector: &DependencyGitSelector) -> String {
    match selector {
        DependencyGitSelector::Rev(value) => format!("rev `{value}`"),
        DependencyGitSelector::Tag(value) => format!("tag `{value}`"),
        DependencyGitSelector::Branch(value) => format!("branch `{value}`"),
    }
}

fn unsupported_dependency_source_diagnostic(
    dependency: &ManifestDependency,
    reason: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "package.unsupported_dependency_source",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "package lock supports only one of path, git, vendor, or mirror dependencies for `{}`",
            dependency.package
        ),
        Some(dependency.package_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("reason", JsonValue::string(reason)),
        ]),
    )
}

fn unavailable_path_dependency_diagnostic(
    dependency: &ManifestDependency,
    path_field: &ManifestField,
) -> Diagnostic {
    Diagnostic::new(
        "package.path_unavailable",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "path dependency `{}` is not available at `{}`",
            dependency.package, path_field.value
        ),
        Some(path_field.value_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies.path")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("path", JsonValue::string(path_field.value.clone())),
        ]),
    )
}

fn unavailable_mirror_dependency_diagnostic(
    dependency: &ManifestDependency,
    mirror_field: &ManifestField,
) -> Diagnostic {
    Diagnostic::new(
        "package.mirror_unavailable",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "mirror dependency `{}` is not available at `{}`",
            dependency.package, mirror_field.value
        ),
        Some(mirror_field.value_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies.mirror")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("path", JsonValue::string(mirror_field.value.clone())),
        ]),
    )
}

fn unavailable_git_dependency_diagnostic(
    dependency: &ManifestDependency,
    git_field: &ManifestField,
    reason: &str,
) -> Diagnostic {
    Diagnostic::new(
        "package.git_unavailable",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "git dependency `{}` is not available at `{}`",
            dependency.package, git_field.value
        ),
        Some(git_field.value_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies.git")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("url", JsonValue::string(git_field.value.clone())),
            ("reason", JsonValue::string(reason)),
        ]),
    )
}

fn unavailable_vendor_dependency_diagnostic(
    dependency: &ManifestDependency,
    vendor_field: &ManifestField,
) -> Diagnostic {
    Diagnostic::new(
        "package.vendor_unavailable",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "vendor dependency `{}` is not available at `{}`",
            dependency.package, vendor_field.value
        ),
        Some(vendor_field.value_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies.vendor")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("path", JsonValue::string(vendor_field.value.clone())),
        ]),
    )
}

fn git_materialization_diagnostic(
    dependency: &ManifestDependency,
    git_field: &ManifestField,
    reason: &str,
) -> Diagnostic {
    Diagnostic::new(
        "package.git_materialization_failed",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "git dependency `{}` could not be materialized from `{}`",
            dependency.package, git_field.value
        ),
        Some(git_field.value_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies.git")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("url", JsonValue::string(git_field.value.clone())),
            ("reason", JsonValue::string(reason)),
        ]),
    )
}

fn dependency_missing_manifest_diagnostic(
    dependency: &ManifestDependency,
    source_field: &ManifestField,
) -> Diagnostic {
    Diagnostic::new(
        "package.dependency_missing_manifest",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "dependency `{}` has no veln.toml at `{}`",
            dependency.package, source_field.value
        ),
        Some(source_field.value_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("source", JsonValue::string(source_field.value.clone())),
        ]),
    )
}

fn dependency_io_diagnostic(
    dependency: &ManifestDependency,
    source_field: &ManifestField,
    error: std::io::Error,
) -> Diagnostic {
    Diagnostic::new(
        "package.dependency_read_failed",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "dependency `{}` could not be read: {error}",
            dependency.package
        ),
        Some(source_field.value_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("source", JsonValue::string(source_field.value.clone())),
        ]),
    )
}

fn unsupported_git_selector_diagnostic(
    dependency: &ManifestDependency,
    selector: Option<&ManifestDependencySelector>,
    reason: &'static str,
) -> Diagnostic {
    let span = selector
        .map(|selector| selector.field.key_span.clone())
        .unwrap_or_else(|| dependency.package_span.clone());
    Diagnostic::new(
        "package.unsupported_git_selector",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "package lock requires git dependency `{}` to specify exactly one selector: `rev`, `tag`, or `branch`",
            dependency.package
        ),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("source_kind", JsonValue::string("git")),
            ("reason", JsonValue::string(reason)),
        ]),
    )
}

fn invalid_git_subdir_diagnostic(
    dependency: &ManifestDependency,
    subdir_field: &ManifestField,
) -> Diagnostic {
    Diagnostic::new(
        "package.invalid_git_subdir",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "git dependency `{}` has invalid subdir `{}`",
            dependency.package, subdir_field.value
        ),
        Some(subdir_field.value_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies.subdir")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("subdir", JsonValue::string(subdir_field.value.clone())),
            ("reason", JsonValue::string("invalid_package_subdir")),
        ]),
    )
}

fn git_selector_resolution_diagnostic(
    dependency: &ManifestDependency,
    selector: &ManifestDependencySelector,
    reason: &str,
) -> Diagnostic {
    let kind = selector.kind.as_str();
    Diagnostic::new(
        "package.git_selector_unresolved",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "git dependency `{}` {} `{}` could not be resolved",
            dependency.package, kind, selector.field.value
        ),
        Some(selector.field.value_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string(format!("dependencies.{kind}"))),
            ("package", JsonValue::string(dependency.package.clone())),
            ("selector_kind", JsonValue::string(kind)),
            ("selector", JsonValue::string(selector.field.value.clone())),
            ("reason", JsonValue::string(reason)),
        ]),
    )
}

fn package_name_mismatch_diagnostic(
    dependency: &ManifestDependency,
    actual_name: Option<&str>,
    actual_span: Option<&SourceSpan>,
) -> Diagnostic {
    let actual_name = actual_name.unwrap_or("<missing>");
    let mut diagnostic = Diagnostic::new(
        "manifest.package_name_mismatch",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "dependency package name `{actual_name}` does not match `{}`",
            dependency.package
        ),
        Some(dependency.package_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("package.name")),
            (
                "expected_package",
                JsonValue::string(dependency.package.clone()),
            ),
            ("actual_package", JsonValue::string(actual_name)),
        ]),
    );
    if let Some(span) = actual_span {
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("dependency_manifest_name")),
            (
                "message",
                JsonValue::string("The dependency manifest declares this package name."),
            ),
            ("span", span_json(span)),
        ]));
    }
    diagnostic
}

fn span_json(span: &SourceSpan) -> JsonValue {
    JsonValue::object([
        ("file", JsonValue::string(span.file.as_str())),
        (
            "start",
            JsonValue::object([
                ("line", JsonValue::Number(span.start.line as i64)),
                ("column", JsonValue::Number(span.start.column as i64)),
                ("offset", JsonValue::Number(span.start.offset as i64)),
            ]),
        ),
        (
            "end",
            JsonValue::object([
                ("line", JsonValue::Number(span.end.line as i64)),
                ("column", JsonValue::Number(span.end.column as i64)),
                ("offset", JsonValue::Number(span.end.offset as i64)),
            ]),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    use veln_project::read_manifest;

    use super::*;

    static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

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
        let package =
            lock_git_dependency_with(project.root(), &manifest.dependencies[0], |repo, _| {
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
        let diagnostic =
            lock_git_dependency_with(project.root(), &manifest.dependencies[0], |_, _| {
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

    struct TempProject {
        root: PathBuf,
    }

    impl TempProject {
        fn new(name: &str) -> Self {
            let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "veln-package-{name}-{}-{nanos}-{id}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("temp project should be created");
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
                fs::create_dir_all(parent).expect("parent directory should be created");
            }
            fs::write(path, contents).expect("fixture should be written");
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}
