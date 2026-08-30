use super::*;

pub(super) fn missing_manifest_diagnostic() -> Diagnostic {
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

pub(super) fn dependency_in_context(
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

pub(super) fn field_in_context(field: ManifestField, manifest_display_path: &str) -> ManifestField {
    ManifestField {
        key_span: span_in_context(&field.key_span, manifest_display_path),
        value_span: span_in_context(&field.value_span, manifest_display_path),
        ..field
    }
}

pub(super) fn span_in_context(span: &SourceSpan, manifest_display_path: &str) -> SourceSpan {
    if manifest_display_path.is_empty() {
        return span.clone();
    }
    let mut span = span.clone();
    span.file = SourcePath::new(format!("{manifest_display_path}/{}", span.file.as_str()));
    span
}

pub(super) fn incompatible_dependency_source_diagnostic(
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

pub(super) fn selection_summary(selection: &DependencySelection) -> String {
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

pub(super) fn git_selector_summary(selector: &DependencyGitSelector) -> String {
    match selector {
        DependencyGitSelector::Rev(value) => format!("rev `{value}`"),
        DependencyGitSelector::Tag(value) => format!("tag `{value}`"),
        DependencyGitSelector::Branch(value) => format!("branch `{value}`"),
    }
}

pub(super) fn unsupported_dependency_source_diagnostic(
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

pub(super) fn unavailable_path_dependency_diagnostic(
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

pub(super) fn unavailable_mirror_dependency_diagnostic(
    dependency: &ManifestDependency,
    mirror_field: &ManifestField,
) -> Diagnostic {
    unavailable_local_dependency_diagnostic(dependency, mirror_field, "mirror")
}

pub(super) fn unavailable_git_dependency_diagnostic(
    dependency: &ManifestDependency,
    git_field: &ManifestField,
    reason: &str,
) -> Diagnostic {
    git_dependency_failure_diagnostic(
        dependency,
        git_field,
        reason,
        "package.git_unavailable",
        format!(
            "git dependency `{}` is not available at `{}`",
            dependency.package, git_field.value
        ),
    )
}

pub(super) fn unavailable_vendor_dependency_diagnostic(
    dependency: &ManifestDependency,
    vendor_field: &ManifestField,
) -> Diagnostic {
    unavailable_local_dependency_diagnostic(dependency, vendor_field, "vendor")
}

fn unavailable_local_dependency_diagnostic(
    dependency: &ManifestDependency,
    source_field: &ManifestField,
    source_kind: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        match source_kind {
            "mirror" => "package.mirror_unavailable",
            "vendor" => "package.vendor_unavailable",
            _ => unreachable!("unsupported local dependency source kind"),
        },
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "{source_kind} dependency `{}` is not available at `{}`",
            dependency.package, source_field.value
        ),
        Some(source_field.value_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            (
                "field",
                JsonValue::string(format!("dependencies.{source_kind}")),
            ),
            ("package", JsonValue::string(dependency.package.clone())),
            ("path", JsonValue::string(source_field.value.clone())),
        ]),
    )
}

pub(super) fn git_materialization_diagnostic(
    dependency: &ManifestDependency,
    git_field: &ManifestField,
    reason: &str,
) -> Diagnostic {
    git_dependency_failure_diagnostic(
        dependency,
        git_field,
        reason,
        "package.git_materialization_failed",
        format!(
            "git dependency `{}` could not be materialized from `{}`",
            dependency.package, git_field.value
        ),
    )
}

fn git_dependency_failure_diagnostic(
    dependency: &ManifestDependency,
    git_field: &ManifestField,
    reason: &str,
    id: &'static str,
    message: String,
) -> Diagnostic {
    Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Module,
        message,
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

pub(super) fn dependency_missing_manifest_diagnostic(
    dependency: &ManifestDependency,
    source_field: &ManifestField,
) -> Diagnostic {
    dependency_source_diagnostic(
        "package.dependency_missing_manifest",
        format!(
            "dependency `{}` has no veln.toml at `{}`",
            dependency.package, source_field.value
        ),
        dependency,
        source_field,
    )
}

pub(super) fn dependency_io_diagnostic(
    dependency: &ManifestDependency,
    source_field: &ManifestField,
    error: std::io::Error,
) -> Diagnostic {
    dependency_source_diagnostic(
        "package.dependency_read_failed",
        format!(
            "dependency `{}` could not be read: {error}",
            dependency.package
        ),
        dependency,
        source_field,
    )
}

fn dependency_source_diagnostic(
    id: &'static str,
    message: String,
    dependency: &ManifestDependency,
    source_field: &ManifestField,
) -> Diagnostic {
    Diagnostic::new(
        id,
        Severity::Error,
        DiagnosticKind::Module,
        message,
        Some(source_field.value_span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("package_lock")),
            ("field", JsonValue::string("dependencies")),
            ("package", JsonValue::string(dependency.package.clone())),
            ("source", JsonValue::string(source_field.value.clone())),
        ]),
    )
}

pub(super) fn unsupported_git_selector_diagnostic(
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

pub(super) fn invalid_git_subdir_diagnostic(
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

pub(super) fn git_selector_resolution_diagnostic(
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

pub(super) fn package_name_mismatch_diagnostic(
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
