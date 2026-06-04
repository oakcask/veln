use std::env;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use veln_diagnostics::{Diagnostic, DiagnosticEnvelope, DiagnosticKind, JsonValue, Severity};
use veln_project::{
    LockfilePackage, LockfileSource, ManifestDependency, ManifestField, ProjectLockfile,
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
        match lock_path_dependency(&root, dependency) {
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
        )));
    };
    if dependency.git.is_some() {
        return Err(Box::new(unsupported_dependency_source_diagnostic(
            dependency,
        )));
    }

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

fn dependency_root(root: &Path, path: &str) -> PathBuf {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
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

fn unsupported_dependency_source_diagnostic(dependency: &ManifestDependency) -> Diagnostic {
    Diagnostic::new(
        "package.unsupported_dependency_source",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "package lock supports only path dependency `{}` already available on disk",
            dependency.package
        ),
        Some(dependency.package_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("reason", JsonValue::string("unsupported_source")),
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

fn dependency_missing_manifest_diagnostic(
    dependency: &ManifestDependency,
    path_field: &ManifestField,
) -> Diagnostic {
    Diagnostic::new(
        "package.dependency_missing_manifest",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "path dependency `{}` has no veln.toml at `{}`",
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

fn dependency_io_diagnostic(
    dependency: &ManifestDependency,
    path_field: &ManifestField,
    error: std::io::Error,
) -> Diagnostic {
    Diagnostic::new(
        "package.dependency_read_failed",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "path dependency `{}` could not be read: {error}",
            dependency.package
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
