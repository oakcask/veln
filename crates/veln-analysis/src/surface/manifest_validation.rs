use super::*;

pub fn validate_manifest_exports(project: &Project) -> Vec<Diagnostic> {
    validate_manifest_exports_with_checked_source_paths(project, &BTreeSet::new())
}

pub(super) fn validate_manifest_exports_with_checked_source_paths(
    project: &Project,
    checked_export_source_paths: &BTreeSet<String>,
) -> Vec<Diagnostic> {
    let Some(manifest) = project.manifest.as_ref() else {
        return Vec::new();
    };
    let mut diagnostics = Vec::new();
    for section in &manifest.unsupported_sections {
        if section.name == "modules" {
            diagnostics.push(unsupported_modules_section_diagnostic(section.span.clone()));
        }
    }

    let mut exported_modules = Vec::<(String, SourceSpan)>::new();
    for export in &manifest.lib.exports {
        let candidate = match validate_manifest_export_path(export) {
            Ok(candidate) => candidate,
            Err(diagnostic) => {
                diagnostics.push(*diagnostic);
                continue;
            }
        };
        let selected_source = match validate_manifest_export_selection(project, export, &candidate)
        {
            Ok(source) => source,
            Err(diagnostic) => {
                diagnostics.push(*diagnostic);
                continue;
            }
        };
        let module_name =
            match derive_visible_source_module_path_with_source_kind(selected_source, "export") {
                Ok(Some(module_name)) => module_name,
                Ok(None) => continue,
                Err(source_diagnostics) => {
                    if !checked_export_source_paths.contains(selected_source.path().as_str()) {
                        diagnostics.extend(source_diagnostics);
                    }
                    continue;
                }
            };
        if let Some((_, first_span)) = exported_modules
            .iter()
            .find(|(known_module, _)| known_module == &module_name)
        {
            diagnostics.push(duplicate_manifest_export_diagnostic(
                &export.path_span,
                &export.path,
                &module_name,
                first_span,
            ));
            continue;
        }
        exported_modules.push((module_name, export.path_span.clone()));
    }
    diagnostics
}

pub(super) fn manifest_export_source_paths(project: &Project) -> BTreeSet<String> {
    let Some(manifest) = project.manifest.as_ref() else {
        return BTreeSet::new();
    };
    manifest
        .lib
        .exports
        .iter()
        .filter_map(|export| validate_manifest_export_path(export).ok())
        .filter(|candidate| {
            project
                .files
                .iter()
                .any(|source| source.path().as_str() == candidate.path.as_str())
        })
        .map(|candidate| candidate.path.as_str().to_string())
        .collect()
}

pub(super) struct ManifestExportCandidate {
    pub(super) path: SourcePath,
}

pub(super) fn validate_manifest_export_path(
    export: &ManifestExport,
) -> Result<ManifestExportCandidate, Box<Diagnostic>> {
    if export.path.contains("::") {
        return Err(Box::new(invalid_manifest_export_path_diagnostic(
            &export.path_span,
            &export.path,
            "module paths are not valid manifest exports; use a package-relative source file path",
        )));
    }
    let path = SourcePath::new(export.path.clone());
    if !is_package_relative_path(path.as_str()) {
        return Err(Box::new(invalid_manifest_export_path_diagnostic(
            &export.path_span,
            &export.path,
            "manifest exports must stay inside the package",
        )));
    }
    if !path.as_str().ends_with(".veln") {
        return Err(Box::new(invalid_manifest_export_path_diagnostic(
            &export.path_span,
            &export.path,
            "manifest exports must name `.veln` source files",
        )));
    }
    if let Some(companion) = classify_companion_source(path.as_str()) {
        return Err(Box::new(companion_manifest_export_diagnostic(
            &export.path_span,
            &export.path,
            &companion.companion_path,
        )));
    }
    Ok(ManifestExportCandidate { path })
}

pub(super) fn validate_manifest_export_selection<'a>(
    project: &'a Project,
    export: &ManifestExport,
    candidate: &ManifestExportCandidate,
) -> Result<&'a SourceFile, Box<Diagnostic>> {
    if let Some(source) = project
        .files
        .iter()
        .find(|source| source.path() == &candidate.path)
    {
        return Ok(source);
    }
    if project.root.join(candidate.path.as_str()).is_file() {
        Err(Box::new(unselected_manifest_export_diagnostic(
            &export.path_span,
            &export.path,
        )))
    } else {
        Err(Box::new(missing_manifest_export_diagnostic(
            &export.path_span,
            &export.path,
        )))
    }
}

pub(super) fn validate_reserved_standard_package(
    project: &Project,
    toolchain_std: bool,
) -> Vec<Diagnostic> {
    let Some(manifest) = project.manifest.as_ref() else {
        return Vec::new();
    };
    let mut diagnostics = Vec::new();
    if !toolchain_std
        && let Some(name) = manifest_package_name(manifest)
        && name.value == veln_stdlib::PACKAGE_NAME
    {
        diagnostics.push(reserved_standard_package_diagnostic(
            name.value_span.clone(),
            "package.name",
            "package name `std` is reserved by the Veln toolchain",
            "Choose a different package name; the standard package is supplied by the toolchain.",
        ));
    }
    for dependency in &manifest.dependencies {
        if dependency.package == veln_stdlib::PACKAGE_NAME {
            diagnostics.push(reserved_standard_package_diagnostic(
                dependency.package_span.clone(),
                "dependencies",
                "dependency package `std` is reserved by the Veln toolchain",
                "Remove this dependency; the standard package is available implicitly and is not replaceable.",
            ));
        }
    }
    diagnostics
}

fn reserved_standard_package_diagnostic(
    span: SourceSpan,
    field: &'static str,
    message: &'static str,
    hint: &'static str,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "manifest.reserved_standard_package",
        Severity::Error,
        DiagnosticKind::Module,
        message,
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string(field)),
            ("package", JsonValue::string(veln_stdlib::PACKAGE_NAME)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("repair_hint")),
        ("message", JsonValue::string(hint)),
    ]));
    diagnostic
}

pub fn validate_manifest_dependencies(project: &Project) -> Vec<Diagnostic> {
    let Some(manifest) = project.manifest.as_ref() else {
        return Vec::new();
    };
    let mut diagnostics = Vec::new();
    for dependency in &manifest.dependencies {
        if dependency.git.is_some() && dependency.selectors.is_empty() {
            diagnostics.push(missing_git_dependency_selector_diagnostic(
                &dependency.package_span,
                &dependency.package,
            ));
        }
        if dependency.git.is_some() && dependency.selectors.len() > 1 {
            let selectors = dependency
                .selectors
                .iter()
                .map(|selector| selector.kind)
                .collect::<Vec<_>>();
            for selector in dependency.selectors.iter().skip(1) {
                diagnostics.push(multiple_git_dependency_selectors_diagnostic(
                    &selector.field.key_span,
                    &dependency.package,
                    &selectors,
                ));
            }
        }
    }
    diagnostics
}

fn unsupported_modules_section_diagnostic(span: SourceSpan) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "manifest.unsupported_section",
        Severity::Error,
        DiagnosticKind::Module,
        "`[modules]` is not supported; use `[lib].exports` for public source files",
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("manifest_section")),
            ("section", JsonValue::string("modules")),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string("Replace `[modules]` entries with `[lib].exports` file paths."),
    )]));
    diagnostic
}

fn invalid_manifest_export_path_diagnostic(
    span: &SourceSpan,
    path: &str,
    reason: &'static str,
) -> Diagnostic {
    Diagnostic::new(
        "manifest.invalid_export",
        Severity::Error,
        DiagnosticKind::Module,
        format!("manifest export `{path}` is invalid: {reason}"),
        Some(span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("lib.exports")),
            ("source_path", JsonValue::string(path)),
            ("reason", JsonValue::string(reason)),
        ]),
    )
}

fn companion_manifest_export_diagnostic(
    span: &SourceSpan,
    source_path: &str,
    companion_path: &str,
) -> Diagnostic {
    Diagnostic::new(
        "manifest.invalid_export",
        Severity::Error,
        DiagnosticKind::Module,
        format!("manifest export `{source_path}` is invalid: export names a test companion"),
        Some(span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("lib.exports")),
            ("source_path", JsonValue::string(source_path)),
            ("companion_path", JsonValue::string(companion_path)),
            ("reason", JsonValue::string("test_companion")),
        ]),
    )
}

fn unselected_manifest_export_diagnostic(span: &SourceSpan, path: &str) -> Diagnostic {
    Diagnostic::new(
        "manifest.unselected_export",
        Severity::Error,
        DiagnosticKind::Module,
        format!("manifest export `{path}` has no matching selected source file"),
        Some(span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("lib.exports")),
            ("source_path", JsonValue::string(path)),
        ]),
    )
}

fn missing_manifest_export_diagnostic(span: &SourceSpan, path: &str) -> Diagnostic {
    Diagnostic::new(
        "manifest.missing_export",
        Severity::Error,
        DiagnosticKind::Module,
        format!("manifest export `{path}` does not exist"),
        Some(span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("lib.exports")),
            ("source_path", JsonValue::string(path)),
        ]),
    )
}

fn duplicate_manifest_export_diagnostic(
    span: &SourceSpan,
    path: &str,
    module_name: &str,
    first_span: &SourceSpan,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "manifest.duplicate_export",
        Severity::Error,
        DiagnosticKind::Module,
        format!("manifest export `{path}` duplicates module export `{module_name}`"),
        Some(span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("lib.exports")),
            ("source_path", JsonValue::string(path)),
            ("module_path", JsonValue::string(module_name)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("duplicate_origin")),
        (
            "message",
            JsonValue::string(format!("The first export for `{module_name}` is here.")),
        ),
        ("span", source_span_json(first_span)),
    ]));
    diagnostic
}

fn missing_git_dependency_selector_diagnostic(span: &SourceSpan, package: &str) -> Diagnostic {
    Diagnostic::new(
        "manifest.missing_git_selector",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "git dependency `{package}` must specify exactly one selector: `rev`, `tag`, or `branch`"
        ),
        Some(span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("dependencies")),
            ("package", JsonValue::string(package)),
            ("source_kind", JsonValue::string("git")),
            ("reason", JsonValue::string("missing_selector")),
        ]),
    )
}

fn multiple_git_dependency_selectors_diagnostic(
    span: &SourceSpan,
    package: &str,
    selectors: &[ManifestDependencySelectorKind],
) -> Diagnostic {
    Diagnostic::new(
        "manifest.multiple_git_selectors",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "git dependency `{package}` specifies multiple selectors; use exactly one of `rev`, `tag`, or `branch`"
        ),
        Some(span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("dependencies")),
            ("package", JsonValue::string(package)),
            ("source_kind", JsonValue::string("git")),
            (
                "selectors",
                JsonValue::array(
                    selectors
                        .iter()
                        .map(|selector| JsonValue::string(selector.as_str())),
                ),
            ),
            ("reason", JsonValue::string("multiple_selectors")),
        ]),
    )
}

fn is_package_relative_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && !path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn source_span_json(span: &SourceSpan) -> JsonValue {
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
