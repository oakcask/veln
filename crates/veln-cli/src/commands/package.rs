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

mod diagnostics;
mod git_resolution;

use diagnostics::*;
use git_resolution::*;

#[cfg(test)]
#[path = "package/tests.rs"]
mod tests;
