use super::*;

pub(super) fn load_external_dependencies(
    project: &Project,
    captured_dependencies: Option<&[CapturedDependencyProject]>,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
) {
    let external_uses = parts
        .module
        .uses
        .iter()
        .filter(|use_decl| use_decl.package.is_some())
        .cloned()
        .collect::<Vec<_>>();
    if external_uses.is_empty() {
        return;
    }

    let mut loaded = BTreeSet::new();
    for use_decl in external_uses {
        let package = use_decl.package.as_deref().unwrap_or_default();
        if package == veln_stdlib::PACKAGE_NAME {
            validate_standard_package_import(&use_decl, diagnostics);
            continue;
        }
        if loaded.contains(package) {
            continue;
        }
        loaded.insert(package.to_string());
        load_external_dependency_package(
            project,
            captured_dependencies,
            package,
            &use_decl,
            diagnostics,
            parts,
        );
    }
}

pub(super) fn validate_standard_package_import(
    use_decl: &UseDecl,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let module_path = external_import_module_path(use_decl);
    if !veln_stdlib::package_bundle().exports.iter().any(|export| {
        derive_source_module_path(&SourceFile::new(*export, ""))
            .is_ok_and(|module| module == module_path)
    }) {
        diagnostics.push(unexported_external_module_diagnostic(use_decl));
    }
}

pub(super) fn load_external_dependency_package(
    project: &Project,
    captured_dependencies: Option<&[CapturedDependencyProject]>,
    package: &str,
    use_decl: &UseDecl,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
) {
    let Some((dependency_project, dependency)) = load_external_dependency_project(
        project,
        captured_dependencies,
        package,
        use_decl,
        diagnostics,
    ) else {
        return;
    };
    let dependency_manifest = dependency_project
        .manifest
        .as_ref()
        .expect("load_external_dependency_project only returns projects with manifests");
    if !dependency_package_name_matches(package, dependency_manifest, diagnostics, dependency) {
        return;
    }

    let exported_source_paths = manifest_export_source_paths(&dependency_project);
    let mut checked_export_source_paths = BTreeSet::new();
    let mut dependency_parts = SurfaceParts::new();
    load_project_sources(
        &dependency_project,
        diagnostics,
        &mut dependency_parts,
        Some(package),
        Some(&exported_source_paths),
        Some(&mut checked_export_source_paths),
    );
    diagnostics.extend(validate_manifest_exports_with_checked_source_paths(
        &dependency_project,
        &checked_export_source_paths,
    ));
    let exported_modules = manifest_exported_modules(&dependency_project);
    for external_use in parts
        .module
        .uses
        .iter()
        .filter(|candidate| candidate.package.as_deref() == Some(package))
    {
        if !exported_modules
            .iter()
            .any(|module_name| module_name == &external_import_module_path(external_use))
        {
            diagnostics.push(unexported_external_module_diagnostic(external_use));
        }
    }
    merge_surface_parts(parts, &dependency_parts);
}

fn load_external_dependency_project<'a>(
    project: &'a Project,
    captured_dependencies: Option<&[CapturedDependencyProject]>,
    package: &str,
    use_decl: &UseDecl,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(Project, &'a veln_project::ManifestDependency)> {
    let Some(manifest) = project.manifest.as_ref() else {
        diagnostics.push(unavailable_external_package_diagnostic(use_decl));
        return None;
    };
    let Some(dependency) = manifest
        .dependencies
        .iter()
        .find(|dependency| dependency.package == package)
    else {
        diagnostics.push(unavailable_external_package_diagnostic(use_decl));
        return None;
    };
    if let Some(captured) = captured_dependencies
        .and_then(|dependencies| captured_dependency_project(dependencies, dependency))
    {
        let Some(dependency_project) = &captured.project else {
            diagnostics.push(unavailable_external_package_diagnostic(use_decl));
            return None;
        };
        if dependency_project.manifest.is_none() {
            diagnostics.push(package_name_mismatch_diagnostic(
                package,
                None,
                &dependency.package_span,
            ));
            return None;
        }
        return Some((dependency_project.clone(), dependency));
    }
    if captured_dependencies.is_some() {
        diagnostics.push(unavailable_external_package_diagnostic(use_decl));
        return None;
    }
    let Some(dependency_root) = dependency
        .direct_analysis_source_root(&project.root)
        .ok()
        .flatten()
    else {
        diagnostics.push(unavailable_external_package_diagnostic(use_decl));
        return None;
    };

    let has_direct_manifest = match read_manifest(&dependency_root) {
        Ok(manifest) => manifest.is_some(),
        Err(_) => {
            diagnostics.push(unavailable_external_package_diagnostic(use_decl));
            return None;
        }
    };
    if !has_direct_manifest {
        diagnostics.push(package_name_mismatch_diagnostic(
            package,
            None,
            &dependency.package_span,
        ));
        return None;
    }

    let dependency_project = match Project::discover(dependency_root, &[]) {
        Ok(project) => project,
        Err(_) => {
            diagnostics.push(unavailable_external_package_diagnostic(use_decl));
            return None;
        }
    };

    if dependency_project.manifest.is_none() {
        diagnostics.push(package_name_mismatch_diagnostic(
            package,
            None,
            &dependency.package_span,
        ));
        return None;
    }
    Some((dependency_project, dependency))
}

fn captured_dependency_project<'a>(
    dependencies: &'a [CapturedDependencyProject],
    dependency: &veln_project::ManifestDependency,
) -> Option<&'a CapturedDependencyProject> {
    let source = captured_dependency_source(dependency)?;
    dependencies
        .iter()
        .find(|captured| captured.package == dependency.package && captured.source == source)
}

fn captured_dependency_source(dependency: &veln_project::ManifestDependency) -> Option<String> {
    if let Some(source) = dependency.direct_local_source() {
        return Some(source.value.clone());
    }
    let git = dependency.git.as_ref()?;
    if let Some(subdir) = &dependency.subdir {
        return Some(format!("{}#{}", git.value, subdir.value));
    }
    Some(git.value.clone())
}

fn dependency_package_name_matches(
    expected: &str,
    manifest: &ProjectManifest,
    diagnostics: &mut Vec<Diagnostic>,
    dependency: &veln_project::ManifestDependency,
) -> bool {
    let Some(name_field) = manifest_package_name(manifest) else {
        diagnostics.push(package_name_mismatch_diagnostic(
            expected,
            None,
            &dependency.package_span,
        ));
        return false;
    };
    if name_field.value != expected {
        diagnostics.push(package_name_mismatch_diagnostic(
            expected,
            Some(name_field),
            &name_field.value_span,
        ));
        return false;
    }
    true
}

pub(super) fn manifest_package_name(manifest: &ProjectManifest) -> Option<&ManifestField> {
    manifest
        .package
        .fields
        .iter()
        .find(|field| field.key == "name")
}

fn manifest_exported_modules(project: &Project) -> Vec<String> {
    let Some(manifest) = project.manifest.as_ref() else {
        return Vec::new();
    };
    manifest
        .lib
        .exports
        .iter()
        .filter_map(|export| {
            let candidate = validate_manifest_export_path(export).ok()?;
            let source = validate_manifest_export_selection(project, export, &candidate).ok()?;
            derive_visible_source_module_path_with_source_kind(source, "export")
                .ok()
                .flatten()
        })
        .collect()
}

fn unavailable_external_package_diagnostic(use_decl: &UseDecl) -> Diagnostic {
    let package = use_decl.package.as_deref().unwrap_or_default();
    let span = use_decl
        .package_span
        .clone()
        .unwrap_or_else(|| use_decl.span.clone());
    Diagnostic::new(
        "module.unavailable_package",
        Severity::Error,
        DiagnosticKind::Module,
        format!("external package `{package}` is not available to this project"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("import_package")),
            ("package", JsonValue::string(package)),
            (
                "module_path",
                JsonValue::string(external_import_module_path(use_decl)),
            ),
        ]),
    )
}

fn unexported_external_module_diagnostic(use_decl: &UseDecl) -> Diagnostic {
    let package = use_decl.package.as_deref().unwrap_or_default();
    let module_path = external_import_module_path(use_decl);
    Diagnostic::new(
        "module.unexported_import",
        Severity::Error,
        DiagnosticKind::Module,
        format!("external module `{module_path}` is not exported by package `{package}`"),
        Some(use_decl.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("lib.exports")),
            ("package", JsonValue::string(package)),
            ("module_path", JsonValue::string(module_path)),
        ]),
    )
}

pub(super) fn external_import_module_path(use_decl: &UseDecl) -> String {
    use_decl.alias.clone()
}

fn package_name_mismatch_diagnostic(
    expected: &str,
    actual: Option<&ManifestField>,
    span: &SourceSpan,
) -> Diagnostic {
    let actual_name = actual
        .map(|field| field.value.as_str())
        .unwrap_or("<missing>");
    Diagnostic::new(
        "manifest.package_name_mismatch",
        Severity::Error,
        DiagnosticKind::Module,
        format!("dependency package name `{actual_name}` does not match `{expected}`"),
        Some(span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("package.name")),
            ("expected_package", JsonValue::string(expected)),
            ("actual_package", JsonValue::string(actual_name)),
        ]),
    )
}

pub(super) fn is_doctest_source(source: &SourceFile) -> bool {
    source.path().as_str().contains("#doctest-")
}
