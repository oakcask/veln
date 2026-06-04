use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use veln_diagnostics::{Diagnostic, DiagnosticEnvelope, DiagnosticKind, JsonValue, Severity};
use veln_project::{
    LockfileGitSelector, LockfilePackage, LockfileSource, ManifestDependency,
    ManifestDependencySelector, ManifestDependencySelectorKind, ManifestField, ProjectLockfile,
    ProjectManifest, normalize_lockfile_path, read_manifest, source_tree_checksum, write_lockfile,
};
use veln_source::{SourcePath, SourceSpan};

use crate::diagnostics::{has_error, print_human_stderr, tool_info};

pub(crate) fn lock() -> Result<ExitCode, String> {
    let root = env::current_dir().map_err(|error| error.to_string())?;
    let Some(manifest) = read_manifest(&root).map_err(|error| error.to_string())? else {
        return report_package_diagnostics(vec![missing_manifest_diagnostic()]);
    };

    let mut diagnostics = Vec::new();
    let mut packages = Vec::new();
    for dependency in &manifest.dependencies {
        match lock_dependency(&root, dependency) {
            Ok(package) => packages.push(package),
            Err(diagnostic) => diagnostics.push(*diagnostic),
        }
    }

    if has_error(&diagnostics) {
        return report_package_diagnostics(diagnostics);
    }

    packages.sort_by(|left, right| left.name.cmp(&right.name));
    let lockfile = ProjectLockfile { packages };
    write_lockfile(&root, &lockfile).map_err(|error| error.to_string())?;
    println!("wrote veln.lock");
    Ok(ExitCode::SUCCESS)
}

fn lock_dependency(
    root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockfilePackage, Box<Diagnostic>> {
    let source_count = usize::from(dependency.path.is_some())
        + usize::from(dependency.git.is_some())
        + usize::from(dependency.vendor.is_some());
    if source_count > 1 {
        return Err(Box::new(unsupported_dependency_source_diagnostic(
            dependency,
            "mixed_sources",
        )));
    }
    if dependency.git.is_some() {
        return lock_git_dependency(root, dependency);
    }
    if dependency.vendor.is_some() {
        return lock_vendor_dependency(root, dependency);
    }
    lock_path_dependency(root, dependency)
}

fn report_package_diagnostics(diagnostics: Vec<Diagnostic>) -> Result<ExitCode, String> {
    let envelope = DiagnosticEnvelope::new(tool_info(), diagnostics);
    print_human_stderr(&envelope)?;
    Ok(ExitCode::from(1))
}

fn lock_path_dependency(
    root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockfilePackage, Box<Diagnostic>> {
    let Some(path_field) = &dependency.path else {
        return Err(Box::new(unsupported_dependency_source_diagnostic(
            dependency,
            "unsupported_source",
        )));
    };

    let dependency_root = dependency_root(root, &path_field.value);
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
    validate_package_name(
        dependency,
        &manifest,
        &normalize_lockfile_path(&path_field.value),
    )?;

    let checksum = source_tree_checksum(&dependency_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, path_field, error)))?;
    Ok(LockfilePackage {
        name: dependency.package.clone(),
        source: LockfileSource::Path {
            path: normalize_lockfile_path(&path_field.value),
        },
        checksum,
    })
}

fn lock_vendor_dependency(
    root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockfilePackage, Box<Diagnostic>> {
    let vendor_field = dependency
        .vendor
        .as_ref()
        .expect("vendor source should exist");

    let dependency_root = dependency_root(root, &vendor_field.value);
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
    validate_package_name(
        dependency,
        &manifest,
        &normalize_lockfile_path(&vendor_field.value),
    )?;

    let checksum = source_tree_checksum(&dependency_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, vendor_field, error)))?;
    Ok(LockfilePackage {
        name: dependency.package.clone(),
        source: LockfileSource::Vendor {
            path: normalize_lockfile_path(&vendor_field.value),
        },
        checksum,
    })
}

fn lock_git_dependency(
    root: &Path,
    dependency: &ManifestDependency,
) -> Result<LockfilePackage, Box<Diagnostic>> {
    lock_git_dependency_with(root, dependency, resolve_git_selector)
}

fn lock_git_dependency_with<F>(
    root: &Path,
    dependency: &ManifestDependency,
    resolve_rev: F,
) -> Result<LockfilePackage, Box<Diagnostic>>
where
    F: Fn(&Path, &ManifestDependencySelector) -> Result<String, String>,
{
    lock_git_dependency_with_materializer(root, dependency, resolve_rev, materialize_git_source)
}

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
    let git_field = dependency.git.as_ref().expect("git source should exist");
    let selector = git_selector(dependency)?;
    let repository_root = match git_source_root(root, &git_field.value).map_err(|reason| {
        Box::new(unavailable_git_dependency_diagnostic(
            dependency, git_field, reason,
        ))
    })? {
        Some(repository_root) => repository_root,
        None => materialize_source(root, git_field, selector).map_err(|reason| {
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

    let package_root = git_package_root(&repository_root, dependency)?;
    let manifest = read_manifest(&package_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, git_field, error)))?
        .ok_or_else(|| {
            Box::new(dependency_missing_manifest_diagnostic(
                dependency, git_field,
            ))
        })?;
    validate_package_name(
        dependency,
        &manifest,
        &git_dependency_display_path(git_field, dependency.subdir.as_ref()),
    )?;

    let resolved_rev = resolve_rev(&repository_root, selector).map_err(|reason| {
        Box::new(git_selector_resolution_diagnostic(
            dependency, selector, &reason,
        ))
    })?;
    let checksum = source_tree_checksum(&package_root)
        .map_err(|error| Box::new(dependency_io_diagnostic(dependency, git_field, error)))?;

    Ok(LockfilePackage {
        name: dependency.package.clone(),
        source: LockfileSource::Git {
            url: normalize_lockfile_path(&git_field.value),
            selector: lockfile_git_selector(selector),
            rev: resolved_rev,
            subdir: dependency
                .subdir
                .as_ref()
                .map(|subdir| normalize_lockfile_path(&subdir.value)),
        },
        checksum,
    })
}

fn dependency_root(root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn git_source_root(root: &Path, source: &str) -> Result<Option<PathBuf>, &'static str> {
    if let Some(path) = local_file_url_path(source)? {
        return Ok(Some(path));
    }
    if is_non_local_git_source(source) {
        return Ok(None);
    }
    Ok(Some(dependency_root(root, source)))
}

fn is_non_local_git_source(source: &str) -> bool {
    source.contains("://") || looks_like_scp_git_source(source)
}

fn looks_like_scp_git_source(source: &str) -> bool {
    let Some(colon) = source.find(':') else {
        return false;
    };
    let slash = source.find('/').unwrap_or(usize::MAX);
    colon < slash && source[..colon].contains('@')
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

fn materialized_git_repository_root(root: &Path, source: &str) -> PathBuf {
    root.join(".veln")
        .join("package")
        .join("git")
        .join(git_source_storage_key(source))
}

fn git_source_storage_key(source: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in source.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
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

fn local_file_url_path(source: &str) -> Result<Option<PathBuf>, &'static str> {
    let Some(rest) = source.strip_prefix("file://") else {
        return Ok(None);
    };
    if let Some(path) = rest.strip_prefix("localhost/") {
        return Ok(Some(PathBuf::from(percent_decode(&format!("/{path}"))?)));
    }
    if rest.starts_with('/') {
        return Ok(Some(PathBuf::from(percent_decode(rest)?)));
    }
    Err("non_local_file_url")
}

fn percent_decode(value: &str) -> Result<String, &'static str> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let Some(hex) = bytes.get(index + 1..index + 3) else {
                return Err("invalid_file_url");
            };
            let text = std::str::from_utf8(hex).map_err(|_| "invalid_file_url")?;
            let byte = u8::from_str_radix(text, 16).map_err(|_| "invalid_file_url")?;
            out.push(byte);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "invalid_file_url")
}

fn git_package_root(
    repository_root: &Path,
    dependency: &ManifestDependency,
) -> Result<PathBuf, Box<Diagnostic>> {
    let Some(subdir) = &dependency.subdir else {
        return Ok(repository_root.to_path_buf());
    };
    if !is_relative_package_subdir(&subdir.value) {
        return Err(Box::new(invalid_git_subdir_diagnostic(dependency, subdir)));
    }
    Ok(repository_root.join(&subdir.value))
}

fn is_relative_package_subdir(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && !path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
}

fn git_selector(
    dependency: &ManifestDependency,
) -> Result<&ManifestDependencySelector, Box<Diagnostic>> {
    if dependency.selectors.len() != 1 {
        return Err(Box::new(unsupported_git_selector_diagnostic(
            dependency,
            dependency.selectors.first(),
            if dependency.selectors.is_empty() {
                "missing_selector"
            } else {
                "multiple_selectors"
            },
        )));
    }
    Ok(&dependency.selectors[0])
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

fn git_dependency_display_path(
    git_field: &ManifestField,
    subdir: Option<&ManifestField>,
) -> String {
    let mut path = normalize_lockfile_path(&git_field.value);
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

fn unsupported_dependency_source_diagnostic(
    dependency: &ManifestDependency,
    reason: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "package.unsupported_dependency_source",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "package lock supports only path, git, or vendor dependencies for `{}`",
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
            "package lock supports only path, git, or vendor dependencies for `github.com/oakcask/mixed`"
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
