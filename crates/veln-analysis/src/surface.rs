use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path};
use std::sync::OnceLock;

use veln_ast::{
    PublicAliasKind, SurfaceModule, UseDecl, Visibility, decode_surface_module, lower_surface_ast,
    lower_surface_ast_with_module_identity,
};
use veln_diagnostics::{Diagnostic, DiagnosticKind, JsonValue, Severity};
use veln_project::{
    ManifestDependencySelectorKind, ManifestExport, ManifestField, Project, ProjectManifest,
    classify_companion_source, read_manifest,
};
use veln_source::{SourceFile, SourcePath, SourceSpan, TextRange};
use veln_syntax::parse;

use crate::diagnostics::parse_diagnostic_to_envelope;

mod source_module_path;

pub use source_module_path::derive as derive_source_module_path;

#[cfg(test)]
pub(crate) mod embedded_standard_counters {
    use std::cell::RefCell;
    use std::collections::BTreeSet;

    #[derive(Debug, Default, PartialEq, Eq)]
    pub(crate) struct Snapshot {
        pub(crate) runtime_standard_parse_lowers: usize,
        pub(crate) materialized_modules: BTreeSet<String>,
        pub(crate) materialized_lowered_bytes: usize,
    }

    thread_local! {
        static OBSERVATION: RefCell<Snapshot> = RefCell::new(Snapshot::default());
    }

    pub(crate) fn observe<R>(action: impl FnOnce() -> R) -> (R, Snapshot) {
        OBSERVATION.with(|observation| {
            let previous = observation.replace(Snapshot::default());
            let result = action();
            let snapshot = observation.replace(previous);
            (result, snapshot)
        })
    }

    pub(super) fn record_runtime_standard_parse_lower() {
        OBSERVATION.with(|observation| {
            observation.borrow_mut().runtime_standard_parse_lowers += 1;
        });
    }

    pub(super) fn record_materialization(path: &str, lowered_bytes: usize) {
        OBSERVATION.with(|observation| {
            let mut observation = observation.borrow_mut();
            if observation.materialized_modules.insert(path.to_string()) {
                observation.materialized_lowered_bytes += lowered_bytes;
            }
        });
    }
}

#[derive(Clone)]
pub(crate) struct LoadedSurfaceModules {
    pub(crate) combined: SurfaceModule,
    pub(crate) application: SurfaceModule,
    pub(crate) selected_standard_module_names: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct CapturedDependencyProject {
    pub package: String,
    pub source: String,
    pub project: Option<Project>,
}

pub fn load_surface_module(project: &Project) -> (SurfaceModule, Vec<Diagnostic>) {
    let (modules, diagnostics) = load_surface_modules_with_combined(project, true, None);
    (modules.combined, diagnostics)
}

pub(crate) fn load_surface_modules(project: &Project) -> (LoadedSurfaceModules, Vec<Diagnostic>) {
    load_surface_modules_with_combined(project, false, None)
}

pub(crate) fn load_surface_modules_with_captured_dependencies(
    project: &Project,
    dependencies: &[CapturedDependencyProject],
) -> (LoadedSurfaceModules, Vec<Diagnostic>) {
    load_surface_modules_with_combined(project, false, Some(dependencies))
}

fn load_surface_modules_with_combined(
    project: &Project,
    include_combined: bool,
    captured_dependencies: Option<&[CapturedDependencyProject]>,
) -> (LoadedSurfaceModules, Vec<Diagnostic>) {
    let mut diagnostics = Vec::new();
    let mut parts = SurfaceParts::new();
    let toolchain_std = is_toolchain_standard_project(project);

    if toolchain_std {
        load_toolchain_standard_sources(project, &mut diagnostics, &mut parts);
    } else {
        load_project_sources(project, &mut diagnostics, &mut parts, None);
    }
    diagnostics.extend(validate_manifest_exports(project));
    diagnostics.extend(validate_manifest_dependencies(project));
    diagnostics.extend(validate_companion_sources(project));
    diagnostics.extend(validate_companion_public_declarations(&parts.module));
    diagnostics.extend(validate_reserved_standard_package(project, toolchain_std));
    load_external_dependencies(project, captured_dependencies, &mut diagnostics, &mut parts);
    rewrite_standard_import_targets(&mut parts.module.uses);
    add_implicit_standard_prelude_imports(&mut parts);
    let selected_standard = if toolchain_std {
        BTreeSet::new()
    } else {
        load_embedded_standard_package(&mut diagnostics, &mut parts, include_combined)
    };
    diagnostics.extend(unresolved_local_import_diagnostics(
        &parts.module,
        &parts.derived_modules,
    ));

    (
        loaded_surface_modules(parts.module, selected_standard, include_combined),
        diagnostics,
    )
}

fn loaded_surface_modules(
    module: SurfaceModule,
    selected_standard_module_names: BTreeSet<String>,
    include_combined: bool,
) -> LoadedSurfaceModules {
    if include_combined {
        return LoadedSurfaceModules {
            combined: module.clone(),
            application: module,
            selected_standard_module_names,
        };
    }
    LoadedSurfaceModules {
        combined: SurfaceParts::new().module,
        application: module,
        selected_standard_module_names,
    }
}

pub(crate) fn load_embedded_standard_surface_module_for_names(
    module_names: &BTreeSet<String>,
) -> SurfaceModule {
    let mut parts = SurfaceParts::new();
    let standard = embedded_standard_package();
    for module_name in module_names {
        let Some(module) = standard
            .modules
            .get(module_name)
            .map(EmbeddedStandardModuleEntry::module)
        else {
            continue;
        };
        merge_surface_parts(&mut parts, &module.parts);
    }
    parts.module
}

pub fn load_embedded_standard_surface_module() -> SurfaceModule {
    let standard = embedded_standard_package();
    let mut parts = SurfaceParts::new();
    for module in standard
        .modules
        .values()
        .map(EmbeddedStandardModuleEntry::module)
    {
        merge_surface_parts(&mut parts, &module.parts);
    }
    parts.module
}

#[derive(Clone)]
struct SurfaceParts {
    module: SurfaceModule,
    derived_modules: Vec<(String, SourceFile)>,
}

struct EmbeddedStandardPackage {
    modules: BTreeMap<String, EmbeddedStandardModuleEntry>,
}

struct EmbeddedStandardModuleEntry {
    path: String,
    lowered: Cow<'static, [u8]>,
    module: OnceLock<EmbeddedStandardModule>,
}

struct EmbeddedStandardModule {
    parts: SurfaceParts,
    diagnostics: Vec<Diagnostic>,
}

impl EmbeddedStandardModuleEntry {
    fn module(&self) -> &EmbeddedStandardModule {
        #[cfg(test)]
        embedded_standard_counters::record_materialization(&self.path, self.lowered.len());
        self.module.get_or_init(|| {
            let module = decode_surface_module(self.lowered.as_ref()).unwrap_or_else(|message| {
                panic!(
                    "embedded standard library lowered module `{}` should decode: {message}",
                    self.path
                )
            });
            EmbeddedStandardModule {
                parts: SurfaceParts {
                    module,
                    derived_modules: vec![(
                        embedded_standard_module_name_from_path(&self.path).unwrap_or_else(|| {
                            panic!(
                                "embedded standard library path `{}` should identify a module",
                                self.path
                            )
                        }),
                        SourceFile::new(self.path.as_str(), ""),
                    )],
                },
                diagnostics: Vec::new(),
            }
        })
    }
}

static EMBEDDED_STANDARD_PACKAGE: OnceLock<EmbeddedStandardPackage> = OnceLock::new();

impl SurfaceParts {
    fn new() -> Self {
        Self {
            module: SurfaceModule {
                module: None,
                uses: Vec::new(),
                aliases: Vec::new(),
                effects: Vec::new(),
                handlers: Vec::new(),
                types: Vec::new(),
                schemas: Vec::new(),
                codecs: Vec::new(),
                functions: Vec::new(),
                invalid_names: Vec::new(),
            },
            derived_modules: Vec::new(),
        }
    }
}

fn load_project_sources(
    project: &Project,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
    package: Option<&str>,
) {
    for source in &project.files {
        if package.is_some() && classify_companion_source(source.path().as_str()).is_some() {
            continue;
        }
        #[cfg(test)]
        if package == Some(veln_stdlib::PACKAGE_NAME) {
            embedded_standard_counters::record_runtime_standard_parse_lower();
        }
        let parsed = parse(source);
        diagnostics.extend(parsed.diagnostics.iter().map(parse_diagnostic_to_envelope));
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        process_parsed_source(source, &parsed.tree, diagnostics, parts, package);
    }
}

fn load_toolchain_standard_sources(
    project: &Project,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
) {
    let standard = embedded_standard_package();
    for module in standard
        .modules
        .values()
        .map(EmbeddedStandardModuleEntry::module)
    {
        merge_surface_parts(parts, &module.parts);
        diagnostics.extend(module.diagnostics.clone());
    }

    let test_project = Project {
        root: project.root.clone(),
        files: project
            .files
            .iter()
            .filter(|source| source.path().as_str().ends_with("_test.veln"))
            .cloned()
            .collect(),
        manifest: project.manifest.clone(),
    };
    load_project_sources(
        &test_project,
        diagnostics,
        parts,
        Some(veln_stdlib::PACKAGE_NAME),
    );
}

fn process_parsed_source(
    source: &SourceFile,
    tree: &veln_syntax::SyntaxTree,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
    package: Option<&str>,
) {
    push_source_parse_semantic_diagnostics(tree, diagnostics);
    let derived_module = derive_and_record_source_module(source, diagnostics, parts, package);
    let mut lowered = lower_source_tree(source, tree, derived_module, package);
    rewrite_import_targets(&mut lowered.uses, package);
    if parts.module.module.is_none() {
        parts.module.module = lowered.module;
    }
    parts.module.uses.extend(lowered.uses);
    parts.module.aliases.extend(lowered.aliases);
    parts.module.effects.extend(lowered.effects);
    parts.module.handlers.extend(lowered.handlers);
    parts.module.types.extend(lowered.types);
    parts.module.schemas.extend(lowered.schemas);
    parts.module.codecs.extend(lowered.codecs);
    parts.module.functions.extend(lowered.functions);
    parts.module.invalid_names.extend(lowered.invalid_names);
}

fn push_source_parse_semantic_diagnostics(
    tree: &veln_syntax::SyntaxTree,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(module) = &tree.module {
        diagnostics.push(source_mod_decl_diagnostic(module));
    }
    for use_decl in &tree.uses {
        if use_decl.name.contains('.') {
            diagnostics.push(dotted_use_decl_diagnostic(use_decl));
        }
    }
}

fn derive_and_record_source_module(
    source: &SourceFile,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
    package: Option<&str>,
) -> Option<String> {
    match derive_source_module_path(source) {
        Ok(module_name) => {
            record_derived_source_module(source, &module_name, diagnostics, parts, package);
            Some(module_name)
        }
        Err(diagnostic) => {
            diagnostics.push((*diagnostic).clone());
            None
        }
    }
}

fn record_derived_source_module(
    source: &SourceFile,
    module_name: &str,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
    package: Option<&str>,
) {
    let internal_module_name = internal_module_name(package, module_name);
    if module_name == "prelude" && package != Some(veln_stdlib::PACKAGE_NAME) {
        diagnostics.push(reserved_source_module_diagnostic(source, module_name));
    }
    if is_doctest_source(source) {
        return;
    }
    if let Some((_, first_source)) = parts
        .derived_modules
        .iter()
        .find(|(known_module, _)| known_module == &internal_module_name)
    {
        diagnostics.push(duplicate_derived_module_diagnostic(
            module_name,
            source,
            first_source,
        ));
    } else {
        parts
            .derived_modules
            .push((internal_module_name, source.clone()));
    }
}

fn lower_source_tree(
    source: &SourceFile,
    tree: &veln_syntax::SyntaxTree,
    derived_module: Option<String>,
    package: Option<&str>,
) -> SurfaceModule {
    match derived_module {
        Some(module_name) => {
            let internal_module_name = internal_module_name(package, &module_name);
            lower_surface_ast_with_module_identity(
                tree,
                internal_module_name,
                source.span(TextRange::new(0, 0)),
            )
        }
        None => lower_surface_ast(tree),
    }
}

fn rewrite_import_targets(uses: &mut [UseDecl], package: Option<&str>) {
    for use_decl in uses {
        if let Some(package) = &use_decl.package {
            use_decl.name = external_module_key(package, &use_decl.name);
        } else if let Some(package) = package {
            use_decl.name = external_module_key(package, &use_decl.name);
        }
    }
}

fn rewrite_standard_import_targets(uses: &mut [UseDecl]) {
    for use_decl in uses {
        if use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME)
            && !use_decl.name.starts_with("std::")
        {
            use_decl.name = external_module_key(veln_stdlib::PACKAGE_NAME, &use_decl.name);
        }
    }
}

fn internal_module_name(package: Option<&str>, module_name: &str) -> String {
    package.map_or_else(
        || module_name.to_string(),
        |package| external_module_key(package, module_name),
    )
}

fn external_module_key(package: &str, module_name: &str) -> String {
    format!("{package}::{module_name}")
}

fn load_external_dependencies(
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

fn load_embedded_standard_package(
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
    merge_into_parts: bool,
) -> BTreeSet<String> {
    let standard = embedded_standard_package();
    load_embedded_standard_package_from(standard, diagnostics, parts, merge_into_parts)
}

fn load_embedded_standard_package_from(
    standard: &EmbeddedStandardPackage,
    diagnostics: &mut Vec<Diagnostic>,
    parts: &mut SurfaceParts,
    merge_into_parts: bool,
) -> BTreeSet<String> {
    let mut pending = vec![external_module_key(veln_stdlib::PACKAGE_NAME, "prelude")];
    pending.extend(
        parts
            .module
            .uses
            .iter()
            .filter(|use_decl| use_decl.package.as_deref() == Some(veln_stdlib::PACKAGE_NAME))
            .map(|use_decl| {
                external_module_key(
                    veln_stdlib::PACKAGE_NAME,
                    &external_import_module_path(use_decl),
                )
            }),
    );
    let mut loaded = BTreeSet::new();
    while let Some(module_name) = pending.pop() {
        if !loaded.insert(module_name.clone()) {
            continue;
        }
        let Some(module) = standard
            .modules
            .get(&module_name)
            .map(EmbeddedStandardModuleEntry::module)
        else {
            continue;
        };
        pending.extend(
            module
                .parts
                .module
                .uses
                .iter()
                .map(|use_decl| use_decl.name.clone()),
        );
        diagnostics.extend(module.diagnostics.clone());
        if merge_into_parts {
            merge_surface_parts(parts, &module.parts);
        }
    }
    loaded
}

fn embedded_standard_package() -> &'static EmbeddedStandardPackage {
    EMBEDDED_STANDARD_PACKAGE.get_or_init(|| {
        let bundle = veln_stdlib::package_bundle();
        let modules = bundle
            .lowered_files
            .iter()
            .filter_map(|file| {
                embedded_standard_module_name_from_path(file.path).map(|module_name| {
                    (
                        module_name,
                        EmbeddedStandardModuleEntry {
                            path: file.path.to_string(),
                            lowered: Cow::Borrowed(file.module),
                            module: OnceLock::new(),
                        },
                    )
                })
            })
            .collect();
        EmbeddedStandardPackage { modules }
    })
}

fn embedded_standard_module_name_from_path(path: &str) -> Option<String> {
    if classify_companion_source(path).is_some() {
        return None;
    }
    path.strip_suffix(".veln").map(|module_name| {
        external_module_key(veln_stdlib::PACKAGE_NAME, &module_name.replace('/', "::"))
    })
}

fn merge_surface_parts(parts: &mut SurfaceParts, additions: &SurfaceParts) {
    if parts.module.module.is_none() {
        parts.module.module = additions.module.module.clone();
    }
    parts.module.uses.extend(additions.module.uses.clone());
    parts
        .module
        .aliases
        .extend(additions.module.aliases.clone());
    parts
        .module
        .effects
        .extend(additions.module.effects.clone());
    parts
        .module
        .handlers
        .extend(additions.module.handlers.clone());
    parts.module.types.extend(additions.module.types.clone());
    parts
        .module
        .schemas
        .extend(additions.module.schemas.clone());
    parts.module.codecs.extend(additions.module.codecs.clone());
    parts
        .module
        .functions
        .extend(additions.module.functions.clone());
    parts
        .module
        .invalid_names
        .extend(additions.module.invalid_names.clone());
    parts
        .derived_modules
        .extend(additions.derived_modules.clone());
}

fn add_implicit_standard_prelude_imports(parts: &mut SurfaceParts) {
    let modules = parts
        .derived_modules
        .iter()
        .filter(|(module, _)| !module.starts_with("std::"))
        .map(|(module, source)| (module.clone(), source.span(TextRange::new(0, 0))))
        .collect::<Vec<_>>();
    parts.module.uses.extend(
        modules
            .into_iter()
            .map(|(module, span)| UseDecl::implicit_standard_prelude(module, span)),
    );
}

fn is_toolchain_standard_project(project: &Project) -> bool {
    let Some(manifest) = &project.manifest else {
        return false;
    };
    let bundle = veln_stdlib::package_bundle();
    if manifest_package_name(manifest).map(|field| field.value.as_str())
        != Some(veln_stdlib::PACKAGE_NAME)
        || manifest.package.fields.len() != 1
        || manifest.lib.exports.len() != bundle.exports.len()
        || !manifest
            .lib
            .exports
            .iter()
            .map(|export| export.path.as_str())
            .eq(bundle.exports.iter().copied())
        || !manifest.dependencies.is_empty()
        || !manifest.unsupported_sections.is_empty()
        || !manifest.tools.is_empty()
    {
        return false;
    }
    let mut actual = project
        .files
        .iter()
        .filter(|source| {
            !source.path().as_str().ends_with("_test.veln")
                && classify_companion_source(source.path().as_str()).is_none()
        })
        .map(|source| (source.path().as_str(), source.text()))
        .collect::<Vec<_>>();
    let mut expected = bundle
        .files
        .iter()
        .map(|file| (file.path, file.text))
        .collect::<Vec<_>>();
    actual.sort_by_key(|(path, _)| *path);
    expected.sort_by_key(|(path, _)| *path);
    actual == expected
}

fn validate_standard_package_import(use_decl: &UseDecl, diagnostics: &mut Vec<Diagnostic>) {
    let module_path = external_import_module_path(use_decl);
    if !veln_stdlib::package_bundle().exports.iter().any(|export| {
        derive_source_module_path(&SourceFile::new(*export, ""))
            .is_ok_and(|module| module == module_path)
    }) {
        diagnostics.push(unexported_external_module_diagnostic(use_decl));
    }
}

fn load_external_dependency_package(
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

    diagnostics.extend(validate_manifest_exports(&dependency_project));
    let exported_modules = manifest_exported_modules(dependency_manifest);
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
    load_project_sources(&dependency_project, diagnostics, parts, Some(package));
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

fn manifest_package_name(manifest: &ProjectManifest) -> Option<&ManifestField> {
    manifest
        .package
        .fields
        .iter()
        .find(|field| field.key == "name")
}

fn manifest_exported_modules(manifest: &ProjectManifest) -> Vec<String> {
    manifest
        .lib
        .exports
        .iter()
        .filter_map(|export| {
            if export.path.contains("::") {
                return None;
            }
            let normalized_path = SourcePath::new(export.path.clone());
            let path = normalized_path.as_str();
            if !is_package_relative_path(path) || !path.ends_with(".veln") {
                return None;
            }
            if classify_companion_source(path).is_some() {
                return None;
            }
            derive_source_module_path(&SourceFile::new(path, "")).ok()
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

fn external_import_module_path(use_decl: &UseDecl) -> String {
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

fn is_doctest_source(source: &SourceFile) -> bool {
    source.path().as_str().contains("#doctest-")
}

fn validate_companion_sources(project: &Project) -> Vec<Diagnostic> {
    let source_paths = project
        .files
        .iter()
        .map(|source| source.path().as_str().to_string())
        .collect::<BTreeSet<_>>();
    project
        .files
        .iter()
        .filter_map(|source| {
            let companion = classify_companion_source(source.path().as_str())?;
            if companion.chained {
                Some(chained_companion_diagnostic(source, &companion.target_path))
            } else if !source_paths.contains(&companion.target_path) {
                Some(missing_companion_target_diagnostic(
                    source,
                    &companion.target_path,
                ))
            } else {
                None
            }
        })
        .collect()
}

fn validate_companion_public_declarations(module: &SurfaceModule) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    diagnostics.extend(module.functions.iter().filter_map(|function| {
        public_companion_declaration(
            &function.visibility,
            &function.span,
            "public_function",
            "function",
            function.name.as_deref(),
        )
    }));
    diagnostics.extend(module.effects.iter().filter_map(|effect| {
        public_companion_declaration(
            &effect.visibility,
            &effect.span,
            "public_effect",
            "effect",
            effect.name.as_deref(),
        )
    }));
    diagnostics.extend(module.handlers.iter().filter_map(|handler| {
        public_companion_declaration(
            &handler.visibility,
            &handler.span,
            "public_handler",
            "handler",
            handler.name.as_deref(),
        )
    }));
    for ty in &module.types {
        diagnostics.extend(public_companion_declaration(
            &ty.visibility,
            &ty.span,
            "public_type",
            "type",
            ty.name.as_deref(),
        ));
        diagnostics.extend(ty.variants.iter().filter_map(|variant| {
            public_companion_declaration(
                &variant.visibility,
                &variant.span,
                "public_type_variant",
                "type variant",
                variant.name.as_deref(),
            )
        }));
    }
    diagnostics.extend(module.schemas.iter().filter_map(|schema| {
        public_companion_declaration(
            &schema.visibility,
            &schema.span,
            "public_schema",
            "schema",
            schema.name.as_deref(),
        )
    }));
    diagnostics.extend(module.aliases.iter().filter_map(|alias| {
        let (reason, declaration_kind) = alias_companion_public_reason(alias.kind);
        companion_path_for_span(&alias.span).map(|companion_path| {
            companion_public_declaration_diagnostic(
                alias.span.clone(),
                companion_path,
                reason,
                declaration_kind,
                alias.name.as_deref(),
            )
        })
    }));
    diagnostics
}

fn public_companion_declaration(
    visibility: &Visibility,
    span: &SourceSpan,
    reason: &'static str,
    declaration_kind: &'static str,
    name: Option<&str>,
) -> Option<Diagnostic> {
    if *visibility == Visibility::Public {
        companion_path_for_span(span).map(|companion_path| {
            companion_public_declaration_diagnostic(
                span.clone(),
                companion_path,
                reason,
                declaration_kind,
                name,
            )
        })
    } else {
        None
    }
}

fn alias_companion_public_reason(kind: PublicAliasKind) -> (&'static str, &'static str) {
    match kind {
        PublicAliasKind::Function => ("public_function_alias", "function alias"),
        PublicAliasKind::Type => ("public_type_alias", "type alias"),
        PublicAliasKind::Schema => ("public_schema_alias", "schema alias"),
    }
}

fn companion_path_for_span(span: &SourceSpan) -> Option<&str> {
    let path = span.file.as_str();
    classify_companion_source(path).map(|_| path)
}

fn companion_public_declaration_diagnostic(
    span: SourceSpan,
    companion_path: &str,
    reason: &'static str,
    declaration_kind: &'static str,
    declaration_name: Option<&str>,
) -> Diagnostic {
    let described_declaration = declaration_name.map_or_else(
        || format!("public {declaration_kind}"),
        |name| format!("public {declaration_kind} `{name}`"),
    );
    let mut diagnostic = Diagnostic::new(
        "module.companion_public_declaration",
        Severity::Error,
        DiagnosticKind::Module,
        format!("test companion `{companion_path}` cannot declare {described_declaration}"),
        Some(span),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("companion_public_declaration")),
            ("companion_path", JsonValue::string(companion_path)),
            ("reason", JsonValue::string(reason)),
            ("declaration_kind", JsonValue::string(declaration_kind)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string("Remove `pub`; test companion declarations are not externally visible."),
    )]));
    diagnostic
}

fn missing_companion_target_diagnostic(source: &SourceFile, target_path: &str) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.companion_missing_target",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "test companion `{}` has no matching target `{target_path}`",
            source.path().as_str()
        ),
        Some(source.span(TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("companion_target")),
            ("companion_path", JsonValue::string(source.path().as_str())),
            ("target_path", JsonValue::string(target_path)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string("Create the target source beside the companion or rename the companion."),
    )]));
    diagnostic
}

fn chained_companion_diagnostic(source: &SourceFile, target_path: &str) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.chained_companion",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "test companion `{}` cannot target another companion `{target_path}`",
            source.path().as_str()
        ),
        Some(source.span(TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("companion_target")),
            ("companion_path", JsonValue::string(source.path().as_str())),
            ("target_path", JsonValue::string(target_path)),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string("Use exactly one `.test.veln` suffix for a test companion."),
    )]));
    diagnostic
}

fn source_mod_decl_diagnostic(module: &veln_syntax::ModuleDecl) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.source_mod",
        Severity::Error,
        DiagnosticKind::Module,
        "source `mod` declarations are not supported",
        Some(module.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("module_path", JsonValue::string(module.name.clone())),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string(
            "Move or rename the source file so its package-relative path derives the intended module path.",
        ),
    )]));
    diagnostic
}

fn dotted_use_decl_diagnostic(use_decl: &veln_syntax::UseDecl) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.invalid_import_path",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "module import `{}` uses `.`; source module paths use `::`",
            use_decl.name
        ),
        Some(use_decl.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("import_path")),
            ("module_path", JsonValue::string(use_decl.name.clone())),
            ("expected_delimiter", JsonValue::string("::")),
            ("observed_delimiter", JsonValue::string(".")),
        ]),
    );
    diagnostic.related.push(JsonValue::object([(
        "message",
        JsonValue::string("Rewrite the import with `::` between module path segments."),
    )]));
    diagnostic
}

fn duplicate_derived_module_diagnostic(
    module_name: &str,
    source: &SourceFile,
    first_source: &SourceFile,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.duplicate_source_path",
        Severity::Error,
        DiagnosticKind::Module,
        format!("multiple source files derive module path `{module_name}`"),
        Some(source.span(veln_source::TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("module_identity")),
            ("module_path", JsonValue::string(module_name)),
            ("source_path", JsonValue::string(source.path().as_str())),
        ]),
    );
    diagnostic.related.push(JsonValue::object([
        ("kind", JsonValue::string("duplicate_origin")),
        (
            "message",
            JsonValue::string(format!(
                "The first source file deriving `{module_name}` is here."
            )),
        ),
        (
            "span",
            JsonValue::object([
                ("file", JsonValue::string(first_source.path().as_str())),
                (
                    "start",
                    JsonValue::object([
                        ("line", JsonValue::Number(1)),
                        ("column", JsonValue::Number(1)),
                        ("offset", JsonValue::Number(0)),
                    ]),
                ),
                (
                    "end",
                    JsonValue::object([
                        ("line", JsonValue::Number(1)),
                        ("column", JsonValue::Number(1)),
                        ("offset", JsonValue::Number(0)),
                    ]),
                ),
            ]),
        ),
    ]));
    diagnostic
}

fn reserved_source_module_diagnostic(source: &SourceFile, module_name: &str) -> Diagnostic {
    Diagnostic::new(
        "name.reserved",
        Severity::Error,
        DiagnosticKind::Name,
        format!("module identity `{module_name}` conflicts with the standard prelude"),
        Some(source.span(TextRange::new(0, 0))),
        JsonValue::object([
            ("phase", JsonValue::string("name")),
            ("node_id", JsonValue::Null),
            ("name", JsonValue::string(module_name)),
            ("namespace", JsonValue::string("module")),
            ("reserved_for", JsonValue::string("standard_prelude")),
        ]),
    )
}

pub fn validate_manifest_exports(project: &Project) -> Vec<Diagnostic> {
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
        if let Err(diagnostic) = validate_manifest_export_selection(project, export, &candidate) {
            diagnostics.push(*diagnostic);
            continue;
        }
        if let Some((_, first_span)) = exported_modules
            .iter()
            .find(|(known_module, _)| known_module == &candidate.module_name)
        {
            diagnostics.push(duplicate_manifest_export_diagnostic(
                &export.path_span,
                &export.path,
                &candidate.module_name,
                first_span,
            ));
            continue;
        }
        exported_modules.push((candidate.module_name, export.path_span.clone()));
    }
    diagnostics
}

struct ManifestExportCandidate {
    path: SourcePath,
    module_name: String,
}

fn validate_manifest_export_path(
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
    let export_source = SourceFile::new(path.as_str(), "");
    let module_name = derive_source_module_path(&export_source).map_err(|_| {
        Box::new(invalid_manifest_export_path_diagnostic(
            &export.path_span,
            &export.path,
            "manifest export path does not derive a valid module path",
        ))
    })?;
    Ok(ManifestExportCandidate { path, module_name })
}

fn validate_manifest_export_selection(
    project: &Project,
    export: &ManifestExport,
    candidate: &ManifestExportCandidate,
) -> Result<(), Box<Diagnostic>> {
    if project
        .files
        .iter()
        .any(|source| source.path() == &candidate.path)
    {
        return Ok(());
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

fn validate_reserved_standard_package(project: &Project, toolchain_std: bool) -> Vec<Diagnostic> {
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

fn unresolved_local_import_diagnostics(
    module: &SurfaceModule,
    derived_modules: &[(String, SourceFile)],
) -> Vec<Diagnostic> {
    module
        .uses
        .iter()
        .filter(|use_decl| {
            use_decl.package.is_none()
                && use_decl.name.contains("::")
                && !derived_modules
                    .iter()
                    .any(|(module_name, _)| module_name == &use_decl.name)
        })
        .map(|use_decl| unresolved_local_import_diagnostic(use_decl, derived_modules))
        .collect()
}

fn unresolved_local_import_diagnostic(
    use_decl: &UseDecl,
    derived_modules: &[(String, SourceFile)],
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(
        "module.unresolved_import",
        Severity::Error,
        DiagnosticKind::Module,
        format!(
            "local import `{}` has no matching selected source file",
            use_decl.name
        ),
        Some(use_decl.span.clone()),
        JsonValue::object([
            ("phase", JsonValue::string("module")),
            ("field", JsonValue::string("import_path")),
            ("module_path", JsonValue::string(use_decl.name.clone())),
        ]),
    );
    for (module_name, source) in derived_modules {
        diagnostic.related.push(JsonValue::object([
            ("kind", JsonValue::string("selected_source_module")),
            (
                "message",
                JsonValue::string(format!(
                    "Selected source `{}` derives `{module_name}`.",
                    source.path().as_str()
                )),
            ),
        ]));
    }
    diagnostic
}

mod reachability;

pub(crate) use reachability::{ReachabilityCache, reachable_entry_module_with_standard_cache};
#[cfg(test)]
pub(crate) use reachability::{
    reachability_counters, reachable_entry_module, reachable_entry_module_with_cache,
};

#[cfg(test)]
mod tests;
